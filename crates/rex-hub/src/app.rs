//! 共享应用状态 — 持有数据库、认证配置和各协议连接池。

use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

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
use crate::sync_coordinator::SyncCoordinator;
use crate::transfer_coordinator::TransferCoordinator;
use crate::update_api::AgentBinaries;

/// 传输进度 WS 广播事件（T5.4）。
/// TransferCoordinator 在状态/进度变化时发布，`file_ws` 订阅后转发给前端。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferProgressEvent {
    pub task_id: String,
    pub transferred_bytes: u64,
    pub total_bytes: u64,
    pub speed_bytes_per_sec: u64,
    pub status: String,
}

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
    /// Hub 侧直连传输协调器（T2）：驱动 source-connector → target-connector。
    pub coordinator: Arc<TransferCoordinator>,
    /// 目录同步协调器（v0.92.0）：compare → diff → apply，数据仍只走连接器直连。
    pub sync_coordinator: Arc<SyncCoordinator>,
    /// 传输进度 WS 广播通道（T5.4）。
    pub transfer_bcast: broadcast::Sender<TransferProgressEvent>,
}
