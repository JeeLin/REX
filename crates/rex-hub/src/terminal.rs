use std::collections::HashMap;
use std::sync::Arc;

use rex_ssh::auth::AuthMethod;
use rex_ssh::client::{SshClient, SshEvent};
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};

/// 终端会话状态
#[derive(Debug, Clone, PartialEq)]
pub enum SessionState {
    /// 正在连接
    Connecting,
    /// 已连接，等待 PTY/shell
    Connected,
    /// PTY 已分配，shell 已打开，可收发数据
    Active,
    /// 已断开
    Closed,
}

/// 会话轻量级信息（只读查询用）
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub id: String,
    pub resource_id: String,
    pub state: SessionState,
    pub cols: u32,
    pub rows: u32,
    pub created_at: Instant,
    pub last_active_at: Instant,
}

/// 终端会话
pub struct TerminalSession {
    /// 会话 ID（sess_xxxxxxxx）
    pub id: String,
    /// 关联的资源 ID
    pub resource_id: String,
    /// 会话状态
    pub state: SessionState,
    /// 终端列数
    pub cols: u32,
    /// 终端行数
    pub rows: u32,
    /// 创建时间
    pub created_at: Instant,
    /// 最后活跃时间
    pub last_active_at: Instant,
    /// SSH 客户端（拥有所有权）
    client: Option<SshClient>,
}

impl TerminalSession {
    /// 创建新会话（不立即连接）
    pub fn new(resource_id: &str, cols: u32, rows: u32) -> Self {
        let now = Instant::now();
        let id = format!("sess_{}", &uuid::Uuid::new_v4().to_string()[..8]);
        Self {
            id,
            resource_id: resource_id.to_string(),
            state: SessionState::Connecting,
            cols,
            rows,
            created_at: now,
            last_active_at: now,
            client: None,
        }
    }

    /// 获取只读信息
    pub fn info(&self) -> SessionInfo {
        SessionInfo {
            id: self.id.clone(),
            resource_id: self.resource_id.clone(),
            state: self.state.clone(),
            cols: self.cols,
            rows: self.rows,
            created_at: self.created_at,
            last_active_at: self.last_active_at,
        }
    }

    /// 连接 SSH 服务器并设置 client
    pub async fn connect(
        &mut self,
        host: &str,
        port: u16,
        username: &str,
        auth: AuthMethod,
    ) -> anyhow::Result<()> {
        let client = SshClient::connect(host, port, username, auth).await?;
        self.client = Some(client);
        self.state = SessionState::Connected;
        self.touch();
        Ok(())
    }

    /// 请求 PTY 和 shell，进入 Active 状态
    pub async fn init_shell(&mut self) -> anyhow::Result<()> {
        let client = self
            .client
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("SSH client not connected"))?;
        client.request_pty(self.cols, self.rows).await?;
        client.request_shell().await?;
        self.state = SessionState::Active;
        self.touch();
        Ok(())
    }

    /// 发送数据
    pub async fn send_data(&mut self, data: &[u8]) -> anyhow::Result<()> {
        let client = self
            .client
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("SSH client not connected"))?;
        client.send_data(data).await?;
        self.touch();
        Ok(())
    }

    /// 接收数据
    pub async fn recv(&mut self) -> Option<SshEvent> {
        let client = self.client.as_mut()?;
        let event = client.recv().await;
        self.touch();
        Some(event)
    }

    /// 窗口大小调整
    pub async fn resize(&mut self, cols: u32, rows: u32) -> anyhow::Result<()> {
        self.cols = cols;
        self.rows = rows;
        if let Some(client) = self.client.as_mut() {
            client.window_change(cols, rows).await?;
        }
        self.touch();
        Ok(())
    }

    /// 关闭会话
    pub async fn close(&mut self) -> anyhow::Result<()> {
        self.state = SessionState::Closed;
        if let Some(mut client) = self.client.take() {
            let _ = client.disconnect().await;
        }
        Ok(())
    }

    /// 更新活跃时间
    pub fn touch(&mut self) {
        self.last_active_at = Instant::now();
    }

    /// 是否空闲超时
    pub fn is_idle_timeout(&self, max_idle_secs: u64) -> bool {
        self.last_active_at.elapsed() > Duration::from_secs(max_idle_secs)
    }
}

/// 会话管理器
pub struct SessionManager {
    sessions: Arc<Mutex<HashMap<String, TerminalSession>>>,
    max_idle_secs: u64,
}

impl SessionManager {
    pub fn new(max_idle_secs: u64) -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            max_idle_secs,
        }
    }

    /// 创建并注册新会话
    pub async fn create_session(&self, resource_id: &str, cols: u32, rows: u32) -> String {
        let session = TerminalSession::new(resource_id, cols, rows);
        let id = session.id.clone();
        self.sessions.lock().await.insert(id.clone(), session);
        id
    }

    /// 获取会话只读信息
    pub async fn get_info(&self, session_id: &str) -> Option<SessionInfo> {
        self.sessions.lock().await.get(session_id).map(|s| s.info())
    }

    /// 获取 sessions 的可变锁，调用方自行操作
    pub async fn lock(&self) -> tokio::sync::MutexGuard<'_, HashMap<String, TerminalSession>> {
        self.sessions.lock().await
    }

    /// 移除并关闭会话
    pub async fn remove_session(&self, session_id: &str) -> anyhow::Result<()> {
        if let Some(mut session) = self.sessions.lock().await.remove(session_id) {
            session.close().await?;
        }
        Ok(())
    }

    /// 清理所有空闲超时的会话
    pub async fn cleanup_idle(&self) {
        let mut sessions = self.sessions.lock().await;
        let to_remove: Vec<String> = sessions
            .iter()
            .filter(|(_, s)| s.is_idle_timeout(self.max_idle_secs))
            .map(|(id, _)| id.clone())
            .collect();

        for id in to_remove {
            if let Some(mut session) = sessions.remove(&id) {
                let _ = session.close().await;
                tracing::info!(session_id = %id, "cleaned up idle session");
            }
        }
    }

    /// 当前会话数量
    pub async fn count(&self) -> usize {
        self.sessions.lock().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn session_new_state_is_connecting() {
        let session = TerminalSession::new("res_001", 80, 24);
        assert_eq!(session.state, SessionState::Connecting);
        assert_eq!(session.resource_id, "res_001");
        assert_eq!(session.cols, 80);
        assert_eq!(session.rows, 24);
    }

    #[test]
    fn session_touch_updates_last_active() {
        let mut session = TerminalSession::new("res_001", 80, 24);
        let before = session.last_active_at;
        std::thread::sleep(Duration::from_millis(10));
        session.touch();
        assert!(session.last_active_at > before);
    }

    #[test]
    fn session_is_idle_timeout() {
        let mut session = TerminalSession::new("res_001", 80, 24);
        session.last_active_at = Instant::now() - Duration::from_secs(1000);
        assert!(session.is_idle_timeout(900));
    }

    #[test]
    fn session_is_not_idle_when_active() {
        let session = TerminalSession::new("res_001", 80, 24);
        assert!(!session.is_idle_timeout(900));
    }

    #[tokio::test]
    async fn manager_create_and_get() {
        let manager = SessionManager::new(900);
        let id = manager.create_session("res_001", 80, 24).await;
        let info = manager.get_info(&id).await;
        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.resource_id, "res_001");
        assert_eq!(info.state, SessionState::Connecting);
    }

    #[tokio::test]
    async fn manager_remove_session() {
        let manager = SessionManager::new(900);
        let id = manager.create_session("res_001", 80, 24).await;
        manager.remove_session(&id).await.unwrap();
        assert!(manager.get_info(&id).await.is_none());
    }

    #[tokio::test]
    async fn manager_cleanup_idle() {
        let manager = SessionManager::new(0); // 0 秒超时
        let id = manager.create_session("res_001", 80, 24).await;
        std::thread::sleep(Duration::from_millis(10));
        manager.cleanup_idle().await;
        assert!(manager.get_info(&id).await.is_none());
    }

    #[tokio::test]
    async fn manager_count() {
        let manager = SessionManager::new(900);
        assert_eq!(manager.count().await, 0);
        manager.create_session("res_001", 80, 24).await;
        assert_eq!(manager.count().await, 1);
        manager.create_session("res_002", 120, 40).await;
        assert_eq!(manager.count().await, 2);
    }

    #[tokio::test]
    async fn manager_get_nonexistent_returns_none() {
        let manager = SessionManager::new(900);
        assert!(manager.get_info("nonexistent").await.is_none());
    }

    #[tokio::test]
    async fn session_send_data_without_client_fails() {
        let mut session = TerminalSession::new("res_001", 80, 24);
        let result = session.send_data(b"hello").await;
        assert!(result.is_err());
    }

    #[test]
    fn session_info_matches_session() {
        let session = TerminalSession::new("res_001", 120, 40);
        let info = session.info();
        assert_eq!(info.id, session.id);
        assert_eq!(info.resource_id, "res_001");
        assert_eq!(info.cols, 120);
        assert_eq!(info.rows, 40);
        assert_eq!(info.state, SessionState::Connecting);
    }
}
