//! SIP 通话录音下载 API（子任务 #2）。
//!
//! 录音文件由 [`crate::sip_recording`] 落盘到 `<data_dir>/recordings/<cdr_id>.wav`，
//! CDR 的 `recording_url` 指向本端点。前端 CDR 详情经 `<audio>` 回放或 `<a download>` 下载。
//! 鉴权沿用 `AuthUser` 中间件（支持 `Authorization: Bearer` 与 `?token=` 两种方式）。

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;

use crate::AppState;

type ApiResult<T> = Result<Json<T>, (StatusCode, Json<serde_json::Value>)>;

pub fn sip_recording_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/:id/start", axum::routing::post(start_recording))
        .route("/:id/stop", axum::routing::post(stop_recording))
        .route("/:id", axum::routing::get(get_recording))
}

async fn start_recording(
    State(state): State<AppState>,
    Path(resource_id): Path<String>,
) -> ApiResult<serde_json::Value> {
    // 当前实现按 resource 全局开启（与 capture 同语义）；按 call_id 分文件在 CDR 状态机落盘。
    state.sip_recording.enable();
    Ok(Json(
        serde_json::json!({ "resource_id": resource_id, "active": true }),
    ))
}

async fn stop_recording(
    State(state): State<AppState>,
    Path(resource_id): Path<String>,
) -> ApiResult<serde_json::Value> {
    state.sip_recording.disable();
    Ok(Json(
        serde_json::json!({ "resource_id": resource_id, "active": false }),
    ))
}

/// `GET /api/sip/recordings/:id.wav` —— 返回该 CDR 的 WAV 录音文件。
pub async fn get_recording(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<axum::response::Response, (StatusCode, &'static str)> {
    // 文件名即 CDR id（含 `cdr:` 前缀，含冒号由 URL 编码安全承载）。
    let path = state.data_dir.join(crate::sip_recording::REC_DIR).join(&id);
    let bytes = std::fs::read(&path).map_err(|_| (StatusCode::NOT_FOUND, "recording not found"))?;
    let resp = axum::response::Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "audio/wav")
        .header(
            "Content-Disposition",
            format!("attachment; filename=\"{id}\""),
        )
        .body(axum::body::Body::from(bytes))
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "build response failed"))?;
    Ok(resp)
}
