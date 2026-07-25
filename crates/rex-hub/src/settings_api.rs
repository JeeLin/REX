//! 设置 REST API。

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use crate::AppState;

type ApiResult<T> = Result<Json<T>, (StatusCode, Json<serde_json::Value>)>;

fn err(status: StatusCode, msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (
        status,
        Json(serde_json::json!({ "error": { "code": "ERROR", "message": msg } })),
    )
}

pub fn settings_routes() -> axum::Router<AppState> {
    axum::Router::new().route("/", axum::routing::get(get_settings).put(update_settings))
}

async fn get_settings(State(state): State<AppState>) -> ApiResult<serde_json::Value> {
    let db = state.db.clone();
    let result = tokio::task::spawn_blocking(move || -> Result<serde_json::Value, String> {
        let theme = db
            .get_setting("theme")
            .map_err(|e| e.to_string())?
            .unwrap_or_else(|| "dark".into());
        let language = db
            .get_setting("language")
            .map_err(|e| e.to_string())?
            .unwrap_or_else(|| "zh".into());
        let terminal_font = db
            .get_setting("terminal_font")
            .map_err(|e| e.to_string())?
            .unwrap_or_else(|| "JetBrains Mono".into());
        let terminal_font_size = db
            .get_setting("terminal_font_size")
            .map_err(|e| e.to_string())?
            .unwrap_or_else(|| "14".into());
        Ok(serde_json::json!({
            "theme": theme,
            "language": language,
            "terminal_font": terminal_font,
            "terminal_font_size": terminal_font_size,
        }))
    })
    .await
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;
    Ok(Json(result))
}

async fn update_settings(
    State(state): State<AppState>,
    Json(body): Json<std::collections::HashMap<String, String>>,
) -> ApiResult<serde_json::Value> {
    if body.is_empty() {
        return Ok(Json(serde_json::json!({ "ok": true })));
    }

    // 读取当前设置值用于对比
    let keys_to_check: Vec<String> = body.keys().cloned().collect();
    let db_snap = state.db.clone();
    let current = tokio::task::spawn_blocking(move || {
        let mut m = std::collections::HashMap::new();
        for key in &keys_to_check {
            if let Ok(Some(v)) = db_snap.get_setting(key) {
                m.insert(key.clone(), v);
            }
        }
        m
    })
    .await
    .unwrap_or_default();

    // 只收集实际变更的 key
    let changed: std::collections::HashMap<&str, &str> = body
        .iter()
        .filter(|(k, v)| current.get(k.as_str()).map(|s| s.as_str()) != Some(v.as_str()))
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();

    if changed.is_empty() {
        return Ok(Json(serde_json::json!({ "ok": true })));
    }

    tracing::info!(
        action = "SETTINGS_UPDATE",
        keys = ?changed.keys().collect::<Vec<_>>(),
        "settings updated"
    );

    let db = state.db.clone();
    let entries: Vec<(String, String)> = changed
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        for (k, v) in &entries {
            db.set_setting(k, v).map_err(|e| e.to_string())?;
        }
        Ok(())
    })
    .await
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;

    // 审计日志
    let audit_db = state.db.clone();
    let detail: Vec<&str> = changed.keys().copied().collect();
    let detail = detail.join(", ");
    let _ = tokio::task::spawn_blocking(move || {
        audit_db.write_audit_log(&crate::models::NewAuditEntry {
            action: "SETTINGS_UPDATE".into(),
            detail: Some(detail),
            result: "success".into(),
            ..Default::default()
        })
    })
    .await;

    Ok(Json(serde_json::json!({ "ok": true })))
}
