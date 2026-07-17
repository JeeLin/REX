//! 更新 API — 版本信息、更新状态查询、Agent 二进制下载。
//!
//! 版本检查和更新触发通过 Agent WebSocket 心跳完成：
//! Agent 心跳上报 version → Hub 对比自身版本 → 版本不一致时通过 WebSocket 推送 update 指令。

use std::collections::HashMap;
use std::path::PathBuf;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use tokio::sync::RwLock;

use crate::AppState;
use rex_common::update::{AgentVersionInfo, UpdateStatus, VersionInfo};

type ApiResult<T> = Result<Json<T>, (StatusCode, Json<serde_json::Value>)>;

fn err(status: StatusCode, msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (
        status,
        Json(serde_json::json!({ "error": { "code": "ERROR", "message": msg } })),
    )
}

// ═══════════════════════════════════════
// 更新运行时状态
// ═══════════════════════════════════════

pub struct UpdateState {
    /// agent_id → UpdateStatus
    pub statuses: RwLock<HashMap<String, UpdateStatus>>,
    /// Agent 二进制搜索目录
    pub agent_bin_dirs: Vec<PathBuf>,
}

impl UpdateState {
    pub fn new() -> Self {
        let mut agent_bin_dirs = Vec::new();
        // Docker 内嵌路径
        agent_bin_dirs.push(PathBuf::from("/app/data/agent-binaries"));
        // 系统路径
        agent_bin_dirs.push(PathBuf::from("/usr/local/lib/rex/agents"));
        // 本地缓存路径
        if let Ok(home) = std::env::var("HOME") {
            agent_bin_dirs.push(PathBuf::from(home).join(".rex/agents"));
        }
        Self {
            statuses: RwLock::new(HashMap::new()),
            agent_bin_dirs,
        }
    }

    /// 更新 Agent 的更新状态
    pub async fn set_status(&self, agent_id: &str, status: UpdateStatus) {
        self.statuses
            .write()
            .await
            .insert(agent_id.to_string(), status);
    }

    /// 获取 Agent 的更新状态
    pub async fn get_status(&self, agent_id: &str) -> UpdateStatus {
        self.statuses
            .read()
            .await
            .get(agent_id)
            .cloned()
            .unwrap_or_default()
    }

    /// 查找 Agent 二进制文件（按 os/arch）
    pub fn find_agent_binary(&self, os: &str, arch: &str) -> Option<PathBuf> {
        let subdir = format!("{os}/{arch}");
        for dir in &self.agent_bin_dirs {
            let path = dir.join(&subdir).join("rex-agent");
            if path.exists() {
                return Some(path);
            }
            let path_exe = dir.join(&subdir).join("rex-agent.exe");
            if path_exe.exists() {
                return Some(path_exe);
            }
        }
        None
    }
}

impl Default for UpdateState {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════
// 路由
// ═══════════════════════════════════════

pub fn update_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route(
            "/api/agents/{id}/update/status",
            axum::routing::get(get_update_status),
        )
}

// ═══════════════════════════════════════
// Handlers
// ═══════════════════════════════════════

/// GET /api/version — 返回 Hub 版本 + 所有 Agent 版本
pub async fn get_version_info(
    State(state): State<AppState>,
) -> ApiResult<VersionInfo> {
    let hub_version = env!("CARGO_PKG_VERSION").to_string();

    let db = state.db.clone();
    let agents = tokio::task::spawn_blocking(move || db.list_all_agents())
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;

    let connections = state.agent_tunnel.connections.read().await;

    let agent_infos: Vec<AgentVersionInfo> = agents
        .iter()
        .map(|a| {
            let is_online = connections.contains_key(&a.id);
            AgentVersionInfo {
                agent_id: a.id.clone(),
                name: a.name.clone(),
                version: a.version.clone(),
                is_online,
                is_up_to_date: a.version == hub_version,
            }
        })
        .collect();

    Ok(Json(VersionInfo {
        hub_version,
        agents: agent_infos,
    }))
}

/// GET /api/agents/:id/update/status — 查询更新状态
pub async fn get_update_status(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
) -> ApiResult<UpdateStatus> {
    let status = state.update_state.get_status(&agent_id).await;
    Ok(Json(status))
}

/// GET /api/agents/download?os=linux&arch=amd64 — 下载 Agent 二进制
pub async fn download_agent_binary(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let os = params.get("os").map(|s| s.as_str()).unwrap_or("linux");
    let arch = params.get("arch").map(|s| s.as_str()).unwrap_or("amd64");

    if let Some(path) = state.update_state.find_agent_binary(os, arch) {
        match tokio::fs::read(&path).await {
            Ok(bytes) => {
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");
                let filename = if ext.is_empty() {
                    format!("rex-agent-{os}-{arch}")
                } else {
                    format!("rex-agent-{os}-{arch}.{ext}")
                };
                (
                    StatusCode::OK,
                    [
                        ("Content-Type", "application/octet-stream"),
                        ("Content-Disposition", &format!("attachment; filename=\"{filename}\"")),
                    ],
                    bytes,
                )
                    .into_response()
            }
            Err(e) => err(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to read binary: {e}"),
            )
            .into_response(),
        }
    } else {
        err(
            StatusCode::NOT_FOUND,
            &format!("agent binary for os={os} arch={arch} not found"),
        )
        .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rex_common::update::UpdatePhase;

    #[test]
    fn test_update_phase_default() {
        assert_eq!(UpdatePhase::default(), UpdatePhase::Idle);
    }

    #[test]
    fn test_update_status_default() {
        let s = UpdateStatus::default();
        assert_eq!(s.phase, UpdatePhase::Idle);
        assert_eq!(s.progress, 0.0);
        assert!(s.error.is_none());
    }

    #[test]
    fn test_update_command_serialize() {
        let cmd = rex_common::update::UpdateCommand {
            version: "0.17.0".into(),
            download_url: "/api/agents/download?os=linux&arch=amd64".into(),
            fallback_url: String::new(),
            sha256: String::new(),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("0.17.0"));
    }
}
