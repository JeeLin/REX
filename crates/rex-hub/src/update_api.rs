//! Agent 二进制下载端点 + Hub 自动更新 API。
//!
//! 版本检查和更新触发通过 Agent WebSocket 心跳完成：
//! Agent 心跳上报 version → Hub 对比自身版本 → 版本不一致时通过 WebSocket 推送 update 指令。
//! Agent 收到 update 后从 Hub 下载新二进制。
//!
//! Hub 自动更新：检查 GitHub Release → 下载 → SHA256 校验 → 写 update-state.json → exit(10)。

use std::collections::HashMap;
use std::path::PathBuf;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use rex_common::update::{UpdatePhase, UpdateStateFile};

use crate::AppState;

// ═══════════════════════════════════════
// Agent 二进制搜索
// ═══════════════════════════════════════

pub struct AgentBinaries {
    pub dirs: Vec<PathBuf>,
}

impl AgentBinaries {
    pub fn new() -> Self {
        let mut dirs = Vec::new();
        // Docker 内嵌路径
        dirs.push(PathBuf::from("/app/data/agent-binaries"));
        // 系统路径
        dirs.push(PathBuf::from("/usr/local/lib/rex/agents"));
        // 本地缓存路径
        if let Ok(home) = std::env::var("HOME") {
            dirs.push(PathBuf::from(home).join(".rex/agents"));
        }
        Self { dirs }
    }

    /// 查找 Agent 二进制文件（按 os/arch）
    pub fn find(&self, os: &str, arch: &str) -> Option<PathBuf> {
        let subdir = format!("{os}/{arch}");
        for dir in &self.dirs {
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

impl Default for AgentBinaries {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════
// Handler
// ═══════════════════════════════════════

/// GET /api/agents/download?os=linux&arch=amd64 — 下载 Agent 二进制
///
/// 查找顺序：
/// 1. 本地预置二进制（Docker 内嵌 / 系统路径 / 本地缓存）
/// 2. GitHub Releases（当前版本，确保兼容）
pub async fn download_agent_binary(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let os = params.get("os").map(|s| s.as_str()).unwrap_or("linux");
    let arch = params.get("arch").map(|s| s.as_str()).unwrap_or("amd64");

    // 1. 尝试本地二进制
    if let Some(path) = state.agent_binaries.find(os, arch) {
        match tokio::fs::read(&path).await {
            Ok(bytes) => {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                let filename = if ext.is_empty() {
                    format!("rex-agent-{os}-{arch}")
                } else {
                    format!("rex-agent-{os}-{arch}.{ext}")
                };
                return (
                    StatusCode::OK,
                    [
                        ("Content-Type", "application/octet-stream"),
                        (
                            "Content-Disposition",
                            &format!("attachment; filename=\"{filename}\""),
                        ),
                    ],
                    bytes,
                )
                    .into_response();
            }
            Err(e) => {
                tracing::warn!(error = %e, path = %path.display(), "failed to read local agent binary");
            }
        }
    }

    // 2. 从 GitHub Releases 下载当前版本（确保兼容）
    let version = env!("CARGO_PKG_VERSION");
    let github_owner =
        std::env::var("REX_UPDATE_GITHUB_OWNER").unwrap_or_else(|_| "JeeLin".into());
    let github_repo =
        std::env::var("REX_UPDATE_GITHUB_REPO").unwrap_or_else(|_| "REX".into());

    let binary_name = format!("rex-agent-{os}-{arch}");
    let download_url = format!(
        "https://github.com/{github_owner}/{github_repo}/releases/download/v{version}/{binary_name}"
    );

    tracing::info!(url = %download_url, "downloading agent binary from GitHub Releases");

    match reqwest::get(&download_url).await {
        Ok(resp) if resp.status().is_success() => {
            match resp.bytes().await {
                Ok(bytes) => {
                    // 缓存到本地
                    let cache_dir = state
                        .data_dir
                        .join("agent-binaries")
                        .join(os)
                        .join(arch);
                    let _ = tokio::fs::create_dir_all(&cache_dir).await;
                    let cache_path = cache_dir.join("rex-agent");
                    let _ = tokio::fs::write(&cache_path, &bytes).await;

                    let filename = format!("rex-agent-{os}-{arch}");
                    (
                        StatusCode::OK,
                        [
                            ("Content-Type", "application/octet-stream"),
                            (
                                "Content-Disposition",
                                &format!("attachment; filename=\"{filename}\""),
                            ),
                        ],
                        bytes.to_vec(),
                    )
                        .into_response()
                }
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("failed to read download response: {e}"),
                )
                    .into_response(),
            }
        }
        Ok(resp) => (
            StatusCode::BAD_GATEWAY,
            format!(
                "GitHub download failed: status {}",
                resp.status()
            ),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            format!("failed to download from GitHub: {e}"),
        )
            .into_response(),
    }
}

// ═══════════════════════════════════════
// Hub 自动更新 API
// ═══════════════════════════════════════

/// GET /api/update/check — 检查是否有新版本
pub async fn check_update(
    State(state): State<AppState>,
) -> Result<(StatusCode, axum::Json<serde_json::Value>), (StatusCode, axum::Json<serde_json::Value>)>
{
    let checker = crate::update_checker::UpdateChecker::from_env(state.data_dir.clone());

    match checker.check_for_update().await {
        Ok(Some(info)) => Ok((
            StatusCode::OK,
            axum::Json(serde_json::json!({
                "has_update": true,
                "current_version": checker.current_version,
                "latest_version": info.version,
                "download_url": info.download_url,
            })),
        )),
        Ok(None) => Ok((
            StatusCode::OK,
            axum::Json(serde_json::json!({
                "has_update": false,
                "current_version": checker.current_version,
                "latest_version": checker.current_version,
                "download_url": "",
            })),
        )),
        Err(e) => Err((
            StatusCode::BAD_GATEWAY,
            axum::Json(serde_json::json!({
                "error": { "code": "UPDATE_CHECK_FAILED", "message": e }
            })),
        )),
    }
}

/// POST /api/update/trigger — 触发后台下载+更新
pub async fn trigger_update(
    State(state): State<AppState>,
) -> Result<(StatusCode, axum::Json<serde_json::Value>), (StatusCode, axum::Json<serde_json::Value>)>
{
    let checker = crate::update_checker::UpdateChecker::from_env(state.data_dir.clone());

    // 先检查是否有新版本
    let info = checker.check_for_update().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            axum::Json(serde_json::json!({
                "error": { "code": "UPDATE_CHECK_FAILED", "message": e }
            })),
        )
    })?;

    let info = match info {
        Some(i) => i,
        None => {
            return Ok((
                StatusCode::OK,
                axum::Json(serde_json::json!({
                    "ok": true,
                    "message": "already on latest version"
                })),
            ));
        }
    };

    // 后台下载+暂存
    let data_dir = state.data_dir.clone();
    tokio::spawn(async move {
        let checker = crate::update_checker::UpdateChecker::from_env(data_dir);
        match checker.download_and_stage(&info).await {
            Ok(()) => {
                tracing::info!(action = "UPDATE_TRIGGERED", version = %info.version, "update staged, setting exit flag");
                // 设置退出标志，由 main loop 检测后调用 std::process::exit(10)
                std::env::set_var("REX_UPDATE_READY", "1");
            }
            Err(e) => {
                tracing::error!(action = "UPDATE_TRIGGER_FAILED", error = %e, "failed to stage update");
            }
        }
    });

    Ok((
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "ok": true,
            "message": "update download started"
        })),
    ))
}

/// GET /api/update/status — 获取当前更新状态
pub async fn update_status(State(state): State<AppState>) -> axum::Json<serde_json::Value> {
    let path = state.data_dir.join("update-state.json");
    let state_file: Option<UpdateStateFile> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|data| serde_json::from_str(&data).ok());

    match state_file {
        Some(s) => axum::Json(serde_json::json!({
            "phase": s.phase,
            "target_version": s.target_version,
            "old_version": s.old_version,
            "attempt": s.attempt,
        })),
        None => axum::Json(serde_json::json!({
            "phase": "idle",
            "target_version": "",
            "old_version": "",
            "attempt": 0,
        })),
    }
}

/// POST /api/update/rollback — 回滚到旧版本
pub async fn rollback_update(
    State(state): State<AppState>,
) -> Result<(StatusCode, axum::Json<serde_json::Value>), (StatusCode, axum::Json<serde_json::Value>)>
{
    let path = state.data_dir.join("update-state.json");
    let state_file: Option<UpdateStateFile> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|data| serde_json::from_str(&data).ok());

    let mut s = match state_file {
        Some(s) => s,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                axum::Json(serde_json::json!({
                    "error": { "code": "NO_UPDATE_STATE", "message": "no update state found" }
                })),
            ));
        }
    };

    if s.rollback_path.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "error": { "code": "NO_ROLLBACK", "message": "no rollback binary available" }
            })),
        ));
    }

    // 写入 rolling_back 状态，supervisor 会检测并执行回滚
    s.phase = UpdatePhase::RollingBack;
    let tmp_path = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(&s).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({
                "error": { "code": "SERIALIZE_FAILED", "message": e.to_string() }
            })),
        )
    })?;
    std::fs::write(&tmp_path, &json).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({
                "error": { "code": "WRITE_FAILED", "message": e.to_string() }
            })),
        )
    })?;
    std::fs::rename(&tmp_path, &path).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({
                "error": { "code": "RENAME_FAILED", "message": e.to_string() }
            })),
        )
    })?;

    tracing::info!(
        action = "UPDATE_ROLLBACK_TRIGGERED",
        "rollback requested, supervisor will restart with old version"
    );

    Ok((
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "ok": true,
            "message": "rollback triggered, server will restart"
        })),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_binaries_default() {
        let ab = AgentBinaries::new();
        assert!(!ab.dirs.is_empty());
    }
}
