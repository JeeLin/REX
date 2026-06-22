use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;

use crate::helpers::{ErrorBody, ErrorResponse};
use crate::routes::AppState;

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
                    message: format!("invalid os '{}', supported: {}", query.os, VALID_OS.join(", ")),
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

    // 构造文件名：agent-{os}-{arch}
    let filename = format!("agent-{}-{}", query.os, query.arch);
    let binaries_dir = state.data_dir.join("agent-binaries");
    let file_path = binaries_dir.join(&filename);

    // 安全检查：防止路径遍历
    if !file_path.starts_with(&binaries_dir) {
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

    // 检查文件是否存在
    match tokio::fs::read(&file_path).await {
        Ok(data) => {
            // 计算 SHA256
            let mut hasher = Sha256::new();
            hasher.update(&data);
            let sha256 = format!("{:x}", hasher.finalize());

            let version = std::env::var("REX_AGENT_VERSION").unwrap_or_else(|_| "unknown".to_string());

            let mut headers = header::HeaderMap::new();
            headers.insert(
                header::CONTENT_TYPE,
                header::HeaderValue::from_static("application/octet-stream"),
            );
            headers.insert(
                header::CONTENT_DISPOSITION,
                header::HeaderValue::from_str(&format!(
                    "attachment; filename=\"{filename}\""
                ))
                .unwrap(),
            );
            headers.insert(
                "X-Agent-Version".parse::<header::HeaderName>().unwrap(),
                header::HeaderValue::from_str(&version).unwrap(),
            );
            headers.insert(
                "X-Agent-SHA256".parse::<header::HeaderName>().unwrap(),
                header::HeaderValue::from_str(&sha256).unwrap(),
            );

            (headers, Body::from(data)).into_response()
        }
        Err(_) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: ErrorBody {
                    code: "NOT_FOUND".to_string(),
                    message: format!("no agent binary for {}/{}", query.os, query.arch),
                },
            }),
        )
            .into_response(),
    }
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
