//! 共享应用状态 — 持有数据库、认证配置和各协议连接池。

use std::sync::Arc;

use crate::auth::AuthConfig;
use crate::db::Database;
use crate::file_api::FileState;
use crate::redis_api::RedisState;
use crate::sql_api::SqlState;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub auth: Arc<AuthConfig>,
    pub sql_pool: SqlState,
    pub redis_pool: RedisState,
    pub file_pool: FileState,
}
