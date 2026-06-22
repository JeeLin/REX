use anyhow::{Context, Result};
use hyper_util::rt::TokioExecutor;
use hyper_util::server::conn::auto::Builder;
use hyper_util::service::TowerToHyperService;
use rustls::ServerConfig;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

/// 从证书和私钥文件构建 TLS acceptor
pub fn create_tls_acceptor(cert_path: &Path, key_path: &Path) -> Result<TlsAcceptor> {
    let cert_file = File::open(cert_path)
        .with_context(|| format!("failed to open TLS certificate: {}", cert_path.display()))?;
    let mut cert_reader = BufReader::new(cert_file);
    let certs: Vec<_> = rustls_pemfile::certs(&mut cert_reader)
        .collect::<std::result::Result<Vec<_>, _>>()
        .with_context(|| format!("failed to parse TLS certificate: {}", cert_path.display()))?
        .into_iter()
        .map(rustls::pki_types::CertificateDer::from)
        .collect();

    let key_file = File::open(key_path)
        .with_context(|| format!("failed to open TLS private key: {}", key_path.display()))?;
    let mut key_reader = BufReader::new(key_file);
    let key_der = rustls_pemfile::private_key(&mut key_reader)
        .with_context(|| format!("failed to parse TLS private key: {}", key_path.display()))?
        .context("no private key found in key file")?;

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key_der.into())
        .map_err(|e| anyhow::anyhow!("failed to build TLS server config: {e}"))?;

    Ok(TlsAcceptor::from(std::sync::Arc::new(config)))
}

/// 运行 TLS HTTP 服务器：接受 TCP 连接 → TLS 握手 → hyper 处理 HTTP
pub async fn run_tls_server(
    bind_addr: &str,
    tls_acceptor: TlsAcceptor,
    app: axum::Router,
) -> Result<()> {
    let listener = TcpListener::bind(bind_addr)
        .await
        .with_context(|| format!("failed to bind to {bind_addr}"))?;
    let tls_acceptor = std::sync::Arc::new(tls_acceptor);
    tracing::info!(addr = bind_addr, "TLS server listening");

    loop {
        let (tcp_stream, addr) = listener.accept().await?;
        let acceptor = tls_acceptor.clone();
        let app = app.clone();

        tokio::spawn(async move {
            let tls_stream = match acceptor.accept(tcp_stream).await {
                Ok(s) => s,
                Err(e) => {
                    tracing::debug!("TLS handshake failed from {addr}: {e}");
                    return;
                }
            };

            let io = hyper_util::rt::TokioIo::new(tls_stream);
            let service = TowerToHyperService::new(app.into_service());

            if let Err(e) = Builder::new(TokioExecutor::new())
                .serve_connection_with_upgrades(io, service)
                .await
            {
                tracing::debug!("connection error from {addr}: {e}");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acceptor_invalid_cert_path() {
        let result = create_tls_acceptor(
            Path::new("/nonexistent/cert.pem"),
            Path::new("/nonexistent/key.pem"),
        );
        assert!(result.is_err());
    }
}
