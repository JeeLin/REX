//! SFTP 文件连接器 — 基于 russh-sftp 实现 FileConnector。

use anyhow::{Context, Result};
use async_trait::async_trait;
use rex_common::file_transfer::{FileConnector, FileEntry, ProgressCallback, UploadResult};
use russh_sftp::client::SftpSession;
use russh_sftp::protocol::{FileAttributes, OpenFlags};
use std::io::SeekFrom;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};

/// SFTP 连接器
pub struct SftpConnector {
    session: SftpSession,
}

impl SftpConnector {
    /// 从 SSH channel 建立 SFTP 连接
    ///
    /// 必须在握手前请求 `sftp` 子系统：session channel 打开后远端只是建好了一条
    /// 空 channel，不会自行启动 `sftp-server`。缺这一步时客户端发出的
    /// SSH_FXP_INIT 无人应答，`SftpSession::new` 只能等到超时并报
    /// "failed to create SFTP session"。所有入口（`connect_from_handle` 复用池化
    /// 句柄 / `open_session_channel` 新建连接）都收敛到这里。
    pub async fn connect(channel: russh::Channel<russh::client::Msg>) -> Result<Self> {
        channel
            .request_subsystem(true, "sftp")
            .await
            .context("failed to request SFTP subsystem")?;
        let session = SftpSession::new(channel.into_stream())
            .await
            .context("failed to create SFTP session")?;
        Ok(Self { session })
    }

    /// 从已有的 SSH Handle 打开新 session channel 建立 SFTP 连接
    /// 用于复用已有的 SSH 连接（避免并发会话限制）
    pub async fn connect_from_handle(
        handle: &russh::client::Handle<crate::SshHandler>,
        host: &str,
    ) -> Result<Self> {
        tracing::info!(action = "SFTP_CONNECT", host = %host, "SFTP: opening session channel from existing SSH handle");
        let channel = handle
            .channel_open_session()
            .await
            .map_err(|e| {
                tracing::error!(action = "SFTP_CONNECT", host = %host, error = %e, "SFTP: channel_open_session failed from existing handle");
                crate::session_open_error("pooled connection", e)
            })?;
        tracing::info!(action = "SFTP_CONNECT", host = %host, "SFTP: session channel opened from existing handle, creating SFTP session");
        Self::connect(channel).await
    }

    /// 从 SSH 配置直接建立 SFTP 连接
    ///
    /// 优先复用连接池中已有的连接（同 `user@host:port`），复用失败则降级为新建连接。
    pub async fn connect_with_config(config: crate::SshConfig) -> Result<Self> {
        tracing::info!(action = "SFTP_CONNECT", host = %config.host, port = config.port, username = %config.username, has_password = config.password.is_some(), has_key = config.private_key.is_some(), "SFTP: opening session channel");
        let channel = open_session_channel(&config).await?;
        tracing::info!(action = "SFTP_CONNECT", host = %config.host, "SFTP: session channel opened, creating SFTP session");
        Self::connect(channel).await
    }
}

/// 池连接 / 新连接打开 session channel 的超时
const OPEN_SESSION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

/// 打开一个 session channel：优先复用连接池中的连接，失败则降级为新建连接。
///
/// 1. 池命中 → 在同一条 TCP 连接上打开新 channel（不触发服务端并发会话限制）；
/// 2. 池打开失败 → 按 [`crate::pool::should_evict`] 决定是否剔除该连接，随后降级；
/// 3. 降级 → 新建连接、认证、打开 channel，失败返回统一的 MaxSessions 指引文案。
pub(crate) async fn open_session_channel(
    config: &crate::SshConfig,
) -> Result<russh::Channel<russh::client::Msg>> {
    let key = crate::pool::pool_key(config);
    if let Some(cell) = crate::pool::get(&key).await {
        tracing::info!(action = "SFTP_CONNECT", key = %key, "SFTP: reusing pooled SSH connection");
        let opened = tokio::time::timeout(OPEN_SESSION_TIMEOUT, async {
            let handle = cell.lock().await;
            handle.channel_open_session().await
        })
        .await;
        match opened {
            Ok(Ok(channel)) => return Ok(channel),
            Ok(Err(e)) => {
                tracing::warn!(action = "SFTP_CONNECT", key = %key, error = %e, "SFTP: pooled channel open failed, falling back to a new connection");
                if crate::pool::should_evict(&e) {
                    crate::pool::evict(&key, &cell).await;
                }
            }
            Err(_) => {
                tracing::warn!(action = "SFTP_CONNECT", key = %key, "SFTP: pooled channel open timed out, falling back to a new connection");
                crate::pool::evict(&key, &cell).await;
            }
        }
    }

    // 降级：新建连接
    let handle = fresh_handle(config).await?;
    let channel = handle
        .channel_open_session()
        .await
        .map_err(|e| {
            tracing::error!(action = "SFTP_CONNECT", host = %config.host, port = config.port, error = %e, "SFTP: channel_open_session failed on new connection");
            crate::session_open_error("new connection", e)
        })?;
    crate::pool::register(&crate::pool::pool_key(config), handle).await;
    Ok(channel)
}

/// 新建一条 SSH 连接并完成认证（不打开 channel）
async fn fresh_handle(
    config: &crate::SshConfig,
) -> Result<russh::client::Handle<crate::SshHandler>> {
    use russh::client;
    use std::sync::Arc;

    let ssh_config = Arc::new(client::Config::default());
    let handler = crate::SshHandler;
    let addr = crate::format_ssh_addr(&config.host, config.port);
    tracing::info!(action = "SFTP_CONNECT", host = %config.host, port = config.port, "SFTP: opening new SSH connection");
    let mut handle = client::connect(ssh_config, &addr, handler)
        .await
        .map_err(|e| {
            tracing::error!(action = "SFTP_CONNECT", host = %config.host, port = config.port, error = %e, "SFTP: SSH connection failed");
            anyhow::anyhow!("SSH connection failed for SFTP: {e}")
        })?;
    tracing::info!(action = "SFTP_CONNECT", host = %config.host, "SFTP: SSH connected, authenticating");

    crate::authenticate(&mut handle, config).await?;

    tracing::info!(action = "SFTP_CONNECT", host = %config.host, "SFTP: auth ok");
    Ok(handle)
}

#[async_trait]
impl FileConnector for SftpConnector {
    async fn list(&mut self, path: &str) -> Result<Vec<FileEntry>> {
        let dir = self
            .session
            .read_dir(path)
            .await
            .context("failed to list directory")?;

        let mut entries = Vec::new();
        for entry in dir {
            let name = entry.file_name();
            if name == "." || name == ".." {
                continue;
            }
            let meta = entry.metadata();
            let full_path = if path.ends_with('/') {
                format!("{path}{name}")
            } else {
                format!("{path}/{name}")
            };
            let modified = meta.modified().ok().and_then(|t| {
                let dur = t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
                chrono::DateTime::from_timestamp(dur.as_secs() as i64, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            });
            entries.push(FileEntry {
                name,
                path: full_path,
                is_dir: meta.is_dir(),
                size: meta.len(),
                modified,
                permissions: Some(format!("{}", meta.permissions())),
                storage_class: None,
                acl: None,
            });
        }
        entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
        Ok(entries)
    }

    async fn stat(&mut self, path: &str) -> Result<FileEntry> {
        let name = path.rsplit('/').next().unwrap_or(path).to_string();
        // 查询元数据以填充权限位（SFTP v3 无独立 stat API，metadata() 走 SSH_FXP_STAT）。
        let permissions = self
            .session
            .metadata(path)
            .await
            .ok()
            .map(|attrs| format!("{}", attrs.permissions()));
        match self.session.canonicalize(path).await {
            Ok(resolved) => Ok(FileEntry {
                name,
                path: resolved,
                is_dir: false,
                size: 0,
                modified: None,
                permissions,
                storage_class: None,
                acl: None,
            }),
            Err(_) => Ok(FileEntry {
                name,
                path: path.to_string(),
                is_dir: false,
                size: 0,
                modified: None,
                permissions,
                storage_class: None,
                acl: None,
            }),
        }
    }

    async fn upload(
        &mut self,
        remote_path: &str,
        data: Vec<u8>,
        offset: u64,
        progress: Option<&ProgressCallback>,
    ) -> Result<UploadResult> {
        let total = data.len() as u64;

        // Clamp offset to data length to prevent panic
        let offset = offset.min(total);

        // offset > 0: 打开已有文件用于续传；否则创建新文件。
        // SFTP APPEND 在 EOF 追加，不能 byte-accurate 恢复指定 offset；
        // 改用 WRITE|CREAT + seek 到 offset，镜像 S3 multipart resume 语义。
        let mut file = if offset > 0 {
            self.session
                .open_with_flags(remote_path, OpenFlags::WRITE | OpenFlags::CREATE)
                .await
                .with_context(|| format!("failed to open {remote_path} for resume"))?
        } else {
            self.session
                .create(remote_path)
                .await
                .with_context(|| format!("failed to create {remote_path}"))?
        };

        if offset > 0 {
            file.seek(SeekFrom::Start(offset))
                .await
                .with_context(|| format!("failed to seek to offset {offset} in {remote_path}"))?;
        }

        let chunk_size = 64 * 1024;
        let mut written = offset;
        // Skip already-uploaded data
        let start = offset as usize;
        for chunk in data[start..].chunks(chunk_size) {
            file.write_all(chunk)
                .await
                .context("failed to write chunk")?;
            written += chunk.len() as u64;
            if let Some(ref cb) = progress {
                cb(written, total);
            }
        }
        file.flush().await.context("failed to flush")?;
        Ok(UploadResult::default())
    }

    async fn download(&mut self, path: &str) -> Result<Vec<u8>> {
        self.session
            .read(path)
            .await
            .with_context(|| format!("failed to read {path}"))
    }

    async fn download_range(
        &mut self,
        path: &str,
        offset: u64,
        limit: Option<u64>,
    ) -> Result<Vec<u8>> {
        let all_data = self
            .session
            .read(path)
            .await
            .with_context(|| format!("failed to read {path}"))?;
        let start = (offset as usize).min(all_data.len());
        let end = match limit {
            Some(len) => ((offset + len) as usize).min(all_data.len()),
            None => all_data.len(),
        };
        Ok(all_data[start..end].to_vec())
    }

    async fn delete(&mut self, path: &str) -> Result<()> {
        if self.session.remove_file(path).await.is_err() {
            self.session
                .remove_dir(path)
                .await
                .with_context(|| format!("failed to delete {path}"))?;
        }
        Ok(())
    }

    async fn rename(&mut self, from: &str, to: &str) -> Result<()> {
        self.session
            .rename(from, to)
            .await
            .with_context(|| format!("failed to rename {from} -> {to}"))?;
        Ok(())
    }

    async fn mkdir(&mut self, path: &str) -> Result<()> {
        self.session
            .create_dir(path)
            .await
            .with_context(|| format!("failed to mkdir {path}"))?;
        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        tracing::info!(action = "SFTP_CLOSE", "SFTP session closing");
        self.session.close().await.ok();
        tracing::debug!("SFTP session closed");
        Ok(())
    }

    async fn read_for_edit(&mut self, path: &str) -> Result<Vec<u8>> {
        let data = self
            .session
            .read(path)
            .await
            .with_context(|| format!("failed to read {path}"))?;
        if data.len() > 5 * 1024 * 1024 {
            anyhow::bail!("File too large for editing (>5MB)");
        }
        Ok(data)
    }

    async fn save_from_edit(&mut self, path: &str, data: Vec<u8>) -> Result<()> {
        let mut file = self
            .session
            .create(path)
            .await
            .with_context(|| format!("failed to create {path} for save"))?;
        file.write_all(&data)
            .await
            .context("failed to write data")?;
        file.flush().await.context("failed to flush")?;
        Ok(())
    }

    async fn chmod(&mut self, path: &str, mode: &str) -> Result<()> {
        let perms =
            u32::from_str_radix(mode, 8).with_context(|| format!("invalid octal mode '{mode}'"))?;
        let mut attrs = FileAttributes::empty();
        attrs.permissions = Some(perms);
        self.session
            .set_metadata(path, attrs)
            .await
            .with_context(|| format!("failed to chmod {path} to {mode}"))?;
        Ok(())
    }

    /// SFTP chmod 已实现（T4）；presigned URL / ACL / multipart 是 S3 专属。
    fn capability(&self) -> rex_common::file_transfer::FileCapabilitySet {
        rex_common::file_transfer::FileCapabilitySet {
            chmod: true,
            ..rex_common::file_transfer::FileCapabilitySet::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex as StdMutex};
    use std::time::{Duration, Instant};

    use russh::keys::decode_secret_key;
    use russh::server::{self, Auth, ChannelOpenHandle, Server as _, Session};
    use russh::{Channel, ChannelId, ChannelOpenFailure, Disconnect, Pty};

    use super::*;
    use crate::test_support::HOST_KEY_PEM;
    use crate::{SshConfig, SshSession};

    /// 保证每个 harness 的 pool key（含 username）全局唯一，隔离并行测试
    static NEXT_HARNESS_ID: AtomicUsize = AtomicUsize::new(0);

    struct ServerState {
        conns: AtomicUsize,
        max_sessions: usize,
        accept_auth: bool,
        server_handles: StdMutex<Vec<server::Handle>>,
        /// 记录所有收到的 subsystem 请求名（如 "sftp"），供测试断言
        subsystems: StdMutex<Vec<String>>,
    }

    struct TestServer {
        state: Arc<ServerState>,
    }

    impl server::Server for TestServer {
        type Handler = TestHandler;

        fn new_client(&mut self, _peer: Option<SocketAddr>) -> TestHandler {
            self.state.conns.fetch_add(1, Ordering::SeqCst);
            TestHandler {
                state: self.state.clone(),
                sessions: 0,
                channels: Vec::new(),
            }
        }
    }

    struct TestHandler {
        state: Arc<ServerState>,
        /// 本条连接上已接受的 session channel 数（模拟 sshd MaxSessions）
        sessions: usize,
        /// 已打开但尚未被 subsystem 认领的 session channel
        channels: Vec<Channel<server::Msg>>,
    }

    impl TestHandler {
        fn auth(&self) -> Auth {
            if self.state.accept_auth {
                Auth::Accept
            } else {
                Auth::reject()
            }
        }
    }

    impl server::Handler for TestHandler {
        type Error = russh::Error;

        async fn auth_none(&mut self, _user: &str) -> Result<Auth, Self::Error> {
            Ok(self.auth())
        }

        async fn auth_password(
            &mut self,
            _user: &str,
            _password: &str,
        ) -> Result<Auth, Self::Error> {
            Ok(self.auth())
        }

        async fn channel_open_session(
            &mut self,
            channel: Channel<server::Msg>,
            reply: ChannelOpenHandle,
            _session: &mut Session,
        ) -> Result<(), Self::Error> {
            if self.sessions >= self.state.max_sessions {
                reply
                    .reject(ChannelOpenFailure::AdministrativelyProhibited)
                    .await;
            } else {
                self.sessions += 1;
                // 存下 channel 供 subsystem_request 交给进程内 SFTP 服务端
                self.channels.push(channel);
                reply.accept().await;
            }
            Ok(())
        }

        /// 模拟 sshd：只有收到 `sftp` 子系统请求才真正启动 sftp-server。
        /// 客户端漏发该请求时，其 SSH_FXP_INIT 不会得到 SSH_FXP_VERSION 应答。
        async fn subsystem_request(
            &mut self,
            channel: ChannelId,
            name: &str,
            session: &mut Session,
        ) -> Result<(), Self::Error> {
            self.state
                .subsystems
                .lock()
                .expect("lock subsystems")
                .push(name.to_string());
            if name != "sftp" {
                let _ = session.channel_failure(channel);
                return Ok(());
            }
            let _ = session.channel_success(channel);
            if let Some(channel) = self.channels.pop() {
                russh_sftp::server::run(channel.into_stream(), TestSftpHandler).await;
            }
            Ok(())
        }

        async fn pty_request(
            &mut self,
            channel: ChannelId,
            _term: &str,
            _col_width: u32,
            _row_height: u32,
            _pix_width: u32,
            _pix_height: u32,
            _modes: &[(Pty, u32)],
            session: &mut Session,
        ) -> Result<(), Self::Error> {
            let _ = session.channel_success(channel);
            Ok(())
        }

        async fn shell_request(
            &mut self,
            channel: ChannelId,
            session: &mut Session,
        ) -> Result<(), Self::Error> {
            let _ = session.channel_success(channel);
            Ok(())
        }
    }

    /// 极简进程内 SFTP 服务端：`init` 用 trait 默认实现回 SSH_FXP_VERSION，
    /// REALPATH 原样回显路径，STAT 回固定大小，用于验证客户端不仅握手成功
    /// 还能真正收发 SFTP 请求。
    struct TestSftpHandler;

    impl russh_sftp::server::Handler for TestSftpHandler {
        type Error = russh_sftp::protocol::StatusCode;

        fn unimplemented(&self) -> Self::Error {
            russh_sftp::protocol::StatusCode::OpUnsupported
        }

        async fn realpath(
            &mut self,
            id: u32,
            path: String,
        ) -> Result<russh_sftp::protocol::Name, Self::Error> {
            Ok(russh_sftp::protocol::Name {
                id,
                files: vec![russh_sftp::protocol::File::dummy(path)],
            })
        }

        async fn stat(
            &mut self,
            id: u32,
            _path: String,
        ) -> Result<russh_sftp::protocol::Attrs, Self::Error> {
            let mut attrs = russh_sftp::protocol::FileAttributes::empty();
            attrs.size = Some(1024);
            Ok(russh_sftp::protocol::Attrs { id, attrs })
        }
    }

    /// 回环测试 SSH server：可配置 MaxSessions 与认证是否放行
    struct Harness {
        addr: SocketAddr,
        state: Arc<ServerState>,
        username: String,
    }

    impl Harness {
        async fn start(max_sessions: usize, accept_auth: bool) -> Self {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("bind loopback sshd");
            let addr = listener.local_addr().expect("local addr");
            let state = Arc::new(ServerState {
                conns: AtomicUsize::new(0),
                max_sessions,
                accept_auth,
                server_handles: StdMutex::new(Vec::new()),
                subsystems: StdMutex::new(Vec::new()),
            });

            let server_config = server::Config {
                auth_rejection_time: Duration::from_millis(10),
                keys: vec![
                    decode_secret_key(&format!("{HOST_KEY_PEM}\n"), None).expect("decode host key")
                ],
                ..Default::default()
            };
            let server_config = Arc::new(server_config);

            let accept_state = state.clone();
            tokio::spawn(async move {
                let mut srv = TestServer {
                    state: accept_state.clone(),
                };
                loop {
                    let Ok((socket, _)) = listener.accept().await else {
                        break;
                    };
                    let handler = srv.new_client(socket.peer_addr().ok());
                    let config = server_config.clone();
                    let state = accept_state.clone();
                    tokio::spawn(async move {
                        if let Ok(running) = server::run_stream(config, socket, handler).await {
                            state
                                .server_handles
                                .lock()
                                .expect("lock handles")
                                .push(running.handle());
                            let _ = running.await;
                        }
                    });
                }
            });

            let id = NEXT_HARNESS_ID.fetch_add(1, Ordering::SeqCst);
            Harness {
                addr,
                state,
                username: format!("tester{id}"),
            }
        }

        fn conns(&self) -> usize {
            self.state.conns.load(Ordering::SeqCst)
        }

        /// 服务端收到的 subsystem 请求名序列
        fn subsystems(&self) -> Vec<String> {
            self.state
                .subsystems
                .lock()
                .expect("lock subsystems")
                .clone()
        }

        fn config(&self) -> SshConfig {
            SshConfig {
                host: "127.0.0.1".to_string(),
                port: self.addr.port(),
                username: self.username.clone(),
                password: Some("secret".to_string()),
                private_key: None,
                keepalive_interval: Some(0),
                init_script: None,
            }
        }

        /// 主动断开所有已建立的服务端连接（模拟服务端掐线）
        async fn disconnect_all(&self) {
            let handles =
                std::mem::take(&mut *self.state.server_handles.lock().expect("lock handles"));
            for handle in handles {
                let _ = handle
                    .disconnect(
                        Disconnect::ByApplication,
                        "test shutdown".to_string(),
                        String::new(),
                    )
                    .await;
            }
        }
    }

    /// 回归（v0.92.0 Bugs🔴1）：SFTP 必须在 session channel 上先请求 `sftp` 子系统。
    /// 漏发该请求时远端不会启动 sftp-server，SSH_FXP_INIT 得不到
    /// SSH_FXP_VERSION，`SftpSession::new` 超时并报 "failed to create SFTP session"。
    #[tokio::test]
    async fn sftp_requests_subsystem_before_handshake_and_serves_operations() {
        let harness = Harness::start(usize::MAX, true).await;
        let cfg = harness.config();
        let mut conn = SftpConnector::connect_with_config(cfg)
            .await
            .expect("sftp session must be established once the subsystem is requested");

        assert_eq!(
            harness.subsystems(),
            vec!["sftp".to_string()],
            "the client must ask sshd to start the sftp subsystem on the session channel"
        );

        // 端到端：会话不仅握手成功，还能真正跑一次 STAT/REALPATH 往返。
        let entry = conn
            .stat("/var/data")
            .await
            .expect("sftp session must answer file operations");
        assert_eq!(entry.path, "/var/data");
        assert_eq!(entry.name, "data");
    }

    /// 回归：agent 模式复用终端 Handle 的路径（`connect_from_handle`）同样要请求子系统
    #[tokio::test]
    async fn sftp_from_pooled_terminal_handle_requests_subsystem() {
        let harness = Harness::start(usize::MAX, true).await;
        let cfg = harness.config();
        let (handle, _terminal) = SshSession::connect_with_handle(cfg)
            .await
            .expect("terminal connect");

        let mut conn = SftpConnector::connect_from_handle(&handle, "127.0.0.1")
            .await
            .expect("sftp over the terminal handle must be established");

        assert_eq!(
            harness.subsystems(),
            vec!["sftp".to_string()],
            "the pooled-handle path must request the sftp subsystem too"
        );
        let entry = conn
            .stat("/home")
            .await
            .expect("sftp session over the terminal handle must answer file operations");
        assert_eq!(entry.path, "/home");
        assert_eq!(
            harness.conns(),
            1,
            "the handle path must still reuse the terminal's TCP connection"
        );
    }

    /// 方向 a：SFTP 在终端已建立的连接上开 channel，不新建第二条连接
    #[tokio::test]
    async fn sftp_reuses_terminal_connection_from_pool() {
        let harness = Harness::start(usize::MAX, true).await;
        let cfg = harness.config();
        let _terminal = SshSession::connect(cfg.clone())
            .await
            .expect("terminal connect");
        assert_eq!(harness.conns(), 1);

        let key = crate::pool::pool_key(&cfg);
        assert!(
            crate::pool::get(&key).await.is_some(),
            "terminal handle must be registered into the pool"
        );

        let channel = open_session_channel(&cfg)
            .await
            .expect("sftp channel over pooled connection");
        assert_eq!(
            harness.conns(),
            1,
            "SFTP must reuse the terminal's TCP connection"
        );
        drop(channel);
    }

    /// 方向 b：池连接已被服务端掐断 → 剔除死句柄并降级为新连接
    #[tokio::test]
    async fn sftp_falls_back_when_pooled_handle_is_dead() {
        let harness = Harness::start(usize::MAX, true).await;
        let cfg = harness.config();
        let _terminal = SshSession::connect(cfg.clone())
            .await
            .expect("terminal connect");
        assert_eq!(harness.conns(), 1);
        let key = crate::pool::pool_key(&cfg);

        harness.disconnect_all().await;

        // 等待客户端感知断开：pool::get 对已关闭连接返回 None 并剔除
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if crate::pool::get(&key).await.is_none() {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "pooled handle never reported closed"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        let channel = open_session_channel(&cfg)
            .await
            .expect("should degrade to a fresh connection");
        assert_eq!(
            harness.conns(),
            2,
            "dead pooled handle must trigger a new SSH connection"
        );
        assert!(
            crate::pool::get(&key).await.is_some(),
            "fresh connection should be re-registered"
        );
        drop(channel);
    }

    /// MaxSessions=1 拒绝第二条 channel：连接仍存活 → 保留池句柄并降级新连接
    #[tokio::test]
    async fn sftp_degrades_after_channel_open_rejection() {
        let harness = Harness::start(1, true).await;
        let cfg = harness.config();
        let _terminal = SshSession::connect(cfg.clone())
            .await
            .expect("terminal connect");
        assert_eq!(harness.conns(), 1);

        let channel = open_session_channel(&cfg)
            .await
            .expect("should degrade after channel open rejection");
        assert_eq!(
            harness.conns(),
            2,
            "rejection on the pooled connection must fall back to a new connection"
        );
        drop(channel);
    }

    /// 认证被拒必须报 authentication failed，而不是误导性的 Disconnected
    #[tokio::test]
    async fn sftp_reports_authentication_failure_clearly() {
        let harness = Harness::start(usize::MAX, false).await;
        let cfg = harness.config();
        let err = match SftpConnector::connect_with_config(cfg).await {
            Ok(_) => panic!("rejected authentication must fail"),
            Err(e) => e,
        };
        let msg = err.to_string();
        assert!(
            msg.contains("authentication failed"),
            "unexpected error: {msg}"
        );
        assert!(
            msg.contains("password"),
            "the attempted method must be named: {msg}"
        );
        assert!(
            !msg.contains("Disconnected"),
            "must not misreport as Disconnected: {msg}"
        );
    }

    /// 同时配置私钥与密码、服务器拒 publickey → 必须回退 password 认证成功
    /// （回归：SFTP 只试 publickey 导致已连上终端的资源点 SFTP 报 auth failed）
    #[tokio::test]
    async fn sftp_auth_falls_back_to_password_after_publickey_rejection() {
        let harness = Harness::start(usize::MAX, true).await;
        let mut cfg = harness.config();
        cfg.private_key = Some(format!("{HOST_KEY_PEM}\n"));

        let channel = open_session_channel(&cfg)
            .await
            .expect("password fallback must authenticate after publickey rejection");
        assert_eq!(
            harness.conns(),
            1,
            "fallback happens on the same connection"
        );
        drop(channel);
    }

    /// 私钥与密码全被拒 → 聚合错误保留每次尝试的 partial_success / remaining
    #[tokio::test]
    async fn sftp_auth_reports_every_rejected_method() {
        let harness = Harness::start(usize::MAX, false).await;
        let mut cfg = harness.config();
        cfg.private_key = Some(format!("{HOST_KEY_PEM}\n"));

        let err = match SftpConnector::connect_with_config(cfg).await {
            Ok(_) => panic!("rejected authentication must fail"),
            Err(e) => e,
        };
        let msg = err.to_string();
        assert!(msg.contains("authentication failed"), "{msg}");
        assert!(msg.contains("publickey"), "{msg}");
        assert!(msg.contains("password"), "{msg}");
        assert!(msg.contains("partial_success"), "{msg}");
        assert!(!msg.contains("Disconnected"), "{msg}");
    }

    /// 在内存 duplex 上完成一次 SFTP 握手（SSH_FXP_INIT → SSH_FXP_VERSION），
    /// 得到零网络的 `SftpConnector` 实例，用于不触达任何服务器的 trait 断言。
    async fn offline_connector() -> SftpConnector {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let (client_io, mut server_io) = tokio::io::duplex(64 * 1024);
        tokio::spawn(async move {
            let mut len = [0u8; 4];
            if server_io.read_exact(&mut len).await.is_err() {
                return;
            }
            let mut init = vec![0u8; u32::from_be_bytes(len) as usize];
            if server_io.read_exact(&mut init).await.is_err() {
                return;
            }
            let body = russh_sftp::ser::to_bytes(&russh_sftp::protocol::Version::new())
                .expect("serialize SSH_FXP_VERSION");
            // 帧 = [len BE][type=2 (SSH_FXP_VERSION)][Version payload]
            let mut frame = ((body.len() + 1) as u32).to_be_bytes().to_vec();
            frame.push(2u8);
            frame.extend_from_slice(&body);
            let _ = server_io.write_all(&frame).await;
        });

        let session = SftpSession::new(client_io)
            .await
            .expect("in-memory sftp session");
        SftpConnector { session }
    }

    /// 能力上报：SFTP chmod 已实现 (T4) → chmod=true；
    /// presigned URL / ACL / multipart 是 S3 专属，SFTP 永不支持。
    #[tokio::test]
    async fn sftp_capability_reports_chmod_supported() {
        use rex_common::file_transfer::FileCapabilitySet;

        let conn = offline_connector().await;
        let caps = conn.capability();
        assert!(caps.chmod, "SFTP must report chmod support (T4)");
        assert!(!caps.presigned_url, "presigned URL is S3-only");
        assert!(!caps.acl, "ACL is S3-only");
        assert!(!caps.multipart, "multipart is S3-only");
        // 其余字段仍为默认 false，与 FileCapabilitySet 其它 S3-only 字段一致
        assert_eq!(
            caps,
            FileCapabilitySet {
                chmod: true,
                ..FileCapabilitySet::default()
            }
        );
    }

    /// S3 专属操作在 SFTP 上必须走 trait 默认实现 → `UnsupportedProtocolError`
    /// （Hub handler 据此映射 `UNSUPPORTED_PROTOCOL`，文案与历史 downcast 分支一致）。
    #[tokio::test]
    async fn sftp_default_s3_operations_are_unsupported() {
        use rex_common::file_transfer::{FileConnector, UnsupportedProtocolError};

        let conn = offline_connector().await;
        let conn: &dyn FileConnector = &conn;

        let errors = [
            conn.presigned_url("k", 60).await.unwrap_err(),
            conn.list_multipart_uploads("p").await.unwrap_err(),
            conn.resume_multipart_upload("k", "u", Vec::new(), None)
                .await
                .unwrap_err(),
            conn.abort_multipart_upload("k", "u").await.unwrap_err(),
            conn.get_acl("k").await.unwrap_err(),
            conn.put_acl("k", "private").await.unwrap_err(),
        ];
        let messages: Vec<String> = errors
            .iter()
            .map(|e| {
                e.downcast_ref::<UnsupportedProtocolError>()
                    .expect("must be UnsupportedProtocolError for UNSUPPORTED_PROTOCOL mapping")
                    .message
                    .clone()
            })
            .collect();
        assert_eq!(
            messages,
            vec![
                "presigned URL only supported for S3",
                "only supported for S3",
                "only supported for S3",
                "only supported for S3",
                "only supported for S3",
                "only supported for S3",
            ]
        );
    }
}
