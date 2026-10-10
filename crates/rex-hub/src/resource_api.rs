//! 资源管理 REST API。

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;

use crate::error::{api_error as err, ErrorBody};
use crate::models::{NewResource, Resource};
use crate::AppState;

type ApiResult<T> = Result<Json<T>, (StatusCode, Json<ErrorBody>)>;

/// 资源路由（嵌套在 /api/environments 下）
pub fn resource_routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route(
            "/{env_id}/resources",
            axum::routing::get(list_resources).post(create_resource),
        )
        .route(
            "/{env_id}/resources/{resource_id}",
            axum::routing::get(get_resource)
                .put(update_resource)
                .delete(delete_resource),
        )
        .route(
            "/{env_id}/resources/{resource_id}/active-account",
            axum::routing::post(set_active_account),
        )
}

// --- API handlers ---

async fn list_resources(
    State(state): State<AppState>,
    Path(env_id): Path<String>,
) -> ApiResult<Vec<Resource>> {
    let db = state.db.clone();
    let mut resources = tokio::task::spawn_blocking(move || db.list_resources_by_env(&env_id))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;
    // 解密每条资源的 config_json
    for r in &mut resources {
        if crate::resource_conn::has_config_json(&r.config_json) {
            if let Ok(dec) = state.crypto.decrypt(&r.config_json) {
                r.config_json = dec;
            } else {
                r.config_json = String::new();
                tracing::warn!(
                    resource_id = %r.id,
                    "config_json decrypt failed (data key mismatch)"
                );
            }
        }
    }
    Ok(Json(resources))
}

async fn get_resource(
    State(state): State<AppState>,
    Path((_env_id, resource_id)): Path<(String, String)>,
) -> ApiResult<Resource> {
    let db = state.db.clone();
    let mut resource = tokio::task::spawn_blocking(move || db.get_resource(&resource_id))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "resource not found"))?;
    // 解密 config_json
    if crate::resource_conn::has_config_json(&resource.config_json) {
        match state.crypto.decrypt(&resource.config_json) {
            Ok(dec) => resource.config_json = dec,
            Err(_) => return Err(crate::resource_conn::credential_decrypt_error()),
        }
    }
    Ok(Json(resource))
}

async fn create_resource(
    State(state): State<AppState>,
    Path(env_id): Path<String>,
    Json(mut body): Json<NewResource>,
) -> ApiResult<Resource> {
    tracing::info!(
        action = "RESOURCE_CREATE",
        env_id = %env_id,
        protocol = %body.protocol,
        name = %body.name,
        host = %body.host,
        "creating resource"
    );

    if body.name.trim().is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "name is required"));
    }
    if body.host.trim().is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "host is required"));
    }
    // 加密 config_json 中的凭据
    if let Some(ref cfg) = body.config_json {
        match state.crypto.encrypt(cfg) {
            Ok(enc) => body.config_json = Some(enc),
            Err(e) => return Err(err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string())),
        }
    }
    // 验证环境存在
    let db = state.db.clone();
    let env_id_check = env_id.clone();
    let env_exists = tokio::task::spawn_blocking(move || db.get_environment(&env_id_check))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .is_some();
    if !env_exists {
        return Err(err(StatusCode::NOT_FOUND, "environment not found"));
    }
    let db = state.db.clone();
    let resource = tokio::task::spawn_blocking(move || db.create_resource(&env_id, &body))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;
    // 审计日志
    let audit_db = state.db.clone();
    let res_name = resource.name.clone();
    let res_env_id = resource.environment_id.clone();
    let _ = tokio::task::spawn_blocking(move || {
        audit_db.write_audit_log(&crate::models::NewAuditEntry {
            action: "RESOURCE_CREATE".into(),
            target: Some(res_name),
            environment_id: Some(res_env_id),
            result: "success".into(),
            ..Default::default()
        })
    })
    .await;
    Ok(Json(resource))
}

async fn update_resource(
    State(state): State<AppState>,
    Path((env_id, resource_id)): Path<(String, String)>,
    Json(mut body): Json<NewResource>,
) -> ApiResult<Resource> {
    tracing::info!(
        action = "RESOURCE_UPDATE",
        env_id = %env_id,
        resource_id = %resource_id,
        protocol = %body.protocol,
        name = %body.name,
        "updating resource"
    );

    // 加密 config_json 中的凭据
    if let Some(ref cfg) = body.config_json {
        match state.crypto.encrypt(cfg) {
            Ok(enc) => body.config_json = Some(enc),
            Err(e) => return Err(err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string())),
        }
    }
    let db = state.db.clone();
    let resource =
        tokio::task::spawn_blocking(move || db.update_resource(&env_id, &resource_id, &body))
            .await
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
            .map_err(|e| {
                let msg = e.to_string();
                if msg.contains("not found") {
                    err(StatusCode::NOT_FOUND, &msg)
                } else {
                    err(StatusCode::INTERNAL_SERVER_ERROR, &msg)
                }
            })?;

    // 审计日志
    let audit_db = state.db.clone();
    let res_name = resource.name.clone();
    let res_env_id = resource.environment_id.clone();
    let _ = tokio::task::spawn_blocking(move || {
        audit_db.write_audit_log(&crate::models::NewAuditEntry {
            action: "RESOURCE_UPDATE".into(),
            target: Some(res_name),
            environment_id: Some(res_env_id),
            result: "success".into(),
            ..Default::default()
        })
    })
    .await;

    Ok(Json(resource))
}

#[derive(serde::Deserialize)]
struct SetActiveAccountBody {
    account_id: String,
}

// 专用端点：仅切换 SIP 资源的生效账户，前端无需先 get 全量再 update。
async fn set_active_account(
    State(state): State<AppState>,
    Path((env_id, resource_id)): Path<(String, String)>,
    Json(body): Json<SetActiveAccountBody>,
) -> ApiResult<Resource> {
    tracing::info!(
        action = "RESOURCE_SET_ACTIVE_ACCOUNT",
        env_id = %env_id,
        resource_id = %resource_id,
        account_id = %body.account_id,
        "switching active sip account"
    );

    let db = state.db.clone();
    let crypto = state.crypto.clone();
    let account_id = body.account_id.clone();
    let resource = tokio::task::spawn_blocking(move || {
        db.set_resource_active_account(&crypto, &env_id, &resource_id, &account_id)
    })
    .await
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("not found") {
            err(StatusCode::NOT_FOUND, &msg)
        } else {
            err(StatusCode::BAD_REQUEST, &msg)
        }
    })?;

    // 审计日志：后台异步写入，不阻塞响应返回（fire-and-forget）。
    let audit_db = state.db.clone();
    let res_name = resource.name.clone();
    let res_env_id = resource.environment_id.clone();
    tokio::task::spawn_blocking(move || {
        let _ = audit_db.write_audit_log(&crate::models::NewAuditEntry {
            action: "RESOURCE_SET_ACTIVE_ACCOUNT".into(),
            target: Some(res_name),
            environment_id: Some(res_env_id),
            result: "success".into(),
            ..Default::default()
        });
    });

    Ok(Json(resource))
}

async fn delete_resource(
    State(state): State<AppState>,
    Path((env_id, resource_id)): Path<(String, String)>,
) -> ApiResult<serde_json::Value> {
    let db = state.db.clone();
    let check_id = resource_id.clone();
    let resource = tokio::task::spawn_blocking(move || db.get_resource(&check_id))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;
    if resource.is_none() {
        return Err(err(StatusCode::NOT_FOUND, "resource not found"));
    }
    let res_name = resource.map(|r| r.name).unwrap_or_default();

    let db = state.db.clone();
    tracing::info!(
        action = "RESOURCE_DELETE",
        env_id = %env_id,
        resource_id = %resource_id,
        resource_name = %res_name,
        "deleting resource"
    );

    let del_env_id = env_id.clone();
    let del_id = resource_id.clone();
    tokio::task::spawn_blocking(move || db.delete_resource(&del_env_id, &del_id))
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;

    // 审计日志
    let audit_db = state.db.clone();
    let _ = tokio::task::spawn_blocking(move || {
        audit_db.write_audit_log(&crate::models::NewAuditEntry {
            action: "RESOURCE_DELETE".into(),
            target: Some(res_name),
            result: "success".into(),
            ..Default::default()
        })
    })
    .await;

    Ok(Json(serde_json::json!({ "ok": true })))
}

// --- Test connection ---

/// 探测类请求的默认超时：Hub 直连 TCP、Agent connect 回帧、redis PING 共用同一口径。
const PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// 探测超时的用户可见文案。
const PROBE_TIMEOUT_MSG: &str = "connection timed out";

/// 下发给 Agent 的探测请求 id 前缀（与 `agent_ws::open_agent_session` 同口径）。
const AGENT_REQUEST_ID_PREFIX: &str = "req_";

/// agent 模式下该环境无在线 Agent 的用户可见文案。
///
/// 保留 base 版 "no online agent available" 的语义并补上可操作提示（用户能直接
/// 看出是「Agent 没上线」而非「目标连不上」）。与 [`AGENT_NOT_CONNECTED_MSG`]
/// 的区别：Agent 在册但离线 vs Agent 在线记录存在而 WS 未建立。
const NO_ONLINE_AGENT_MSG: &str = "no online agent available for this environment: \
     start or reconnect an Agent for it, then retry the test connection";

/// agent 在册且在线、但隧道 WS 尚未建立时的文案。
const AGENT_NOT_CONNECTED_MSG: &str = "agent not connected";

#[derive(serde::Deserialize)]
pub struct TestConnectionRequest {
    pub protocol: String,
    pub host: String,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub config_json: Option<String>,
    pub environment_id: Option<String>,
}

#[derive(serde::Serialize)]
pub struct TestConnectionResult {
    pub ok: bool,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

/// 取环境的真实 `connection_mode`，供 [`crate::resource_conn::resolve_agent_mode`] 判定。
///
/// 真实数据路径按 `connection_mode` 选 Agent 连接器，测试连接必须同口径，否则内网目标
/// 在 Hub 直连探测上必然 `No route to host`。取值失败（DB 报错或 spawn_blocking
/// panic）记 warn 后回退直连 —— 静默 `unwrap_or(false)` 会把基础设施故障伪装成
/// 「非 agent 环境」，排查时看不出探测为何走了直连。
///
/// 返回 `None` 表示**拿不到 mode**：无 `environment_id` / 环境查不到 / 取值失败，
/// 三者都是「不需要走 Agent」，调用方据此回退 Hub 直连。
///
/// 只负责取 mode，**不**判定「是不是 agent 模式」——那是
/// [`crate::resource_conn::resolve_agent_mode`] 的收敛点：把真实 mode 原样交给它，
/// 不在这里查完库又把结论退化成字面量回传（那样既让收敛点失效，也丢掉真实取值）。
/// 「agent 模式下无在线 Agent」属探测失败而非「不需要走 Agent」，由调用方以
/// `Some(Err(..))` 表达（见 [`test_connect_via_agent`] 关于 `None` 语义的说明）。
async fn env_connection_mode(state: &crate::AppState, env_id: Option<&str>) -> Option<String> {
    let env_id = env_id?;
    let db = state.db.clone();
    let eid = env_id.to_string();
    let lookup_eid = eid.clone();
    match tokio::task::spawn_blocking(move || db.get_environment(&lookup_eid)).await {
        Ok(Ok(env)) => env.map(|e| e.connection_mode),
        Ok(Err(e)) => {
            tracing::warn!(
                env_id = %eid,
                error = %e,
                "environment lookup failed, falling back to direct probe"
            );
            None
        }
        Err(e) => {
            tracing::warn!(
                env_id = %eid,
                error = %e,
                "environment lookup task failed, falling back to direct probe"
            );
            None
        }
    }
}

/// Hub 直连 TCP 可达性探测（ssh / sftp / sql / mysql / postgresql 共用）。
///
/// 只验地址可达，不验凭据。Agent 侧走隧道探测：sql/redis 连真库验凭据，
/// ssh 走 `probe_ssh` 握手+认证验凭据；sftp 走真实 SFTP 连接器验凭据。
/// 深度不同，但两者都只回答「这个资源现在能不能连」。
async fn direct_tcp_probe(host: &str, port: u16) -> Result<(), String> {
    let addr = if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    };
    match tokio::time::timeout(PROBE_TIMEOUT, tokio::net::TcpStream::connect(&addr)).await {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(e)) => Err(format!("TCP connect failed: {e}")),
        Err(_) => Err(PROBE_TIMEOUT_MSG.to_string()),
    }
}

/// Hub 直连 redis 探测：真实握手 + `PING`（[`probe_target`] 的 redis 回退）。
///
/// 裸 TCP 可达不代表对端是 redis，故直连回退走 `redis::Client` 完整握手并
/// `PING`（与 Agent 侧 `agent_redis` 同口径）。口令不在此处注入：直连回退
/// 只验可达，验凭据由 agent 侧与 redis 数据面负责。
async fn direct_redis_probe(host: &str, port: u16) -> Result<(), String> {
    let redis_host = if host.contains(':') {
        format!("[{host}]")
    } else {
        host.to_string()
    };
    let addr = format!("redis://{redis_host}:{port}/");
    match tokio::time::timeout(PROBE_TIMEOUT, async {
        let client = redis::Client::open(addr.as_str()).map_err(|e| format!("redis error: {e}"))?;
        let mut conn = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| format!("redis connect error: {e}"))?;
        redis::Cmd::new()
            .arg("PING")
            .query_async::<String>(&mut conn)
            .await
            .map_err(|e| format!("redis PING failed: {e}"))?;
        Ok::<(), String>(())
    })
    .await
    {
        Ok(r) => r,
        Err(_) => Err(PROBE_TIMEOUT_MSG.to_string()),
    }
}

/// 从 `config_json` 取 sqlite 的 `file_path`，缺省为内存库。
///
/// `sql`+`subtype=sqlite` 与 bare `sqlite` 两个 arm 都要用它：sqlite 的 host
/// 就是文件路径，解析规则必须同源，否则改任一处会让两个 arm 静默失配。
fn sqlite_probe_path(config_json: Option<&str>) -> String {
    config_json
        .and_then(|c| serde_json::from_str::<serde_json::Value>(c).ok())
        .and_then(|v| v.get("file_path")?.as_str().map(String::from))
        .unwrap_or_else(|| ":memory:".into())
}

/// Hub 直连 sqlite 探测：打开 `config_json.file_path` 并跑一次 `SELECT 1`。
///
/// sqlite 无网络层，Agent 侧收到的是同一个文件路径（`SqliteConnector` 以
/// `ConnectRequest.host` 作 `db_path`），故直连回退就是本机打开该文件。
/// 同步 rusqlite 调用，与迁移前 inline 版本同线程同语义（打开+一条查询即返回）。
async fn direct_sqlite_probe(path: &str) -> Result<(), String> {
    match rusqlite::Connection::open(path) {
        Ok(conn) => {
            if conn.execute_batch("SELECT 1").is_ok() {
                Ok(())
            } else {
                Err("SQLite query failed".into())
            }
        }
        Err(e) => Err(format!("SQLite open failed: {e}")),
    }
}

/// Hub 直连 S3 探测：`list_buckets`（真正验凭据）。
///
/// S3 是 HTTP 签名协议，裸 TCP 可达不代表凭据有效，故直连回退发一次真实
/// ListBuckets；endpoint/region/凭据取自 `config_json`（S3 的 host/port 不生效）。
async fn direct_s3_probe(config_json: Option<&str>) -> Result<(), String> {
    let Some(cfg) = config_json else {
        return Err("missing config_json for S3".into());
    };
    let v: serde_json::Value = serde_json::from_str(cfg).unwrap_or(serde_json::Value::Null);
    let endpoint = v.get("endpoint").and_then(|e| e.as_str()).unwrap_or("");
    let access_key = v.get("access_key").and_then(|e| e.as_str()).unwrap_or("");
    let secret_key = v.get("secret_key").and_then(|e| e.as_str()).unwrap_or("");
    let region = v
        .get("region")
        .and_then(|e| e.as_str())
        .unwrap_or("us-east-1");
    if endpoint.is_empty() || access_key.is_empty() || secret_key.is_empty() {
        return Err("missing endpoint, access_key, or secret_key".into());
    }
    let config = aws_sdk_s3::Config::builder()
        .endpoint_url(endpoint)
        .region(aws_sdk_s3::config::Region::new(region.to_string()))
        .credentials_provider(aws_sdk_s3::config::Credentials::new(
            access_key.to_string(),
            secret_key.to_string(),
            None,
            None,
            "rex-hub-test",
        ))
        .behavior_version_latest()
        .build();
    let client = aws_sdk_s3::Client::from_conf(config);
    match tokio::time::timeout(PROBE_TIMEOUT, client.list_buckets().send()).await {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(e)) => Err(format!("S3 ListBuckets failed: {e}")),
        Err(_) => Err("S3 request timed out".into()),
    }
}

/// Hub 直连 SIP 探测：建真 UA₂ 做一次真实 REGISTER，验完即 drop（释放注册）。
///
/// 与 Agent 侧 `handle_connect_sip` 的 probe 分支同一实现：`rex_sip::SipUa::real`
/// 建 UA、`register` 发 REGISTER、`PROBE_TIMEOUT` 兜 5s 超时，语义与深度都对齐。
/// 配置能解析不等于 SIP server 可达，恒返回 `Ok` 的「假成功」正是本函数要消除的
/// （base 版 sip arm 无任何网络探测）。
///
/// 阻塞点：baresip 的账户建立 / `ua_register` 在 `re_main` 主线程内完成，Hub 侧
/// 只 await 其 oneshot 回包（与 `sip_ws` 跑 UA₁ 同款，不额外隔离线程）。
async fn direct_sip_probe(cfg: rex_sip::SipConfig) -> Result<(), String> {
    // `PROBE_TIMEOUT` 含 UA 构造（首次还要初始化 baresip 运行时），与 Agent 侧
    // `SIP_PROBE_TIMEOUT` 同口径：整个探测过程共用一个上限。
    match tokio::time::timeout(PROBE_TIMEOUT, sip_probe_register(cfg)).await {
        Ok(r) => r,
        Err(_) => Err("SIP probe timed out".into()),
    }
}

/// [`direct_sip_probe`] 的实际 REGISTER 流程（与 Agent 侧 probe 同序）。
///
/// 与 Agent 侧 `handle_connect_sip` / Hub 侧 `sip_ws` 的 UA₁ 构造同款：直接 await，
/// 不 `spawn_blocking` —— `BaresipSipUa::new` 的裸指针只活到 `mqueue_push` 这一句，
/// 不跨 await，`SipUa` 又已 `unsafe impl Send + Sync`，故 future 本身是 Send，
/// 无需额外隔离线程。整个流程由 `PROBE_TIMEOUT` 兜住上界。
async fn sip_probe_register(cfg: rex_sip::SipConfig) -> Result<(), String> {
    // `SipUa` 只导出枚举本身，`register` 定义在 `SipUaTrait` 上（与 `sip_ws` 同款）。
    use rex_sip::SipUaTrait;

    let ua = rex_sip::SipUa::real(cfg)
        .await
        .map_err(|e| format!("SIP UA init failed: {e}"))?;
    // UA 在本作用域结束即 drop → `ua_stop_register` 释放注册（与 Agent 侧 probe
    // 同款「握手+认证后拆除」的轻探针，不留注册状态）。
    ua.register()
        .await
        .map_err(|e| format!("SIP REGISTER failed: {e}"))
}

/// 从下发给 Agent 的 connect config 中取出凭据片段。
///
/// Agent 侧驱动会把连接串回显到错误里（`agent_sql` 的
/// `SQL connection failed: {e}`、`agent_file::probe_s3` 的
/// `S3 verify failed: {e}`——后者内层是 `S3 HeadBucket failed bucket=...`），
/// 该文案经 `connect_error` 进用户可见 toast 又进日志；凭据只应留在内存里。
///
/// 收集口径 = 「本次下发的 config 里所有可能出现在错误串中的明文口令」，
/// 少收一个键就是一处 redact 盲区：键名以各协议 driver 实际读取的为准
/// （redis/SQL/SFTP 读 `password`，S3 读 `access_key`/`secret_key`，
/// SIP 的平坦 config 带 `password`，ssh/sftp 私钥两种拼写）。
/// 空串由 `redact_secrets` 忽略（空密码不该把整条文案抹成 `***`）。
fn connect_config_secrets(cfg: &serde_json::Value) -> Vec<String> {
    let str_field = |key: &str| {
        cfg.get(key)
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(String::from)
    };
    let mut secrets: Vec<String> = ["password", "access_key", "secret_key"]
        .into_iter()
        .filter_map(str_field)
        .collect();
    secrets.extend(rex_common::resource_config::config_private_key(cfg));
    secrets
}

/// 抹掉 Agent 回传错误中的凭据明文（用户可见文案与日志共用这一份 redact 结果）。
fn redact_agent_error(message: &str, cfg: &serde_json::Value) -> String {
    let secrets = connect_config_secrets(cfg);
    let refs: Vec<&str> = secrets.iter().map(String::as_str).collect();
    crate::error::redact_secrets(message, &refs)
}

/// 一次 `pending_requests` 登记的所有权句柄：Drop 时反注册。
///
/// `pending_requests` 是无界 `HashMap`（`agent_ws::AgentTunnelState`），且只在
/// 收到 Agent 回帧时按 request_id remove。发送失败 / oneshot 通道关闭 / 超时
/// 这些提前退出路径若各自补 `remove`，新增分支极易再漏 —— 每漏一次就永久残留
/// 一个 `req_*` 键与 oneshot Sender，反复「测试连接」即无界增长。Drop 兜底让
/// 清理与控制流无关（对照 `tunnel_ws` 握手超时分支的手工 remove）。
struct PendingRequestSlot {
    tunnel: Arc<crate::agent_ws::AgentTunnelState>,
    request_id: String,
}

impl PendingRequestSlot {
    async fn register(
        tunnel: Arc<crate::agent_ws::AgentTunnelState>,
        request_id: String,
    ) -> (
        Self,
        tokio::sync::oneshot::Receiver<crate::agent_ws::ConnectResponse>,
    ) {
        let (tx, rx) = tokio::sync::oneshot::channel();
        tunnel
            .pending_requests
            .write()
            .await
            .insert(request_id.clone(), tx);
        (Self { tunnel, request_id }, rx)
    }
}

impl Drop for PendingRequestSlot {
    fn drop(&mut self) {
        let tunnel = self.tunnel.clone();
        let request_id = std::mem::take(&mut self.request_id);
        // 正常路径上回帧消费方已 remove（remove 缺失键是 no-op）；抢不到写锁时
        // 退到后台任务，避免因锁竞争再漏一次清理。
        if let Ok(mut pending) = tunnel.pending_requests.try_write() {
            pending.remove(&request_id);
            return;
        }
        tokio::spawn(async move {
            tunnel.pending_requests.write().await.remove(&request_id);
        });
    }
}

/// agent 模式下把探测请求转发给 Agent（Agent 侧按 protocol 分发）。
///
/// 返回 `None` 表示**不适用**，只有一种成因：环境不是 agent 模式
/// （无 environment_id / 环境查不到 / `connection_mode != "agent"`）——调用方据此
/// 回退 Hub 直连。
///
/// 一旦判定为 agent 模式，后续每种失败都返回 `Some(Err(..))` 而**不是** `None`，
/// 包括「无在线 Agent」与「WS 未建立」：这些是探测失败，绝不能降级成 Hub 直连，
/// 否则内网目标会由 Hub 本机去连（`No route to host`），既掩盖了基础设施缺失
/// 也违背「agent-mode 测试连接必在正确主机上执行」。`Some(Err(..))` 与
/// `None` 的区别即调用方是否回退。
///
/// 探测请求带 `probe` 标记：ssh 在 Agent 侧走 `probe_ssh` 完成**真实认证**后即时
/// 拆连（`connect_with_handle` 跑完 TCP → KEX → authenticate；错凭据回
/// `connect_error` 不入共享会话池 `ssh_handles`）；sftp 不走 `probe`（其
/// `connect_with_handle` 会启动 shell，纯 SFTP 服务端会拒绝），改走 Agent 侧
/// 真实 SFTP 连接器 `handle_connect_file`，用 SessionOpened/SessionError 回传
/// 结果。其余协议无此轻量路径，标记被 Agent 忽略，走各自原有的 connect 流程。
///
/// config 形状由 [`crate::resource_conn::merge_resource_config`] 统一构建
/// （顶层 username 归一后为权威字段，Agent 侧以空用户认证必被拒，CR15）。
async fn test_connect_via_agent(
    state: &crate::AppState,
    protocol: &str,
    host: &str,
    port: u16,
    username: Option<&str>,
    config_json: Option<&str>,
    environment_id: Option<&str>,
) -> Option<Result<(), String>> {
    // 真实 `connection_mode` 只查一次（[`env_connection_mode`]），原样交给
    // `resolve_agent_mode` 这一个收敛点判定 —— 不在这里先判一次 `== "agent"`
    // 再把 `"agent"` 字面量回传（那既让收敛点失效，也让 mode 判定散落两处）。
    // `resolve_agent_mode` 对非 agent 模式返回 `use_agent: false`，据此外推
    // 「不适用」；与 `resource_conn::resolve_agent_for_resource` / `terminal_ws`
    // 两处调用点「传 `env.connection_mode`」同构。
    let connection_mode = env_connection_mode(state, environment_id).await?;
    let db = state.db.clone();
    let eid = environment_id.unwrap_or("").to_string();
    let resolution = tokio::task::spawn_blocking(move || {
        crate::resource_conn::resolve_agent_mode(&db, &eid, &connection_mode)
    })
    .await
    .map_err(|e| {
        tracing::warn!(
            error = %e,
            "agent lookup task failed while resolving online agent"
        );
        "failed to look up online agent for this environment".to_string()
    });
    // spawn_blocking 的 JoinError 与「无在线 Agent」同为探测失败，绝不能降级成
    // `None`（调用方会因此直连 Hub 本机）。`?` 在 `Option` 返回值的函数里只能
    // 解 `Option`，故各臂显式回 `Some(Err(..))`。
    let resolution = match resolution {
        Ok(r) => r,
        Err(msg) => return Some(Err(msg)),
    };
    // 非 agent 模式是唯一允许回 `None` 的情形（调用方据此直连）。
    if !resolution.use_agent {
        return None;
    }
    // 「无在线 Agent」是**探测失败**而非「不适用」：返回 `Some(Err(..))`，
    // 调用方绝不回退 Hub 直连。回退成 `None` 会让 agent-mode 环境在内网目标上
    // 由 Hub 本机去连（`TCP connect failed` / `No route to host`），既把基础设施
    // 缺失伪装成目标不可达，也与 S7-F7/F8/F9「测试连接必须在正确主机上执行」
    // 的目标相反。base 版此处即 `None => Err("no online agent available")`。
    let Some(agent_id) = resolution.agent_id else {
        return Some(Err(NO_ONLINE_AGENT_MSG.to_string()));
    };

    let conn = {
        let conns = state.agent_tunnel.connections.read().await;
        conns.get(&agent_id).cloned()
    };
    let conn = match conn {
        Some(c) => c,
        None => return Some(Err(AGENT_NOT_CONNECTED_MSG.into())),
    };

    let request_id = format!(
        "{AGENT_REQUEST_ID_PREFIX}{}",
        &uuid::Uuid::new_v4().to_string()[..8]
    );
    let connect_config = crate::resource_conn::merge_resource_config(
        host,
        port,
        username.unwrap_or(""),
        &match config_json {
            // config_json 解析失败按空配置处理（静默丢弃），合并逻辑归口
            // `merge_resource_config`（与 `file_api::agent_file_config` 同源）。
            Some(cfg_str) => serde_json::from_str::<serde_json::Value>(cfg_str)
                .unwrap_or(serde_json::Value::Null),
            None => serde_json::Value::Null,
        },
    );
    let connect_msg = serde_json::json!({
        "type": "connect",
        "payload": {
            "request_id": request_id,
            "resource_id": "test",
            "protocol": protocol,
            "config": connect_config,
            // 探测是「握手+认证后拆除」的轻探针：ssh 走 Agent 侧 `probe_ssh`
            // （`connect_with_handle` 完成认证）；s3 走 Agent 侧 `S3Connector::verify`
            // （list_buckets/head_bucket 真正验凭据 —— `connect_from_request` 只建
            // client 不验凭据，测试时无 bucket 会假阳性）；sip 走 Agent 侧 UA₂ 的
            // 真实 REGISTER（`handle_connect_sip` probe 分支），验完即 drop UA 释放
            // 注册。sftp/sql/redis/mysql/postgresql/sqlite 不走探测（sftp 的
            // probe_ssh 会启动 shell，纯 SFTP 服务端拒绝；其余需要真会话）。
            // sftp 走 Agent 侧 `handle_connect_file` 的真实 SFTP 连接器
            // （`channel_open_session` + sftp subsystem，完成认证且不开 PTY/shell），
            // `probe=false` 让 Hub 把 Test Connection 当一次「即开即关」的真实 sftp
            // 会话处理，借助已有的 SessionOpened/SessionError 回传回路回收结果。
            "probe": matches!(protocol, "ssh" | "s3" | "sip"),
        }
    });

    // 句柄活到本函数返回：发送失败 / 通道关闭 / 超时等提前 return 路径一律由
    // Drop 反注册，新增分支无需记得补 remove。
    let (_slot, resp_rx) =
        PendingRequestSlot::register(state.agent_tunnel.clone(), request_id.clone()).await;

    if conn
        .sender
        .send(crate::agent_ws::AgentEvent::Text(connect_msg.to_string()))
        .await
        .is_err()
    {
        return Some(Err("failed to send connect request to agent".into()));
    }

    // 等待 Agent 响应
    let resp = match tokio::time::timeout(PROBE_TIMEOUT, resp_rx).await {
        Ok(Ok(resp)) => resp,
        Ok(Err(_)) => return Some(Err("agent response channel closed".into())),
        Err(_) => return Some(Err("agent connection timed out".into())),
    };
    if let Some(e) = resp.error {
        return Some(Err(redact_agent_error(&e, &connect_config)));
    }
    // 探测成功：关闭通道。Hub 侧 `channels` 映射交给 Agent 回帧（`closed`）清理，
    // 本地不删 —— Agent 尚未处理 close 的窗口内必须留着映射，否则后续帧无处可路由。
    if let Some(channel_id) = resp.channel_id {
        let close_msg = serde_json::json!({
            "type": "close",
            "payload": { "channel_id": channel_id }
        });
        let _ = conn
            .sender
            .send(crate::agent_ws::AgentEvent::Text(close_msg.to_string()))
            .await;
    }
    Some(Ok(()))
}

/// Hub 直连回退探测：agent 隧道不适用时（`test_connect_via_agent` → `None`）才跑。
///
/// 收敛点只统一「先 Agent、不适用才直连」的骨架；**直连段本身按协议各不相同**，
/// 强行统一成 TCP 探针会让「凭据错误 / 路径不存在」退化成假阳性，故由 `fallback`
/// 闭包按协议带入各自的 `direct_*_probe`：
///
/// | 协议 | 直连回退 | 为何不能用 TCP 探针代替 |
/// |------|----------|------------------------|
/// | ssh/sftp/sql/mysql/postgresql | [`direct_tcp_probe`] | 只验地址可达，与 Agent 侧深度不同但都不验凭据（见该函数说明） |
/// | redis | [`direct_redis_probe`] PING | 握手协议，裸 TCP 连上不代表是 redis |
/// | sqlite | [`direct_sqlite_probe`] | sqlite 无网络层，`host` 是文件路径 |
/// | s3 | [`direct_s3_probe`] ListBuckets | HTTP 签名协议，裸 TCP 连上不代表凭据有效 |
/// | sip | [`direct_sip_probe`] REGISTER | SIP 有自己的信令栈，见该函数说明 |
///
/// 本表是这份清单的**唯一**来源：新增协议请在此登记并新增对应 `direct_*_probe`，
/// 不要在 `test_connection` 的 arm 里另写一份 `if let Some(r) = … else { … }`。
// 参数数超过 clippy 默认上限：前 7 个是全协议共有的下发形状（与 Agent 侧
// ConnectRequest 同构），第 8 个才是各协议的直连口径；把直连口径打包成结构体
// 反而会逼每个 arm 填满自己用不上的字段。
#[allow(clippy::too_many_arguments)]
async fn probe_target<F, Fut>(
    state: &crate::AppState,
    protocol: &str,
    host: &str,
    port: u16,
    username: Option<&str>,
    config_json: Option<&str>,
    environment_id: Option<&str>,
    fallback: F,
) -> Result<(), String>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<(), String>>,
{
    match test_connect_via_agent(
        state,
        protocol,
        host,
        port,
        username,
        config_json,
        environment_id,
    )
    .await
    {
        Some(r) => r,
        None => fallback().await,
    }
}

pub async fn test_connection(
    State(state): State<crate::AppState>,
    Json(body): Json<TestConnectionRequest>,
) -> ApiResult<TestConnectionResult> {
    // For S3, log endpoint from config_json instead of empty host
    let log_host = if body.protocol == "s3" {
        body.config_json
            .as_ref()
            .and_then(|c| serde_json::from_str::<serde_json::Value>(c).ok())
            .and_then(|v| v.get("endpoint")?.as_str().map(String::from))
            .unwrap_or_default()
    } else {
        body.host.clone()
    };
    tracing::info!(
        action = "TEST_CONNECTION",
        protocol = %body.protocol,
        host = %log_host,
        port = body.port.unwrap_or(0),
        "testing connection"
    );

    let start = std::time::Instant::now();
    let result = match body.protocol.as_str() {
        "ssh" | "sftp" => {
            let port = body.port.unwrap_or(22);
            let host = body.host.clone();
            probe_target(
                &state,
                &body.protocol,
                &body.host,
                port,
                body.username.as_deref(),
                body.config_json.as_deref(),
                body.environment_id.as_deref(),
                || direct_tcp_probe(&host, port),
            )
            .await
        }
        "redis" => {
            let port = body.port.unwrap_or(6379);
            let redis_host = body.host.clone();
            probe_target(
                &state,
                &body.protocol,
                &body.host,
                port,
                body.username.as_deref(),
                body.config_json.as_deref(),
                body.environment_id.as_deref(),
                || direct_redis_probe(&redis_host, port),
            )
            .await
        }
        "sql" => {
            // v0.73.2：统一 SQL 协议迁移后，protocol='sql' + subtype 携带方言。
            // 从 config_json 读取 subtype 路由到对应的直连检测分支。
            let subtype = body
                .config_json
                .as_ref()
                .and_then(|c| serde_json::from_str::<serde_json::Value>(c).ok())
                .and_then(|v| v.get("subtype").and_then(|s| s.as_str()).map(String::from))
                .or_else(|| {
                    body.config_json.as_ref().and_then(|c| {
                        let v: serde_json::Value = serde_json::from_str(c).ok()?;
                        v.get("database_type")
                            .and_then(|s| s.as_str())
                            .map(String::from)
                    })
                })
                .unwrap_or_else(|| "mysql".to_string());
            match subtype.as_str() {
                "sqlite" => {
                    // sqlite 以文件路径作为 host 传给 Agent 侧 SqliteConnector
                    // （rex_sqlite::connect 使用 ConnectRequest.host 作为 db_path）；
                    // agent-first-then-fallback，mirroring the `_` branch below.
                    let path = sqlite_probe_path(body.config_json.as_deref());
                    let cfg_json = {
                        let mut v: serde_json::Value = body
                            .config_json
                            .as_deref()
                            .and_then(|c| serde_json::from_str::<serde_json::Value>(c).ok())
                            .unwrap_or_else(|| serde_json::json!({}));
                        if let serde_json::Value::Object(m) = &mut v {
                            m.insert("subtype".to_string(), serde_json::json!("sqlite"));
                        }
                        v.to_string()
                    };
                    probe_target(
                        &state,
                        &body.protocol,
                        &path,
                        0,
                        body.username.as_deref(),
                        Some(&cfg_json),
                        body.environment_id.as_deref(),
                        || direct_sqlite_probe(&path),
                    )
                    .await
                }
                _ => {
                    let host = body.host.clone();
                    let port = body
                        .port
                        .unwrap_or(if subtype == "mysql" { 3306 } else { 5432 });
                    // Agent 以 config.subtype 选方言下发（agent_ws handle_connect_sql），
                    // 探测出的 subtype 需显式带入，否则回退按 protocol='sql' 解析失败。
                    let cfg_json = {
                        let mut v: serde_json::Value = body
                            .config_json
                            .as_deref()
                            .and_then(|c| serde_json::from_str::<serde_json::Value>(c).ok())
                            .unwrap_or_else(|| serde_json::json!({}));
                        if let serde_json::Value::Object(m) = &mut v {
                            m.insert("subtype".to_string(), serde_json::json!(subtype.clone()));
                        }
                        v.to_string()
                    };
                    probe_target(
                        &state,
                        &body.protocol,
                        &host,
                        port,
                        body.username.as_deref(),
                        Some(&cfg_json),
                        body.environment_id.as_deref(),
                        || direct_tcp_probe(&host, port),
                    )
                    .await
                }
            }
        }
        "sqlite" => {
            // sqlite 以文件路径作为 host 传给 Agent 侧 SqliteConnector；
            // agent-first-then-fallback，mirroring the sql subtype="sqlite" arm.
            // 差异：bare arm 不注入 subtype（Agent 侧按 protocol="sqlite" 解析），
            // 直连回退与另一处共用 `direct_sqlite_probe`。
            let path = sqlite_probe_path(body.config_json.as_deref());
            probe_target(
                &state,
                "sqlite",
                &path,
                0,
                body.username.as_deref(),
                body.config_json.as_deref(),
                body.environment_id.as_deref(),
                || direct_sqlite_probe(&path),
            )
            .await
        }
        "mysql" | "postgresql" => {
            let port = body
                .port
                .unwrap_or(if body.protocol == "mysql" { 3306 } else { 5432 });
            let host = body.host.clone();
            probe_target(
                &state,
                &body.protocol,
                &body.host,
                port,
                body.username.as_deref(),
                body.config_json.as_deref(),
                body.environment_id.as_deref(),
                || direct_tcp_probe(&host, port),
            )
            .await
        }
        "s3" => {
            // agent-first-then-fallback：agent-mode 时路由探测到 Agent 侧
            // `S3Connector::verify`（真正验凭据），直连模式回退 Hub-local list_buckets。
            let s3_port = body.port.unwrap_or(0);
            probe_target(
                &state,
                "s3",
                &body.host,
                s3_port,
                body.username.as_deref(),
                body.config_json.as_deref(),
                body.environment_id.as_deref(),
                || direct_s3_probe(body.config_json.as_deref()),
            )
            .await
        }
        "sip" => match &body.config_json {
            None => Err("missing config_json for SIP".into()),
            Some(cfg) => match serde_json::from_str::<serde_json::Value>(cfg) {
                Err(e) => Err(format!("invalid config_json: {e}")),
                Ok(value) => {
                    // SIP 的 server/port 完全下沉到账户层，load_sip_conn 不读取
                    // 顶层 host/port/username（子任务 #1 已移除回退），故 info 仅带
                    // config。先用与信令注册一致的 load_sip_conn 校验 SipProfile
                    // 能选出生效账户；匿名注册（无 password）是合法的。
                    let info = crate::resource_conn::ResourceConnInfo {
                        resource_id: String::new(),
                        name: String::new(),
                        protocol: "sip".into(),
                        host: String::new(),
                        port: None,
                        username: String::new(),
                        config: value,
                        subtype: None,
                        use_agent: false,
                        agent_id: None,
                    };
                    match crate::resource_conn::load_sip_conn(&info) {
                        Err(e) => Err(format!("invalid SIP config: {e}")),
                        Ok(sip_cfg) => {
                            // agent-first-then-fallback：agent-mode 时把生效账户的
                            // 平坦 SipConfig 下发到 Agent，由 UA₂ 做真实 REGISTER
                            //（验凭据可达内网 SIP server）；直连模式由 Hub 本地起一个
                            // 一次性 UA₂ 做同样的真实 REGISTER —— 不再是「配置能解析
                            // 就算成功」（恒 `Ok(())` 的假阳性）。
                            let flat = serde_json::json!({
                                "server": sip_cfg.server,
                                "port": sip_cfg.port,
                                "username": sip_cfg.username,
                                "password": sip_cfg.password,
                                "displayName": sip_cfg.display_name,
                                "transport": sip_cfg.transport.as_str(),
                            });
                            let flat_str = flat.to_string();
                            let sip_probe_cfg = sip_cfg.clone();
                            probe_target(
                                &state,
                                "sip",
                                "",
                                0,
                                Some(&sip_cfg.username),
                                Some(&flat_str),
                                body.environment_id.as_deref(),
                                move || direct_sip_probe(sip_probe_cfg),
                            )
                            .await
                        }
                    }
                }
            },
        },
        _ => Err(format!("unsupported protocol: {}", body.protocol)),
    };
    let latency = start.elapsed().as_millis() as u64;
    let ok = result.is_ok();
    let err_msg = match &result {
        Ok(()) => None,
        Err(e) => Some(e.clone()),
    };

    tracing::info!(
        action = "TEST_CONNECTION",
        protocol = %body.protocol,
        host = %body.host,
        ok = ok,
        latency_ms = latency,
        error = err_msg.as_deref().unwrap_or(""),
        "connection test completed"
    );

    match result {
        Ok(()) => Ok(Json(TestConnectionResult {
            ok: true,
            latency_ms: Some(latency),
            error: None,
        })),
        Err(e) => Ok(Json(TestConnectionResult {
            ok: false,
            latency_ms: Some(latency),
            error: Some(e),
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试连接的 connect config 形状（agent 下发）：host/port + 合并
    /// `config_json` + 顶层 username 归一。走 `merge_resource_config`（与
    /// `file_api::agent_file_config` 同源）。
    fn test_connect_config(
        host: &str,
        port: u16,
        username: Option<&str>,
        config_json: Option<&str>,
    ) -> serde_json::Value {
        let config = match config_json {
            Some(cfg_str) => serde_json::from_str::<serde_json::Value>(cfg_str)
                .unwrap_or(serde_json::Value::Null),
            None => serde_json::Value::Null,
        };
        crate::resource_conn::merge_resource_config(host, port, username.unwrap_or(""), &config)
    }

    /// CR15 回归：agent 模式测试连接的 connect config 必须透出顶层 username，
    /// 空值归一为 `root`（与 `file_api::agent_file_config` 同源），
    /// 否则 Agent 以空用户名认证，测试连接必被拒。
    #[test]
    fn agent_test_config_normalizes_empty_username() {
        let cfg = test_connect_config("10.0.0.1", 22, Some(""), Some(r#"{"password":"pw"}"#));
        assert_eq!(
            cfg.get("username").and_then(|v| v.as_str()),
            Some(crate::resource_conn::normalize_username("").as_str()),
            "test connection and file entries must share one normalization source"
        );
        assert_eq!(cfg.get("host").and_then(|v| v.as_str()), Some("10.0.0.1"));
        assert_eq!(cfg.get("port").and_then(|v| v.as_u64()), Some(22));
        assert_eq!(cfg.get("password").and_then(|v| v.as_str()), Some("pw"));
    }

    /// 顶层 username 为权威：显式值保留，不被 config_json 中的历史键覆盖；
    /// 缺失字段（`None`）同样按空值归一。
    #[test]
    fn agent_test_config_keeps_explicit_username() {
        let cfg = test_connect_config(
            "10.0.0.1",
            22,
            Some("alice"),
            Some(r#"{"username":"stale"}"#),
        );
        assert_eq!(cfg.get("username").and_then(|v| v.as_str()), Some("alice"));

        let cfg = test_connect_config("10.0.0.1", 22, None, None);
        assert_eq!(cfg.get("username").and_then(|v| v.as_str()), Some("root"));
    }

    /// [`env_connection_mode`] 必须原样返回环境里存的真实 mode —— 它的唯一调用方
    /// [`test_connect_via_agent`] 把该值交给 `resolve_agent_mode` 作判定入参，
    /// 若这里改判定（只回 `Some("agent")`）或吞掉 mode，那道收敛点就失效。
    /// 无 environment_id / 环境查不到都回 `None`（= 不走 Agent）。
    #[tokio::test]
    async fn env_connection_mode_returns_real_mode() {
        let (_dir, state) = crate::testutil::make_state();
        let direct = state
            .db
            .create_environment(&crate::models::NewEnvironment {
                name: "direct-env".into(),
                description: None,
                connection_mode: Some("direct".into()),
            })
            .unwrap();
        let agent = state
            .db
            .create_environment(&crate::models::NewEnvironment {
                name: "agent-env".into(),
                description: None,
                connection_mode: Some("agent".into()),
            })
            .unwrap();

        assert_eq!(
            env_connection_mode(&state, None).await,
            None,
            "no environment means direct connect"
        );
        assert_eq!(env_connection_mode(&state, Some("env_missing")).await, None);
        assert_eq!(
            env_connection_mode(&state, Some(&direct.id))
                .await
                .as_deref(),
            Some("direct"),
            "the stored mode is passed through verbatim, not re-classified"
        );
        assert_eq!(
            env_connection_mode(&state, Some(&agent.id))
                .await
                .as_deref(),
            Some("agent")
        );
    }

    /// Agent 侧 sqlx 会把连接串回显进错误文案；该文案既进用户可见 toast 又进
    /// 日志，凭据必须被 redact（password / 私钥两种键名都算）。
    #[test]
    fn agent_error_is_redacted_for_both_secret_fields() {
        let cfg = test_connect_config(
            "10.0.0.1",
            3306,
            Some("ops"),
            Some(r#"{"password":"s3cr3t-pw","private_key":"PEM-SECRET"}"#),
        );

        let pw = redact_agent_error(
            "SQL connection failed: mysql://ops:s3cr3t-pw@10.0.0.1",
            &cfg,
        );
        assert!(
            !pw.contains("s3cr3t-pw"),
            "password must not reach response or log: {pw}"
        );
        assert!(pw.contains("mysql://ops:***@10.0.0.1"));

        let key = redact_agent_error("failed to decode private key PEM-SECRET", &cfg);
        assert!(!key.contains("PEM-SECRET"));

        // 无凭据时原文原样返回（空密码不应把整条文案抹成 ***）
        let bare = test_connect_config("10.0.0.1", 22, Some("ops"), None);
        assert_eq!(
            redact_agent_error("Connection refused", &bare),
            "Connection refused"
        );
    }

    /// S3 探测的错误链（`agent_file::probe_s3` 的 `S3 verify failed: {e}`，
    /// 内层 `rex_s3` 的 `S3 HeadBucket failed bucket=...`）可能回显 access/secret
    /// key，而 `redact_agent_error` 在 s3 arm 被调用 → 白名单必须收这两个键，
    /// 否则 key 明文直接进 `TestConnectionResult.error` 与日志。
    #[test]
    fn agent_error_redacts_s3_access_and_secret_keys() {
        let cfg = test_connect_config(
            "s3.example.com",
            443,
            None,
            Some(
                r#"{"endpoint":"https://s3.example.com","access_key":"AKIA-SECRET","secret_key":"sk-S3-SECRET","bucket":"b1"}"#,
            ),
        );

        let err = redact_agent_error(
            "S3 verify failed: S3 HeadBucket failed bucket=b1 \
             (access AKIA-SECRET secret sk-S3-SECRET)",
            &cfg,
        );
        assert!(
            !err.contains("AKIA-SECRET"),
            "access_key must not reach response or log: {err}"
        );
        assert!(
            !err.contains("sk-S3-SECRET"),
            "secret_key must not reach response or log: {err}"
        );
        assert!(
            err.contains("access *** secret ***"),
            "both keys are replaced in place: {err}"
        );
    }

    /// S5-10 回归：agent 模式但该环境无在线 Agent 时，探测必须报
    /// 「无在线 Agent」而**不是** `None`（后者会让调用方静默回退 Hub 直连，
    /// 在内网目标上得到误导性的 `TCP connect failed`）。
    #[tokio::test]
    async fn agent_mode_without_online_agent_reports_error_not_direct_fallback() {
        let (_dir, state) = crate::testutil::make_state();
        let env = state
            .db
            .create_environment(&crate::models::NewEnvironment {
                name: "agent-env-no-agent".into(),
                description: None,
                connection_mode: Some("agent".into()),
            })
            .unwrap();

        let r = test_connect_via_agent(
            &state,
            "ssh",
            "10.0.0.1",
            22,
            Some("ops"),
            None,
            Some(&env.id),
        )
        .await;

        let Some(result) = r else {
            panic!(
                "agent-mode environment without online agent must NOT return None: \
                 None makes every caller fall back to a Hub-local direct probe"
            );
        };
        let msg = result.expect_err("no online agent must be a probe failure");
        assert!(
            msg.contains("no online agent"),
            "error must name the actual cause, got: {msg}"
        );
    }

    /// `probe_target` 的骨架契约：agent 探测返回 `Some(Err(..))` 时该错误**原样**
    /// 上抛，绝不跑直连回退（否则 S5-10 的修法被调用方旁路掉）。
    #[tokio::test]
    async fn probe_target_never_falls_back_when_agent_probe_failed() {
        let (_dir, state) = crate::testutil::make_state();
        let env = state
            .db
            .create_environment(&crate::models::NewEnvironment {
                name: "agent-env-probe-target".into(),
                description: None,
                connection_mode: Some("agent".into()),
            })
            .unwrap();

        let err = probe_target(
            &state,
            "ssh",
            "10.0.0.1",
            22,
            Some("ops"),
            None,
            Some(&env.id),
            || direct_tcp_probe("127.0.0.1", 1),
        )
        .await
        .expect_err("agent failure must propagate, not fall back to direct TCP");

        assert!(
            err.contains("no online agent"),
            "expected the agent-mode error, got: {err}"
        );
    }

    /// `probe_target` 的另一半：非 agent 环境（`None`）才跑传入的直连回退。
    /// 用「直连回退被调用」这一可观测行为断言分派，而不是断言恒真的 `Ok(())`。
    #[tokio::test]
    async fn probe_target_runs_fallback_for_direct_environment() {
        let (_dir, state) = crate::testutil::make_state();
        let direct = state
            .db
            .create_environment(&crate::models::NewEnvironment {
                name: "direct-env-probe-target".into(),
                description: None,
                connection_mode: Some("direct".into()),
            })
            .unwrap();

        let called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = called.clone();
        let err = probe_target(
            &state,
            "ssh",
            "10.0.0.1",
            22,
            Some("ops"),
            None,
            Some(&direct.id),
            move || {
                flag.store(true, std::sync::atomic::Ordering::SeqCst);
                std::future::ready(Err("fallback-ran".to_string()))
            },
        )
        .await
        .expect_err("the fallback's own verdict is propagated verbatim");

        assert_eq!(err, "fallback-ran", "error comes from the fallback closure");
        assert!(
            called.load(std::sync::atomic::Ordering::SeqCst),
            "direct environment must run the Hub-local fallback probe"
        );
    }

    /// 直连 sqlite 回退的真实验证：可打开的临时库 → `Ok`；父目录不存在 →
    /// `Err`（而不是恒 `Ok` 的假成功）。
    #[tokio::test]
    async fn direct_sqlite_probe_distinguishes_openable_from_unreachable_path() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("probe.db");
        let db_str = db_path.to_str().unwrap();

        direct_sqlite_probe(db_str)
            .await
            .expect("an openable sqlite file must pass the probe");

        let missing = dir.path().join("no_such_dir").join("probe.db");
        let err = direct_sqlite_probe(missing.to_str().unwrap())
            .await
            .expect_err("an unopenable path must fail the probe");
        assert!(
            err.starts_with("SQLite open failed"),
            "unexpected error shape: {err}"
        );
    }

    /// S3 直连回退在缺 config_json / 缺凭据时必须报错，不得「解析即成功」。
    #[tokio::test]
    async fn direct_s3_probe_requires_config_and_credentials() {
        let missing = direct_s3_probe(None)
            .await
            .expect_err("missing config_json must fail");
        assert_eq!(missing, "missing config_json for S3");

        let partial = direct_s3_probe(Some(r#"{"endpoint":"https://s3.example.com"}"#))
            .await
            .expect_err("missing keys must fail before any network call");
        assert_eq!(partial, "missing endpoint, access_key, or secret_key");
    }
}
