//! Agent 二进制下载端点。
//!
//! 版本检查和更新触发通过 Agent WebSocket 心跳完成：
//! Agent 心跳上报 version → Hub 对比自身版本 → 版本不一致时通过 WebSocket 推送 update 指令。
//! Agent 收到 update 后从 Hub 下载新二进制。

use std::collections::HashMap;
use std::path::PathBuf;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

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
pub async fn download_agent_binary(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let os = params.get("os").map(|s| s.as_str()).unwrap_or("linux");
    let arch = params.get("arch").map(|s| s.as_str()).unwrap_or("amd64");

    if let Some(path) = state.agent_binaries.find(os, arch) {
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
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to read binary: {e}"),
            )
                .into_response(),
        }
    } else {
        (
            StatusCode::NOT_FOUND,
            format!("agent binary for os={os} arch={arch} not found"),
        )
            .into_response()
    }
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
