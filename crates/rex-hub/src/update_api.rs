//! 更新 API — 版本检查、更新触发、状态查询、Agent 二进制下载。

use std::collections::HashMap;
use std::path::PathBuf;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use tokio::sync::RwLock;

use crate::agent_ws::AgentEvent;
use crate::AppState;
use rex_common::update::{
    AgentVersionInfo, UpdateCheckResult, UpdatePhase, UpdateStatus, VersionInfo,
};

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
    /// GitHub 最新版本缓存
    pub latest_release: RwLock<Option<CachedRelease>>,
    /// Agent 二进制搜索目录
    pub agent_bin_dirs: Vec<PathBuf>,
}

pub struct CachedRelease {
    version: String,
    download_url: String,
    release_notes: String,
    fetched_at: std::time::Instant,
}

impl UpdateState {
    pub fn new() -> Self {
        let mut agent_bin_dirs = Vec::new();
        // Docker 内嵌路径
        agent_bin_dirs.push(PathBuf::from("/usr/local/lib/rex/agents"));
        // 本地缓存路径
        if let Ok(home) = std::env::var("HOME") {
            agent_bin_dirs.push(PathBuf::from(home).join(".rex/agents"));
        }
        Self {
            statuses: RwLock::new(HashMap::new()),
            latest_release: RwLock::new(None),
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
            "/api/agents/{id}/update/trigger",
            axum::routing::post(trigger_update),
        )
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

    // 查询所有 Agent
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

    // GitHub 最新版本（缓存 5 分钟）
    let cached = state.update_state.latest_release.read().await;
    let (latest_version, download_url) = if let Some(ref c) = *cached {
        if c.fetched_at.elapsed() < std::time::Duration::from_secs(300) {
            (Some(c.version.clone()), Some(c.download_url.clone()))
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };
    drop(cached);

    Ok(Json(VersionInfo {
        hub_version,
        latest_version,
        download_url,
        agents: agent_infos,
    }))
}

/// GET /api/version/check — 检查 GitHub 最新版本
pub async fn check_latest_version(
    State(state): State<AppState>,
) -> ApiResult<UpdateCheckResult> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();

    // 尝试从缓存读取
    {
        let cached = state.update_state.latest_release.read().await;
        if let Some(ref c) = *cached {
            if c.fetched_at.elapsed() < std::time::Duration::from_secs(300) {
                return Ok(Json(UpdateCheckResult {
                    current_version: current_version.clone(),
                    latest_version: c.version.clone(),
                    update_available: c.version != current_version,
                    download_url: c.download_url.clone(),
                    release_notes: c.release_notes.clone(),
                }));
            }
        }
    }

    // 从 GitHub 获取
    let release = fetch_github_release().await;
    match release {
        Ok((version, url, notes)) => {
            // 缓存
            {
                let mut cached = state.update_state.latest_release.write().await;
                *cached = Some(CachedRelease {
                    version: version.clone(),
                    download_url: url.clone(),
                    release_notes: notes.clone(),
                    fetched_at: std::time::Instant::now(),
                });
            }
            Ok(Json(UpdateCheckResult {
                current_version,
                latest_version: version,
                update_available: true,
                download_url: url,
                release_notes: notes,
            }))
        }
        Err(e) => Err(err(
            StatusCode::BAD_GATEWAY,
            &format!("failed to check GitHub: {e}"),
        )),
    }
}

/// POST /api/agents/:id/update/trigger — 触发 Agent 更新
pub async fn trigger_update(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
) -> ApiResult<serde_json::Value> {
    // 检查 Agent 是否在线
    let connections = state.agent_tunnel.connections.read().await;
    let conn = connections
        .get(&agent_id)
        .ok_or_else(|| err(StatusCode::BAD_REQUEST, "agent is offline"))?
        .clone();
    drop(connections);

    // 获取最新版本
    let (version, download_url) = {
        let cached = state.update_state.latest_release.read().await;
        let release = cached.as_ref().ok_or_else(|| {
            err(
                StatusCode::BAD_REQUEST,
                "no version info available, check for updates first",
            )
        })?;
        (release.version.clone(), release.download_url.clone())
    };

    // 获取当前 Agent 版本
    let db = state.db.clone();
    let aid = agent_id.clone();
    let current_version = tokio::task::spawn_blocking(move || db.get_agent(&aid))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map(|a| a.version)
        .unwrap_or_default();

    if current_version == version {
        return Err(err(StatusCode::BAD_REQUEST, "agent is already up to date"));
    }

    // 从 DB 获取 Agent 的 os/arch，用于构造下载 URL
    let (agent_os, agent_arch) = {
        let db2 = state.db.clone();
        let aid2 = agent_id.clone();
        let info = tokio::task::spawn_blocking(move || db2.get_agent(&aid2))
            .await
            .ok()
            .and_then(|r| r.ok())
            .flatten();
        match info {
            Some(a) => {
                let os = if a.os.is_empty() { "linux" } else { &a.os };
                let arch = if a.arch.is_empty() { "amd64" } else { &a.arch };
                (os.to_string(), arch.to_string())
            }
            None => ("linux".into(), "amd64".into()),
        }
    };

    let cmd = rex_common::update::UpdateCommand {
        version: version.clone(),
        download_url: download_url.clone(),
        fallback_url: format!(
            "https://github.com/{}/{}/releases/download/v{}/rex-agent-{}-{}-{}.tar.gz",
            option_env!("GITHUB_OWNER").unwrap_or("user"),
            option_env!("GITHUB_REPO").unwrap_or("rex"),
            version,
            version,
            agent_os,
            agent_arch,
        ),
        sha256: String::new(), // TODO: 从 release asset 获取
    };

    // 初始化更新状态
    state
        .update_state
        .set_status(
            &agent_id,
            UpdateStatus {
                phase: UpdatePhase::Downloading,
                progress: 0.0,
                current_version,
                target_version: version.clone(),
                error: None,
                started_at: Some(chrono::Utc::now().to_rfc3339()),
            },
        )
        .await;

    // 通过 WebSocket 发送更新指令
    let msg = serde_json::to_string(&serde_json::json!({
        "type": "update",
        "payload": cmd
    }))
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;

    if let Err(e) = conn.sender.send(AgentEvent::Text(msg)).await {
        // 发送失败，重置状态
        state
            .update_state
            .set_status(&agent_id, UpdateStatus::default())
            .await;
        return Err(err(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to send update command: {e}"),
        ));
    }

    Ok(Json(serde_json::json!({
        "ok": true,
        "message": format!("update command sent to agent, target version: {}", version)
    })))
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

// ═══════════════════════════════════════
// GitHub API
// ═══════════════════════════════════════

async fn fetch_github_release() -> anyhow::Result<(String, String, String)> {
    let owner = option_env!("GITHUB_OWNER").unwrap_or("user");
    let repo = option_env!("GITHUB_REPO").unwrap_or("rex");
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        owner, repo
    );

    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("User-Agent", "REX-Hub")
        .send()
        .await?
        .error_for_status()?;

    let body: serde_json::Value = resp.json().await?;

    let version = body["tag_name"]
        .as_str()
        .unwrap_or("unknown")
        .trim_start_matches('v')
        .to_string();

    let release_notes = body["body"].as_str().unwrap_or("").to_string();

    // 找到 Agent 二进制下载链接
    let download_url = body["assets"]
        .as_array()
        .and_then(|assets| {
            assets
                .iter()
                .find(|a| {
                    a["name"]
                        .as_str()
                        .map(|n| n.contains("agent") && n.contains("linux"))
                        .unwrap_or(false)
                })
                .and_then(|a| a["browser_download_url"].as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_default();

    Ok((version, download_url, release_notes))
}

#[cfg(test)]
mod tests {
    use super::*;

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
            download_url: "http://hub:3000/api/agents/download?os=linux&arch=amd64".into(),
            fallback_url: "https://github.com/example/rex/releases/download/v0.17.0/rex-agent-0.17.0-linux-amd64.tar.gz".into(),
            sha256: "abc123".into(),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("0.17.0"));
        assert!(json.contains("download_url"));
    }
}
