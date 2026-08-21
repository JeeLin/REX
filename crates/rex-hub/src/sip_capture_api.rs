//! SIP 信令抓包 REST API（子任务 #3）。
//!
//! 抓包见 `sip_capture.rs`/`rex_sip::capture` 说明：UA₁ 经 baresip `sip_trace` 钩子捕获
//! 真实 SIP 信令字节(全局)，UA₂ 经 Hub 中继层捕获 `SipEvent` JSON(按 resource)，停止时
//! 合并导出为 libpcap(链路类型 RAW)。

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;

use crate::models::SipCaptureRecord;
use crate::AppState;

use rex_sip::capture::encode_pcap;

type ApiResult<T> = Result<Json<T>, (StatusCode, Json<serde_json::Value>)>;

fn err(status: StatusCode, msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (
        status,
        Json(serde_json::json!({ "error": { "code": "ERROR", "message": msg } })),
    )
}

pub fn sip_capture_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/:id/start", axum::routing::post(start_capture))
        .route("/:id/stop", axum::routing::post(stop_capture))
        .route("/:id/packets", axum::routing::get(list_packets))
        .route("/:id/pcap", axum::routing::get(export_pcap))
}

#[derive(serde::Deserialize)]
pub struct CaptureQuery {
    /// 最大返回报文数（分页/预览用），默认 1000。
    pub limit: Option<usize>,
}

async fn start_capture(
    State(state): State<AppState>,
    Path(resource_id): Path<String>,
) -> ApiResult<serde_json::Value> {
    state.sip_capture.start(&resource_id);
    Ok(Json(
        serde_json::json!({ "resource_id": resource_id, "active": true }),
    ))
}

async fn stop_capture(
    State(state): State<AppState>,
    Path(resource_id): Path<String>,
) -> ApiResult<serde_json::Value> {
    let packets = state.sip_capture.stop(&resource_id);
    Ok(Json(serde_json::json!({
        "resource_id": resource_id,
        "active": false,
        "count": packets.len(),
    })))
}

async fn list_packets(
    State(state): State<AppState>,
    Path(resource_id): Path<String>,
    Query(q): Query<CaptureQuery>,
) -> ApiResult<Vec<SipCaptureRecord>> {
    let packets = state.sip_capture.snapshot(&resource_id);
    let limit = q.limit.unwrap_or(1000);
    let records: Vec<SipCaptureRecord> = packets
        .into_iter()
        .take(limit)
        .map(|p| SipCaptureRecord {
            ts_us: p.ts_us,
            direction: p.direction,
            raw: p.raw,
        })
        .collect();
    Ok(Json(records))
}

async fn export_pcap(
    State(state): State<AppState>,
    Path(resource_id): Path<String>,
) -> Result<axum::response::Response, (StatusCode, Json<serde_json::Value>)> {
    let packets = state.sip_capture.snapshot(&resource_id);
    if packets.is_empty() {
        return Err(err(StatusCode::NOT_FOUND, "no capture data"));
    }
    let bytes = encode_pcap(&packets);
    let body = axum::body::Body::from(bytes);
    let resp = axum::response::Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/vnd.tcpdump.pcap")
        .header(
            "Content-Disposition",
            format!("attachment; filename=\"sip-capture-{resource_id}.pcap\""),
        )
        .body(body)
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;
    Ok(resp)
}
