use rex_common::app;
use rex_common::Parser;
use rex_hub::acme::{self, TlsMode};
use rex_hub::config::HubConfig;
use rex_hub::db::Database;
use rex_hub::routes;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tower::{Service, ServiceExt};

fn main() -> anyhow::Result<()> {
    // Load environment variables from .env file if it exists
    let _ = dotenvy::dotenv();

    app::init_tracing();
    tracing::info!("rex-hub starting");

    let args: Vec<String> = std::env::args().collect();
    let cli = rex_common::cli::Cli::parse_from(&args);

    if cli.worker {
        tracing::info!("worker started");
        let config = HubConfig::load(
            cli.config.as_deref(),
            cli.tls_cert.as_deref(),
            cli.tls_key.as_deref(),
            cli.acme_domain.as_deref(),
            cli.acme_email.as_deref(),
            cli.acme_staging,
            cli.acme_http_port,
        )?;

        let db_path = config.data_dir.join("hub.db");
        std::fs::create_dir_all(&config.data_dir)?;
        let db = Database::new(&db_path)?;

        let secret_key = config.secret_key.clone();
        if secret_key.is_empty() {
            tracing::warn!("secret_key is empty, using default - this is insecure in production");
        }

        // 确定 TLS 模式（优先级：manual > acme > self-signed > none）
        let tls_mode = acme::determine_tls_mode(&config);

        let rt = tokio::runtime::Runtime::new()?;
        let static_dir = config.static_dir.clone();
        let data_dir = config.data_dir.clone();
        let shared_acme_status = acme::new_shared_acme_status();
        let app = routes::app_with_static(
            Arc::new(db),
            secret_key,
            static_dir,
            data_dir,
            config.clone(),
            shared_acme_status.clone(),
        );

        match tls_mode {
            TlsMode::Manual => {
                let tls = config.tls.as_ref().unwrap();
                tracing::info!(
                    listen = %config.listen,
                    cert = %tls.cert.display(),
                    key = %tls.key.display(),
                    "TLS mode: manual — serving HTTPS"
                );
                let tls_acceptor = rex_hub::tls::create_tls_acceptor(&tls.cert, &tls.key)?;
                rt.block_on(rex_hub::tls::run_tls_server(
                    &config.listen,
                    tls_acceptor,
                    app,
                ))?;
            }
            TlsMode::AcmeDomain | TlsMode::AcmeIp => {
                let acme_cfg = config.acme.as_ref().unwrap().clone();
                let challenge = acme::challenge_description(&acme_cfg.domain);

                tracing::info!(
                    listen = %config.listen,
                    domain = %acme_cfg.domain,
                    challenge = challenge,
                    "TLS mode: acme — requesting cert"
                );

                let listen = config.listen.clone();
                let http_port = acme_cfg.http_port;

                rt.block_on(async move {
                    let (default_config, challenge_config, http01_service) =
                        acme::start_acme_driver(
                            acme_cfg,
                            config.data_dir.clone(),
                            shared_acme_status.clone(),
                        )
                        .await?;

                    // TLS-ALPN-01：使用 challenge config 接受连接（支持 ALPN 协商）
                    // HTTP-01：使用 default config（普通 TLS 连接）
                    let server_config = challenge_config.unwrap_or(default_config);
                    let tls_acceptor =
                        rex_hub::tls::create_tls_acceptor_from_config((*server_config).clone());

                    // HTTP-01 需要单独端口的 listener
                    if let Some(http01_service) = http01_service {
                        tokio::spawn(async move {
                            let bind_addr = format!("0.0.0.0:{}", http_port);
                            let listener =
                                tokio::net::TcpListener::bind(&bind_addr).await;
                            match listener {
                                Ok(listener) => {
                                    tracing::info!(port = http_port, "HTTP-01 challenge server listening");
                                    loop {
                                        let (mut stream, _) = match listener.accept().await {
                                            Ok(s) => s,
                                            Err(e) => {
                                                tracing::error!(error = %e, "HTTP-01 accept error");
                                                continue;
                                            }
                                        };
                                        let mut svc = http01_service.clone();
                                        tokio::spawn(async move {
                                            let mut buf = [0u8; 2048];
                                            let n = stream.read(&mut buf).await.unwrap_or(0);
                                            let request = String::from_utf8_lossy(&buf[..n]);
                                            let path = request
                                                .lines()
                                                .next()
                                                .and_then(|line| line.split_whitespace().nth(1))
                                                .unwrap_or("/");

                                            let req = axum::http::Request::builder()
                                                .uri(path)
                                                .body(String::new())
                                                .unwrap();
                                            let resp = <rustls_acme::tower::TowerHttp01ChallengeService as ServiceExt<axum::http::Request<String>>>::ready(&mut svc)
                                                .await?
                                                .call(req)
                                                .await?;
                                            let (_, body) = resp.into_parts();
                                            let response = format!(
                                                "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n{}",
                                                body.len(),
                                                body
                                            );
                                            stream.write_all(response.as_bytes()).await.ok();
                                            Ok::<(), anyhow::Error>(())
                                        });
                                    }
                                }
                                Err(e) => {
                                    tracing::error!(error = %e, port = http_port, "failed to bind HTTP-01 challenge server");
                                }
                            }
                        });
                    }

                    // 启动主 TLS 服务器
                    rex_hub::tls::run_tls_server(&listen, tls_acceptor, app).await
                })?;
            }
            TlsMode::SelfSigned => {
                let sans = rex_hub::self_signed::infer_self_signed_sans(&config.listen);
                tracing::info!(
                    listen = %config.listen,
                    sans = ?sans,
                    "TLS mode: self-signed — checking certificate"
                );

                let cert = rex_hub::self_signed::load_or_generate_self_signed(
                    &config.data_dir,
                    &sans,
                )?;

                let tls_acceptor = rex_hub::tls::create_tls_acceptor_from_config(
                    rustls::ServerConfig::builder_with_provider(
                        rustls::crypto::ring::default_provider().into(),
                    )
                    .with_safe_default_protocol_versions()
                    .map_err(|e| anyhow::anyhow!("failed to build TLS config: {e}"))?
                    .with_no_client_auth()
                    .with_single_cert(vec![cert.cert_der], cert.key_der)
                    .map_err(|e| anyhow::anyhow!("failed to build TLS config: {e}"))?,
                );

                tracing::info!(listen = %config.listen, "TLS mode: self-signed — serving HTTPS");
                rt.block_on(rex_hub::tls::run_tls_server(
                    &config.listen,
                    tls_acceptor,
                    app,
                ))?;
            }
            TlsMode::None => {
                tracing::info!(listen = %config.listen, "TLS mode: none — HTTP only");
                rt.block_on(async {
                    let listener = tokio::net::TcpListener::bind(&config.listen).await?;
                    axum::serve(listener, app).await?;
                    Ok::<(), anyhow::Error>(())
                })?;
            }
        }
    } else {
        app::run_from(args)?;
    }

    Ok(())
}

/// 从 "host:port" 格式提取端口号
fn extract_port(addr: &str) -> Option<u16> {
    addr.rsplit_once(':')
        .and_then(|(_, port)| port.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rex_hub::config::{AcmeConfig, TlsConfig};
    use std::path::PathBuf;

    #[test]
    fn determine_tls_mode_manual() {
        let dir = tempfile::tempdir().unwrap();
        let cert = dir.path().join("cert.pem");
        let key = dir.path().join("key.pem");
        std::fs::write(&cert, "cert").unwrap();
        std::fs::write(&key, "key").unwrap();
        let config = HubConfig {
            tls: Some(TlsConfig { cert, key }),
            ..Default::default()
        };
        assert_eq!(acme::determine_tls_mode(&config), TlsMode::Manual);
    }

    #[test]
    fn determine_tls_mode_acme_domain() {
        let config = HubConfig {
            acme: Some(AcmeConfig {
                domain: "hub.example.com".to_string(),
                email: "admin@example.com".to_string(),
                staging: false,
                http_port: 80,
            }),
            ..Default::default()
        };
        assert_eq!(acme::determine_tls_mode(&config), TlsMode::AcmeDomain);
    }

    #[test]
    fn determine_tls_mode_acme_ip() {
        let config = HubConfig {
            acme: Some(AcmeConfig {
                domain: "203.0.113.1".to_string(),
                email: "admin@example.com".to_string(),
                staging: false,
                http_port: 80,
            }),
            ..Default::default()
        };
        assert_eq!(acme::determine_tls_mode(&config), TlsMode::AcmeIp);
    }

    #[test]
    fn determine_tls_mode_none_when_no_config() {
        let config = HubConfig::default();
        assert_eq!(acme::determine_tls_mode(&config), TlsMode::None);
    }

    #[test]
    fn determine_tls_mode_manual_takes_priority_over_acme() {
        let dir = tempfile::tempdir().unwrap();
        let cert = dir.path().join("cert.pem");
        let key = dir.path().join("key.pem");
        std::fs::write(&cert, "cert").unwrap();
        std::fs::write(&key, "key").unwrap();
        let config = HubConfig {
            tls: Some(TlsConfig { cert, key }),
            acme: Some(AcmeConfig {
                domain: "hub.example.com".to_string(),
                email: "admin@example.com".to_string(),
                staging: false,
                http_port: 80,
            }),
            ..Default::default()
        };
        assert_eq!(acme::determine_tls_mode(&config), TlsMode::Manual);
    }

    #[test]
    fn determine_tls_mode_manual_missing_files_falls_back() {
        let config = HubConfig {
            tls: Some(TlsConfig {
                cert: PathBuf::from("/nonexistent/cert.pem"),
                key: PathBuf::from("/nonexistent/key.pem"),
            }),
            ..Default::default()
        };
        assert_eq!(acme::determine_tls_mode(&config), TlsMode::None);
    }

    #[test]
    fn extract_port_from_addr() {
        assert_eq!(extract_port("0.0.0.0:3000"), Some(3000));
        assert_eq!(extract_port("127.0.0.1:8080"), Some(8080));
        assert_eq!(extract_port("hub.example.com:443"), Some(443));
        assert_eq!(extract_port("no-port"), None);
    }
}
