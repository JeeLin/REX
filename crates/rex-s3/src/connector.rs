use anyhow::{bail, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ── 数据模型 ─────────────────────────────────────────────

/// S3/MinIO 连接配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Config {
    /// S3 endpoint URL（如 https://s3.amazonaws.com 或 http://minio:9000）
    pub endpoint: String,
    /// Access Key ID
    pub access_key: String,
    /// Secret Access Key
    pub secret_key: String,
    /// 区域（如 us-east-1）
    pub region: Option<String>,
    /// 默认 Bucket（可选，连接后可切换）
    pub bucket: Option<String>,
    /// 强制 path-style（MinIO 兼容）
    #[serde(default = "default_force_path_style")]
    pub force_path_style: bool,
    /// 实例名称
    pub name: Option<String>,
}

fn default_force_path_style() -> bool {
    true
}

impl Default for S3Config {
    fn default() -> Self {
        Self {
            endpoint: String::new(),
            access_key: String::new(),
            secret_key: String::new(),
            region: None,
            bucket: None,
            force_path_style: true,
            name: None,
        }
    }
}

/// Bucket 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketInfo {
    pub name: String,
    pub creation_date: Option<String>,
}

/// Object 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectInfo {
    pub key: String,
    pub size: u64,
    pub last_modified: Option<String>,
    pub etag: Option<String>,
    pub content_type: Option<String>,
    pub storage_class: Option<String>,
    /// 是否为虚拟目录（前缀 + `/` 结尾）
    pub is_dir: bool,
}

// ── S3Connector trait ─────────────────────────────────────

#[async_trait]
pub trait S3Connector: Send + Sync {
    /// 连接到 S3 服务并验证凭据
    async fn connect(&mut self) -> Result<()>;

    /// 列出所有 Bucket
    async fn list_buckets(&self) -> Result<Vec<BucketInfo>>;

    /// 列出指定 Bucket 下的对象（支持虚拟目录前缀）
    async fn list_objects(&self, bucket: &str, prefix: &str) -> Result<Vec<ObjectInfo>>;

    /// 获取单个对象的元数据
    async fn get_object_info(&self, bucket: &str, key: &str) -> Result<ObjectInfo>;

    /// 上传对象
    async fn upload_object(&self, bucket: &str, key: &str, data: Vec<u8>) -> Result<()>;

    /// 下载对象
    async fn download_object(&self, bucket: &str, key: &str) -> Result<Vec<u8>>;

    /// 删除对象
    async fn delete_object(&self, bucket: &str, key: &str) -> Result<()>;

    /// 关闭连接
    async fn close(&self) -> Result<()>;
}

// ── S3Connector stub ──────────────────────────────────────

/// S3 连接器（stub 实现）
///
/// 实际连接通过 Agent 代理或 Hub 直连的 HTTP 隧道完成。
/// 此 stub 用于 trait 定义和配置解析。
pub struct S3ConnectorImpl {
    config: S3Config,
    connected: bool,
}

impl S3ConnectorImpl {
    pub fn new(config: S3Config) -> Self {
        Self {
            config,
            connected: false,
        }
    }

    pub fn from_json(json: &str) -> Result<Self> {
        let config: S3Config = serde_json::from_str(json)?;
        Ok(Self::new(config))
    }

    pub fn config(&self) -> &S3Config {
        &self.config
    }

    pub fn into_config(self) -> S3Config {
        self.config
    }
}

#[async_trait]
impl S3Connector for S3ConnectorImpl {
    async fn connect(&mut self) -> Result<()> {
        // TODO: 实际连接 S3 endpoint，验证凭据
        self.connected = true;
        Ok(())
    }

    async fn list_buckets(&self) -> Result<Vec<BucketInfo>> {
        if !self.connected {
            bail!("not connected");
        }
        // TODO: 调用 ListBuckets API
        Ok(vec![])
    }

    async fn list_objects(&self, _bucket: &str, _prefix: &str) -> Result<Vec<ObjectInfo>> {
        if !self.connected {
            bail!("not connected");
        }
        // TODO: 调用 ListObjectsV2 API
        Ok(vec![])
    }

    async fn get_object_info(&self, _bucket: &str, _key: &str) -> Result<ObjectInfo> {
        if !self.connected {
            bail!("not connected");
        }
        // TODO: 调用 HeadObject API
        Ok(ObjectInfo {
            key: String::new(),
            size: 0,
            last_modified: None,
            etag: None,
            content_type: None,
            storage_class: None,
            is_dir: false,
        })
    }

    async fn upload_object(&self, _bucket: &str, _key: &str, _data: Vec<u8>) -> Result<()> {
        if !self.connected {
            bail!("not connected");
        }
        // TODO: 调用 PutObject API
        Ok(())
    }

    async fn download_object(&self, _bucket: &str, _key: &str) -> Result<Vec<u8>> {
        if !self.connected {
            bail!("not connected");
        }
        // TODO: 调用 GetObject API
        Ok(vec![])
    }

    async fn delete_object(&self, _bucket: &str, _key: &str) -> Result<()> {
        if !self.connected {
            bail!("not connected");
        }
        // TODO: 调用 DeleteObject API
        Ok(())
    }

    async fn close(&self) -> Result<()> {
        // TODO: 关闭连接
        Ok(())
    }
}

// ── Tests ────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn s3_config_default() {
        let config = S3Config::default();
        assert!(config.endpoint.is_empty());
        assert!(config.access_key.is_empty());
        assert!(config.secret_key.is_empty());
        assert!(config.region.is_none());
        assert!(config.bucket.is_none());
        assert!(config.force_path_style);
        assert!(config.name.is_none());
    }

    #[test]
    fn s3_config_deserializes() {
        let json = r#"{"endpoint":"http://minio:9000","access_key":"minioadmin","secret_key":"minioadmin","region":"us-east-1","bucket":"my-bucket","force_path_style":true,"name":"MinIO"}"#;
        let config: S3Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.endpoint, "http://minio:9000");
        assert_eq!(config.access_key, "minioadmin");
        assert_eq!(config.secret_key, "minioadmin");
        assert_eq!(config.region, Some("us-east-1".into()));
        assert_eq!(config.bucket, Some("my-bucket".into()));
        assert!(config.force_path_style);
        assert_eq!(config.name, Some("MinIO".into()));
    }

    #[test]
    fn s3_config_optional_fields() {
        let json = r#"{"endpoint":"http://minio:9000","access_key":"key","secret_key":"secret"}"#;
        let config: S3Config = serde_json::from_str(json).unwrap();
        assert!(config.region.is_none());
        assert!(config.bucket.is_none());
        assert!(config.force_path_style); // default
        assert!(config.name.is_none());
    }

    #[test]
    fn s3_config_force_path_style_defaults_true() {
        let json = r#"{"endpoint":"http://minio:9000","access_key":"key","secret_key":"secret"}"#;
        let config: S3Config = serde_json::from_str(json).unwrap();
        assert!(config.force_path_style);
    }

    #[test]
    fn s3_config_can_set_force_path_style_false() {
        let json = r#"{"endpoint":"https://s3.amazonaws.com","access_key":"key","secret_key":"secret","force_path_style":false}"#;
        let config: S3Config = serde_json::from_str(json).unwrap();
        assert!(!config.force_path_style);
    }

    #[test]
    fn s3_connector_from_json() {
        let json = r#"{"endpoint":"http://minio:9000","access_key":"key","secret_key":"secret"}"#;
        let connector = S3ConnectorImpl::from_json(json).unwrap();
        assert_eq!(connector.config().endpoint, "http://minio:9000");
        assert!(!connector.connected);
    }

    #[test]
    fn s3_connector_is_object_safe() {
        fn _assert_object_safe(_: &dyn S3Connector) {}
    }

    #[tokio::test]
    async fn s3_connect_sets_connected() {
        let json = r#"{"endpoint":"http://minio:9000","access_key":"key","secret_key":"secret"}"#;
        let mut connector = S3ConnectorImpl::from_json(json).unwrap();
        assert!(!connector.connected);
        connector.connect().await.unwrap();
        assert!(connector.connected);
    }

    #[tokio::test]
    async fn s3_list_buckets_fails_when_not_connected() {
        let json = r#"{"endpoint":"http://minio:9000","access_key":"key","secret_key":"secret"}"#;
        let connector = S3ConnectorImpl::from_json(json).unwrap();
        let result = connector.list_buckets().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn s3_list_objects_fails_when_not_connected() {
        let json = r#"{"endpoint":"http://minio:9000","access_key":"key","secret_key":"secret"}"#;
        let connector = S3ConnectorImpl::from_json(json).unwrap();
        let result = connector.list_objects("bucket", "").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn s3_get_object_info_fails_when_not_connected() {
        let json = r#"{"endpoint":"http://minio:9000","access_key":"key","secret_key":"secret"}"#;
        let connector = S3ConnectorImpl::from_json(json).unwrap();
        let result = connector.get_object_info("bucket", "key").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn s3_upload_object_fails_when_not_connected() {
        let json = r#"{"endpoint":"http://minio:9000","access_key":"key","secret_key":"secret"}"#;
        let connector = S3ConnectorImpl::from_json(json).unwrap();
        let result = connector.upload_object("bucket", "key", vec![]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn s3_download_object_fails_when_not_connected() {
        let json = r#"{"endpoint":"http://minio:9000","access_key":"key","secret_key":"secret"}"#;
        let connector = S3ConnectorImpl::from_json(json).unwrap();
        let result = connector.download_object("bucket", "key").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn s3_delete_object_fails_when_not_connected() {
        let json = r#"{"endpoint":"http://minio:9000","access_key":"key","secret_key":"secret"}"#;
        let connector = S3ConnectorImpl::from_json(json).unwrap();
        let result = connector.delete_object("bucket", "key").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn s3_list_buckets_returns_empty_when_connected() {
        let json = r#"{"endpoint":"http://minio:9000","access_key":"key","secret_key":"secret"}"#;
        let mut connector = S3ConnectorImpl::from_json(json).unwrap();
        connector.connect().await.unwrap();
        let buckets = connector.list_buckets().await.unwrap();
        assert!(buckets.is_empty());
    }

    #[tokio::test]
    async fn s3_list_objects_returns_empty_when_connected() {
        let json = r#"{"endpoint":"http://minio:9000","access_key":"key","secret_key":"secret"}"#;
        let mut connector = S3ConnectorImpl::from_json(json).unwrap();
        connector.connect().await.unwrap();
        let objects = connector.list_objects("bucket", "").await.unwrap();
        assert!(objects.is_empty());
    }

    #[tokio::test]
    async fn s3_close_succeeds() {
        let json = r#"{"endpoint":"http://minio:9000","access_key":"key","secret_key":"secret"}"#;
        let mut connector = S3ConnectorImpl::from_json(json).unwrap();
        connector.connect().await.unwrap();
        connector.close().await.unwrap();
    }

    #[test]
    fn bucket_info_serializes() {
        let info = BucketInfo {
            name: "my-bucket".into(),
            creation_date: Some("2024-01-15T00:00:00Z".into()),
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("my-bucket"));
        assert!(json.contains("2024-01-15"));
    }

    #[test]
    fn object_info_serializes() {
        let info = ObjectInfo {
            key: "path/to/file.txt".into(),
            size: 1024,
            last_modified: Some("2024-01-15T00:00:00Z".into()),
            etag: Some("\"abc123\"".into()),
            content_type: Some("text/plain".into()),
            storage_class: Some("STANDARD".into()),
            is_dir: false,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("path/to/file.txt"));
        assert!(json.contains("1024"));
        assert!(json.contains("text/plain"));
    }

    #[test]
    fn object_info_dir_serializes() {
        let info = ObjectInfo {
            key: "images/".into(),
            size: 0,
            last_modified: None,
            etag: None,
            content_type: None,
            storage_class: None,
            is_dir: true,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("images/"));
        assert!(json.contains("true"));
    }
}
