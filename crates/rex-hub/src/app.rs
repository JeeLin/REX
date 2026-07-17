//! 共享应用状态 — 持有数据库、认证配置和各协议连接池。

use std::sync::Arc;

use crate::agent_ws::AgentTunnelState;
use crate::auth::AuthConfig;
use crate::crypto::CredentialCrypto;
use crate::db::Database;
use crate::file_api::FileState;
use crate::redis_api::RedisState;
use crate::sql_api::SqlState;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub auth: Arc<AuthConfig>,
    pub crypto: Arc<CredentialCrypto>,
    pub sql_pool: SqlState,
    pub redis_pool: RedisState,
    pub file_pool: FileState,
    pub agent_tunnel: Arc<AgentTunnelState>,
}
