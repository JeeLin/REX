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

#[derive(serde::Deserialize)]
struct SettingsUpdate {
    theme: Option<String>,
    language: Option<String>,
    terminal_font: Option<String>,
    terminal_font_size: Option<String>,
}

async fn update_settings(
    State(state): State<AppState>,
    Json(body): Json<SettingsUpdate>,
) -> ApiResult<serde_json::Value> {
    // 收集变更的 key
    let mut changed_keys = Vec::new();
    if body.theme.is_some() { changed_keys.push("theme"); }
    if body.language.is_some() { changed_keys.push("language"); }
    if body.terminal_font.is_some() { changed_keys.push("terminal_font"); }
    if body.terminal_font_size.is_some() { changed_keys.push("terminal_font_size"); }

    tracing::info!(
        action = "SETTINGS_UPDATE",
        keys = ?changed_keys,
        "settings updated"
    );

    let db = state.db.clone();
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        if let Some(v) = &body.theme {
            db.set_setting("theme", v).map_err(|e| e.to_string())?;
        }
        if let Some(v) = &body.language {
            db.set_setting("language", v).map_err(|e| e.to_string())?;
        }
        if let Some(v) = &body.terminal_font {
            db.set_setting("terminal_font", v)
                .map_err(|e| e.to_string())?;
        }
        if let Some(v) = &body.terminal_font_size {
            db.set_setting("terminal_font_size", v)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    })
    .await
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e))?;

    // 审计日志
    let audit_db = state.db.clone();
    let detail = changed_keys.join(", ");
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
