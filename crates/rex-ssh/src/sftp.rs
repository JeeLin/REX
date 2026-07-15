//! SFTP 文件连接器 — 基于 russh-sftp 实现 FileConnector。

use anyhow::{Context, Result};
use async_trait::async_trait;
use rex_common::file_transfer::{FileConnector, FileEntry, ProgressCallback};
use tokio::io::AsyncReadExt;

/// SFTP 连接器
pub struct SftpConnector {
    client: russh_sftp::Client,
}

impl SftpConnector {
    /// 通过已有的 SSH session 建立 SFTP 连接
    pub async fn connect(session: russh::client::Handle<crate::SshHandler>) -> Result<Self> {
        let channel = session
            .channel_open_session()
            .await
            .context("failed to open session for SFTP")?;

        let subsystem = channel
            .request_subsystem(true, "sftp")
            .await
            .context("failed to request SFTP subsystem")?;

        let client = russh_sftp::Client::new(subsystem)
            .await
            .context("failed to create SFTP client")?;

        Ok(Self { client })
    }

    /// 从 SSH 配置直接建立 SFTP 连接
    pub async fn connect_with_config(config: crate::SshConfig) -> Result<Self> {
        use russh::client;
        use std::sync::Arc;

        let ssh_config = Arc::new(client::Config::default());
        let handler = crate::SshHandler;
        let mut handle = client::connect(ssh_config, &format!("{}:{}", config.host, config.port), handler)
            .await
            .context("SSH connection failed for SFTP")?;

        // 认证
        if let Some(ref key_pem) = config.private_key {
            let private_key = russh::keys::decode_secret_key(key_pem, config.password.as_deref())
                .context("failed to decode private key")?;
            let key_with_hash = russh::keys::PrivateKeyWithHashAlg::new(Arc::new(private_key), None);
            handle
                .authenticate_publickey(&config.username, key_with_hash)
                .await
                .context("SSH auth failed")?;
        } else if let Some(ref password) = config.password {
            handle
                .authenticate_password(&config.username, password)
                .await
                .context("SSH auth failed")?;
        } else {
            handle
                .authenticate_none(&config.username)
                .await
                .context("SSH auth failed")?;
        }

        Self::connect(handle).await
    }
}

#[async_trait]
impl FileConnector for SftpConnector {
    async fn list(&mut self, path: &str) -> Result<Vec<FileEntry>> {
        let mut dir = self.client.read_dir(path).await
            .context("failed to list directory")?;

        let mut entries = Vec::new();
        while let Some(entry) = dir.next().await {
            let entry = entry.context("failed to read dir entry")?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "." || name == ".." {
                continue;
            }
            let metadata = entry.metadata().await.unwrap_or_default();
            let full_path = if path.ends_with('/') {
                format!("{path}{name}")
            } else {
                format!("{path}/{name}")
            };
            entries.push(FileEntry {
                name,
                path: full_path,
                is_dir: metadata.is_dir(),
                size: metadata.len(),
                modified: metadata.modified.map(|t| {
                    t.duration_since(std::time::UNIX_EPOCH)
                        .map(|d| format!("{}", d.as_secs()))
                        .unwrap_or_default()
                }),
                permissions: Some(format!("{:o}", metadata.permissions)),
            });
        }
        entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
        Ok(entries)
    }

    async fn stat(&mut self, path: &str) -> Result<FileEntry> {
        let metadata = self.client.metadata(path).await
            .with_context(|| format!("failed to stat {path}"))?;
        let name = path.rsplit('/').next().unwrap_or(path).to_string();
        Ok(FileEntry {
            name,
            path: path.to_string(),
            is_dir: metadata.is_dir(),
            size: metadata.len(),
            modified: metadata.modified.map(|t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .map(|d| format!("{}", d.as_secs()))
                    .unwrap_or_default()
            }),
            permissions: Some(format!("{:o}", metadata.permissions)),
        })
    }
        &mut self,
        remote_path: &str,
        data: Vec<u8>,
        progress: Option<&ProgressCallback>,
    ) -> Result<()> {
        use tokio::io::AsyncWriteExt;

        let total = data.len() as u64;
        let mut file = self.client.create(remote_path).await
            .with_context(|| format!("failed to create {remote_path}"))?;

        // 分块写入以支持进度回调
        let chunk_size = 64 * 1024; // 64KB
        let mut offset = 0u64;
        for chunk in data.chunks(chunk_size) {
            file.write_all(chunk).await.context("failed to write chunk")?;
            offset += chunk.len() as u64;
            if let Some(ref cb) = progress {
                cb(offset, total);
            }
        }
        file.flush().await.context("failed to flush")?;
        Ok(())
    }

    async fn download(&mut self, path: &str) -> Result<Vec<u8>> {
        let mut file = self.client.open(path).await
            .with_context(|| format!("failed to open {path}"))?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).await
            .with_context(|| format!("failed to read {path}"))?;
        Ok(buf)
    }

    async fn delete(&mut self, path: &str) -> Result<()> {
        // 尝试作为目录删除，失败则作为文件删除
        if self.client.rmdir(path).await.is_err() {
            self.client.remove_file(path).await
                .with_context(|| format!("failed to delete {path}"))?;
        }
        Ok(())
    }

    async fn rename(&mut self, from: &str, to: &str) -> Result<()> {
        self.client.rename(from, to).await
            .with_context(|| format!("failed to rename {from} -> {to}"))?;
        Ok(())
    }

    async fn mkdir(&mut self, path: &str) -> Result<()> {
        self.client.create_dir(path).await
            .with_context(|| format!("failed to mkdir {path}"))?;
        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        // russh-sftp client 在 drop 时自动关闭
        Ok(())
    }
}
