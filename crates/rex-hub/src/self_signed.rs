use anyhow::{Context, Result};
use rcgen::{CertificateParams, KeyPair};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::net::IpAddr;
use std::path::Path;

/// 自签名证书数据（证书 + 私钥的 DER 格式）
pub struct SelfSignedCert {
    pub cert_der: CertificateDer<'static>,
    pub key_der: PrivateKeyDer<'static>,
    pub cert_pem: String,
    pub key_pem: String,
}

/// 生成自签名证书
///
/// `sans` 应包含 IP 地址或 DNS 名称（如 "192.168.1.100"、"localhost"、"hub.local"）。
pub fn generate_self_signed(sans: &[String]) -> Result<SelfSignedCert> {
    let key_pair = KeyPair::generate().context("failed to generate key pair")?;

    let mut params =
        CertificateParams::new(sans.to_vec()).context("failed to create certificate params")?;
    params.distinguished_name = rcgen::DistinguishedName::new();

    // 设置有效期：1 年
    let now = time::OffsetDateTime::now_utc();
    params.not_before = now;
    params.not_after = now + time::Duration::days(365);

    let cert = params
        .self_signed(&key_pair)
        .context("failed to generate self-signed certificate")?;

    let cert_pem = cert.pem();
    let key_pem = key_pair.serialize_pem();
    let cert_der = cert.der().clone();
    let key_der = PrivateKeyDer::Pkcs8(rustls::pki_types::PrivatePkcs8KeyDer::from(
        key_pair.serialize_der(),
    ));

    Ok(SelfSignedCert {
        cert_der,
        key_der,
        cert_pem,
        key_pem,
    })
}

/// 从磁盘加载已有的自签名证书
pub fn load_self_signed(data_dir: &Path) -> Option<SelfSignedCert> {
    let dir = data_dir.join("self-signed");
    let cert_path = dir.join("cert.pem");
    let key_path = dir.join("cert_key.pem");

    if !cert_path.exists() || !key_path.exists() {
        return None;
    }

    let cert_pem = std::fs::read_to_string(&cert_path).ok()?;
    let key_pem = std::fs::read_to_string(&key_path).ok()?;

    // 解析 PEM → DER
    let cert_der = parse_cert_pem(&cert_pem).ok()?;
    let key_der = parse_key_pem(&key_pem).ok()?;

    Some(SelfSignedCert {
        cert_der,
        key_der,
        cert_pem,
        key_pem,
    })
}

/// 保存自签名证书到磁盘
pub fn save_self_signed(data_dir: &Path, cert: &SelfSignedCert) -> Result<()> {
    let dir = data_dir.join("self-signed");
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create dir: {}", dir.display()))?;

    let cert_path = dir.join("cert.pem");
    let key_path = dir.join("cert_key.pem");

    std::fs::write(&cert_path, &cert.cert_pem)
        .with_context(|| format!("failed to write {}", cert_path.display()))?;
    std::fs::write(&key_path, &cert.key_pem)
        .with_context(|| format!("failed to write {}", key_path.display()))?;

    Ok(())
}

fn parse_cert_pem(pem: &str) -> Result<CertificateDer<'static>> {
    let mut reader = std::io::BufReader::new(pem.as_bytes());
    let certs: Vec<_> = rustls_pemfile::certs(&mut reader)
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to parse certificate PEM")?;
    certs
        .into_iter()
        .next()
        .map(CertificateDer::from)
        .context("no certificate found in PEM")
}

fn parse_key_pem(pem: &str) -> Result<PrivateKeyDer<'static>> {
    let mut reader = std::io::BufReader::new(pem.as_bytes());
    let key = rustls_pemfile::private_key(&mut reader)
        .context("failed to parse private key PEM")?
        .context("no private key found in PEM")?;
    Ok(key.into())
}

/// 根据 Hub 监听地址推断 SAN 列表
///
/// 从 bind address（如 "0.0.0.0:3000" 或 "192.168.1.100:3000"）推断应包含的 SAN。
/// - 如果 bind 到具体 IP，使用该 IP + localhost
/// - 如果 bind 到 0.0.0.0 或 ::，使用 localhost + 自动探测的本机 IP
pub fn infer_self_signed_sans(listen: &str) -> Vec<String> {
    let host = listen.split(':').next().unwrap_or("0.0.0.0");
    let mut sans = vec!["localhost".to_string()];

    if host != "0.0.0.0" && host != "::" && !host.is_empty() {
        if host.parse::<IpAddr>().is_ok() {
            sans.push(host.to_string());
        }
    } else {
        // 绑定到通配地址时，自动探测本机 IP 加入 SAN
        if let Some(local_ip) = detect_local_ip() {
            let ip_str = local_ip.to_string();
            if !sans.contains(&ip_str) {
                sans.push(ip_str);
            }
        }
    }

    sans
}

/// 通过 UDP socket 探测本机 IP 地址（不发送任何数据）
fn detect_local_ip() -> Option<IpAddr> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    let addr = socket.local_addr().ok()?;
    Some(addr.ip())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_self_signed_with_dns() {
        let cert = generate_self_signed(&["localhost".to_string()]).unwrap();
        assert!(!cert.cert_pem.is_empty());
        assert!(!cert.key_pem.is_empty());
        assert!(cert.cert_pem.contains("BEGIN CERTIFICATE"));
        assert!(cert.key_pem.contains("BEGIN PRIVATE KEY"));
    }

    #[test]
    fn generate_self_signed_with_ip() {
        let cert =
            generate_self_signed(&["192.168.1.100".to_string(), "localhost".to_string()]).unwrap();
        assert!(!cert.cert_pem.is_empty());
        // 验证证书可以被解析
        let mut reader = std::io::BufReader::new(cert.cert_pem.as_bytes());
        let certs: Vec<_> = rustls_pemfile::certs(&mut reader)
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(certs.len(), 1);
    }

    #[test]
    fn save_and_load_self_signed() {
        let dir = tempfile::tempdir().unwrap();
        let cert = generate_self_signed(&["localhost".to_string()]).unwrap();

        save_self_signed(dir.path(), &cert).unwrap();

        let loaded = load_self_signed(dir.path()).unwrap();
        assert_eq!(loaded.cert_pem, cert.cert_pem);
        assert_eq!(loaded.key_pem, cert.key_pem);
    }

    #[test]
    fn load_self_signed_missing_dir() {
        let dir = tempfile::tempdir().unwrap();
        let result = load_self_signed(dir.path());
        assert!(result.is_none());
    }

    #[test]
    fn infer_sans_from_localhost() {
        let sans = infer_self_signed_sans("0.0.0.0:3000");
        // 应包含 localhost，且如果探测到本机 IP 则也会包含
        assert!(sans.contains(&"localhost".to_string()));
    }

    #[test]
    fn infer_sans_from_specific_ip() {
        let sans = infer_self_signed_sans("192.168.1.100:3000");
        assert_eq!(
            sans,
            vec!["localhost".to_string(), "192.168.1.100".to_string()]
        );
    }
}
