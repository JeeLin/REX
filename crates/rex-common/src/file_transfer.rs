//! 文件传输抽象 — 统一 SFTP / S3 / 本地文件连接器。

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// 文件/目录条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<String>,
    pub permissions: Option<String>,
    /// S3: Storage Class (STANDARD, STANDARD_IA, GLACIER, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_class: Option<String>,
    /// S3: Canned ACL (private, public-read, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acl: Option<String>,
}

/// 连接请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileConnectRequest {
    pub protocol: String, // "sftp" | "s3"
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub private_key: Option<String>,
    /// SSH KeepAlive 间隔（秒），0 表示禁用
    pub keepalive_interval: Option<u32>,
    /// S3 专用
    pub bucket: Option<String>,
    pub region: Option<String>,
    pub endpoint: Option<String>,
    pub access_key: Option<String>,
    pub secret_key: Option<String>,
}

/// 进度回调
pub type ProgressCallback = Box<dyn Fn(u64, u64) + Send + Sync>;

/// 上传结果
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UploadResult {
    /// S3 multipart upload_id (用于续传)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upload_id: Option<String>,
}

/// 文件连接器 trait
#[async_trait]
pub trait FileConnector: Send + Sync {
    /// 列出目录内容
    async fn list(&mut self, path: &str) -> Result<Vec<FileEntry>>;

    /// 获取文件/目录信息
    async fn stat(&mut self, path: &str) -> Result<FileEntry>;

    /// 上传文件
    async fn upload(
        &mut self,
        remote_path: &str,
        data: Vec<u8>,
        offset: u64,
        progress: Option<&ProgressCallback>,
    ) -> Result<UploadResult>;

    /// 下载文件
    async fn download(&mut self, path: &str) -> Result<Vec<u8>>;

    /// 下载文件（支持 Range：从 offset 开始，最多 limit 字节；limit=None 表示到文件末尾）
    async fn download_range(
        &mut self,
        path: &str,
        offset: u64,
        limit: Option<u64>,
    ) -> Result<Vec<u8>>;

    /// 删除文件/目录
    async fn delete(&mut self, path: &str) -> Result<()>;

    /// 重命名/移动
    async fn rename(&mut self, from: &str, to: &str) -> Result<()>;

    /// 创建目录
    async fn mkdir(&mut self, path: &str) -> Result<()>;

    /// 读取文件内容用于编辑（限小文件，最大 5MB）
    async fn read_for_edit(&mut self, path: &str) -> Result<Vec<u8>>;

    /// 从编辑器保存文件内容（覆盖写入）
    async fn save_from_edit(&mut self, path: &str, data: Vec<u8>) -> Result<()>;

    /// 关闭连接
    async fn close(&mut self) -> Result<()>;

    /// Downcast support for protocol-specific methods
    fn as_any(&self) -> &dyn std::any::Any;

    /// Mutable downcast support for protocol-specific methods
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

// ---------------------------------------------------------------------------
// Base64 helpers
// ---------------------------------------------------------------------------

/// 将字节数组编码为 Base64 字符串。
pub fn base64_chunk(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}

/// 将 Base64 字符串解码为字节数组。
pub fn base64_decode(s: &str) -> anyhow::Result<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(s)
        .map_err(|e| anyhow::anyhow!("base64 decode failed: {e}"))
}

// ---------------------------------------------------------------------------
// 统一文件操作分发
// ---------------------------------------------------------------------------

/// 统一文件操作分发：按 kind 调用 connector 对应方法，返回 JSON。
/// Agent（WebSocket 隧道）和 Hub（HTTP handler）共用此函数。
pub async fn dispatch_file(
    conn: &mut dyn FileConnector,
    kind: &str,
    payload: &serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    match kind {
        "list" => {
            let path = payload.get("path").and_then(|v| v.as_str()).unwrap_or("/");
            let entries = conn.list(path).await?;
            Ok(serde_json::json!({ "entries": entries }))
        }
        "stat" => {
            let path = payload.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let e = conn.stat(path).await?;
            Ok(serde_json::json!({ "entry": e }))
        }
        "mkdir" => {
            let path = payload.get("path").and_then(|v| v.as_str()).unwrap_or("");
            conn.mkdir(path).await?;
            Ok(serde_json::json!({ "ok": true }))
        }
        "delete" => {
            let path = payload.get("path").and_then(|v| v.as_str()).unwrap_or("");
            conn.delete(path).await?;
            Ok(serde_json::json!({ "ok": true }))
        }
        "rename" => {
            let from = payload.get("from").and_then(|v| v.as_str()).unwrap_or("");
            let to = payload.get("to").and_then(|v| v.as_str()).unwrap_or("");
            conn.rename(from, to).await?;
            Ok(serde_json::json!({ "ok": true }))
        }
        "download" => {
            let path = payload.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let offset = payload.get("offset").and_then(|v| v.as_u64()).unwrap_or(0);
            let limit = payload.get("limit").and_then(|v| v.as_u64());
            let data = if limit.is_some() || offset > 0 {
                conn.download_range(path, offset, limit).await?
            } else {
                conn.download(path).await?
            };
            // 文件分块走 session_response 的 data.b64；大文件由前端切片下发。
            Ok(serde_json::json!({ "data": base64_chunk(&data), "len": data.len() }))
        }
        "download_meta" => {
            let path = payload.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let e = conn.stat(path).await?;
            Ok(serde_json::json!({ "size": e.size }))
        }
        "read_for_edit" => {
            let path = payload.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let data = conn.read_for_edit(path).await?;
            Ok(serde_json::json!({ "data": base64_chunk(&data), "len": data.len() }))
        }
        "upload" => {
            let path = payload.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let offset = payload.get("offset").and_then(|v| v.as_u64()).unwrap_or(0);
            let b64 = payload.get("data").and_then(|v| v.as_str()).unwrap_or("");
            let data = base64_decode(b64)?;
            conn.upload(path, data, offset, None).await?;
            Ok(serde_json::json!({ "ok": true }))
        }
        "save_from_edit" => {
            let path = payload.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let b64 = payload.get("data").and_then(|v| v.as_str()).unwrap_or("");
            let data = base64_decode(b64)?;
            conn.save_from_edit(path, data).await?;
            Ok(serde_json::json!({ "ok": true }))
        }
        "close" => {
            let _ = conn.close().await;
            Ok(serde_json::json!({ "closed": true }))
        }
        other => anyhow::bail!("unsupported file request kind: {other}"),
    }
}
