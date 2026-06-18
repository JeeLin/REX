use std::path::Path;

use anyhow::{Context, Result};
use async_trait::async_trait;
use rex_ssh::auth::AuthMethod;
use rex_ssh::client::SshClient;
use russh_sftp::client::SftpSession;
use tracing::info;

use crate::{FileConnector, FileEntry, FileRead, FileType};

// ── SftpConnector ────────────────────────────────────────

/// SFTP 文件连接器，通过 SSH SFTP 子系统操作远程文件
pub struct SftpConnector {
    session: SftpSession,
    _client: SshClient,
}

impl SftpConnector {
    /// 连接到远程 SSH 服务器并打开 SFTP 会话
    pub async fn connect(host: &str, port: u16, username: &str, auth: AuthMethod) -> Result<Self> {
        let mut client = SshClient::connect(host, port, username, auth)
            .await
            .context("SSH connection failed")?;

        let stream = client
            .open_sftp_channel()
            .await
            .context("failed to open SFTP channel")?;

        let session = SftpSession::new(stream)
            .await
            .context("SFTP session init failed")?;

        info!(host = %host, port = port, "SFTP session established");

        Ok(Self {
            session,
            _client: client,
        })
    }
}

// ── 辅助函数 ──────────────────────────────────────────────

/// 将 russh-sftp FileType 转换为我们的 FileType
fn convert_file_type(attrs: &russh_sftp::client::fs::Metadata) -> FileType {
    if attrs.is_dir() {
        FileType::Directory
    } else {
        FileType::File
    }
}

/// 从文件名和 Metadata 构建 FileEntry
fn build_entry(name: &str, parent: &str, attrs: &russh_sftp::client::fs::Metadata) -> FileEntry {
    let path = if parent.is_empty() {
        name.to_string()
    } else if parent.ends_with('/') {
        format!("{}{}", parent, name)
    } else {
        format!("{}/{}", parent, name)
    };

    FileEntry {
        name: name.to_string(),
        path: path.into(),
        file_type: convert_file_type(attrs),
        size: Some(attrs.len()),
    }
}

// ── FileConnector 实现 ──────────────────────────────────

#[async_trait]
impl FileConnector for SftpConnector {
    fn connector_name(&self) -> &'static str {
        "sftp"
    }

    async fn list(&self, path: &Path) -> Result<Vec<FileEntry>> {
        let path_str = path.to_string_lossy();
        let read_dir = self
            .session
            .read_dir(path_str.as_ref())
            .await
            .context("SFTP read_dir failed")?;

        let entries = read_dir
            .map(|entry| {
                let attrs = entry.metadata();
                build_entry(&entry.file_name(), &path_str, &attrs)
            })
            .collect();

        Ok(entries)
    }

    async fn metadata(&self, path: &Path) -> Result<FileEntry> {
        let path_str = path.to_string_lossy();
        let attrs = self
            .session
            .metadata(path_str.as_ref())
            .await
            .context("SFTP metadata failed")?;

        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let parent = path
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        Ok(build_entry(&name, &parent, &attrs))
    }

    async fn read(&self, path: &Path) -> Result<FileRead> {
        let path_str = path.to_string_lossy();
        let bytes = self
            .session
            .read(path_str.as_ref())
            .await
            .context("SFTP read failed")?;

        let entry = self.metadata(path).await?;

        Ok(FileRead { entry, bytes })
    }

    async fn write(&self, path: &Path, bytes: &[u8]) -> Result<()> {
        let path_str = path.to_string_lossy();
        self.session
            .write(path_str.as_ref(), bytes)
            .await
            .context("SFTP write failed")?;

        Ok(())
    }
}

// ── Tests ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn convert_file_type_directory() {
        let mut attrs = russh_sftp::client::fs::Metadata::default();
        attrs.set_dir(true);
        assert_eq!(convert_file_type(&attrs), FileType::Directory);
    }

    #[test]
    fn convert_file_type_regular() {
        let mut attrs = russh_sftp::client::fs::Metadata::default();
        // default permissions 包含 DIR bits，需要显式设置为普通文件
        attrs.set_regular(true);
        attrs.set_dir(false);
        assert_eq!(convert_file_type(&attrs), FileType::File);
    }

    #[test]
    fn build_entry_root_level() {
        let mut attrs = russh_sftp::client::fs::Metadata::default();
        attrs.set_regular(true);
        attrs.set_dir(false);
        attrs.size = Some(1024);
        let entry = build_entry("test.txt", "", &attrs);
        assert_eq!(entry.name, "test.txt");
        assert_eq!(entry.path, PathBuf::from("test.txt"));
        assert_eq!(entry.size, Some(1024));
        assert_eq!(entry.file_type, FileType::File);
    }

    #[test]
    fn build_entry_nested() {
        let mut attrs = russh_sftp::client::fs::Metadata::default();
        attrs.set_dir(true);
        attrs.size = Some(0);
        let entry = build_entry("subdir", "/home/user", &attrs);
        assert_eq!(entry.name, "subdir");
        assert_eq!(entry.path, PathBuf::from("/home/user/subdir"));
        assert_eq!(entry.file_type, FileType::Directory);
    }

    #[test]
    fn build_entry_nested_no_trailing_slash() {
        let mut attrs = russh_sftp::client::fs::Metadata::default();
        attrs.set_regular(true);
        attrs.set_dir(false);
        attrs.size = Some(512);
        let entry = build_entry("file.rs", "/src", &attrs);
        assert_eq!(entry.path, PathBuf::from("/src/file.rs"));
    }
}
