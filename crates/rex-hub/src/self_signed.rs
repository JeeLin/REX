use anyhow::{Context, Result};
use rcgen::{CertificateParams, KeyPair};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::net::IpAddr;
use std::path::Path;
use x509_parser::prelude::FromDer;

/// 自签名证书数据（证书 + 私钥的 DER 格式）
pub struct SelfSignedCert {
    pub cert_der: CertificateDer<'static>,
    pub key_der: PrivateKeyDer<'static>,
    pub cert_pem: String,
    pub key_pem: String,
}

/// 证书过期检测阈值（7 天内过期视为"即将过期"）
const EXPIRY_WARNING_DAYS: i64 = 7;

/// 解析证书的 notAfter 时间（UTC 毫秒时间戳）
fn parse_cert_not_after(cert_der: &[u8]) -> Result<i64> {
    let (_, cert) = x509_parser::certificate::X509Certificate::from_der(cert_der)
        .context("failed to parse X.509 certificate DER")?;
    let not_after_dt = cert
        .tbs_certificate
        .validity
        .not_after
        .to_datetime();
    // x509-parser 的 to_datetime() 返回 time::OffsetDateTime
    // 使用 unix_timestamp 获取秒级时间戳，再转毫秒
    let ts_secs = not_after_dt.unix_timestamp();
    Ok(ts_secs * 1000)
}

/// 获取当前 UTC 毫秒时间戳
fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// 检查证书是否有效（未过期且距过期 > 7 天）
///
/// 返回 `Ok(days_remaining)` 如果有效，`Err(reason)` 如果需要重新生成。
fn check_cert_validity(cert_der: &[u8]) -> Result<i64> {
    let not_after_ms = parse_cert_not_after(cert_der)?;
    let now_ms = now_millis();
    let days_remaining = (not_after_ms - now_ms) / (1000 * 60 * 60 * 24);

    if days_remaining <= 0 {
        anyhow::bail!("certificate expired");
    }
    if days_remaining <= EXPIRY_WARNING_DAYS {
        anyhow::bail!("certificate expiring soon ({days_remaining} days)");
    }
    Ok(days_remaining)
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

/// 加载或生成自签名证书，过期时自动重新生成
///
/// 优先级：
/// 1. 加载磁盘上的已有证书 → 验证有效期 → 有效则返回
/// 2. 已有证书过期/即将过期/损坏 → 重新生成并覆盖
/// 3. 无证书文件 → 生成新证书并保存
pub fn load_or_generate_self_signed(
    data_dir: &Path,
    sans: &[String],
) -> Result<SelfSignedCert> {
    if let Some(existing) = load_self_signed(data_dir) {
        match check_cert_validity(existing.cert_der.as_ref()) {
            Ok(days) => {
                tracing::info!(
                    days_remaining = days,
                    "loaded existing self-signed certificate"
                );
                return Ok(existing);
            }
            Err(e) => {
                tracing::warn!(
                    reason = %e,
                    "self-signed certificate invalid, regenerating"
                );
            }
        }
    }

    let cert = generate_self_signed(sans)?;
    save_self_signed(data_dir, &cert)?;
    tracing::info!("generated and saved new self-signed certificate");
    Ok(cert)
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

    #[test]
    fn parse_cert_not_after_returns_valid_timestamp() {
        let cert = generate_self_signed(&["localhost".to_string()]).unwrap();
        let not_after = parse_cert_not_after(cert.cert_der.as_ref()).unwrap();
        let now = now_millis();
        // notAfter should be ~365 days in the future (within 1 day tolerance)
        let days_diff = (not_after - now) / (1000 * 60 * 60 * 24);
        assert!(days_diff >= 364 && days_diff <= 366, "expected ~365 days, got {days_diff}");
    }

    #[test]
    fn check_cert_validity_valid_cert() {
        let cert = generate_self_signed(&["localhost".to_string()]).unwrap();
        let days = check_cert_validity(cert.cert_der.as_ref()).unwrap();
        assert!(days > 350, "expected >350 days remaining, got {days}");
    }

    #[test]
    fn load_or_generate_creates_new_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let sans = vec!["localhost".to_string()];
        let result = load_or_generate_self_signed(dir.path(), &sans).unwrap();
        assert!(!result.cert_pem.is_empty());
        // Verify file was saved
        assert!(dir.path().join("self-signed/cert.pem").exists());
        assert!(dir.path().join("self-signed/cert_key.pem").exists());
    }

    #[test]
    fn load_or_generate_loads_existing_valid_cert() {
        let dir = tempfile::tempdir().unwrap();
        let sans = vec!["localhost".to_string()];

        // First call: generate and save
        let cert1 = load_or_generate_self_signed(dir.path(), &sans).unwrap();
        // Second call: should load existing (same cert)
        let cert2 = load_or_generate_self_signed(dir.path(), &sans).unwrap();
        assert_eq!(cert1.cert_pem, cert2.cert_pem);
        assert_eq!(cert1.key_pem, cert2.key_pem);
    }

    #[test]
    fn load_or_generate_regenerates_corrupted_cert() {
        let dir = tempfile::tempdir().unwrap();
        let sans = vec!["localhost".to_string()];

        // Generate and save a valid cert
        let cert1 = load_or_generate_self_signed(dir.path(), &sans).unwrap();

        // Corrupt the cert file
        let cert_path = dir.path().join("self-signed/cert.pem");
        std::fs::write(&cert_path, "not a valid cert").unwrap();

        // Should regenerate (different cert PEM)
        let cert2 = load_or_generate_self_signed(dir.path(), &sans).unwrap();
        assert_ne!(cert1.cert_pem, cert2.cert_pem);
    }
}
