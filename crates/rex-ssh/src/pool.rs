//! 进程内 SSH 连接池 — 让 SFTP 复用终端已建立的连接，避免触发服务端
//! 并发会话限制（MaxSessions）或被服务端直接掐断第二条连接。

use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};

use russh::client;
use tokio::sync::Mutex;

use crate::{SshConfig, SshHandler};

/// 池中的连接单元；`client::Handle` 不可 Clone，用 `Arc<Mutex<_>>` 共享。
pub(crate) type PooledHandle = Arc<Mutex<client::Handle<SshHandler>>>;

/// Idle connections older than this TTL are dropped on the next `get`,
/// so a host that was visited once no longer leaks an open connection
/// until process exit (or same-key overwrite).
const IDLE_TTL: Duration = Duration::from_secs(30 * 60);

/// Pool cell plus the timestamp of its last use (`register` / `get`).
struct Entry {
    cell: PooledHandle,
    last_used: Instant,
}

static POOL: LazyLock<Mutex<HashMap<String, Entry>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// 计算连接池键：`user@host:port`。
pub(crate) fn pool_key(config: &SshConfig) -> String {
    format!("{}@{}:{}", config.username, config.host, config.port)
}

/// 注册连接；同键覆盖旧连接（旧连接的生命周期由其持有方负责）。
pub(crate) async fn register(key: &str, handle: client::Handle<SshHandler>) {
    tracing::debug!(key = %key, "pooled SSH handle registered");
    POOL.lock().await.insert(
        key.to_string(),
        Entry {
            cell: Arc::new(Mutex::new(handle)),
            last_used: Instant::now(),
        },
    );
}

/// 取出仍然存活的池连接；连接已关闭则剔除并返回 `None`。
///
/// Sweeps entries idle longer than [`IDLE_TTL`] first; once the pool drops its
/// reference, the connection closes with its last Sender, so a still-open
/// terminal channel keeps its connection alive.
pub(crate) async fn get(key: &str) -> Option<PooledHandle> {
    let mut guard = POOL.lock().await;
    sweep_expired(&mut guard);
    let entry = guard.get_mut(key)?;
    entry.last_used = Instant::now();
    let cell = entry.cell.clone();
    drop(guard);

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
    if guard.get(key).is_some_and(|e| Arc::ptr_eq(&e.cell, cell)) {
        guard.remove(key);
        tracing::debug!(key = %key, "pooled SSH handle evicted");
    }
}

/// Lazily drop entries idle for longer than [`IDLE_TTL`] (runs on `get`).
fn sweep_expired(pool: &mut HashMap<String, Entry>) {
    let now = Instant::now();
    pool.retain(|key, entry| {
        let idle = now.saturating_duration_since(entry.last_used);
        if idle <= IDLE_TTL {
            return true;
        }
        tracing::debug!(key = %key, idle_secs = idle.as_secs(), "idle pooled SSH handle expired");
        false
    });
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
    use std::net::SocketAddr;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::test_support;

    fn config(username: &str, host: &str, port: u16) -> SshConfig {
        SshConfig {
            host: host.to_string(),
            port,
            username: username.to_string(),
            password: Some("pw".into()),
            private_key: None,
            keepalive_interval: Some(0),
            init_script: None,
        }
    }

    /// Unique key per test so parallel tests stay isolated in the process-global POOL.
    fn unique_key() -> String {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        format!("pool-test-{id}@127.0.0.1:22")
    }

    /// Establish a live (key-exchange only) handle against the loopback test server.
    async fn live_handle(addr: SocketAddr) -> client::Handle<SshHandler> {
        let ssh_config = Arc::new(client::Config::default());
        let target = format!("127.0.0.1:{}", addr.port());
        client::connect(ssh_config, &target, SshHandler)
            .await
            .expect("test client connect")
    }

    #[test]
    fn pool_key_contains_user_host_port() {
        assert_eq!(
            pool_key(&config("root", "10.0.0.1", 22)),
            "root@10.0.0.1:22"
        );
        assert_eq!(pool_key(&config("root", "::1", 2222)), "root@::1:2222");
    }

    #[tokio::test]
    async fn get_returns_fresh_entry() {
        let addr = test_support::spawn_kex_server().await;
        let key = unique_key();
        register(&key, live_handle(addr).await).await;

        assert!(get(&key).await.is_some(), "fresh entry must be served");
        assert!(POOL.lock().await.contains_key(&key));
    }

    /// Rewind `last_used` past the idle TTL: `get` must lazily evict the entry.
    #[tokio::test]
    async fn get_evicts_entry_idle_past_ttl() {
        let addr = test_support::spawn_kex_server().await;
        let key = unique_key();
        register(&key, live_handle(addr).await).await;

        {
            let mut guard = POOL.lock().await;
            let entry = guard.get_mut(&key).expect("entry registered");
            let past = Instant::now().checked_sub(IDLE_TTL + Duration::from_secs(1));
            entry.last_used = past.expect("monotonic clock must reach back past IDLE_TTL");
        }

        assert!(
            get(&key).await.is_none(),
            "expired entry must be evicted, not served"
        );
        assert!(!POOL.lock().await.contains_key(&key));
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
