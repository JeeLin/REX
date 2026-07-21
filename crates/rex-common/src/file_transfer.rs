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
    /// S3 专用
    pub bucket: Option<String>,
    pub region: Option<String>,
    pub endpoint: Option<String>,
    pub access_key: Option<String>,
    pub secret_key: Option<String>,
}

/// 进度回调
pub type ProgressCallback = Box<dyn Fn(u64, u64) + Send + Sync>;

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
        progress: Option<&ProgressCallback>,
    ) -> Result<()>;

    /// 下载文件
    async fn download(&mut self, path: &str) -> Result<Vec<u8>>;

    /// 删除文件/目录
    async fn delete(&mut self, path: &str) -> Result<()>;

    /// 重命名/移动
    async fn rename(&mut self, from: &str, to: &str) -> Result<()>;

    /// 创建目录
    async fn mkdir(&mut self, path: &str) -> Result<()>;

    /// 关闭连接
    async fn close(&mut self) -> Result<()>;

    /// Downcast support for protocol-specific methods
    fn as_any(&self) -> &dyn std::any::Any;
}
