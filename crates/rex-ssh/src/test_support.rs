//! Shared test support: fixed host key and a minimal loopback SSH server.
//! Declared as `#[cfg(test)]` in `lib.rs`, so it is test-only.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use russh::keys::decode_secret_key;
use russh::server;

/// Fixed test-only host key (Ed25519) for building `server::Config::keys`.
pub(crate) const HOST_KEY_PEM: &str = r#"-----BEGIN OPENSSH PRIVATE KEY-----
b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW
QyNTUxOQAAACDu4wlRHuujJzs4p1gNufzdP8Sn/XiaM6ydBzASDNKtyQAAAJDrqQL166kC
9QAAAAtzc2gtZWQyNTUxOQAAACDu4wlRHuujJzs4p1gNufzdP8Sn/XiaM6ydBzASDNKtyQ
AAAEA7muFkuswZkYvrEUDbhXtDlUfD31ZM8GUS7P85sXisyO7jCVEe66MnOzinWA25/N0/
xKf9eJozrJ0HMBIM0q3JAAAADHJleC1zc2gtdGVzdAE=
-----END OPENSSH PRIVATE KEY-----"#;

/// Handler that only completes key exchange (no auth, no channels).
struct KexOnlyHandler;

impl server::Handler for KexOnlyHandler {
    type Error = russh::Error;
}

/// Spawn a minimal loopback SSH server and return its bound address.
pub(crate) async fn spawn_kex_server() -> SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback sshd");
    let addr = listener.local_addr().expect("local addr");

    let mut config = server::Config::default();
    config.auth_rejection_time = Duration::from_millis(10);
    config.keys =
        vec![decode_secret_key(&format!("{HOST_KEY_PEM}\n"), None).expect("decode host key")];
    let config = Arc::new(config);

    tokio::spawn(async move {
        loop {
            let Ok((socket, _)) = listener.accept().await else {
                break;
            };
            let config = config.clone();
            tokio::spawn(async move {
                let _ = server::run_stream(config, socket, KexOnlyHandler).await;
            });
        }
    });
    addr
}
