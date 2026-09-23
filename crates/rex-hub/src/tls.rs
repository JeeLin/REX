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

    fn load_cert_chain(
        &self,
    ) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>), String> {
        let (cert_path, key_path) = self.paths()?;
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

/// 构建显式 CryptoProvider。
///
/// 本 workspace 同时启用 rustls 的 `ring`（reqwest/hyper-rustls 链路）与
/// `aws-lc-rs`（rustls 默认特性）两个 provider，进程默认 provider 有歧义、
/// `ServerConfig::builder()` 会 panic，因此显式选择 ring（与 PRODUCT.md 对齐）。
fn tls_provider() -> Arc<rustls::crypto::CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
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

}
