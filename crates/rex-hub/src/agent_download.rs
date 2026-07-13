use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::{Cursor, Read};
use std::sync::Arc;

use crate::helpers::{ErrorBody, ErrorResponse};
use crate::routes::AppState;

/// GitHub 仓库（Release 兜底下载源）
const AGENT_REPO: &str = "JeeLin/REX";

#[derive(Deserialize)]
pub struct DownloadQuery {
    os: String,
    arch: String,
}

const VALID_OS: &[&str] = &["linux", "darwin", "windows"];
const VALID_ARCH: &[&str] = &["amd64", "arm64", "armv7l"];

pub async fn download_agent(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DownloadQuery>,
) -> impl IntoResponse {
    // 验证 os 参数
    if !VALID_OS.contains(&query.os.as_str()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: ErrorBody {
                    code: "INVALID_OS".to_string(),
                    message: format!(
                        "invalid os '{}', supported: {}",
                        query.os,
                        VALID_OS.join(", ")
                    ),
                },
            }),
        )
            .into_response();
    }

    // 验证 arch 参数
    if !VALID_ARCH.contains(&query.arch.as_str()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: ErrorBody {
                    code: "INVALID_ARCH".to_string(),
                    message: format!(
                        "invalid arch '{}', supported: {}",
                        query.arch,
                        VALID_ARCH.join(", ")
                    ),
                },
            }),
        )
            .into_response();
    }
    // 本地查找：CI 将二进制按 arch 组织到子目录（dist/agents/<arch>/rex-agent），
    // 因此同时检查扁平布局（agent-{os}-{arch}）与嵌套布局（<arch>/rex-agent*）。
    let binaries_dir = state.data_dir.join("agent-binaries");
    let flat_path = binaries_dir.join(format!("agent-{}-{}", query.os, query.arch));
    let nested_path = binaries_dir.join(&query.arch).join("rex-agent");

    let local_candidate = if flat_path.exists() {
        Some(flat_path)
    } else if nested_path.exists() {
        Some(nested_path)
    } else if query.os == "windows"
        && binaries_dir
            .join(&query.arch)
            .join("rex-agent.exe")
            .exists()
    {
        Some(binaries_dir.join(&query.arch).join("rex-agent.exe"))
    } else {
        None
    };

    if let Some(path) = local_candidate {
        // 安全检查：防止路径遍历
        if !path.starts_with(&binaries_dir) {
            return (
                StatusCode::FORBIDDEN,
                Json(ErrorResponse {
                    error: ErrorBody {
                        code: "PATH_TRAVERSAL".to_string(),
                        message: "path traversal not allowed".to_string(),
                    },
                }),
            )
                .into_response();
        }
        return serve_local(&path, &query.os, &query.arch, &state).await;
    }

    // 本地缺失 → 从 GitHub Release 兜底下载
    match download_from_github(&query.os, &query.arch).await {
        Ok(data) => {
            let version =
                std::env::var("REX_AGENT_VERSION").unwrap_or_else(|_| "unknown".to_string());
            respond_binary(data, &query.os, &query.arch, &version)
        }
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: ErrorBody {
                    code: "NOT_FOUND".to_string(),
                    message: format!(
                        "no agent binary for {}/{} (local missing, github fallback failed: {})",
                        query.os, query.arch, e
                    ),
                },
            }),
        )
            .into_response(),
    }
}

async fn serve_local(
    path: &std::path::Path,
    os: &str,
    arch: &str,
    _state: &AppState,
) -> axum::response::Response {
    match tokio::fs::read(path).await {
        Ok(data) => {
            let version =
                std::env::var("REX_AGENT_VERSION").unwrap_or_else(|_| "unknown".to_string());
            respond_binary(data, os, arch, &version)
        }
        Err(_) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: ErrorBody {
                    code: "NOT_FOUND".to_string(),
                    message: format!("no agent binary for {}/{}", os, arch),
                },
            }),
        )
            .into_response(),
    }
}

/// 构造响应（含 SHA256 / 版本头）
fn respond_binary(data: Vec<u8>, os: &str, arch: &str, version: &str) -> axum::response::Response {
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let sha256 = format!("{:x}", hasher.finalize());
    let filename = format!("agent-{}-{}", os, arch);

    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/octet-stream"),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        header::HeaderValue::from_str(&format!("attachment; filename=\"{filename}\""))
            .unwrap(),
    );
    headers.insert(
        "X-Agent-Version".parse::<header::HeaderName>().unwrap(),
        header::HeaderValue::from_str(version).unwrap(),
    );
    headers.insert(
        "X-Agent-SHA256".parse::<header::HeaderName>().unwrap(),
        header::HeaderValue::from_str(&sha256).unwrap(),
    );

    (headers, Body::from(data)).into_response()
}

/// 从 GitHub Release 兜底下载 agent 二进制。
/// CI 产出 `rex-agent-<arch>.zip`（arch 如 linux-amd64），内含 `rex-agent` / `rex-agent.exe`。
async fn download_from_github(os: &str, arch: &str) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::builder()
        .user_agent("rex-hub-agent-download")
        .build()
        .map_err(|e| format!("client build: {e}"))?;

    // 解析版本：优先 REX_AGENT_VERSION，否则取 latest release
    let version = match std::env::var("REX_AGENT_VERSION") {
        Ok(v) if !v.is_empty() => v,
        _ => {
            let url = format!("https://api.github.com/repos/{AGENT_REPO}/releases/latest");
            let resp = client
                .get(&url)
                .send()
                .await
                .map_err(|e| format!("github api: {e}"))?;
            if !resp.status().is_success() {
                return Err(format!("github api status {}", resp.status()));
            }
            let release: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| format!("github api parse: {e}"))?;
            release
                .get("tag_name")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .trim_start_matches('v')
                .to_string()
        }
    };

    let asset_arch = format!("{}-{}", os, arch);
    let url = format!(
        "https://github.com/{AGENT_REPO}/releases/download/v{version}/rex-agent-{asset_arch}.zip"
    );

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("download: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("download status {}", resp.status()));
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("read body: {e}"))?;

    // 解压 zip 提取对应平台的二进制
    extract_agent_binary(&bytes, os)
}

/// 从 agent zip 中提取当前平台的二进制（linux/mac → rex-agent，windows → rex-agent.exe）
fn extract_agent_binary(zip_bytes: &[u8], os: &str) -> Result<Vec<u8>, String> {
    let reader = Cursor::new(zip_bytes);
    let mut archive = zip::ZipArchive::new(reader).map_err(|e| format!("zip open: {e}"))?;
    let target = if os == "windows" {
        "rex-agent.exe"
    } else {
        "rex-agent"
    };
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| format!("zip read: {e}"))?;
        let name = file.name().to_string();
        let base = name.rsplit('/').next().unwrap_or(&name);
        if base == target && !file.is_dir() {
            let mut buf = Vec::with_capacity(file.size() as usize);
            file.read_to_end(&mut buf).map_err(|e| format!("zip extract: {e}"))?;
            return Ok(buf);
        }
    }
    Err(format!("agent binary '{target}' not found in release zip"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_os_values() {
        assert!(VALID_OS.contains(&"linux"));
        assert!(VALID_OS.contains(&"darwin"));
        assert!(VALID_OS.contains(&"windows"));
        assert!(!VALID_OS.contains(&"freebsd"));
    }

    #[test]
    fn valid_arch_values() {
        assert!(VALID_ARCH.contains(&"amd64"));
        assert!(VALID_ARCH.contains(&"arm64"));
        assert!(VALID_ARCH.contains(&"armv7l"));
        assert!(!VALID_ARCH.contains(&"x86"));
    }

    #[test]
    fn filename_construction() {
        let filename = format!("agent-{}-{}", "linux", "amd64");
        assert_eq!(filename, "agent-linux-amd64");
    }
}
