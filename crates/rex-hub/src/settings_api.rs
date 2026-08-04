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

// 默认配置值
fn default_settings() -> std::collections::HashMap<&'static str, &'static str> {
    let mut m = std::collections::HashMap::new();
    m.insert("theme", "dark");
    m.insert("language", "zh");
    m.insert("terminal_font", "JetBrains Mono");
    m.insert("terminal_font_size", "14");
    m.insert("auto_update", "true");
    m
}

async fn get_settings(State(state): State<AppState>) -> ApiResult<serde_json::Value> {
    let db = state.db.clone();
    let result = tokio::task::spawn_blocking(move || -> Result<serde_json::Value, String> {
        let defaults = default_settings();
        let stored = db.get_all_settings().map_err(|e| e.to_string())?;
        let mut map = serde_json::Map::new();
        for (key, default_val) in &defaults {
            let val = stored.get(*key).map(|s| s.as_str()).unwrap_or(default_val);
            // auto_update 存为 "true"/"false" 字符串，返回为 bool
            if *key == "auto_update" {
                map.insert(key.to_string(), serde_json::Value::Bool(val == "true"));
            } else {
                map.insert(key.to_string(), serde_json::Value::String(val.to_string()));
            }
        }
        // 追加 DB 中存在但 defaults 中没有的自定义 key
        for (k, v) in &stored {
            if !defaults.contains_key(k.as_str()) {
                map.insert(k.clone(), serde_json::Value::String(v.clone()));
            }
        }
        Ok(serde_json::Value::Object(map))
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
