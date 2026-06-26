use anyhow::Result;
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

/// S3 连接器的公开 trait
#[async_trait]
pub trait S3Connector: Send + Sync {
    /// 连接到 S3 服务
    async fn connect(&mut self) -> Result<()>;

    /// 列出所有 Bucket
    async fn list_buckets(&self) -> Result<Vec<BucketInfo>>;

    /// 列出指定 Bucket 下的对象
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

// ── S3Connector 实现 ─────────────────────────────────────

/// S3 连接器
pub struct S3ConnectorImpl {
    config: S3Config,
    client: Option<aws_sdk_s3::Client>,
}

impl S3ConnectorImpl {
    pub fn new(config: S3Config) -> Self {
        Self {
            config,
            client: None,
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

    fn require_client(&self) -> Result<&aws_sdk_s3::Client> {
        self.client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("not connected"))
    }
}

#[async_trait]
impl S3Connector for S3ConnectorImpl {
    async fn connect(&mut self) -> Result<()> {
        use aws_config::BehaviorVersion;
        use aws_sdk_s3::config::{Credentials, Region};

        let region = self
            .config
            .region
            .clone()
            .unwrap_or_else(|| "us-east-1".to_string());

        let mut loader = aws_config::from_env()
            .region(Region::new(region))
            .behavior_version(BehaviorVersion::latest());

        // 传递凭据到 SDK（支持 MinIO 等自托管服务）
        if !self.config.access_key.is_empty() && !self.config.secret_key.is_empty() {
            let creds = Credentials::new(
                &self.config.access_key,
                &self.config.secret_key,
                None, // session token
                None, // expiry
                "rex-s3-connector",
            );
            loader = loader.credentials_provider(creds);
        }

        if !self.config.endpoint.is_empty() {
            loader = loader.endpoint_url(&self.config.endpoint);
        }

        let sdk_config = loader.load().await;

        let s3_config = aws_sdk_s3::Config::from(&sdk_config)
            .to_builder()
            .force_path_style(self.config.force_path_style)
            .build();

        self.client = Some(aws_sdk_s3::Client::from_conf(s3_config));
        Ok(())
    }

    async fn list_buckets(&self) -> Result<Vec<BucketInfo>> {
        let output = self.require_client()?.list_buckets().send().await?;
        let buckets = output
            .buckets()
            .iter()
            .map(|b| BucketInfo {
                name: b.name().unwrap_or_default().to_string(),
                creation_date: b.creation_date().map(|d| d.to_string()),
            })
            .collect();
        Ok(buckets)
    }

    async fn list_objects(&self, bucket: &str, prefix: &str) -> Result<Vec<ObjectInfo>> {
        let client = self.require_client()?;

        let mut objects = Vec::new();
        let mut continuation_token: Option<String> = None;

        loop {
            let mut req = client.list_objects_v2().bucket(bucket).prefix(prefix);

            if let Some(ref token) = continuation_token {
                req = req.continuation_token(token);
            }

            let output = req.send().await?;

            for o in output.contents() {
                let key = o.key().unwrap_or_default().to_string();
                objects.push(ObjectInfo {
                    is_dir: key.ends_with('/'),
                    size: o.size().unwrap_or_default() as u64,
                    last_modified: o.last_modified().map(|d| d.to_string()),
                    etag: o.e_tag().map(|s| s.to_string()),
                    content_type: None,
                    storage_class: o.storage_class().map(|s| s.as_str().to_string()),
                    key,
                });
            }

            if output.is_truncated() == Some(true) {
                continuation_token = output.next_continuation_token().map(|s| s.to_string());
            } else {
                break;
            }
        }

        Ok(objects)
    }

    async fn get_object_info(&self, bucket: &str, key: &str) -> Result<ObjectInfo> {
        let output = self
            .require_client()?
            .head_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await?;

        Ok(ObjectInfo {
            key: key.to_string(),
            size: output.content_length().unwrap_or_default() as u64,
            last_modified: output.last_modified().map(|d| d.to_string()),
            etag: output.e_tag().map(|s| s.to_string()),
            content_type: output.content_type().map(|s| s.to_string()),
            storage_class: output.storage_class().map(|s| s.as_str().to_string()),
            is_dir: false,
        })
    }

    async fn upload_object(&self, bucket: &str, key: &str, data: Vec<u8>) -> Result<()> {
        self.require_client()?
            .put_object()
            .bucket(bucket)
            .key(key)
            .body(data.into())
            .send()
            .await?;
        Ok(())
    }

    async fn download_object(&self, bucket: &str, key: &str) -> Result<Vec<u8>> {
        let output = self
            .require_client()?
            .get_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await?;
        let bytes = output.body.collect().await?;
        Ok(bytes.into_bytes().to_vec())
    }

    async fn delete_object(&self, bucket: &str, key: &str) -> Result<()> {
        self.require_client()?
            .delete_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await?;
        Ok(())
    }

    async fn close(&self) -> Result<()> {
        // S3 客户端基于 HTTP，无需显式关闭
        Ok(())
    }
}

// ── Tests ────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// 测试用 mock S3 连接器（直接实现 S3Connector trait）
    struct MockS3Connector {
        buckets: Mutex<Vec<String>>,
        objects: Mutex<HashMap<String, Vec<u8>>>,
    }

    impl MockS3Connector {
        fn new() -> Self {
            Self {
                buckets: Mutex::new(Vec::new()),
                objects: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl S3Connector for MockS3Connector {
        async fn connect(&mut self) -> Result<()> {
            Ok(())
        }

        async fn list_buckets(&self) -> Result<Vec<BucketInfo>> {
            let buckets = self.buckets.lock().unwrap();
            let result = buckets
                .iter()
                .map(|b| BucketInfo {
                    name: b.clone(),
                    creation_date: None,
                })
                .collect();
            Ok(result)
        }

        async fn list_objects(&self, _bucket: &str, prefix: &str) -> Result<Vec<ObjectInfo>> {
            let objects = self.objects.lock().unwrap();
            let result = objects
                .iter()
                .filter(|(key, _)| key.starts_with(prefix))
                .map(|(key, value)| ObjectInfo {
                    key: key.clone(),
                    size: value.len() as u64,
                    last_modified: None,
                    etag: None,
                    content_type: None,
                    storage_class: None,
                    is_dir: key.ends_with('/'),
                })
                .collect();
            Ok(result)
        }

        async fn get_object_info(&self, _bucket: &str, key: &str) -> Result<ObjectInfo> {
            let objects = self.objects.lock().unwrap();
            match objects.get(key) {
                Some(value) => Ok(ObjectInfo {
                    key: key.to_string(),
                    size: value.len() as u64,
                    last_modified: None,
                    etag: None,
                    content_type: None,
                    storage_class: None,
                    is_dir: key.ends_with('/'),
                }),
                None => Err(anyhow::anyhow!("object not found")),
            }
        }

        async fn upload_object(&self, _bucket: &str, key: &str, data: Vec<u8>) -> Result<()> {
            let mut objects = self.objects.lock().unwrap();
            objects.insert(key.to_string(), data);
            Ok(())
        }

        async fn download_object(&self, _bucket: &str, key: &str) -> Result<Vec<u8>> {
            let objects = self.objects.lock().unwrap();
            match objects.get(key) {
                Some(value) => Ok(value.clone()),
                None => Err(anyhow::anyhow!("object not found")),
            }
        }

        async fn delete_object(&self, _bucket: &str, key: &str) -> Result<()> {
            let mut objects = self.objects.lock().unwrap();
            objects.remove(key);
            Ok(())
        }

        async fn close(&self) -> Result<()> {
            Ok(())
        }
    }

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
        let json = r#"{"endpoint":"http://minio:9000","access_key":"key","secret_key":"secret","region":"us-east-1","bucket":"my-bucket","force_path_style":false,"name":"test"}"#;
        let config: S3Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.endpoint, "http://minio:9000");
        assert_eq!(config.access_key, "key");
        assert_eq!(config.secret_key, "secret");
        assert_eq!(config.region, Some("us-east-1".to_string()));
        assert_eq!(config.bucket, Some("my-bucket".to_string()));
        assert!(!config.force_path_style);
        assert_eq!(config.name, Some("test".to_string()));
    }

    #[test]
    fn s3_connector_from_json() {
        let json = r#"{"endpoint":"http://minio:9000","access_key":"key","secret_key":"secret"}"#;
        let connector = S3ConnectorImpl::from_json(json).unwrap();
        assert_eq!(connector.config().endpoint, "http://minio:9000");
        assert!(connector.client.is_none());
    }

    #[test]
    fn s3_connector_is_object_safe() {
        fn _assert_object_safe(_: &dyn S3Connector) {}
    }

    #[tokio::test]
    async fn s3_operations_fail_when_not_connected() {
        let connector = S3ConnectorImpl::new(S3Config::default());
        assert!(connector.list_buckets().await.is_err());
        assert!(connector.list_objects("b", "").await.is_err());
        assert!(connector.get_object_info("b", "k").await.is_err());
        assert!(connector.upload_object("b", "k", vec![]).await.is_err());
        assert!(connector.download_object("b", "k").await.is_err());
        assert!(connector.delete_object("b", "k").await.is_err());
    }

    #[tokio::test]
    async fn s3_list_buckets_returns_empty() {
        let connector = MockS3Connector::new();
        let buckets = connector.list_buckets().await.unwrap();
        assert!(buckets.is_empty());
    }

    #[tokio::test]
    async fn s3_list_buckets_returns_inserted_buckets() {
        let connector = MockS3Connector::new();
        connector
            .buckets
            .lock()
            .unwrap()
            .push("test-bucket".to_string());

        let buckets = connector.list_buckets().await.unwrap();
        assert_eq!(buckets.len(), 1);
        assert_eq!(buckets[0].name, "test-bucket");
    }

    #[tokio::test]
    async fn s3_list_objects_returns_empty() {
        let connector = MockS3Connector::new();
        let objects = connector.list_objects("bucket", "").await.unwrap();
        assert!(objects.is_empty());
    }

    #[tokio::test]
    async fn s3_list_objects_returns_matching_objects() {
        let connector = MockS3Connector::new();
        {
            let mut objects = connector.objects.lock().unwrap();
            objects.insert("dir/file1.txt".to_string(), b"content1".to_vec());
            objects.insert("dir/file2.txt".to_string(), b"content2".to_vec());
            objects.insert("other/file3.txt".to_string(), b"content3".to_vec());
        }

        let objects = connector.list_objects("bucket", "dir/").await.unwrap();
        assert_eq!(objects.len(), 2);
        for obj in &objects {
            assert!(obj.key.starts_with("dir/"));
        }
    }

    #[tokio::test]
    async fn s3_get_object_info_returns_info() {
        let connector = MockS3Connector::new();
        connector
            .objects
            .lock()
            .unwrap()
            .insert("hello.txt".to_string(), b"hello world".to_vec());

        let info = connector
            .get_object_info("bucket", "hello.txt")
            .await
            .unwrap();
        assert_eq!(info.key, "hello.txt");
        assert_eq!(info.size, 11);
    }

    #[tokio::test]
    async fn s3_get_object_info_fails_when_not_found() {
        let connector = MockS3Connector::new();
        let result = connector.get_object_info("bucket", "nonexistent.txt").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn s3_upload_and_download_object() {
        let connector = MockS3Connector::new();
        let data = b"hello world".to_vec();
        connector
            .upload_object("test-bucket", "hello.txt", data.clone())
            .await
            .unwrap();
        let downloaded = connector
            .download_object("test-bucket", "hello.txt")
            .await
            .unwrap();
        assert_eq!(downloaded, data);
    }

    #[tokio::test]
    async fn s3_upload_and_download_binary_data() {
        let connector = MockS3Connector::new();
        let data: Vec<u8> = (0..=255).collect();
        connector
            .upload_object("bucket", "binary.bin", data.clone())
            .await
            .unwrap();
        let downloaded = connector
            .download_object("bucket", "binary.bin")
            .await
            .unwrap();
        assert_eq!(downloaded, data);
    }

    #[tokio::test]
    async fn s3_delete_object() {
        let connector = MockS3Connector::new();
        connector
            .upload_object("test-bucket", "to_delete.txt", b"delete me".to_vec())
            .await
            .unwrap();
        connector
            .delete_object("test-bucket", "to_delete.txt")
            .await
            .unwrap();
        let result = connector
            .download_object("test-bucket", "to_delete.txt")
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn s3_delete_nonexistent_object_succeeds() {
        let connector = MockS3Connector::new();
        let result = connector.delete_object("bucket", "nonexistent.txt").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn s3_close_succeeds() {
        let connector = MockS3Connector::new();
        assert!(connector.close().await.is_ok());
    }

    #[test]
    fn bucket_info_serializes() {
        let info = BucketInfo {
            name: "test-bucket".to_string(),
            creation_date: Some("2024-01-01T00:00:00Z".to_string()),
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("test-bucket"));
        assert!(json.contains("2024-01-01"));
    }

    #[test]
    fn object_info_serializes() {
        let info = ObjectInfo {
            key: "test/file.txt".to_string(),
            size: 1024,
            last_modified: None,
            etag: Some("\"abc123\"".to_string()),
            content_type: Some("text/plain".to_string()),
            storage_class: Some("STANDARD".to_string()),
            is_dir: false,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("test/file.txt"));
        assert!(json.contains("1024"));
    }
}
