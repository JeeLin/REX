//! Agent HTTP server — static assets + local `/api/health` + edge reverse proxy.
//!
//! Leg 1 of the v0.89 Alt 1 design: browser traffic that is same-origin to this
//! Agent is dialed outbound straight to `REX_HUB_URL`:
//!
//! - `/api/{*path}` (except `/api/health`) → reqwest streaming proxy
//! - `/ws/{*path}` → tokio-tungstenite upgrade with message level relay
//! - `/ws/agent` → the Hub⇄Agent tunnel is dialed outbound, never served here
//! - anything else → embedded frontend (the SPA fallback never swallows `/api`/`/ws`)
//!
//! See `docs/architecture/connection-channels.md` ("Agent Web 访问通道").

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::extract::connect_info::ConnectInfo;
use axum::extract::ws::{Message as ClientMessage, WebSocket};
use axum::extract::{Request, State, WebSocketUpgrade};
use axum::http::header::{CONNECTION, HOST, ORIGIN, REFERER, SEC_WEBSOCKET_PROTOCOL};
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use futures_util::{SinkExt, StreamExt};
use include_dir::{include_dir, Dir};
use rex_common::embedded_static::EmbeddedStatic;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::handshake::client::Request as WsRequest;
use tokio_tungstenite::tungstenite::protocol::CloseFrame as HubCloseFrame;
use tokio_tungstenite::tungstenite::Message as HubMessage;
use tokio_tungstenite::{
    connect_async, connect_async_tls_with_config, Connector, MaybeTlsStream, WebSocketStream,
};
use url::Url;

use crate::agent_ws::{
    apply_hub_tls_to_reqwest, hub_origin, resolve_hub_tls_settings, HubTlsSettings,
};

/// 嵌入的前端 dist 目录
static DIST: Dir = include_dir!("$CARGO_MANIFEST_DIR/../../packages/rex-console-web/dist");

/// Hop-by-hop headers (RFC 9110 §7.6.1) that must never cross one proxy hop.
///
/// `Upgrade` belongs to this list too: the HTTP leg never proxies upgrades
/// (upgrade requests are routed to `/ws/{*path}` instead).
const HOP_BY_HOP: [&str; 9] = [
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "proxy-connection",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
];

/// Only connect timeouts are allowed: a total timeout would kill SSE, large
/// downloads and any other long lived response body.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

// ═══════════════════════════════════════
// Shared proxy state
// ═══════════════════════════════════════

struct AgentState {
    /// `http(s)://hub…` base used by the `/api` leg (path prefix preserved).
    hub_http_base: Url,
    /// `ws(s)://hub…` base used by the `/ws` leg (path prefix preserved).
    hub_ws_base: Url,
    /// `scheme://host[:port]` — written into `Origin`/`Referer` so the Hub CSRF
    /// check (`Origin` host must equal `Host`) passes from LAN IPs.
    hub_origin: HeaderValue,
    /// `host[:port]` — written into `Host`.
    hub_host: HeaderValue,
    /// Streaming HTTP client. Connect timeout only, no total timeout.
    client: reqwest::Client,
    /// Trust mode for the upstream `wss` leg (`None` = tungstenite system roots).
    wss_config: Option<Arc<rustls::ClientConfig>>,
}

impl AgentState {
    fn new(hub_url: &str, tls: &HubTlsSettings) -> anyhow::Result<Self> {
        let (http_base, ws_base) = hub_origin(hub_url).map_err(anyhow::Error::msg)?;
        let hub_http_base = Url::parse(&http_base)?;
        let hub_ws_base = Url::parse(&ws_base)?;

        let authority = authority_of(&hub_http_base);
        let origin = format!("{}://{}", hub_http_base.scheme(), authority);
        let hub_origin = HeaderValue::from_str(&origin)
            .map_err(|e| anyhow::anyhow!("invalid hub origin {origin:?}: {e}"))?;
        let hub_host = HeaderValue::from_str(&authority)
            .map_err(|e| anyhow::anyhow!("invalid hub authority {authority:?}: {e}"))?;

        Ok(Self {
            hub_http_base,
            hub_ws_base,
            hub_origin,
            hub_host,
            client: build_http_client(tls)?,
            wss_config: hub_rustls_config(tls).map_err(anyhow::Error::msg)?,
        })
    }

    /// Resolve the upstream URL for one proxied request.
    fn target(&self, base: &Url, path: &str, query: Option<&str>) -> Result<Url, StatusCode> {
        let mut url = base.clone();
        let mut merged = url.path().trim_end_matches('/').to_string();
        merged.push_str(path);
        url.set_path(&merged);
        url.set_query(query);
        Ok(url)
    }
}

/// `host[:port]` of an already parsed hub URL (IPv6 gets brackets).
fn authority_of(url: &Url) -> String {
    let host = url.host_str().unwrap_or_default();
    let host = if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]")
    } else {
        host.to_string()
    };
    match url.port() {
        Some(port) => format!("{host}:{port}"),
        None => host,
    }
}

/// HTTP client for the `/api` leg: connect timeout only, no total timeout,
/// no redirect following (a reverse proxy relays redirects instead of eating
/// them), no ambient proxy env (the tunnel dialer does not honor it either).
fn build_http_client(tls: &HubTlsSettings) -> anyhow::Result<reqwest::Client> {
    let builder = reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy();
    let builder = apply_hub_tls_to_reqwest(builder, tls).map_err(anyhow::Error::msg)?;
    Ok(builder.build()?)
}

// ═══════════════════════════════════════
// TLS for the upstream wss leg
// ═══════════════════════════════════════

/// Resolve the wss trust mode from shared settings.
///
/// The decision itself comes from `resolve_hub_tls_settings` (insecure >
/// `REX_CA_CERT` > system roots). The rustls `ClientConfig` construction is
/// mirrored from `agent_ws::hub_client_config`, which is private to that
/// module — reusing it would require editing `agent_ws.rs` (owned by a
/// parallel lane), so the same trust inputs are applied here instead of being
/// resolved again.
fn hub_rustls_config(tls: &HubTlsSettings) -> Result<Option<Arc<rustls::ClientConfig>>, String> {
    if tls.insecure {
        ensure_crypto_provider();
        let config = rustls::ClientConfig::builder_with_provider(crypto_provider())
            .with_safe_default_protocol_versions()
            .map_err(|e| e.to_string())?
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(InsecureVerifier))
            .with_no_client_auth();
        return Ok(Some(Arc::new(config)));
    }
    let Some(pem) = &tls.ca_pem else {
        // System roots: let tokio-tungstenite use its built-in webpki-roots set.
        return Ok(None);
    };

    ensure_crypto_provider();
    let mut roots = rustls::RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    };
    let mut reader = std::io::BufReader::new(pem.as_slice());
    let certs = rustls_pemfile::certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("parse REX_CA_CERT PEM: {e}"))?;
    if certs.is_empty() {
        return Err("REX_CA_CERT contains no certificates".into());
    }
    for cert in certs {
        roots
            .add(cert)
            .map_err(|e| format!("invalid CA certificate: {e}"))?;
    }
    let config = rustls::ClientConfig::builder_with_provider(crypto_provider())
        .with_safe_default_protocol_versions()
        .map_err(|e| e.to_string())?
        .with_root_certificates(roots)
        .with_no_client_auth();
    Ok(Some(Arc::new(config)))
}

/// Prefer the process default crypto provider, otherwise `ring` — this build
/// graph registers both `ring` and `aws-lc-rs`, so an implicit builder panics.
fn crypto_provider() -> Arc<rustls::crypto::CryptoProvider> {
    rustls::crypto::CryptoProvider::get_default()
        .cloned()
        .unwrap_or_else(|| Arc::new(rustls::crypto::ring::default_provider()))
}

/// Idempotently install the process default crypto provider so the default
/// (system roots) wss path inside tokio-tungstenite cannot panic on ambiguity.
fn ensure_crypto_provider() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let _ = rustls::crypto::CryptoProvider::install_default(
            rustls::crypto::ring::default_provider(),
        );
    });
}

/// `REX_TLS_INSECURE` verifier: skips server certificate validation for the
/// upstream wss leg only. Mirrors `agent_ws::InsecureVerifier` (module private).
#[derive(Debug)]
struct InsecureVerifier;

impl rustls::client::danger::ServerCertVerifier for InsecureVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::ED25519,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
        ]
    }
}

// ═══════════════════════════════════════
// Header policy
// ═══════════════════════════════════════

/// Drop hop-by-hop headers, including every name the `Connection` header lists.
fn filter_hop_by_hop(headers: &HeaderMap) -> Vec<(HeaderName, HeaderValue)> {
    let mut connection_tokens: Vec<HeaderName> = Vec::new();
    for value in headers.get_all(CONNECTION) {
        let Ok(value) = value.to_str() else { continue };
        for token in value.split(',') {
            let token = token.trim();
            if token.is_empty() {
                continue;
            }
            if let Ok(name) = HeaderName::from_bytes(token.as_bytes()) {
                connection_tokens.push(name);
            }
        }
    }

    headers
        .iter()
        .filter(|(name, _)| {
            !HOP_BY_HOP.contains(&name.as_str()) && !connection_tokens.contains(name)
        })
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect()
}

/// `X-Forwarded-For` / `X-Forwarded-Proto` / `X-Forwarded-Host`.
///
/// Values are validated by `HeaderValue::from_str` (rejects CR/LF injection) and
/// `X-Forwarded-For` is overwritten rather than appended so a browser cannot
/// forge the client address the Hub logs.
fn insert_forwarded(
    headers: &mut HeaderMap,
    peer: SocketAddr,
    inbound_scheme: &str,
    inbound_host: Option<&HeaderValue>,
) -> Result<(), StatusCode> {
    let invalid = || StatusCode::INTERNAL_SERVER_ERROR;
    headers.insert(
        HeaderName::from_static("x-forwarded-for"),
        HeaderValue::from_str(&peer.ip().to_string()).map_err(|_| invalid())?,
    );
    headers.insert(
        HeaderName::from_static("x-forwarded-proto"),
        HeaderValue::from_str(inbound_scheme).map_err(|_| invalid())?,
    );
    if let Some(host) = inbound_host {
        headers.insert(HeaderName::from_static("x-forwarded-host"), host.clone());
    }
    Ok(())
}

/// Build the request headers sent upstream: strip hop-by-hop, rewrite
/// `Host`/`Origin`/`Referer` to the Hub origin, then add `X-Forwarded-*`.
fn forward_headers(
    incoming: &HeaderMap,
    state: &AgentState,
    peer: SocketAddr,
    inbound_scheme: &str,
) -> Result<HeaderMap, StatusCode> {
    let mut out = HeaderMap::new();
    for (name, value) in filter_hop_by_hop(incoming) {
        if matches!(name.as_str(), "host" | "origin" | "referer") {
            continue;
        }
        out.append(name, value);
    }

    out.insert(HOST, state.hub_host.clone());
    out.insert(ORIGIN, state.hub_origin.clone());
    if incoming.contains_key(REFERER) {
        out.insert(REFERER, state.hub_origin.clone());
    }

    insert_forwarded(&mut out, peer, inbound_scheme, incoming.get(HOST))?;
    Ok(out)
}

// ═══════════════════════════════════════
// Handlers
// ═══════════════════════════════════════

/// Health check — answered locally, never proxied, never leaks the hub URL.
async fn health_check() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "ok",
        "mode": "agent",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// `/ws/agent` is the Hub⇄Agent tunnel, dialed outbound by `agent_ws`; the
/// browser must never reach it through this server.
async fn ws_tunnel_excluded() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        "the agent tunnel is dialed outbound, it is not served by this http server",
    )
}

/// `/api/{*path}` → streaming reverse proxy to `REX_HUB_URL`.
async fn proxy_api(
    State(state): State<Arc<AgentState>>,
    peer: ConnectInfo<SocketAddr>,
    req: Request,
) -> Result<Response, StatusCode> {
    let (parts, body) = req.into_parts();
    let path = parts.uri.path().to_string();
    let query = parts.uri.query().map(str::to_string);
    let scheme = parts.uri.scheme_str().unwrap_or("http");
    let target = state.target(&state.hub_http_base, &path, query.as_deref())?;
    let headers = forward_headers(&parts.headers, &state, peer.0, scheme)?;

    tracing::debug!(target = %target, "proxying api request to hub");

    // Body is streamed through: never buffered on the Agent side.
    let upstream = state
        .client
        .request(parts.method, target.as_str())
        .headers(headers)
        .body(reqwest::Body::wrap_stream(body.into_data_stream()))
        .send()
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, path = %path, "api reverse proxy to hub failed");
            StatusCode::BAD_GATEWAY
        })?;

    let status = StatusCode::from_u16(upstream.status().as_u16())
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let headers = filter_hop_by_hop(upstream.headers());
    let mut response = Response::new(Body::from_stream(upstream.bytes_stream()));
    *response.status_mut() = status;
    for (name, value) in headers {
        response.headers_mut().append(name, value);
    }
    Ok(response)
}

/// `/ws/{*path}` → WS upgrade reverse proxy to `REX_HUB_URL`.
///
/// The 101 handshake is performed by both libraries (never hand written); the
/// upstream dial happens before the downstream upgrade so a dead Hub answers
/// `502` instead of a hanging handshake.
async fn proxy_ws(
    State(state): State<Arc<AgentState>>,
    peer: ConnectInfo<SocketAddr>,
    upgrade: WebSocketUpgrade,
    req: Request,
) -> Result<Response, StatusCode> {
    let (parts, _body) = req.into_parts();
    let path = parts.uri.path().to_string();
    let query = parts.uri.query().map(str::to_string);
    let scheme = parts.uri.scheme_str().unwrap_or("http");
    let target = state.target(&state.hub_ws_base, &path, query.as_deref())?;

    // tungstenite generates Host/Connection/Upgrade/Sec-WebSocket-* itself, so
    // the browser's hop-by-hop and extension headers are never forwarded
    // (compression and fragmentation terminate on each leg).
    let mut request: WsRequest = target.as_str().into_client_request().map_err(|e| {
        tracing::warn!(error = %e, path = %path, "failed to build hub ws request");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    {
        let headers = request.headers_mut();
        headers.insert(HOST, state.hub_host.clone());
        headers.insert(ORIGIN, state.hub_origin.clone());
        if parts.headers.contains_key(REFERER) {
            headers.insert(REFERER, state.hub_origin.clone());
        }
        if let Some(protocol) = parts.headers.get(SEC_WEBSOCKET_PROTOCOL) {
            headers.insert(SEC_WEBSOCKET_PROTOCOL, protocol.clone());
        }
        insert_forwarded(headers, peer.0, scheme, parts.headers.get(HOST))?;
    }

    tracing::debug!(target = %target, "proxying ws upgrade to hub");

    let connector = if target.scheme() == "wss" {
        state.wss_config.clone().map(Connector::Rustls)
    } else {
        None
    };
    let (hub_socket, hub_response) = match connector {
        Some(connector) => {
            connect_async_tls_with_config(request, None, false, Some(connector)).await
        }
        None => connect_async(request).await,
    }
    .map_err(|e| {
        tracing::warn!(error = %e, path = %path, "ws reverse proxy to hub failed");
        StatusCode::BAD_GATEWAY
    })?;

    // Propagate the sub protocol the Hub picked back to the browser (it is
    // echoed only when the browser asked for it).
    let mut upgrade = upgrade;
    if let Some(protocol) = hub_response.headers().get(SEC_WEBSOCKET_PROTOCOL) {
        if let Ok(protocol) = protocol.to_str() {
            upgrade = upgrade.protocols([protocol.to_string()]);
        }
    }

    Ok(upgrade.on_upgrade(move |client_socket| relay_ws(client_socket, hub_socket)))
}

// ═══════════════════════════════════════
// WS message relay
// ═══════════════════════════════════════

fn client_to_hub(msg: ClientMessage) -> Option<HubMessage> {
    Some(match msg {
        ClientMessage::Text(text) => HubMessage::Text(text.to_string()),
        ClientMessage::Binary(data) => HubMessage::Binary(data.to_vec()),
        ClientMessage::Ping(data) => HubMessage::Ping(data.to_vec()),
        ClientMessage::Pong(data) => HubMessage::Pong(data.to_vec()),
        ClientMessage::Close(frame) => HubMessage::Close(frame.map(|frame| HubCloseFrame {
            code: frame.code.into(),
            reason: frame.reason.to_string().into(),
        })),
    })
}

fn hub_to_client(msg: HubMessage) -> Option<ClientMessage> {
    Some(match msg {
        HubMessage::Text(text) => ClientMessage::Text(text.to_string().into()),
        HubMessage::Binary(data) => ClientMessage::Binary(data.into()),
        HubMessage::Ping(data) => ClientMessage::Ping(data.into()),
        HubMessage::Pong(data) => ClientMessage::Pong(data.into()),
        HubMessage::Close(frame) => {
            ClientMessage::Close(frame.map(|frame| axum::extract::ws::CloseFrame {
                code: frame.code.into(),
                reason: frame.reason.to_string().into(),
            }))
        }
        HubMessage::Frame(_) => return None,
    })
}

/// Bidirectional text/binary/ping/pong/close relay between the browser leg and
/// the Hub leg. Whichever side finishes first closes the other one.
async fn relay_ws(
    client_socket: WebSocket,
    hub_socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
) {
    let (mut client_sink, mut client_stream) = client_socket.split();
    let (mut hub_sink, mut hub_stream) = hub_socket.split();

    loop {
        tokio::select! {
            incoming = client_stream.next() => {
                let Some(Ok(msg)) = incoming else { break };
                let Some(out) = client_to_hub(msg) else { break };
                if hub_sink.send(out).await.is_err() {
                    break;
                }
            }
            incoming = hub_stream.next() => {
                let Some(Ok(msg)) = incoming else { break };
                let Some(out) = hub_to_client(msg) else { break };
                if client_sink.send(out).await.is_err() {
                    break;
                }
            }
        }
    }

    let _ = hub_sink.send(HubMessage::Close(None)).await;
    let _ = client_sink.send(ClientMessage::Close(None)).await;
    let _ = hub_sink.close().await;
    let _ = client_sink.close().await;
}

// ═══════════════════════════════════════
// Router / server
// ═══════════════════════════════════════

fn build_router(state: Arc<AgentState>) -> Router {
    let embedded = EmbeddedStatic::new("/", &DIST);

    Router::new()
        .route("/api/health", get(health_check))
        .route("/api/{*path}", axum::routing::any(proxy_api))
        .route("/ws/agent", axum::routing::any(ws_tunnel_excluded))
        .route("/ws/{*path}", axum::routing::any(proxy_ws))
        .fallback_service(axum::routing::any_service(embedded))
        .with_state(state)
}

/// 启动 Agent HTTP server
pub async fn start_http_server(port: u16, hub_url: String) -> anyhow::Result<()> {
    let tls_insecure = std::env::var("REX_TLS_INSECURE")
        .map(|v| v == "true")
        .unwrap_or(false);
    let ca_cert = std::env::var("REX_CA_CERT").ok().filter(|v| !v.is_empty());
    let tls =
        resolve_hub_tls_settings(tls_insecure, ca_cert.as_deref()).map_err(anyhow::Error::msg)?;

    let app = build_router(Arc::new(AgentState::new(&hub_url, &tls)?));

    let addr = format!("0.0.0.0:{}", port);
    tracing::info!(addr = %addr, hub_url = %hub_url, "starting agent HTTP server");

    // Windows: binding to 0.0.0.0 may fail with WSAEACCES (os error 10013) when
    // the port is restricted by Windows Firewall or another process. Fall back to
    // 127.0.0.1 so the agent HTTP server still starts for local access.
    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            let fallback = format!("127.0.0.1:{}", port);
            tracing::warn!(
                error = %e,
                original_addr = %addr,
                fallback_addr = %fallback,
                "failed to bind HTTP server on 0.0.0.0, trying 127.0.0.1"
            );
            match tokio::net::TcpListener::bind(&fallback).await {
                Ok(l) => {
                    tracing::info!(
                        addr = %fallback,
                        "agent HTTP server bound to localhost only (0.0.0.0 unavailable on this platform)"
                    );
                    l
                }
                Err(e2) => {
                    return Err(anyhow::anyhow!(
                        "failed to start HTTP server: tried 0.0.0.0:{} ({}) and 127.0.0.1:{} ({})",
                        port,
                        e,
                        port,
                        e2
                    ));
                }
            }
        }
    };
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::connect_info::MockConnectInfo;
    use axum::http::Uri;
    use serde_json::Value;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::sync::mpsc;
    use tower::ServiceExt;

    /// Fixed peer injected through `MockConnectInfo` so `X-Forwarded-For` is deterministic.
    fn peer() -> SocketAddr {
        SocketAddr::from(([203, 0, 113, 9], 54321))
    }

    fn agent_app(hub_url: &str) -> Router {
        let tls = resolve_hub_tls_settings(false, None).expect("tls settings");
        let state = Arc::new(AgentState::new(hub_url, &tls).expect("agent state"));
        build_router(state).layer(MockConnectInfo(peer()))
    }

    async fn spawn_agent(hub_url: &str) -> SocketAddr {
        let app = agent_app(hub_url);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .unwrap();
        });
        addr
    }

    fn get(uri: &str) -> Request {
        Request::builder().uri(uri).body(Body::empty()).unwrap()
    }

    async fn json_body(resp: Response) -> Value {
        let bytes = axum::body::to_bytes(resp.into_body(), 16 * 1024 * 1024)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).expect("json response")
    }

    // ── header policy ────────────────────────────────────────────────

    #[test]
    fn hop_by_hop_headers_are_stripped_including_connection_tokens() {
        let mut headers = HeaderMap::new();
        headers.insert(HOST, HeaderValue::from_static("hub.example:3000"));
        headers.insert(CONNECTION, HeaderValue::from_static("keep-alive, X-Trace"));
        headers.insert("keep-alive", HeaderValue::from_static("timeout=5"));
        headers.insert("transfer-encoding", HeaderValue::from_static("chunked"));
        headers.insert("te", HeaderValue::from_static("trailers"));
        headers.insert("trailer", HeaderValue::from_static("Expires"));
        headers.insert("upgrade", HeaderValue::from_static("h2c"));
        headers.insert("proxy-connection", HeaderValue::from_static("keep-alive"));
        headers.insert("proxy-authorization", HeaderValue::from_static("Basic zzz"));
        headers.insert("x-trace", HeaderValue::from_static("abc"));
        headers.insert("content-type", HeaderValue::from_static("application/json"));

        let kept: Vec<String> = filter_hop_by_hop(&headers)
            .into_iter()
            .map(|(name, _)| name.as_str().to_string())
            .collect();

        assert_eq!(kept, vec!["host".to_string(), "content-type".to_string()]);
    }

    // ── routing ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn health_is_answered_locally() {
        let app = agent_app("http://127.0.0.1:1");
        let resp = app.oneshot(get("/api/health")).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let bytes = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["mode"], "agent");
        assert_eq!(json["status"], "ok");
    }

    #[tokio::test]
    async fn static_fallback_serves_spa_without_swallowing_api_or_ws() {
        let app = agent_app("http://127.0.0.1:1");

        let root = app.clone().oneshot(get("/")).await.unwrap();
        assert_eq!(root.status(), StatusCode::OK);
        assert!(root
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("text/html"));

        let spa = app
            .clone()
            .oneshot(get("/workspace/some/deep/route"))
            .await
            .unwrap();
        assert_eq!(spa.status(), StatusCode::OK);

        // /api and /ws never reach the SPA fallback: the proxy answers 502 when
        // the hub is unreachable instead of 200 text/html.
        let api = app.clone().oneshot(get("/api/resources")).await.unwrap();
        assert_eq!(api.status(), StatusCode::BAD_GATEWAY);
    }

    #[tokio::test]
    async fn ws_agent_tunnel_route_is_excluded_from_proxy() {
        let app = agent_app("http://127.0.0.1:1");
        let resp = app.oneshot(get("/ws/agent?token=secret")).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn ws_upgrade_to_unreachable_hub_is_502_not_static() {
        let addr = spawn_agent("http://127.0.0.1:1").await;
        let mut stream = TcpStream::connect(addr).await.unwrap();
        let request = format!(
            "GET /ws/terminal?token=abc HTTP/1.1\r\n\
             Host: {addr}\r\n\
             Connection: Upgrade\r\n\
             Upgrade: websocket\r\n\
             Sec-WebSocket-Version: 13\r\n\
             Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n"
        );
        stream.write_all(request.as_bytes()).await.unwrap();

        let mut buf = vec![0u8; 4096];
        let n = stream.read(&mut buf).await.unwrap();
        let head = String::from_utf8_lossy(&buf[..n]).to_string();
        assert!(
            head.starts_with("HTTP/1.1 502"),
            "expected 502 from the ws proxy, got: {head}"
        );
    }

    // ── api reverse proxy ────────────────────────────────────────────

    #[tokio::test]
    async fn api_proxy_rewrites_origin_host_and_adds_forwarded_headers() {
        let hub = spawn_echo_hub().await;
        let app = agent_app(&format!("http://{hub}"));

        let request = Request::builder()
            .method("POST")
            .uri("/api/resources?cursor=1")
            .header(HOST, "agent.lan:3000")
            .header(ORIGIN, "http://192.168.1.5:3000")
            .header(REFERER, "http://192.168.1.5:3000/workspace")
            .header(CONNECTION, "keep-alive, X-Trace")
            .header("keep-alive", "timeout=5")
            .header("te", "trailers")
            .header("trailer", "Expires")
            .header("proxy-connection", "keep-alive")
            .header("x-trace", "abc")
            .header("content-type", "application/json")
            .body(Body::from("{\"a\":1}"))
            .unwrap();

        let resp = app.oneshot(request).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let hub_mark = resp.headers().get("x-hub-mark").unwrap().clone();
        assert_eq!(hub_mark, "1");
        assert!(!resp.headers().contains_key("keep-alive"));

        let json = json_body(resp).await;
        let headers = json["headers"].as_object().unwrap();
        let hub_origin = format!("http://{hub}");

        assert_eq!(json["method"], "POST");
        assert_eq!(json["path"], "/api/resources");
        assert_eq!(json["query"], "cursor=1");
        assert_eq!(headers["host"].as_str().unwrap(), hub.to_string());
        assert_eq!(headers["origin"].as_str().unwrap(), hub_origin);
        assert_eq!(headers["referer"].as_str().unwrap(), hub_origin);
        assert_eq!(headers["x-forwarded-for"].as_str().unwrap(), "203.0.113.9");
        assert_eq!(headers["x-forwarded-proto"].as_str().unwrap(), "http");
        assert_eq!(
            headers["x-forwarded-host"].as_str().unwrap(),
            "agent.lan:3000"
        );
        assert_eq!(
            headers["content-type"].as_str().unwrap(),
            "application/json"
        );

        for stripped in [
            "connection",
            "keep-alive",
            "te",
            "trailer",
            "proxy-connection",
            "x-trace",
        ] {
            assert!(
                headers.get(stripped).is_none(),
                "{stripped} must not reach the hub"
            );
        }
    }

    #[tokio::test]
    async fn api_proxy_forwards_large_request_body() {
        let hub = spawn_echo_hub().await;
        let app = agent_app(&format!("http://{hub}"));

        let payload = vec![0xABu8; 1024 * 1024];
        let request = Request::builder()
            .method("POST")
            .uri("/api/echo")
            .body(Body::from(payload))
            .unwrap();

        let resp = app.oneshot(request).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let json = json_body(resp).await;
        assert_eq!(json["body_len"], 1024 * 1024);
    }

    #[tokio::test]
    async fn api_proxy_has_no_total_timeout_on_slow_hub() {
        let hub = spawn_echo_hub().await;
        let app = agent_app(&format!("http://{hub}"));

        let start = std::time::Instant::now();
        let resp = app.oneshot(get("/api/slow")).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        assert_eq!(&bytes[..], b"slow");
        assert!(
            start.elapsed() >= Duration::from_millis(1400),
            "the delayed hub response must not be cut short"
        );
    }

    #[tokio::test]
    async fn api_proxy_streams_response_without_buffering() {
        let hub = spawn_echo_hub().await;
        let app = agent_app(&format!("http://{hub}"));
        let addr = spawn_served(app).await;

        let client = reqwest::Client::new();
        let start = std::time::Instant::now();
        let mut resp = client
            .get(format!("http://{addr}/api/chunked"))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let first = resp
            .chunk()
            .await
            .unwrap()
            .expect("first chunk must arrive");
        let first_elapsed = start.elapsed();
        assert_eq!(&first[..], b"chunk0;");
        assert!(
            first_elapsed < Duration::from_millis(1200),
            "first chunk must be relayed before the hub sends the rest (took {first_elapsed:?})"
        );

        let mut rest = Vec::new();
        while let Some(chunk) = resp.chunk().await.unwrap() {
            rest.extend_from_slice(&chunk);
        }
        assert_eq!(rest, b"chunk1;chunk2;chunk3;");
        assert!(start.elapsed() >= Duration::from_millis(1200));
    }

    // ── ws reverse proxy ─────────────────────────────────────────────

    #[tokio::test]
    async fn ws_upgrade_is_relayed_and_upstream_headers_are_rewritten() {
        let (hub, mut captured) = spawn_ws_hub().await;
        let agent = spawn_agent(&format!("http://{hub}")).await;

        let mut request = format!("ws://{agent}/ws/terminal?token=abc")
            .into_client_request()
            .unwrap();
        request
            .headers_mut()
            .insert(ORIGIN, HeaderValue::from_static("http://192.168.1.5:3000"));
        request.headers_mut().insert(
            REFERER,
            HeaderValue::from_static("http://192.168.1.5:3000/workspace"),
        );

        let (mut client, response) = connect_async(request).await.expect("ws handshake");
        assert_eq!(response.status(), 101);

        client.send(HubMessage::Text("hello".into())).await.unwrap();
        let echoed = client.next().await.unwrap().unwrap();
        assert_eq!(echoed, HubMessage::Text("hello".into()));

        client
            .send(HubMessage::Binary(vec![1, 2, 3]))
            .await
            .unwrap();
        let echoed = client.next().await.unwrap().unwrap();
        assert_eq!(echoed, HubMessage::Binary(vec![1, 2, 3]));

        let headers = tokio::time::timeout(Duration::from_secs(5), captured.recv())
            .await
            .expect("hub handshake")
            .expect("captured headers");
        let hub_origin = format!("http://{hub}");
        assert_eq!(headers.get(ORIGIN).unwrap(), hub_origin.as_str());
        assert_eq!(headers.get(REFERER).unwrap(), hub_origin.as_str());
        assert_eq!(headers.get(HOST).unwrap(), hub.to_string().as_str());
        assert_eq!(
            headers.get("x-forwarded-proto").unwrap(),
            "http",
            "rewritten scheme"
        );
        assert!(headers.get("x-forwarded-for").is_some());

        client.send(HubMessage::Close(None)).await.unwrap();
        let close = tokio::time::timeout(Duration::from_secs(5), client.next())
            .await
            .expect("close must be propagated to the browser");
        assert!(
            matches!(close, Some(Ok(HubMessage::Close(_)))),
            "expected a close frame, got {close:?}"
        );
    }

    // ── test infrastructure ──────────────────────────────────────────

    async fn spawn_served(app: Router) -> SocketAddr {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        addr
    }

    async fn hub_echo(
        method: axum::http::Method,
        uri: Uri,
        headers: HeaderMap,
        body: Body,
    ) -> Response {
        let bytes = axum::body::to_bytes(body, 16 * 1024 * 1024)
            .await
            .unwrap_or_default();
        // Single-value headers echo as a plain string (the common case the
        // assertions read directly); only repeated headers stay arrays.
        let mut echoed = serde_json::Map::new();
        for (name, value) in headers.iter() {
            let entry = echoed
                .entry(name.as_str().to_string())
                .or_insert_with(|| Value::Array(Vec::new()));
            if !entry.is_array() {
                let prev = entry.take();
                *entry = Value::Array(vec![prev]);
            }
            let arr = entry.as_array_mut().unwrap();
            arr.push(Value::String(
                value.to_str().unwrap_or_default().to_string(),
            ));
            if arr.len() == 1 {
                let single = arr[0].clone();
                *entry = single;
            }
        }

        let mut resp = axum::Json(serde_json::json!({
            "method": method.as_str(),
            "path": uri.path(),
            "query": uri.query(),
            "headers": echoed,
            "body_len": bytes.len(),
        }))
        .into_response();
        resp.headers_mut()
            .insert("keep-alive", HeaderValue::from_static("timeout=5"));
        resp.headers_mut()
            .insert("x-hub-mark", HeaderValue::from_static("1"));
        resp
    }

    async fn hub_slow() -> &'static str {
        tokio::time::sleep(Duration::from_millis(1500)).await;
        "slow"
    }

    async fn hub_chunked() -> Body {
        let stream = futures_util::stream::iter(0..4).then(|i| async move {
            if i > 0 {
                tokio::time::sleep(Duration::from_millis(600)).await;
            }
            Ok::<_, std::io::Error>(bytes::Bytes::from(format!("chunk{i};")))
        });
        Body::from_stream(stream)
    }

    async fn spawn_echo_hub() -> SocketAddr {
        // The agent proxy forwards browser paths verbatim (same-origin contract:
        // /api/X on the agent must land on /api/X on the hub), so the echo hub
        // registers the same /api-prefixed routes the real hub serves.
        let app = Router::new()
            .route("/api/echo", axum::routing::any(hub_echo))
            .route("/api/slow", axum::routing::get(hub_slow))
            .route("/api/chunked", axum::routing::get(hub_chunked))
            .route("/api/{*path}", axum::routing::any(hub_echo));
        spawn_served(app).await
    }

    /// Echo hub that records the handshake headers of every accepted WS upgrade.
    async fn spawn_ws_hub() -> (SocketAddr, mpsc::UnboundedReceiver<HeaderMap>) {
        use tokio_tungstenite::tungstenite::handshake::server::{
            Request as ServerRequest, Response as ServerResponse,
        };

        let (tx, rx) = mpsc::unbounded_channel();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    break;
                };
                let tx = tx.clone();
                tokio::spawn(async move {
                    let callback = move |request: &ServerRequest, response: ServerResponse| {
                        let _ = tx.send(request.headers().clone());
                        Ok(response)
                    };
                    let Ok(mut socket) =
                        tokio_tungstenite::accept_hdr_async(stream, callback).await
                    else {
                        return;
                    };
                    while let Some(Ok(msg)) = socket.next().await {
                        let closing = matches!(msg, HubMessage::Close(_));
                        if socket.send(msg).await.is_err() {
                            break;
                        }
                        if closing {
                            break;
                        }
                    }
                });
            }
        });

        (addr, rx)
    }
}
