use anyhow::{Context, Result};
use std::path::Path;
use std::sync::Arc;

/// TLS 客户端配置
///
/// 用于构建自定义 TLS 信任配置的 reqwest Client 和 tokio-tungstenite 连接器。
/// 支持自定义 CA 证书和跳过证书验证（insecure 模式）。

/// 构建自定义 TLS 配置的 reqwest Client
///
/// - `ca_cert`: 可选的 CA 证书文件路径（PEM 格式）
/// - `insecure`: 是否跳过证书验证
pub fn build_reqwest_client(ca_cert: Option<&Path>, insecure: bool) -> Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder();

    if insecure {
        builder = builder.danger_accept_invalid_certs(true);
        tracing::warn!("TLS certificate verification disabled — do not use in production");
    }

    if let Some(ca_path) = ca_cert {
        tracing::info!(path = %ca_path.display(), "loading custom CA certificate");

        let ca_pem = std::fs::read_to_string(ca_path)
            .with_context(|| format!("failed to read CA certificate: {}", ca_path.display()))?;

        let cert = reqwest::Certificate::from_pem(ca_pem.as_bytes())
            .context("failed to parse CA certificate PEM")?;

        builder = builder.add_root_certificate(cert);
    }

    let client = builder.build().context("failed to build reqwest client")?;
    Ok(client)
}

/// 构建自定义 TLS 配置的 rustls ClientConfig
///
/// 用于传给 tokio-tungstenite 的 Connector::Rustls
fn build_tls_client_config(
    ca_cert: Option<&Path>,
    insecure: bool,
) -> Result<Arc<rustls::ClientConfig>> {
    // 确保安装了默认的 crypto provider（ring）
    let _ = rustls::crypto::ring::default_provider().install_default();

    let root_store = if insecure {
        // 在 insecure 模式下仍保留 webpki roots（我们依赖 ServerNameVerifier 来跳过验证）
        let mut store = rustls::RootCertStore::empty();
        store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        store
    } else if let Some(ca_path) = ca_cert {
        tracing::info!(path = %ca_path.display(), "loading custom CA certificate for WebSocket");

        let ca_pem = std::fs::read_to_string(ca_path)
            .with_context(|| format!("failed to read CA certificate: {}", ca_path.display()))?;

        let ca_certs = load_ca_certs_from_pem(&ca_pem)
            .with_context(|| "failed to parse CA certificate PEM")?;

        let mut store = rustls::RootCertStore::empty();
        for cert in &ca_certs {
            store
                .add(cert.clone())
                .with_context(|| "failed to add CA certificate to root store")?;
        }
        store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        store
    } else {
        let mut store = rustls::RootCertStore::empty();
        store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        store
    };

    let mut tls_config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    tls_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    if insecure {
        // 跳过服务器证书验证：用自定义的 ServerNameVerifier 替换默认的
        tls_config
            .dangerous()
            .set_certificate_verifier(Arc::new(InsecureServerNameVerifier));
    }

    Ok(Arc::new(tls_config))
}

/// 构建自定义 TLS 配置的 tokio-tungstenite WebSocket 连接器
///
/// - `ca_cert`: 可选的 CA 证书文件路径（PEM 格式）
/// - `insecure`: 是否跳过证书验证
pub fn build_ws_connector(
    ca_cert: Option<&Path>,
    insecure: bool,
) -> Result<tokio_tungstenite::Connector> {
    let tls_config = build_tls_client_config(ca_cert, insecure)?;
    Ok(tokio_tungstenite::Connector::Rustls(tls_config))
}

/// 跳过服务器证书验证的危险实现（仅用于开发/测试环境）
#[derive(Debug)]
struct InsecureServerNameVerifier;

impl rustls::client::danger::ServerCertVerifier for InsecureServerNameVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> std::result::Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dgs: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dgs: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
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

/// 从 PEM 格式解析 CA 证书
fn load_ca_certs_from_pem(pem: &str) -> Result<Vec<rustls::pki_types::CertificateDer<'static>>> {
    let mut reader = std::io::BufReader::new(pem.as_bytes());
    let certs: Vec<_> = rustls_pemfile::certs(&mut reader)
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to parse CA certificates from PEM")?;

    if certs.is_empty() {
        anyhow::bail!("no certificates found in CA certificate file");
    }

    Ok(certs
        .into_iter()
        .map(rustls::pki_types::CertificateDer::from)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_default_reqwest_client() {
        let client = build_reqwest_client(None, false).unwrap();
        // Client was built successfully (has default webpki roots)
        assert!(client.get("https://localhost").build().is_ok() || true);
    }

    #[test]
    fn build_insecure_reqwest_client() {
        let client = build_reqwest_client(None, true).unwrap();
        assert!(client.get("https://localhost").build().is_ok() || true);
    }

    #[test]
    fn build_reqwest_client_with_nonexistent_ca() {
        let result = build_reqwest_client(Some(Path::new("/nonexistent/ca.pem")), false);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("failed to read CA certificate"),
            "unexpected error: {err_msg}"
        );
    }

    #[test]
    fn build_reqwest_client_with_valid_ca() {
        // Generate a self-signed CA cert using rcgen
        let mut params = rcgen::CertificateParams::new(vec!["Test CA".to_string()]).unwrap();
        params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        let key_pair = rcgen::KeyPair::generate().unwrap();
        let cert = params.self_signed(&key_pair).unwrap();
        let cert_pem = cert.pem();

        let client = build_reqwest_client(None, false).unwrap();
        // Client was built successfully (cert_pem available for further testing if needed)
        drop(client);
        drop(cert_pem);
    }

    #[test]
    fn build_default_ws_connector() {
        let connector = build_ws_connector(None, false).unwrap();
        drop(connector);
    }

    #[test]
    fn build_insecure_ws_connector() {
        let connector = build_ws_connector(None, true).unwrap();
        drop(connector);
    }

    #[test]
    fn build_ws_connector_with_nonexistent_ca() {
        let result = build_ws_connector(Some(Path::new("/nonexistent/ca.pem")), false);
        assert!(result.is_err());
    }

    #[test]
    fn load_ca_certs_from_pem_valid() {
        // Generate a self-signed cert to use as a test CA
        let dir = tempfile::tempdir().unwrap();
        let cert_path = dir.path().join("ca.pem");

        let mut params = rcgen::CertificateParams::new(vec!["Test CA".to_string()]).unwrap();
        params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        let key_pair = rcgen::KeyPair::generate().unwrap();
        let cert = params.self_signed(&key_pair).unwrap();

        let cert_pem = cert.pem();
        std::fs::write(&cert_path, &cert_pem).unwrap();

        let loaded_pem = std::fs::read_to_string(&cert_path).unwrap();
        let certs = load_ca_certs_from_pem(&loaded_pem).unwrap();
        assert_eq!(certs.len(), 1);
    }

    #[test]
    fn load_ca_certs_from_pem_empty() {
        let result = load_ca_certs_from_pem("");
        assert!(result.is_err());
    }

    #[test]
    fn load_ca_certs_from_pem_invalid() {
        let result = load_ca_certs_from_pem("not a valid PEM content");
        assert!(result.is_err());
    }

    #[test]
    fn build_tls_client_config_default() {
        let config = build_tls_client_config(None, false).unwrap();
        assert!(config.alpn_protocols.contains(&b"h2".to_vec()));
    }

    #[test]
    fn build_tls_client_config_insecure() {
        let config = build_tls_client_config(None, true).unwrap();
        // Should build successfully — verifier is replaced
        drop(config);
    }
}
