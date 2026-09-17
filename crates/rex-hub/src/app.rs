//! 共享应用状态 — 持有数据库、认证配置和各协议连接池。

use std::path::PathBuf;
use std::sync::Arc;

use crate::agent_ws::AgentTunnelState;
use crate::auth::AuthConfig;
use crate::crypto::CredentialCrypto;
use crate::db::Database;
use crate::file_api::FileState;
use crate::mongodb_api;
use crate::redis_api::RedisState;
use crate::sip_capture::SipCaptureRegistry;
use crate::sip_recording::SipRecordingRegistry;
use crate::sql_api::SqlState;
use crate::update_api::AgentBinaries;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub auth: Arc<AuthConfig>,
    pub crypto: Arc<CredentialCrypto>,
    pub sql_pool: SqlState,
    pub redis_pool: RedisState,
    pub file_pool: FileState,
    pub mongo_pool: mongodb_api::MongoState,
    pub agent_tunnel: Arc<AgentTunnelState>,
    pub agent_binaries: Arc<AgentBinaries>,
    pub sip_capture: Arc<SipCaptureRegistry>,
    pub sip_recording: Arc<SipRecordingRegistry>,
    pub data_dir: PathBuf,
    pub http_client: reqwest::Client,
    /// Actual HTTP listener port, shared with the Agent API loopback proxy.
    pub http_port: u16,
}
