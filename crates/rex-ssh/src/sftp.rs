//! SFTP 文件连接器 — 基于 russh-sftp 实现 FileConnector。

use anyhow::{Context, Result};
use async_trait::async_trait;
use rex_common::file_transfer::{FileConnector, FileEntry, ProgressCallback};
use russh_sftp::client::SftpSession;
use tokio::io::AsyncWriteExt;

/// SFTP 连接器
pub struct SftpConnector {
    session: SftpSession,
}

impl SftpConnector {
    /// 从 SSH channel 建立 SFTP 连接
    pub async fn connect(channel: russh::Channel<russh::client::Msg>) -> Result<Self> {
        let session = SftpSession::new(channel.into_stream())
            .await
            .context("failed to create SFTP session")?;
        Ok(Self { session })
    }

    /// 从 SSH 配置直接建立 SFTP 连接
    pub async fn connect_with_config(config: crate::SshConfig) -> Result<Self> {
        use russh::client;
        use std::sync::Arc;

        let ssh_config = Arc::new(client::Config::default());
        let handler = crate::SshHandler;
        let mut handle = client::connect(
            ssh_config,
            &format!("{}:{}", config.host, config.port),
            handler,
        )
        .await
        .context("SSH connection failed for SFTP")?;

        if let Some(ref key_pem) = config.private_key {
            let private_key = russh::keys::decode_secret_key(key_pem, config.password.as_deref())
                .context("failed to decode private key")?;
            let key_with_hash =
                russh::keys::PrivateKeyWithHashAlg::new(Arc::new(private_key), None);
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

        let channel = handle
            .channel_open_session()
            .await
            .context("failed to open session")?;

        Self::connect(channel).await
    }
}

#[async_trait]
impl FileConnector for SftpConnector {
    async fn list(&mut self, path: &str) -> Result<Vec<FileEntry>> {
        let dir = self
            .session
            .read_dir(path)
            .await
            .context("failed to list directory")?;

        let mut entries = Vec::new();
        for entry in dir {
            let name = entry.file_name();
            if name == "." || name == ".." {
                continue;
            }
            let meta = entry.metadata();
            let full_path = if path.ends_with('/') {
                format!("{path}{name}")
            } else {
                format!("{path}/{name}")
            };
            entries.push(FileEntry {
                name,
                path: full_path,
                is_dir: meta.is_dir(),
                size: meta.len(),
                modified: None,
                permissions: None,
                storage_class: None,
            });
        }
        entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
        Ok(entries)
    }

    async fn stat(&mut self, path: &str) -> Result<FileEntry> {
        let name = path.rsplit('/').next().unwrap_or(path).to_string();
        // 简化：尝试读取路径，成功则为文件
        match self.session.canonicalize(path).await {
            Ok(resolved) => Ok(FileEntry {
                name,
                path: resolved,
                is_dir: false,
                size: 0,
                modified: None,
                permissions: None,
                storage_class: None,
            }),
            Err(_) => Ok(FileEntry {
                name,
                path: path.to_string(),
                is_dir: false,
                size: 0,
                modified: None,
                permissions: None,
                storage_class: None,
            }),
        }
    }

    async fn upload(
        &mut self,
        remote_path: &str,
        data: Vec<u8>,
        progress: Option<&ProgressCallback>,
    ) -> Result<()> {
        let total = data.len() as u64;
        let mut file = self
            .session
            .create(remote_path)
            .await
            .with_context(|| format!("failed to create {remote_path}"))?;

        let chunk_size = 64 * 1024;
        let mut offset = 0u64;
        for chunk in data.chunks(chunk_size) {
            file.write_all(chunk)
                .await
                .context("failed to write chunk")?;
            offset += chunk.len() as u64;
            if let Some(ref cb) = progress {
                cb(offset, total);
            }
        }
        file.flush().await.context("failed to flush")?;
        Ok(())
    }

    async fn download(&mut self, path: &str) -> Result<Vec<u8>> {
        self.session
            .read(path)
            .await
            .with_context(|| format!("failed to read {path}"))
    }

    async fn delete(&mut self, path: &str) -> Result<()> {
        if self.session.remove_file(path).await.is_err() {
            self.session
                .remove_dir(path)
                .await
                .with_context(|| format!("failed to delete {path}"))?;
        }
        Ok(())
    }

    async fn rename(&mut self, from: &str, to: &str) -> Result<()> {
        self.session
            .rename(from, to)
            .await
            .with_context(|| format!("failed to rename {from} -> {to}"))?;
        Ok(())
    }

    async fn mkdir(&mut self, path: &str) -> Result<()> {
        self.session
            .create_dir(path)
            .await
            .with_context(|| format!("failed to mkdir {path}"))?;
        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        self.session.close().await.ok();
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
