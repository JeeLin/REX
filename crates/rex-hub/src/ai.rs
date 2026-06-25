use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use axum::Json;
use futures_util::stream::Stream;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::Arc;

use crate::helpers::{bad_request, err_resp, ApiResponse, ErrorResponse};
use crate::routes::AppState;
use rusqlite::OptionalExtension;

// ── 数据模型 ──────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub context: ChatContext,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct ChatContext {
    pub database: Option<String>,
    pub tables: Option<Vec<String>>,
    pub dialect: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AiConfigResponse {
    pub provider: String,
    pub model: String,
    pub base_url: String,
    pub configured: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAiConfigRequest {
    pub provider: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub base_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AiConfigRow {
    pub provider: String,
    pub api_key_encrypted: String,
    pub model: String,
    pub base_url: String,
}

// ── System Prompt ──────────────────────────────────────

fn system_prompt(context: &ChatContext) -> String {
    let mut prompt = String::from(
        "你是一个 SQL 专家助手。根据用户提供的 SQL 上下文（数据库类型、表结构、当前查询），\
         帮助用户优化 SQL、分析问题、生成查询。\n\n\
         规则：\n\
         1. 回复中如果包含 SQL 代码，使用 ```sql 代码块包裹\n\
         2. 解释要简洁明了\n\
         3. 如果用户没有提供足够的上下文，主动询问",
    );

    if let Some(ref db) = context.database {
        prompt.push_str(&format!("\n\n当前数据库：{}", db));
    }
    if let Some(ref tables) = context.tables {
        if !tables.is_empty() {
            prompt.push_str(&format!("\n相关表：{}", tables.join(", ")));
        }
    }
    if let Some(ref dialect) = context.dialect {
        prompt.push_str(&format!("\n数据库方言：{}", dialect));
    }

    prompt
}

// ── 数据库操作 ────────────────────────────────────────

fn get_ai_config(db: &crate::db::Database) -> Result<Option<AiConfigRow>, String> {
    let conn = db.pool.get().map_err(|e| format!("数据库连接失败: {e}"))?;
    conn.query_row(
        "SELECT provider, api_key_encrypted, model, base_url FROM ai_config WHERE id = 'default'",
        [],
        |row| {
            Ok(AiConfigRow {
                provider: row.get(0)?,
                api_key_encrypted: row.get(1)?,
                model: row.get(2)?,
                base_url: row.get(3)?,
            })
        },
    )
    .optional()
    .map_err(|e| format!("查询 AI 配置失败: {e}"))
}

fn save_ai_config(
    db: &crate::db::Database,
    provider: &str,
    api_key_encrypted: &str,
    model: &str,
    base_url: &str,
) -> Result<(), String> {
    let conn = db.pool.get().map_err(|e| format!("数据库连接失败: {e}"))?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string();
    conn.execute(
        "INSERT OR REPLACE INTO ai_config (id, provider, api_key_encrypted, model, base_url, updated_at) \
         VALUES ('default', ?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![provider, api_key_encrypted, model, base_url, now],
    )
    .map_err(|e| format!("保存 AI 配置失败: {e}"))?;
    Ok(())
}

// ── API Handlers ──────────────────────────────────────

/// GET /api/ai/config — 获取 AI 配置
pub async fn get_config(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<AiConfigResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let config = get_ai_config(&state.db).map_err(|e| err_resp("DB_ERROR", &e))?;

    match config {
        Some(c) => Ok(Json(ApiResponse {
            data: AiConfigResponse {
                provider: c.provider,
                model: c.model,
                base_url: c.base_url,
                configured: !c.api_key_encrypted.is_empty(),
            },
        })),
        None => Ok(Json(ApiResponse {
            data: AiConfigResponse {
                provider: "openai".into(),
                model: "gpt-4o".into(),
                base_url: "https://api.openai.com/v1".into(),
                configured: false,
            },
        })),
    }
}

/// PUT /api/ai/config — 更新 AI 配置
pub async fn update_config(
    State(state): State<Arc<AppState>>,
    Json(input): Json<UpdateAiConfigRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let existing = get_ai_config(&state.db).map_err(|e| err_resp("DB_ERROR", &e))?;

    let provider = input.provider.unwrap_or_else(|| {
        existing
            .as_ref()
            .map(|c| c.provider.clone())
            .unwrap_or_else(|| "openai".into())
    });
    let model = input.model.unwrap_or_else(|| {
        existing
            .as_ref()
            .map(|c| c.model.clone())
            .unwrap_or_else(|| "gpt-4o".into())
    });
    let base_url = input.base_url.unwrap_or_else(|| {
        existing
            .as_ref()
            .map(|c| c.base_url.clone())
            .unwrap_or_else(|| "https://api.openai.com/v1".into())
    });

    let api_key_encrypted = if let Some(key) = input.api_key {
        if key.is_empty() {
            return Err(bad_request("api_key 不能为空"));
        }
        rex_ssh::crypto::encrypt(&key, &state.secret_key)
    } else {
        existing.map(|c| c.api_key_encrypted).unwrap_or_default()
    };

    save_ai_config(&state.db, &provider, &api_key_encrypted, &model, &base_url)
        .map_err(|e| err_resp("DB_ERROR", &e))?;

    tracing::info!(provider = %provider, model = %model, "AI config updated");
    Ok(StatusCode::OK)
}

/// POST /api/ai/chat — AI 聊天（SSE 流式响应）
pub async fn chat(
    State(state): State<Arc<AppState>>,
    Json(input): Json<ChatRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, (StatusCode, Json<ErrorResponse>)> {
    if input.messages.is_empty() {
        return Err(bad_request("消息列表不能为空"));
    }

    // 限制消息数量
    if input.messages.len() > 50 {
        return Err(bad_request("消息数量不能超过 50 条"));
    }

    // 限制单条消息长度
    for msg in &input.messages {
        if msg.content.len() > 50000 {
            return Err(bad_request("单条消息长度不能超过 50000 字符"));
        }
    }

    // 读取 AI 配置
    let config = get_ai_config(&state.db).map_err(|e| err_resp("DB_ERROR", &e))?;
    let config = config.ok_or_else(|| bad_request("AI 未配置，请先设置 API Key"))?;
    if config.api_key_encrypted.is_empty() {
        return Err(bad_request("AI 未配置 API Key"));
    }

    // 解密 API key
    let api_key = rex_ssh::crypto::decrypt(&config.api_key_encrypted, &state.secret_key)
        .map_err(|e| err_resp("CONFIG_ERROR", &format!("API Key 解密失败: {e}")))?;

    // 构建请求体
    let system_prompt = system_prompt(&input.context);
    let mut openai_messages: Vec<serde_json::Value> = vec![serde_json::json!({
        "role": "system",
        "content": system_prompt
    })];
    for msg in &input.messages {
        openai_messages.push(serde_json::json!({
            "role": msg.role,
            "content": msg.content
        }));
    }

    let url = format!("{}/chat/completions", config.base_url.trim_end_matches('/'));
    let body = serde_json::json!({
        "model": config.model,
        "messages": openai_messages,
        "stream": true,
    });

    // 发起 SSE 流式请求
    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| err_resp("AI_REQUEST_FAILED", &format!("AI 请求失败: {e}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "unknown error".into());
        return Err(err_resp(
            "AI_PROVIDER_ERROR",
            &format!("AI provider 返回 {status}: {error_text}"),
        ));
    }

    // 将 reqwest SSE 流转换为 axum SSE 流
    let stream = async_stream::stream! {
        use futures_util::StreamExt;
        let mut buffer = String::new();
        let mut full_response = String::new();
        let mut byte_stream = response.bytes_stream();

        while let Some(chunk_result) = byte_stream.next().await {
            let chunk = match chunk_result {
                Ok(c) => c,
                Err(e) => {
                    let _ = yield Ok(Event::default()
                        .event("error")
                        .data(format!("流读取错误: {e}")));
                    break;
                }
            };

            buffer.push_str(&String::from_utf8_lossy(&chunk));

            // 按行处理 SSE 数据
            while let Some(line_end) = buffer.find('\n') {
                let line = buffer[..line_end].trim().to_string();
                buffer = buffer[line_end + 1..].to_string();

                if line.is_empty() {
                    continue;
                }

                if let Some(data) = line.strip_prefix("data: ") {
                    let data = data.trim().to_string();
                    if data == "[DONE]" {
                        // 发送完成事件
                        let _ = yield Ok(Event::default()
                            .event("done")
                            .data(full_response.clone()));
                        break;
                    }

                    // 解析 OpenAI SSE chunk
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&data) {
                        if let Some(delta) = json["choices"][0]["delta"]["content"].as_str() {
                            full_response.push_str(delta);
                            let _ = yield Ok(Event::default()
                                .event("token")
                                .data(delta.to_string()));
                        }
                    }
                }
            }
        }

        // 如果流结束但没收到 [DONE]，仍发送完成事件
        if !full_response.is_empty() {
            let _ = yield Ok(Event::default()
                .event("done")
                .data(full_response));
        }
    };

    Ok(Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(30))
            .text("ping"),
    ))
}

// ── Tests ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_prompt_includes_context() {
        let ctx = ChatContext {
            database: Some("mydb".into()),
            tables: Some(vec!["users".into(), "orders".into()]),
            dialect: Some("mysql".into()),
        };
        let prompt = system_prompt(&ctx);
        assert!(prompt.contains("mydb"));
        assert!(prompt.contains("users, orders"));
        assert!(prompt.contains("mysql"));
    }

    #[test]
    fn system_prompt_without_context() {
        let ctx = ChatContext::default();
        let prompt = system_prompt(&ctx);
        assert!(prompt.contains("SQL 专家助手"));
        assert!(!prompt.contains("当前数据库"));
    }

    #[test]
    fn chat_request_validates_empty_messages() {
        let req = ChatRequest {
            messages: vec![],
            context: ChatContext::default(),
        };
        assert!(req.messages.is_empty());
    }

    #[test]
    fn ai_config_response_serializes() {
        let resp = AiConfigResponse {
            provider: "openai".into(),
            model: "gpt-4o".into(),
            base_url: "https://api.openai.com/v1".into(),
            configured: true,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("configured"));
        assert!(json.contains("gpt-4o"));
    }
}
