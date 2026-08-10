//! SFTP 文件连接器 — 基于 russh-sftp 实现 FileConnector。

use anyhow::{Context, Result};
use async_trait::async_trait;
use rex_common::file_transfer::{FileConnector, FileEntry, ProgressCallback, UploadResult};
use russh_sftp::client::SftpSession;
use russh_sftp::protocol::OpenFlags;
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
        // IPv6 addresses need brackets: [::1]:22
        // 已有方括号的不再重复添加
        let addr = if config.host.contains(':') && !config.host.starts_with('[') {
            format!("[{}]:{}", config.host, config.port)
        } else {
            format!("{}:{}", config.host, config.port)
        };
        let mut handle = client::connect(ssh_config, &addr, handler)
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
            let modified = meta.modified().ok().and_then(|t| {
                let dur = t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
                chrono::DateTime::from_timestamp(dur.as_secs() as i64, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            });
            entries.push(FileEntry {
                name,
                path: full_path,
                is_dir: meta.is_dir(),
                size: meta.len(),
                modified,
                permissions: None,
                storage_class: None,
                acl: None,
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
                acl: None,
            }),
            Err(_) => Ok(FileEntry {
                name,
                path: path.to_string(),
                is_dir: false,
                size: 0,
                modified: None,
                permissions: None,
                storage_class: None,
                acl: None,
            }),
        }
    }

    async fn upload(
        &mut self,
        remote_path: &str,
        data: Vec<u8>,
        offset: u64,
        progress: Option<&ProgressCallback>,
    ) -> Result<UploadResult> {
        let total = data.len() as u64;

        // Clamp offset to data length to prevent panic
        let offset = offset.min(total);

        // If offset > 0, open existing file for append; otherwise create new
        let mut file = if offset > 0 {
            self.session
                .open_with_flags(remote_path, OpenFlags::WRITE | OpenFlags::APPEND)
                .await
                .with_context(|| format!("failed to open {remote_path} for resume"))?
        } else {
            self.session
                .create(remote_path)
                .await
                .with_context(|| format!("failed to create {remote_path}"))?
        };

        let chunk_size = 64 * 1024;
        let mut written = offset;
        // Skip already-uploaded data
        let start = offset as usize;
        for chunk in data[start..].chunks(chunk_size) {
            file.write_all(chunk)
                .await
                .context("failed to write chunk")?;
            written += chunk.len() as u64;
            if let Some(ref cb) = progress {
                cb(written, total);
            }
        }
        file.flush().await.context("failed to flush")?;
        Ok(UploadResult::default())
    }

    async fn download(&mut self, path: &str) -> Result<Vec<u8>> {
        self.session
            .read(path)
            .await
            .with_context(|| format!("failed to read {path}"))
    }

    async fn download_range(
        &mut self,
        path: &str,
        offset: u64,
        limit: Option<u64>,
    ) -> Result<Vec<u8>> {
        let all_data = self
            .session
            .read(path)
            .await
            .with_context(|| format!("failed to read {path}"))?;
        let start = (offset as usize).min(all_data.len());
        let end = match limit {
            Some(len) => ((offset + len) as usize).min(all_data.len()),
            None => all_data.len(),
        };
        Ok(all_data[start..end].to_vec())
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

    async fn read_for_edit(&mut self, path: &str) -> Result<Vec<u8>> {
        let data = self
            .session
            .read(path)
            .await
            .with_context(|| format!("failed to read {path}"))?;
        if data.len() > 5 * 1024 * 1024 {
            anyhow::bail!("File too large for editing (>5MB)");
        }
        Ok(data)
    }

    async fn save_from_edit(&mut self, path: &str, data: Vec<u8>) -> Result<()> {
        let mut file = self
            .session
            .create(path)
            .await
            .with_context(|| format!("failed to create {path} for save"))?;
        file.write_all(&data)
            .await
            .context("failed to write data")?;
        file.flush().await.context("failed to flush")?;
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
