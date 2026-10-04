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

// ---------------------------------------------------------------------------
// 能力模型（v0.91.0 T3）
// ---------------------------------------------------------------------------

/// 连接器可选能力集：按协议如实上报当前支持面，取代后端
/// `downcast::<S3Connector>` 判断与前端 `isS3` 分支。
///
/// 平坦 bool 字段直接展开为 JSON（`{"chmod": false, ...}`）。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileCapabilitySet {
    /// 修改权限（chmod）。SFTP 实现属 T4，当前所有连接器均为 false。
    pub chmod: bool,
    /// 生成预签名 URL（S3 专属）。
    pub presigned_url: bool,
    /// 读写对象 Canned ACL（S3 专属）。
    pub acl: bool,
    /// multipart 断点续传：list / resume / abort（S3 专属）。
    pub multipart: bool,
}

/// S3 专属操作在非 S3 连接器上的默认错误。
///
/// 文案与历史 `downcast` 失败分支保持一致；调用方（Hub handler）按具体类型
/// 把它映射回 `UNSUPPORTED_PROTOCOL` 错误码，其余错误保持各自的业务码。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedProtocolError {
    pub message: String,
}

impl UnsupportedProtocolError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for UnsupportedProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for UnsupportedProtocolError {}

/// 构造 [`UnsupportedProtocolError`]（历史文案见 Hub `file_api` 各 handler）。
pub fn unsupported_protocol(message: &str) -> anyhow::Error {
    UnsupportedProtocolError::new(message).into()
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

    /// 连接器能力集（非 async，默认全 false）。
    ///
    /// 各协议按真实支持面覆写；Hub 的能力查询端点、`AgentFileProxy` 与前端
    /// 能力开关都读这里，全程不建立连接。
    fn capability(&self) -> FileCapabilitySet {
        FileCapabilitySet::default()
    }

    /// 生成预签名 URL（S3 专属）。
    async fn presigned_url(&self, key: &str, expires_in_secs: u64) -> Result<String> {
        let _ = (key, expires_in_secs);
        Err(unsupported_protocol("presigned URL only supported for S3"))
    }

    /// 列出进行中的 multipart uploads（S3 专属）。
    async fn list_multipart_uploads(&self, prefix: &str) -> Result<Vec<(String, String)>> {
        let _ = prefix;
        Err(unsupported_protocol("only supported for S3"))
    }

    /// 续传进行中的 multipart upload（S3 专属）。
    async fn resume_multipart_upload(
        &self,
        key: &str,
        upload_id: &str,
        data: Vec<u8>,
        progress: Option<&ProgressCallback>,
    ) -> Result<()> {
        let _ = (key, upload_id, data, progress);
        Err(unsupported_protocol("only supported for S3"))
    }

    /// 取消进行中的 multipart upload（S3 专属）。
    async fn abort_multipart_upload(&self, key: &str, upload_id: &str) -> Result<()> {
        let _ = (key, upload_id);
        Err(unsupported_protocol("only supported for S3"))
    }

    /// 读取对象 Canned ACL（S3 专属）。
    async fn get_acl(&self, key: &str) -> Result<String> {
        let _ = key;
        Err(unsupported_protocol("only supported for S3"))
    }

    /// 写入对象 Canned ACL（S3 专属）。
    async fn put_acl(&self, key: &str, canned_acl: &str) -> Result<()> {
        let _ = (key, canned_acl);
        Err(unsupported_protocol("only supported for S3"))
    }
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
// S3 专属操作的隧道载荷（v0.91.0 T3）
// ---------------------------------------------------------------------------
//
// Hub `AgentFileProxy` 构造这些 Request 经隧道下发，Agent 侧 `dispatch_file`
// 反序列化后调用本地 connector 的同名 trait 方法，再以对应 Response 回传。
// hub handler 的 HTTP 响应形状与此保持一致（`{"url": ...}` / `{"acl": ...}` …）。

fn default_presigned_expires() -> u64 {
    3600
}

/// `presigned_url` 请求。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresignedUrlRequest {
    pub path: String,
    #[serde(default = "default_presigned_expires")]
    pub expires_in: u64,
}

/// `presigned_url` 响应。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresignedUrlResponse {
    pub url: String,
}

/// `list_multipart_uploads` 请求。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListMultipartUploadsRequest {
    pub prefix: String,
}

/// 进行中的 multipart upload 引用。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultipartUploadInfo {
    pub key: String,
    pub upload_id: String,
}

/// `list_multipart_uploads` 响应。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListMultipartUploadsResponse {
    pub uploads: Vec<MultipartUploadInfo>,
}

/// `resume_multipart_upload` 请求（`data` 为 Base64）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeMultipartUploadRequest {
    pub path: String,
    pub upload_id: String,
    pub data: String,
}

/// `abort_multipart_upload` 请求。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbortMultipartUploadRequest {
    pub path: String,
    pub upload_id: String,
}

/// `get_acl` 请求。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetAclRequest {
    pub path: String,
}

/// `get_acl` 响应。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AclResponse {
    pub acl: String,
}

/// `put_acl` 请求。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PutAclRequest {
    pub path: String,
    pub acl: String,
}

/// 写操作统一响应（resume / abort / put_acl）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OkResponse {
    pub ok: bool,
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
        "presigned_url" => {
            let req: PresignedUrlRequest = serde_json::from_value(payload.clone())?;
            let url = conn.presigned_url(&req.path, req.expires_in).await?;
            Ok(serde_json::to_value(PresignedUrlResponse { url })?)
        }
        "list_multipart_uploads" => {
            let req: ListMultipartUploadsRequest = serde_json::from_value(payload.clone())?;
            let uploads = conn.list_multipart_uploads(&req.prefix).await?;
            Ok(serde_json::to_value(ListMultipartUploadsResponse {
                uploads: uploads
                    .into_iter()
                    .map(|(key, upload_id)| MultipartUploadInfo { key, upload_id })
                    .collect(),
            })?)
        }
        "resume_multipart_upload" => {
            let req: ResumeMultipartUploadRequest = serde_json::from_value(payload.clone())?;
            let data = base64_decode(&req.data)?;
            conn.resume_multipart_upload(&req.path, &req.upload_id, data, None)
                .await?;
            Ok(serde_json::to_value(OkResponse { ok: true })?)
        }
        "abort_multipart_upload" => {
            let req: AbortMultipartUploadRequest = serde_json::from_value(payload.clone())?;
            conn.abort_multipart_upload(&req.path, &req.upload_id)
                .await?;
            Ok(serde_json::to_value(OkResponse { ok: true })?)
        }
        "get_acl" => {
            let req: GetAclRequest = serde_json::from_value(payload.clone())?;
            let acl = conn.get_acl(&req.path).await?;
            Ok(serde_json::to_value(AclResponse { acl })?)
        }
        "put_acl" => {
            let req: PutAclRequest = serde_json::from_value(payload.clone())?;
            conn.put_acl(&req.path, &req.acl).await?;
            Ok(serde_json::to_value(OkResponse { ok: true })?)
        }
        other => anyhow::bail!("unsupported file request kind: {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    fn round_trip<T>(value: &T) -> T
    where
        T: serde::Serialize + serde::de::DeserializeOwned,
    {
        serde_json::from_value(serde_json::to_value(value).expect("serialize"))
            .expect("deserialize")
    }

    // -------------------------------------------------------------------
    // FileCapabilitySet serde
    // -------------------------------------------------------------------

    #[test]
    fn capability_set_serializes_flat_default_all_false() {
        let caps = FileCapabilitySet::default();
        let v = serde_json::to_value(caps).unwrap();
        assert_eq!(
            v,
            serde_json::json!({
                "chmod": false,
                "presigned_url": false,
                "acl": false,
                "multipart": false
            }),
            "serde must be a flat JSON object (no nesting / rename)"
        );
        assert_eq!(round_trip(&caps), caps);
    }

    #[test]
    fn capability_set_round_trips_with_flags_set() {
        let caps = FileCapabilitySet {
            chmod: false,
            presigned_url: true,
            acl: true,
            multipart: true,
        };
        let v = serde_json::to_value(caps).unwrap();
        assert_eq!(v["presigned_url"], serde_json::json!(true));
        assert_eq!(v["multipart"], serde_json::json!(true));
        assert_eq!(v["chmod"], serde_json::json!(false));
        assert_eq!(round_trip(&caps), caps);
    }

    // -------------------------------------------------------------------
    // 6 个 S3-only dispatch kind 的 Request / Response serde 往返
    // -------------------------------------------------------------------

    #[test]
    fn presigned_url_payload_round_trip() {
        let req = PresignedUrlRequest {
            path: "dir/a.txt".into(),
            expires_in: 60,
        };
        let back = round_trip(&req);
        assert_eq!(back.path, "dir/a.txt");
        assert_eq!(back.expires_in, 60);

        // expires_in 缺省 3600（与 hub handler 的 default_expires 一致）
        let dflt: PresignedUrlRequest =
            serde_json::from_value(serde_json::json!({ "path": "x" })).unwrap();
        assert_eq!(dflt.expires_in, 3600);

        let resp = PresignedUrlResponse {
            url: "https://bucket.s3.amazonaws.com/x?X-Amz-Signature=abc".into(),
        };
        assert_eq!(round_trip(&resp).url, resp.url);
    }

    #[test]
    fn list_multipart_uploads_payload_round_trip() {
        let req = ListMultipartUploadsRequest {
            prefix: "incoming/".into(),
        };
        assert_eq!(round_trip(&req).prefix, "incoming/");

        let resp = ListMultipartUploadsResponse {
            uploads: vec![
                MultipartUploadInfo {
                    key: "k1".into(),
                    upload_id: "u1".into(),
                },
                MultipartUploadInfo {
                    key: "k2".into(),
                    upload_id: "u2".into(),
                },
            ],
        };
        let back = round_trip(&resp);
        assert_eq!(back.uploads.len(), 2);
        assert_eq!(back.uploads[0].key, "k1");
        assert_eq!(back.uploads[1].upload_id, "u2");
    }

    #[test]
    fn resume_and_abort_multipart_payload_round_trip() {
        let resume = ResumeMultipartUploadRequest {
            path: "big.bin".into(),
            upload_id: "upload-1".into(),
            data: base64_chunk(b"hello"),
        };
        let back = round_trip(&resume);
        assert_eq!(back.path, "big.bin");
        assert_eq!(back.upload_id, "upload-1");
        assert_eq!(base64_decode(&back.data).unwrap(), b"hello");

        let abort = AbortMultipartUploadRequest {
            path: "big.bin".into(),
            upload_id: "upload-1".into(),
        };
        let back = round_trip(&abort);
        assert_eq!(back.path, "big.bin");
        assert_eq!(back.upload_id, "upload-1");
    }

    #[test]
    fn acl_payload_round_trip() {
        let get = GetAclRequest {
            path: "public/index.html".into(),
        };
        assert_eq!(round_trip(&get).path, "public/index.html");

        let resp = AclResponse {
            acl: "public-read".into(),
        };
        assert_eq!(round_trip(&resp).acl, "public-read");

        let put = PutAclRequest {
            path: "public/index.html".into(),
            acl: "private".into(),
        };
        let back = round_trip(&put);
        assert_eq!(back.acl, "private");
        assert_eq!(back.path, "public/index.html");

        let ok = OkResponse { ok: true };
        assert!(round_trip(&ok).ok);
    }

    // -------------------------------------------------------------------
    // dispatch_file：6 个新 kind + 未知 kind 分支
    // -------------------------------------------------------------------

    /// 记录 S3-only 调用并返回合成结果；基础方法从不被这些 kind 触达。
    #[derive(Default)]
    struct RecordingConnector {
        calls: Mutex<Vec<String>>,
    }

    impl RecordingConnector {
        fn record(&self, call: String) {
            self.calls.lock().expect("lock").push(call);
        }

        fn calls(&self) -> Vec<String> {
            self.calls.lock().expect("lock").clone()
        }
    }

    #[async_trait]
    impl FileConnector for RecordingConnector {
        async fn list(&mut self, _path: &str) -> Result<Vec<FileEntry>> {
            unimplemented!()
        }
        async fn stat(&mut self, _path: &str) -> Result<FileEntry> {
            unimplemented!()
        }
        async fn upload(
            &mut self,
            _remote_path: &str,
            _data: Vec<u8>,
            _offset: u64,
            _progress: Option<&ProgressCallback>,
        ) -> Result<UploadResult> {
            unimplemented!()
        }
        async fn download(&mut self, _path: &str) -> Result<Vec<u8>> {
            unimplemented!()
        }
        async fn download_range(
            &mut self,
            _path: &str,
            _offset: u64,
            _limit: Option<u64>,
        ) -> Result<Vec<u8>> {
            unimplemented!()
        }
        async fn delete(&mut self, _path: &str) -> Result<()> {
            unimplemented!()
        }
        async fn rename(&mut self, _from: &str, _to: &str) -> Result<()> {
            unimplemented!()
        }
        async fn mkdir(&mut self, _path: &str) -> Result<()> {
            unimplemented!()
        }
        async fn read_for_edit(&mut self, _path: &str) -> Result<Vec<u8>> {
            unimplemented!()
        }
        async fn save_from_edit(&mut self, _path: &str, _data: Vec<u8>) -> Result<()> {
            unimplemented!()
        }
        async fn close(&mut self) -> Result<()> {
            unimplemented!()
        }

        async fn presigned_url(&self, key: &str, expires_in_secs: u64) -> Result<String> {
            self.record(format!("presigned_url:{key}:{expires_in_secs}"));
            Ok(format!("https://signed/{key}"))
        }

        async fn list_multipart_uploads(&self, prefix: &str) -> Result<Vec<(String, String)>> {
            self.record(format!("list_multipart_uploads:{prefix}"));
            Ok(vec![("k1".to_string(), "u1".to_string())])
        }

        async fn resume_multipart_upload(
            &self,
            key: &str,
            upload_id: &str,
            data: Vec<u8>,
            _progress: Option<&ProgressCallback>,
        ) -> Result<()> {
            self.record(format!(
                "resume_multipart_upload:{key}:{upload_id}:{}",
                data.len()
            ));
            Ok(())
        }

        async fn abort_multipart_upload(&self, key: &str, upload_id: &str) -> Result<()> {
            self.record(format!("abort_multipart_upload:{key}:{upload_id}"));
            Ok(())
        }

        async fn get_acl(&self, key: &str) -> Result<String> {
            self.record(format!("get_acl:{key}"));
            Ok("private".to_string())
        }

        async fn put_acl(&self, key: &str, canned_acl: &str) -> Result<()> {
            self.record(format!("put_acl:{key}:{canned_acl}"));
            Ok(())
        }
    }

    #[tokio::test]
    async fn dispatch_file_unknown_kind_is_rejected_as_unsupported() {
        let mut conn = RecordingConnector::default();
        let err = dispatch_file(&mut conn, "chmod", &serde_json::json!({ "path": "/a" }))
            .await
            .expect_err("chmod kind is T4 scope, not implemented yet");
        assert_eq!(err.to_string(), "unsupported file request kind: chmod");
    }

    #[tokio::test]
    async fn dispatch_file_routes_the_six_s3_kinds_to_trait_methods() {
        let mut conn = RecordingConnector::default();

        let v = dispatch_file(
            &mut conn,
            "presigned_url",
            &serde_json::json!({ "path": "a.txt", "expires_in": 60 }),
        )
        .await
        .unwrap();
        assert_eq!(v["url"], "https://signed/a.txt");

        let v = dispatch_file(
            &mut conn,
            "list_multipart_uploads",
            &serde_json::json!({ "prefix": "in/" }),
        )
        .await
        .unwrap();
        assert_eq!(v["uploads"][0]["upload_id"], "u1");

        let v = dispatch_file(
            &mut conn,
            "resume_multipart_upload",
            &serde_json::json!({ "path": "big.bin", "upload_id": "u9", "data": base64_chunk(b"12345") }),
        )
        .await
        .unwrap();
        assert_eq!(v["ok"], true);

        let v = dispatch_file(
            &mut conn,
            "abort_multipart_upload",
            &serde_json::json!({ "path": "big.bin", "upload_id": "u9" }),
        )
        .await
        .unwrap();
        assert_eq!(v["ok"], true);

        let v = dispatch_file(
            &mut conn,
            "get_acl",
            &serde_json::json!({ "path": "pub.html" }),
        )
        .await
        .unwrap();
        assert_eq!(v["acl"], "private");

        let v = dispatch_file(
            &mut conn,
            "put_acl",
            &serde_json::json!({ "path": "pub.html", "acl": "public-read" }),
        )
        .await
        .unwrap();
        assert_eq!(v["ok"], true);

        assert_eq!(
            conn.calls(),
            vec![
                "presigned_url:a.txt:60".to_string(),
                "list_multipart_uploads:in/".to_string(),
                "resume_multipart_upload:big.bin:u9:5".to_string(),
                "abort_multipart_upload:big.bin:u9".to_string(),
                "get_acl:pub.html".to_string(),
                "put_acl:pub.html:public-read".to_string(),
            ]
        );
    }

    /// 非 S3 connector 走 trait 默认实现 → `UnsupportedProtocolError`，
    /// 调用方据此映射 `UNSUPPORTED_PROTOCOL`。
    #[tokio::test]
    async fn default_s3_operations_report_unsupported_protocol() {
        struct DefaultsOnly;
        #[async_trait]
        impl FileConnector for DefaultsOnly {
            async fn list(&mut self, _path: &str) -> Result<Vec<FileEntry>> {
                unimplemented!()
            }
            async fn stat(&mut self, _path: &str) -> Result<FileEntry> {
                unimplemented!()
            }
            async fn upload(
                &mut self,
                _remote_path: &str,
                _data: Vec<u8>,
                _offset: u64,
                _progress: Option<&ProgressCallback>,
            ) -> Result<UploadResult> {
                unimplemented!()
            }
            async fn download(&mut self, _path: &str) -> Result<Vec<u8>> {
                unimplemented!()
            }
            async fn download_range(
                &mut self,
                _path: &str,
                _offset: u64,
                _limit: Option<u64>,
            ) -> Result<Vec<u8>> {
                unimplemented!()
            }
            async fn delete(&mut self, _path: &str) -> Result<()> {
                unimplemented!()
            }
            async fn rename(&mut self, _from: &str, _to: &str) -> Result<()> {
                unimplemented!()
            }
            async fn mkdir(&mut self, _path: &str) -> Result<()> {
                unimplemented!()
            }
            async fn read_for_edit(&mut self, _path: &str) -> Result<Vec<u8>> {
                unimplemented!()
            }
            async fn save_from_edit(&mut self, _path: &str, _data: Vec<u8>) -> Result<()> {
                unimplemented!()
            }
            async fn close(&mut self) -> Result<()> {
                unimplemented!()
            }
        }

        let conn = DefaultsOnly;
        assert_eq!(conn.capability(), FileCapabilitySet::default());

        let errors = [
            conn.presigned_url("k", 60).await.unwrap_err(),
            conn.list_multipart_uploads("p").await.unwrap_err(),
            conn.resume_multipart_upload("k", "u", Vec::new(), None)
                .await
                .unwrap_err(),
            conn.abort_multipart_upload("k", "u").await.unwrap_err(),
            conn.get_acl("k").await.unwrap_err(),
            conn.put_acl("k", "private").await.unwrap_err(),
        ];
        let messages: Vec<String> = errors
            .iter()
            .map(|e| {
                let u = e.downcast_ref::<UnsupportedProtocolError>().expect(
                    "must be an UnsupportedProtocolError so handlers map UNSUPPORTED_PROTOCOL",
                );
                u.message.clone()
            })
            .collect();
        assert_eq!(
            messages,
            vec![
                "presigned URL only supported for S3",
                "only supported for S3",
                "only supported for S3",
                "only supported for S3",
                "only supported for S3",
                "only supported for S3",
            ]
        );
    }
}
