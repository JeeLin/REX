//! 进程内 SSH 连接池 — 让 SFTP 复用终端已建立的连接，避免触发服务端
//! 并发会话限制（MaxSessions）或被服务端直接掐断第二条连接。

use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

use russh::client;
use tokio::sync::Mutex;

use crate::{SshConfig, SshHandler};

/// 池中的连接单元；`client::Handle` 不可 Clone，用 `Arc<Mutex<_>>` 共享。
pub(crate) type PooledHandle = Arc<Mutex<client::Handle<SshHandler>>>;

static POOL: LazyLock<Mutex<HashMap<String, PooledHandle>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// 计算连接池键：`user@host:port`。
///
/// ProxyJump 链路的连接走的是跳板机通道，`SftpConnector::connect_with_config`
/// 无法直连复用，因此不入池。
pub(crate) fn pool_key(config: &SshConfig) -> Option<String> {
    if config.proxy_jump.is_some() {
        return None;
    }
    Some(format!(
        "{}@{}:{}",
        config.username, config.host, config.port
    ))
}

/// 注册连接；同键覆盖旧连接（旧连接的生命周期由其持有方负责）。
pub(crate) async fn register(key: &str, handle: client::Handle<SshHandler>) {
    tracing::debug!(key = %key, "pooled SSH handle registered");
    POOL.lock()
        .await
        .insert(key.to_string(), Arc::new(Mutex::new(handle)));
}

/// 取出仍然存活的池连接；连接已关闭则剔除并返回 `None`。
pub(crate) async fn get(key: &str) -> Option<PooledHandle> {
    let cell = POOL.lock().await.get(key).cloned()?;
    let closed = cell.lock().await.is_closed();
    if closed {
        evict(key, &cell).await;
        return None;
    }
    Some(cell)
}

/// 剔除池连接；仅当仍是同一连接时移除，避免误删已被替换的新连接。
pub(crate) async fn evict(key: &str, cell: &PooledHandle) {
    let mut guard = POOL.lock().await;
    if guard.get(key).is_some_and(|c| Arc::ptr_eq(c, cell)) {
        guard.remove(key);
        tracing::debug!(key = %key, "pooled SSH handle evicted");
    }
}

/// 池连接打开 channel 失败时是否应剔除该连接。
///
/// - [`russh::Error::ChannelOpenFailure`]：服务端明确拒绝（如 `MaxSessions=1`），
///   连接本身仍然可用 → 保留，仅降级为新建连接；
/// - 其余（断开、超时、协议错乱等）：连接已不可用 → 剔除。
pub(crate) fn should_evict(err: &russh::Error) -> bool {
    !matches!(err, russh::Error::ChannelOpenFailure(_))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(username: &str, host: &str, port: u16, proxy_jump: Option<&str>) -> SshConfig {
        SshConfig {
            host: host.to_string(),
            port,
            username: username.to_string(),
            password: Some("pw".into()),
            private_key: None,
            keepalive_interval: Some(0),
            init_script: None,
            proxy_jump: proxy_jump.map(str::to_string),
        }
    }

    #[test]
    fn pool_key_contains_user_host_port() {
        assert_eq!(
            pool_key(&config("root", "10.0.0.1", 22, None)).as_deref(),
            Some("root@10.0.0.1:22")
        );
        assert_eq!(
            pool_key(&config("root", "::1", 2222, None)).as_deref(),
            Some("root@::1:2222")
        );
    }

    #[test]
    fn pool_key_skips_proxy_jump_chain() {
        assert_eq!(
            pool_key(&config("root", "10.0.0.1", 22, Some("jump.example.com:22"))),
            None
        );
    }

    #[test]
    fn should_evict_keeps_connection_on_channel_open_rejection() {
        // 服务端拒绝 channel（如 MaxSessions=1）时连接仍存活，不应剔除
        assert!(!should_evict(&russh::Error::ChannelOpenFailure(
            russh::ChannelOpenFailure::AdministrativelyProhibited
        )));
    }

    #[test]
    fn should_evict_drops_broken_connection() {
        assert!(should_evict(&russh::Error::Disconnect));
        assert!(should_evict(&russh::Error::Inconsistent));
        assert!(should_evict(&russh::Error::NotAuthenticated));
    }
}
