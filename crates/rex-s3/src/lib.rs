//! S3 协议实现 — 基于 AWS SDK 的 FileConnector。

use anyhow::{Context, Result};
use async_trait::async_trait;
use aws_sdk_s3::Client;
use rex_common::file_transfer::{FileConnector, FileEntry, ProgressCallback, UploadResult};

/// S3 连接器
pub struct S3Connector {
    client: Client,
    bucket: String,
}

impl S3Connector {
    /// 建立 S3 连接
    pub async fn connect(
        bucket: String,
        region: Option<String>,
        endpoint: Option<String>,
        access_key: Option<String>,
        secret_key: Option<String>,
    ) -> Result<Self> {
        let mut config_loader = aws_config::from_env();

        if let Some(ref r) = region {
            config_loader = config_loader.region(aws_config::Region::new(r.clone()));
        }

        if let Some(ref ep) = endpoint {
            config_loader = config_loader.endpoint_url(ep);
        }

        if let (Some(ak), Some(sk)) = (&access_key, &secret_key) {
            let credentials =
                aws_sdk_s3::config::Credentials::new(ak.clone(), sk.clone(), None, None, "rex-hub");
            config_loader = config_loader.credentials_provider(credentials);
        }

        let sdk_config = config_loader.load().await;
        let client = Client::new(&sdk_config);

        Ok(Self { client, bucket })
    }

    /// 从 FileConnectRequest 建立连接
    pub async fn connect_from_request(
        req: &rex_common::file_transfer::FileConnectRequest,
    ) -> Result<Self> {
        Self::connect(
            req.bucket.clone().unwrap_or_default(),
            req.region.clone(),
            req.endpoint.clone(),
            req.access_key.clone(),
            req.secret_key.clone(),
        )
        .await
    }
}

#[async_trait]
impl FileConnector for S3Connector {
    async fn list(&mut self, path: &str) -> Result<Vec<FileEntry>> {
        let prefix = if path.is_empty() || path == "/" {
            String::new()
        } else {
            let p = path.trim_start_matches('/');
            format!("{p}/")
        };

        let result = self
            .client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(&prefix)
            .delimiter("/")
            .send()
            .await
            .context("failed to list S3 objects")?;

        let mut entries = Vec::new();

        // 添加目录（common prefixes）
        if let Some(prefixes) = result.common_prefixes {
            for p in prefixes {
                if let Some(name) = p.prefix {
                    let name = name
                        .strip_prefix(&prefix)
                        .unwrap_or(&name)
                        .trim_end_matches('/');
                    if !name.is_empty() {
                        entries.push(FileEntry {
                            name: name.to_string(),
                            path: format!("{prefix}{name}"),
                            is_dir: true,
                            size: 0,
                            modified: None,
                            permissions: None,
                            storage_class: None,
                            acl: None,
                        });
                    }
                }
            }
        }

        // 添加文件
        if let Some(objects) = result.contents {
            for obj in objects {
                if let Some(key) = obj.key {
                    let name = key.strip_prefix(&prefix).unwrap_or(&key);
                    if !name.is_empty() && !name.ends_with('/') {
                        entries.push(FileEntry {
                            name: name.to_string(),
                            path: key,
                            is_dir: false,
                            size: obj.size.unwrap_or(0) as u64,
                            modified: obj.last_modified.map(|t| {
                                let secs = t.secs();
                                format!("{secs}")
                            }),
                            permissions: None,
                            storage_class: obj
                                .storage_class
                                .as_ref()
                                .map(|sc| sc.as_str().to_string()),
                            acl: None,
                        });
                    }
                }
            }
        }

        entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
        Ok(entries)
    }

    async fn stat(&mut self, path: &str) -> Result<FileEntry> {
        let key = path.trim_start_matches('/');
        let result = self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await;

        match result {
            Ok(obj) => {
                let name = key.rsplit('/').next().unwrap_or(key).to_string();
                Ok(FileEntry {
                    name,
                    path: path.to_string(),
                    is_dir: false,
                    size: obj.content_length.unwrap_or(0) as u64,
                    modified: obj.last_modified.map(|t| format!("{}", t.secs())),
                    permissions: None,
                    storage_class: obj.storage_class.as_ref().map(|sc| sc.as_str().to_string()),
                    acl: None,
                })
            }
            Err(_) => {
                // 可能是目录，尝试 list 来确认
                let name = key.rsplit('/').next().unwrap_or(key).to_string();
                Ok(FileEntry {
                    name,
                    path: path.to_string(),
                    is_dir: true,
                    size: 0,
                    modified: None,
                    permissions: None,
                    storage_class: None,
                    acl: None,
                })
            }
        }
    }

    async fn upload(
        &mut self,
        remote_path: &str,
        data: Vec<u8>,
        _offset: u64,
        progress: Option<&ProgressCallback>,
    ) -> Result<rex_common::file_transfer::UploadResult> {
        let key = remote_path.trim_start_matches('/');
        let total = data.len() as u64;

        // 小文件直接上传，大文件分片
        if total <= 5 * 1024 * 1024 {
            // 5MB 以下直接上传
            self.client
                .put_object()
                .bucket(&self.bucket)
                .key(key)
                .body(data.into())
                .send()
                .await
                .context("failed to upload to S3")?;
            if let Some(cb) = progress {
                cb(total, total);
            }
            Ok(UploadResult::default())
        } else {
            // 分片上传
            let part_size = 5 * 1024 * 1024; // 5MB
            let multipart = self
                .client
                .create_multipart_upload()
                .bucket(&self.bucket)
                .key(key)
                .send()
                .await
                .context("failed to initiate multipart upload")?;

            let upload_id = multipart.upload_id().context("no upload id")?;
            let mut parts = Vec::new();
            let mut offset = 0u64;

            for (i, chunk) in data.chunks(part_size).enumerate() {
                let part_number = (i as i32) + 1;
                let result = self
                    .client
                    .upload_part()
                    .bucket(&self.bucket)
                    .key(key)
                    .upload_id(upload_id)
                    .part_number(part_number)
                    .body(chunk.to_vec().into())
                    .send()
                    .await
                    .context("failed to upload part")?;

                parts.push(
                    aws_sdk_s3::types::CompletedPart::builder()
                        .part_number(part_number)
                        .e_tag(result.e_tag().unwrap_or_default())
                        .build(),
                );

                offset += chunk.len() as u64;
                if let Some(cb) = progress {
                    cb(offset, total);
                }
            }

            self.client
                .complete_multipart_upload()
                .bucket(&self.bucket)
                .key(key)
                .upload_id(upload_id)
                .multipart_upload(
                    aws_sdk_s3::types::CompletedMultipartUpload::builder()
                        .set_parts(Some(parts))
                        .build(),
                )
                .send()
                .await
                .context("failed to complete multipart upload")?;
            Ok(UploadResult {
                upload_id: Some(upload_id.to_string()),
            })
        }
    }

    async fn download(&mut self, path: &str) -> Result<Vec<u8>> {
        let key = path.trim_start_matches('/');
        let result = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .with_context(|| format!("failed to download {key}"))?;

        let bytes = result
            .body
            .collect()
            .await
            .with_context(|| format!("failed to read body of {key}"))?;
        Ok(bytes.into_bytes().to_vec())
    }

    async fn delete(&mut self, path: &str) -> Result<()> {
        let key = path.trim_start_matches('/');
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .with_context(|| format!("failed to delete {key}"))?;
        Ok(())
    }

    async fn rename(&mut self, from: &str, to: &str) -> Result<()> {
        let from_key = from.trim_start_matches('/');
        let to_key = to.trim_start_matches('/');

        // S3 不支持 rename，需要 copy + delete
        self.client
            .copy_object()
            .bucket(&self.bucket)
            .key(to_key)
            .copy_source(format!("{}/{}", self.bucket, from_key))
            .send()
            .await
            .context("failed to copy object")?;

        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(from_key)
            .send()
            .await
            .context("failed to delete source object")?;

        Ok(())
    }

    async fn mkdir(&mut self, path: &str) -> Result<()> {
        let key = if path.ends_with('/') {
            path.trim_start_matches('/').to_string()
        } else {
            format!("{}/", path.trim_start_matches('/'))
        };

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(Vec::new().into())
            .send()
            .await
            .context("failed to create S3 prefix")?;
        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }

    async fn read_for_edit(&mut self, path: &str) -> Result<Vec<u8>> {
        let key = path.trim_start_matches('/');
        let result = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .with_context(|| format!("failed to download {key}"))?;

        let bytes = result
            .body
            .collect()
            .await
            .with_context(|| format!("failed to read body of {key}"))?;
        let data = bytes.into_bytes().to_vec();
        if data.len() > 5 * 1024 * 1024 {
            anyhow::bail!("File too large for editing (>5MB)");
        }
        Ok(data)
    }

    async fn save_from_edit(&mut self, path: &str, data: Vec<u8>) -> Result<()> {
        let key = path.trim_start_matches('/');
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(data.into())
            .send()
            .await
            .with_context(|| format!("failed to save {key}"))?;
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl S3Connector {
    /// 生成 presigned URL（临时访问链接）
    pub async fn presigned_url(&self, key: &str, expires_in_secs: u64) -> Result<String> {
        use aws_sdk_s3::presigning::PresigningConfig;

        let key = key.trim_start_matches('/');
        let expires = std::time::Duration::from_secs(expires_in_secs);

        let presigned = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(PresigningConfig::expires_in(expires)?)
            .await
            .context("failed to generate presigned URL")?;

        Ok(presigned.uri().to_string())
    }

    /// 列出进行中的 multipart uploads
    pub async fn list_multipart_uploads(&self, prefix: &str) -> Result<Vec<(String, String)>> {
        let result = self
            .client
            .list_multipart_uploads()
            .bucket(&self.bucket)
            .prefix(prefix)
            .send()
            .await
            .context("failed to list multipart uploads")?;

        let mut uploads = Vec::new();
        if let Some(uploads_list) = result.uploads {
            for upload in uploads_list {
                if let (Some(key), Some(upload_id)) = (upload.key, upload.upload_id) {
                    uploads.push((key, upload_id));
                }
            }
        }
        Ok(uploads)
    }

    /// 恢复进行中的 multipart upload
    /// 续传 multipart upload
    pub async fn resume_multipart_upload(
        &self,
        key: &str,
        upload_id: &str,
        data: Vec<u8>,
        progress: Option<&ProgressCallback>,
    ) -> Result<()> {
        let total = data.len() as u64;
        let part_size = 5 * 1024 * 1024; // 5MB

        // 先获取已上传的 parts，使用 list_parts 结果作为权威来源
        let list_result = self
            .client
            .list_parts()
            .bucket(&self.bucket)
            .key(key)
            .upload_id(upload_id)
            .send()
            .await
            .context("failed to list parts")?;

        let completed_parts = list_result.parts.unwrap_or_default();
        let completed_part_numbers: std::collections::HashSet<i32> = completed_parts
            .iter()
            .filter_map(|p| p.part_number())
            .collect();

        let mut parts: Vec<aws_sdk_s3::types::CompletedPart> = completed_parts
            .iter()
            .filter_map(|p| {
                Some(
                    aws_sdk_s3::types::CompletedPart::builder()
                        .part_number(p.part_number()?)
                        .e_tag(p.e_tag()?.to_string())
                        .build(),
                )
            })
            .collect();

        let mut offset = 0u64;

        // 上传剩余分片（跳过已完成的）
        for (i, chunk) in data.chunks(part_size).enumerate() {
            let part_number = (i as i32) + 1;
            if completed_part_numbers.contains(&part_number) {
                // 跳过已完成的分片
                offset += chunk.len() as u64;
                if let Some(ref cb) = progress {
                    cb(offset, total);
                }
                continue;
            }

            let result = self
                .client
                .upload_part()
                .bucket(&self.bucket)
                .key(key)
                .upload_id(upload_id)
                .part_number(part_number)
                .body(chunk.to_vec().into())
                .send()
                .await
                .context("failed to upload part")?;

            parts.push(
                aws_sdk_s3::types::CompletedPart::builder()
                    .part_number(part_number)
                    .e_tag(result.e_tag().unwrap_or_default())
                    .build(),
            );

            offset += chunk.len() as u64;
            if let Some(ref cb) = progress {
                cb(offset, total);
            }
        }

        // 完成 multipart upload
        self.client
            .complete_multipart_upload()
            .bucket(&self.bucket)
            .key(key)
            .upload_id(upload_id)
            .multipart_upload(
                aws_sdk_s3::types::CompletedMultipartUpload::builder()
                    .set_parts(Some(parts))
                    .build(),
            )
            .send()
            .await
            .context("failed to complete multipart upload")?;

        Ok(())
    }

    /// 取消进行中的 multipart upload
    pub async fn abort_multipart_upload(&self, key: &str, upload_id: &str) -> Result<()> {
        self.client
            .abort_multipart_upload()
            .bucket(&self.bucket)
            .key(key)
            .upload_id(upload_id)
            .send()
            .await
            .context("failed to abort multipart upload")?;
        Ok(())
    }

    /// 获取对象的 Canned ACL
    pub async fn get_acl(&self, key: &str) -> Result<String> {
        let key = key.trim_start_matches('/');
        let result = self
            .client
            .get_object_acl()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .context("failed to get object ACL")?;

        // 从 ACL grants 推断 Canned ACL
        let grants = result.grants();
        let has_all = grants.iter().any(|g| {
            g.grantee()
                .map(|a| a.uri().map(|u| u.contains("AllUsers")).unwrap_or(false))
                .unwrap_or(false)
                && g.permission() == Some(&aws_sdk_s3::types::Permission::Read)
        });
        let has_auth = grants.iter().any(|g| {
            g.grantee()
                .map(|a| {
                    a.uri()
                        .map(|u| u.contains("AuthenticatedUsers"))
                        .unwrap_or(false)
                })
                .unwrap_or(false)
        });

        if has_all {
            Ok("public-read".to_string())
        } else if has_auth {
            Ok("authenticated-read".to_string())
        } else {
            Ok("private".to_string())
        }
    }

    /// 设置对象的 Canned ACL
    pub async fn put_acl(&self, key: &str, canned_acl: &str) -> Result<()> {
        let key = key.trim_start_matches('/');
        let acl = match canned_acl {
            "public-read" => aws_sdk_s3::types::ObjectCannedAcl::PublicRead,
            "public-read-write" => aws_sdk_s3::types::ObjectCannedAcl::PublicReadWrite,
            "authenticated-read" => aws_sdk_s3::types::ObjectCannedAcl::AuthenticatedRead,
            _ => aws_sdk_s3::types::ObjectCannedAcl::Private,
        };

        self.client
            .put_object_acl()
            .bucket(&self.bucket)
            .key(key)
            .acl(acl)
            .send()
            .await
            .context("failed to set object ACL")?;
        Ok(())
    }
}
