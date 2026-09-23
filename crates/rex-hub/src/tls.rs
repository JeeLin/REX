//! TLS 配置与 serve 实现。
//!
//! 三种模式（`TlsConfig::from_env` 解析，优先级 Manual > SelfSigned > None：
//! `REX_TLS_CERT`/`REX_TLS_KEY` 成对存在即 Manual）：
//!
//! - `None`：纯 HTTP，行为与历史版本完全一致
//! - `SelfSigned`：首启用 rcgen 生成 `{REX_DATA_DIR}/tls/{cert,key}.pem`
//!   （SAN 覆盖 localhost / hostname / 宿主 IP，尽力而为），已存在则复用；
//!   复用前校验证书有效期与 key 匹配，过期或损坏时自动重新生成
//! - `Manual`：`REX_TLS_CERT` / `REX_TLS_KEY` 指定 PEM，rustls-pemfile 加载，
//!   启动时校验有效期与 key 匹配，配错即拒绝启动（panic 退出，错误含路径）
//!
//! TLS 最低版本 1.3（PRODUCT.md「传输 TLS 1.3」），ALPN 仅声明 http/1.1。

use std::io::BufReader;
use std::net::{IpAddr, UdpSocket};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::serve::Listener;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_rustls::server::TlsStream;
use tokio_rustls::TlsAcceptor;

/// TLS 配置
pub enum TlsConfig {
    None,
    SelfSigned {
        data_dir: PathBuf,
    },
    Manual {
        cert_path: PathBuf,
        key_path: PathBuf,
    },
}

/// 自签名证书存放的子目录（位于 data_dir 之下）。
const SELF_SIGNED_DIR: &str = "tls";

impl TlsConfig {
    pub fn from_env() -> Self {
        if let (Ok(cert), Ok(key)) = (std::env::var("REX_TLS_CERT"), std::env::var("REX_TLS_KEY")) {
            return Self::Manual {
                cert_path: PathBuf::from(cert),
                key_path: PathBuf::from(key),
            };
        }
        if std::env::var("REX_TLS_SELF_SIGNED")
            .map(|v| v == "true")
            .unwrap_or(false)
        {
            return Self::SelfSigned {
                data_dir: data_dir_or_default(),
            };
        }
        Self::None
    }

    pub fn is_enabled(&self) -> bool {
        !matches!(self, Self::None)
    }

    /// 证书/私钥文件路径。
    fn paths(&self) -> Result<(PathBuf, PathBuf), String> {
        match self {
            Self::None => Err("TLS is not enabled".to_string()),
            Self::SelfSigned { data_dir } => {
                let dir = data_dir.join(SELF_SIGNED_DIR);
                Ok((dir.join("cert.pem"), dir.join("key.pem")))
            }
            Self::Manual {
                cert_path,
                key_path,
            } => Ok((cert_path.clone(), key_path.clone())),
        }
    }

    /// 加载证书链与私钥；SelfSigned 模式先确保磁盘上的材料可用（首启生成、过期重生成）。
    fn load_cert_chain(
        &self,
    ) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>), String> {
        let (cert_path, key_path) = self.paths()?;
        if let Self::SelfSigned { .. } = self {
            ensure_self_signed(&cert_path, &key_path)?;
        }
        let chain = read_cert_chain(&cert_path)?;
        let key = read_private_key(&key_path)?;
        Ok((chain, key))
    }

    /// 加载 + 校验（有效期、key 匹配）并构造 rustls `ServerConfig`（TLS 下限 1.3）。
    fn server_config(&self) -> Result<rustls::ServerConfig, String> {
        let (cert_path, key_path) = self.paths()?;
        let (chain, key) = self.load_cert_chain()?;
        ensure_not_expired(&chain, &cert_path)?;
        let provider = tls_provider();
        let certified = rustls::sign::CertifiedKey::from_der(chain, key, &provider)
            .map_err(|e| key_pair_error(e, &key_path, &cert_path))?;
        let mut config = rustls::ServerConfig::builder_with_provider(provider)
            .with_protocol_versions(&[&rustls::version::TLS13])
            .map_err(|e| format!("failed to restrict TLS to 1.3: {e}"))?
            .with_no_client_auth()
            .with_cert_resolver(Arc::new(rustls::sign::SingleCertAndKey::from(certified)));
        config.alpn_protocols = vec![b"http/1.1".to_vec()];
        Ok(config)
    }

    fn log_mode(&self) {
        match self {
            Self::None => {}
            Self::SelfSigned { data_dir } => tracing::warn!(
                dir = %data_dir.join(SELF_SIGNED_DIR).display(),
                "TLS self-signed mode enabled"
            ),
            Self::Manual { cert_path, .. } => {
                tracing::info!(cert = %cert_path.display(), "TLS manual certificate mode enabled")
            }
        }
    }
}

fn data_dir_or_default() -> PathBuf {
    std::env::var("REX_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| rex_common::config::default_data_dir())
}

/// 确保自签名证书存在且可用：已存在则校验（有效期 + key 匹配）后复用，
/// 过期/损坏/缺失时重新生成。
fn ensure_self_signed(cert_path: &Path, key_path: &Path) -> Result<(), String> {
    if cert_path.is_file() && key_path.is_file() {
        match check_self_signed_pair(cert_path, key_path) {
            Ok(()) => return Ok(()),
            Err(e) => tracing::warn!(
                cert = %cert_path.display(),
                error = %e,
                "regenerating self-signed TLS certificate"
            ),
        }
    }
    let dir = cert_path
        .parent()
        .ok_or_else(|| format!("invalid TLS certificate path {}", cert_path.display()))?;
    generate_self_signed(dir, cert_path, key_path)
}

/// 复用前校验已存在的自签名证书对。
fn check_self_signed_pair(cert_path: &Path, key_path: &Path) -> Result<(), String> {
    let chain = read_cert_chain(cert_path)?;
    ensure_not_expired(&chain, cert_path)?;
    let key = read_private_key(key_path)?;
    let provider = tls_provider();
    rustls::sign::CertifiedKey::from_der(chain, key, &provider)
        .map_err(|e| key_pair_error(e, key_path, cert_path))?;
    Ok(())
}

fn generate_self_signed(dir: &Path, cert_path: &Path, key_path: &Path) -> Result<(), String> {
    let mut params = rcgen::CertificateParams::new(Vec::<String>::new())
        .map_err(|e| format!("failed to prepare self-signed TLS parameters: {e}"))?;
    for candidate in san_candidates() {
        if let Ok(ip) = candidate.parse::<IpAddr>() {
            params.subject_alt_names.push(rcgen::SanType::IpAddress(ip));
        } else if let Ok(name) = rcgen::Ia5String::try_from(candidate.as_str()) {
            params.subject_alt_names.push(rcgen::SanType::DnsName(name));
        } else {
            tracing::debug!(candidate = %candidate, "skipping invalid TLS SAN candidate");
        }
    }
    let key = rcgen::KeyPair::generate()
        .map_err(|e| format!("failed to generate TLS private key: {e}"))?;
    let cert = params
        .self_signed(&key)
        .map_err(|e| format!("failed to generate self-signed TLS certificate: {e}"))?;
    std::fs::create_dir_all(dir).map_err(|e| format!("failed to create {}: {e}", dir.display()))?;
    std::fs::write(cert_path, cert.pem())
        .map_err(|e| format!("failed to write {}: {e}", cert_path.display()))?;
    std::fs::write(key_path, key.serialize_pem())
        .map_err(|e| format!("failed to write {}: {e}", key_path.display()))?;
    tracing::info!(cert = %cert_path.display(), "generated self-signed TLS certificate");
    Ok(())
}

/// 自签名 SAN 候选：localhost（名称 + 回环）、hostname、宿主 IP（尽力而为）。
fn san_candidates() -> Vec<String> {
    let mut out = vec![
        "localhost".to_string(),
        "127.0.0.1".to_string(),
        "::1".to_string(),
    ];
    if let Ok(hostname) = hostname::get() {
        let hostname = hostname.to_string_lossy();
        let hostname = hostname.trim();
        if !hostname.is_empty() {
            out.push(hostname.to_string());
        }
    }
    if let Some(ip) = primary_local_ip() {
        out.push(ip);
    }
    out
}

/// 通过 UDP connect（不实际发包）探测宿主对外主 IP。
fn primary_local_ip() -> Option<String> {
    let sock = UdpSocket::bind("0.0.0.0:0").ok()?;
    sock.connect("8.8.8.8:80").ok()?;
    let ip = sock.local_addr().ok()?.ip();
    if ip.is_loopback() {
        None
    } else {
        Some(ip.to_string())
    }
}

fn read_cert_chain(path: &Path) -> Result<Vec<CertificateDer<'static>>, String> {
    let file = std::fs::File::open(path)
        .map_err(|e| format!("failed to open TLS certificate {}: {e}", path.display()))?;
    let certs = rustls_pemfile::certs(&mut BufReader::new(file))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("failed to read TLS certificate {}: {e}", path.display()))?;
    if certs.is_empty() {
        return Err(format!("no certificate found in {}", path.display()));
    }
    Ok(certs)
}

fn read_private_key(path: &Path) -> Result<PrivateKeyDer<'static>, String> {
    let file = std::fs::File::open(path)
        .map_err(|e| format!("failed to open TLS private key {}: {e}", path.display()))?;
    rustls_pemfile::private_key(&mut BufReader::new(file))
        .map_err(|e| format!("failed to read TLS private key {}: {e}", path.display()))?
        .ok_or_else(|| format!("no private key found in {}", path.display()))
}

/// 校验证书处于有效期内（叶子证书的 notBefore/notAfter）。
fn ensure_not_expired(chain: &[CertificateDer<'static>], cert_path: &Path) -> Result<(), String> {
    let cert = chain
        .first()
        .ok_or_else(|| format!("no certificate found in {}", cert_path.display()))?;
    let (not_before, not_after) = validity_window(cert).map_err(|e| {
        format!(
            "failed to parse TLS certificate {}: {e}",
            cert_path.display()
        )
    })?;
    let now = chrono::Utc::now().timestamp();
    if now > not_after {
        return Err(format!(
            "TLS certificate {} expired at {}",
            cert_path.display(),
            format_timestamp(not_after)
        ));
    }
    if now < not_before {
        return Err(format!(
            "TLS certificate {} is not valid until {}",
            cert_path.display(),
            format_timestamp(not_before)
        ));
    }
    Ok(())
}

/// 从 DER 证书中提取 (not_before, not_after)（Unix 秒）。
/// 只走读 Certificate -> tbsCertificate -> validity 所需的最小 DER 解析。
fn validity_window(cert_der: &[u8]) -> Result<(i64, i64), String> {
    let (tag, cert_body, _) = read_tlv(cert_der)?;
    if tag != 0x30 {
        return Err("not a DER-encoded certificate".to_string());
    }
    let (tag, tbs, _) = read_tlv(cert_body)?;
    if tag != 0x30 {
        return Err("missing tbsCertificate".to_string());
    }
    let mut rest = tbs;
    // version [0] EXPLICIT OPTIONAL
    if rest.first() == Some(&0xa0) {
        let (_, _, next) = read_tlv(rest)?;
        rest = next;
    }
    // serialNumber INTEGER, signature AlgorithmIdentifier, issuer Name
    for expected in [0x02u8, 0x30, 0x30] {
        let (tag, _, next) = read_tlv(rest)?;
        if tag != expected {
            return Err(format!("unexpected tbsCertificate field tag 0x{tag:02x}"));
        }
        rest = next;
    }
    // validity SEQUENCE { notBefore Time, notAfter Time }
    let (tag, validity, _) = read_tlv(rest)?;
    if tag != 0x30 {
        return Err("missing validity".to_string());
    }
    let (nb_tag, nb, after_nb) = read_tlv(validity)?;
    let (na_tag, na, _) = read_tlv(after_nb)?;
    Ok((parse_asn1_time(nb_tag, nb)?, parse_asn1_time(na_tag, na)?))
}

/// 读取一个 DER TLV，返回 (tag, content, rest)。
fn read_tlv(input: &[u8]) -> Result<(u8, &[u8], &[u8]), String> {
    if input.len() < 2 {
        return Err("truncated DER".to_string());
    }
    let tag = input[0];
    let len_byte = input[1];
    let (len, header) = if len_byte & 0x80 == 0 {
        (len_byte as usize, 2)
    } else {
        let n = (len_byte & 0x7f) as usize;
        if n == 0 || n > 4 || input.len() < 2 + n {
            return Err("unsupported DER length".to_string());
        }
        let mut len = 0usize;
        for &b in &input[2..2 + n] {
            len = (len << 8) | b as usize;
        }
        (len, 2 + n)
    };
    if input.len() < header + len {
        return Err("truncated DER".to_string());
    }
    Ok((tag, &input[header..header + len], &input[header + len..]))
}

/// 解析 X.509 Time（UTCTime 0x17 / GeneralizedTime 0x18）为 Unix 秒。
fn parse_asn1_time(tag: u8, bytes: &[u8]) -> Result<i64, String> {
    let text =
        std::str::from_utf8(bytes).map_err(|_| "certificate time is not ASCII".to_string())?;
    let (year, month, day, hour, minute, second) = match tag {
        0x17 => {
            // YYMMDDHHMM[SS]Z，RFC 5280 要求带秒
            if text.len() < 11 {
                return Err(format!("invalid UTCTime {text:?}"));
            }
            let yy = parse_component(&text[0..2])?;
            let year = if yy >= 50 { 1900 + yy } else { 2000 + yy };
            let sec = if text.len() >= 12 {
                parse_component(&text[10..12])?
            } else {
                0
            };
            (
                year,
                parse_component(&text[2..4])?,
                parse_component(&text[4..6])?,
                parse_component(&text[6..8])?,
                parse_component(&text[8..10])?,
                sec,
            )
        }
        0x18 => {
            // YYYYMMDDHHMMSS[.frac]Z
            if text.len() < 14 {
                return Err(format!("invalid GeneralizedTime {text:?}"));
            }
            (
                parse_component(&text[0..4])?,
                parse_component(&text[4..6])?,
                parse_component(&text[6..8])?,
                parse_component(&text[8..10])?,
                parse_component(&text[10..12])?,
                parse_component(&text[12..14])?,
            )
        }
        other => return Err(format!("unsupported certificate time tag 0x{other:02x}")),
    };
    Ok(days_from_civil(year, month, day) * 86400 + hour * 3600 + minute * 60 + second)
}

fn parse_component(value: &str) -> Result<i64, String> {
    value
        .parse::<i64>()
        .map_err(|_| format!("invalid certificate time component {value:?}"))
}

/// Howard Hinnant 的 civil-from-days 正向算法：公历日期 -> 距 1970-01-01 的天数。
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let doy = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn format_timestamp(secs: i64) -> String {
    chrono::DateTime::from_timestamp(secs, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| format!("unix timestamp {secs}"))
}

/// 构建显式 CryptoProvider。
///
/// 本 workspace 同时启用 rustls 的 `ring`（reqwest/hyper-rustls 链路）与
/// `aws-lc-rs`（rustls 默认特性）两个 provider，进程默认 provider 有歧义、
/// `ServerConfig::builder()` 会 panic，因此显式选择 ring（与 PRODUCT.md 对齐）。
fn tls_provider() -> Arc<rustls::crypto::CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

fn key_pair_error(error: rustls::Error, key_path: &Path, cert_path: &Path) -> String {
    match error {
        rustls::Error::InconsistentKeys(rustls::InconsistentKeys::KeyMismatch) => format!(
            "TLS private key {} does not match certificate {}",
            key_path.display(),
            cert_path.display()
        ),
        error => format!(
            "failed to load TLS key {} with certificate {}: {error}",
            key_path.display(),
            cert_path.display()
        ),
    }
}

/// 在后台完成 TLS 握手后再把流交给 axum，避免慢握手阻塞 listener 的 accept 循环。
struct TlsListener {
    inner: tokio::net::TcpListener,
    acceptor: TlsAcceptor,
    ready_tx: mpsc::UnboundedSender<(TlsStream<TcpStream>, std::net::SocketAddr)>,
    ready_rx: mpsc::UnboundedReceiver<(TlsStream<TcpStream>, std::net::SocketAddr)>,
}

impl TlsListener {
    fn new(inner: tokio::net::TcpListener, acceptor: TlsAcceptor) -> Self {
        let (ready_tx, ready_rx) = mpsc::unbounded_channel();
        Self {
            inner,
            acceptor,
            ready_tx,
            ready_rx,
        }
    }
}

impl Listener for TlsListener {
    type Io = TlsStream<TcpStream>;
    type Addr = std::net::SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        loop {
            let acceptor = self.acceptor.clone();
            let ready_tx = self.ready_tx.clone();
            tokio::select! {
                ready = self.ready_rx.recv() => {
                    if let Some(ready) = ready {
                        return ready;
                    }
                }
                (stream, addr) = Listener::accept(&mut self.inner) => {
                    tokio::spawn(async move {
                        match acceptor.accept(stream).await {
                            Ok(tls) => {
                                let _ = ready_tx.send((tls, addr));
                            }
                            Err(e) => tracing::warn!(%addr, "TLS handshake failed: {e}"),
                        }
                    });
                }
            }
        }
    }

    fn local_addr(&self) -> std::io::Result<Self::Addr> {
        self.inner.local_addr()
    }
}

/// 启动监听 — `None` 走纯 HTTP（与历史行为一致），其余模式由 tokio-rustls 接管
/// accept 后交给 axum service（支持 WebSocket upgrade）。
///
/// TLS 材料配错（过期 / key 不匹配 / 路径不可读）时给出含路径的错误并 panic
/// 退出，拒绝以错误配置启动。
pub async fn serve(
    app: axum::Router,
    listener: tokio::net::TcpListener,
    config: TlsConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    match config {
        TlsConfig::None => {
            tracing::info!("listening on HTTP");
            axum::serve(listener, app).await?;
            Ok(())
        }
        tls => {
            tls.log_mode();
            let server_config = tls
                .server_config()
                .unwrap_or_else(|e| panic!("TLS configuration error: {e}"));
            let acceptor = TlsAcceptor::from(Arc::new(server_config));
            axum::serve(TlsListener::new(listener, acceptor), app).await?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::get;
    use std::time::Duration;
    use tempfile::tempdir;

    /// 生成一对默认有效期（1975..4096，notAfter 为 GeneralizedTime）的证书。
    fn valid_pair() -> (String, String) {
        let params = rcgen::CertificateParams::new(vec!["localhost".to_string()]).unwrap();
        let key = rcgen::KeyPair::generate().unwrap();
        let cert = params.self_signed(&key).unwrap();
        (cert.pem(), key.serialize_pem())
    }

    /// 生成一对已过期（notAfter 为 UTCTime）的证书。
    fn expired_pair() -> (String, String) {
        let mut params = rcgen::CertificateParams::new(vec!["localhost".to_string()]).unwrap();
        params.not_before = rcgen::date_time_ymd(2000, 1, 1);
        params.not_after = rcgen::date_time_ymd(2000, 2, 1);
        let key = rcgen::KeyPair::generate().unwrap();
        let cert = params.self_signed(&key).unwrap();
        (cert.pem(), key.serialize_pem())
    }

    fn write_pair(dir: &Path, cert_pem: &str, key_pem: &str) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("cert.pem"), cert_pem).unwrap();
        std::fs::write(dir.join("key.pem"), key_pem).unwrap();
    }

    fn manual_config(dir: &Path) -> TlsConfig {
        TlsConfig::Manual {
            cert_path: dir.join("cert.pem"),
            key_path: dir.join("key.pem"),
        }
    }

    #[test]
    fn test_tls_config_default() {
        let config = TlsConfig::from_env();
        assert!(!config.is_enabled());
    }

    #[tokio::test]
    async fn none_falls_back_to_plain_http() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let app = axum::Router::new().route("/ping", get(|| async { "pong" }));
        // serve returns Box<dyn Error> which is not Send; drop the output so the
        // task itself satisfies tokio::spawn bounds.
        let handle = tokio::spawn(async move {
            let _ = serve(app, listener, TlsConfig::None).await;
        });

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap();
        let resp = client
            .get(format!("http://{addr}/ping"))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), reqwest::StatusCode::OK);
        assert_eq!(resp.text().await.unwrap(), "pong");
        handle.abort();
    }

    #[tokio::test]
    async fn self_signed_serves_https() {
        let dir = tempdir().unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let app = axum::Router::new().route("/ping", get(|| async { "pong" }));
        let config = TlsConfig::SelfSigned {
            data_dir: dir.path().to_path_buf(),
        };
        let handle = tokio::spawn(async move {
            let _ = serve(app, listener, config).await;
        });

        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap();
        let resp = client
            .get(format!("https://{addr}/ping"))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), reqwest::StatusCode::OK);
        assert_eq!(resp.text().await.unwrap(), "pong");
        // HTTPS 握手成功后，自签名材料必然已生成
        assert!(dir.path().join("tls/cert.pem").is_file());
        assert!(dir.path().join("tls/key.pem").is_file());
        handle.abort();
    }

    #[test]
    fn self_signed_generates_then_reuses() {
        let dir = tempdir().unwrap();
        let config = TlsConfig::SelfSigned {
            data_dir: dir.path().to_path_buf(),
        };
        let (chain, _key) = config.load_cert_chain().unwrap();
        ensure_not_expired(&chain, &dir.path().join("tls/cert.pem")).unwrap();
        let cert_path = dir.path().join("tls/cert.pem");
        let key_path = dir.path().join("tls/key.pem");
        assert!(cert_path.is_file());
        assert!(key_path.is_file());
        let first = std::fs::read(&cert_path).unwrap();

        // 第二次加载必须复用已生成的证书，不重新生成
        config.load_cert_chain().unwrap();
        let second = std::fs::read(&cert_path).unwrap();
        assert_eq!(first, second, "existing certificate must be reused");
    }

    #[test]
    fn self_signed_regenerates_expired_certificate() {
        let dir = tempdir().unwrap();
        let (cert_pem, key_pem) = expired_pair();
        write_pair(&dir.path().join("tls"), &cert_pem, &key_pem);
        let cert_path = dir.path().join("tls/cert.pem");
        let expired_bytes = std::fs::read(&cert_path).unwrap();

        let config = TlsConfig::SelfSigned {
            data_dir: dir.path().to_path_buf(),
        };
        let (chain, _key) = config.load_cert_chain().unwrap();
        // 过期证书已被重新生成，复用校验（有效期）必须通过
        ensure_not_expired(&chain, &cert_path).unwrap();
        let new_bytes = std::fs::read(&cert_path).unwrap();
        assert_ne!(
            expired_bytes, new_bytes,
            "expired certificate must be regenerated"
        );
    }

    #[test]
    fn manual_valid_pem_loads() {
        let dir = tempdir().unwrap();
        let (cert_pem, key_pem) = valid_pair();
        write_pair(dir.path(), &cert_pem, &key_pem);
        let config = manual_config(dir.path());

        let (chain, _key) = config.load_cert_chain().unwrap();
        assert_eq!(chain.len(), 1);
        let server_config = config.server_config().unwrap();
        assert_eq!(server_config.alpn_protocols, vec![b"http/1.1".to_vec()]);
    }

    #[test]
    fn manual_expired_certificate_reports_expired() {
        let dir = tempdir().unwrap();
        let (cert_pem, key_pem) = expired_pair();
        write_pair(dir.path(), &cert_pem, &key_pem);
        let config = manual_config(dir.path());

        let err = config.server_config().unwrap_err();
        assert!(
            err.contains("expired"),
            "error should mention expiry: {err}"
        );
        assert!(
            err.contains("cert.pem"),
            "error should contain cert path: {err}"
        );
    }

    #[test]
    fn manual_key_mismatch_reports_paths() {
        let dir = tempdir().unwrap();
        let (cert_pem, _key_pem) = valid_pair();
        let (_other_cert_pem, other_key_pem) = valid_pair();
        write_pair(dir.path(), &cert_pem, &other_key_pem);
        let config = manual_config(dir.path());

        let err = config.server_config().unwrap_err();
        assert!(
            err.contains("does not match"),
            "error should mention mismatch: {err}"
        );
        assert!(
            err.contains("cert.pem"),
            "error should contain cert path: {err}"
        );
        assert!(
            err.contains("key.pem"),
            "error should contain key path: {err}"
        );
    }
}
