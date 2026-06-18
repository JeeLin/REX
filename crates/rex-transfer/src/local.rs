use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use async_trait::async_trait;
use tracing::info;

use crate::{FileConnector, FileEntry, FileRead, FileType};

// ── LocalConnector ──────────────────────────────────────

/// 本地文件系统连接器，以 base_path 为根目录操作文件
pub struct LocalConnector {
    base_path: PathBuf,
}

impl LocalConnector {
    /// 创建本地文件连接器，验证 base_path 存在且是目录
    pub fn new(base_path: PathBuf) -> Result<Self> {
        if !base_path.exists() {
            anyhow::bail!("base_path does not exist: {}", base_path.display());
        }
        if !base_path.is_dir() {
            anyhow::bail!("base_path is not a directory: {}", base_path.display());
        }

        let canonical = base_path.canonicalize().with_context(|| {
            format!("failed to canonicalize base_path: {}", base_path.display())
        })?;

        info!(path = %canonical.display(), "local connector created");

        Ok(Self {
            base_path: canonical,
        })
    }

    /// 将相对路径解析为绝对路径，并验证不逃逸 base_path
    fn resolve(&self, path: &Path) -> Result<PathBuf> {
        let resolved = if path.is_absolute() {
            self.base_path.join(path.strip_prefix("/").unwrap_or(path))
        } else {
            self.base_path.join(path)
        };

        // canonicalize() 只在路径存在时有效；对不存在的路径验证其父目录
        let canonical = if resolved.exists() {
            resolved
                .canonicalize()
                .with_context(|| format!("failed to canonicalize: {}", resolved.display()))?
        } else {
            // 文件不存在时，验证父目录可达且在 base_path 内
            let parent = resolved.parent().context("path has no parent directory")?;
            if parent.exists() {
                let parent_canonical = parent.canonicalize().with_context(|| {
                    format!("failed to canonicalize parent: {}", parent.display())
                })?;
                parent_canonical.join(resolved.file_name().context("path has no file name")?)
            } else {
                anyhow::bail!("parent directory does not exist: {}", parent.display());
            }
        };

        // 安全检查：确保解析后的路径在 base_path 下
        if !canonical.starts_with(&self.base_path) {
            anyhow::bail!(
                "path escapes base directory: {} is outside {}",
                canonical.display(),
                self.base_path.display()
            );
        }

        Ok(canonical)
    }
}

// ── FileConnector 实现 ──────────────────────────────────

#[async_trait]
impl FileConnector for LocalConnector {
    fn connector_name(&self) -> &'static str {
        "local"
    }

    async fn list(&self, path: &Path) -> Result<Vec<FileEntry>> {
        let resolved = self.resolve(path)?;

        let mut entries = Vec::new();
        let mut dir = tokio::fs::read_dir(&resolved)
            .await
            .with_context(|| format!("failed to read directory: {}", resolved.display()))?;

        while let Some(entry) = dir
            .next_entry()
            .await
            .with_context(|| format!("failed to read directory entry: {}", resolved.display()))?
        {
            let metadata = entry
                .metadata()
                .await
                .with_context(|| format!("failed to read metadata: {}", entry.path().display()))?;

            let name = entry.file_name().to_string_lossy().to_string();
            let rel_path = path.join(&name);

            entries.push(FileEntry {
                name,
                path: rel_path,
                file_type: if metadata.is_dir() {
                    FileType::Directory
                } else {
                    FileType::File
                },
                size: if metadata.is_file() {
                    Some(metadata.len())
                } else {
                    None
                },
            });
        }

        Ok(entries)
    }

    async fn metadata(&self, path: &Path) -> Result<FileEntry> {
        let resolved = self.resolve(path)?;
        let meta = tokio::fs::metadata(&resolved)
            .await
            .with_context(|| format!("failed to read metadata: {}", resolved.display()))?;

        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        Ok(FileEntry {
            name,
            path: path.to_path_buf(),
            file_type: if meta.is_dir() {
                FileType::Directory
            } else {
                FileType::File
            },
            size: if meta.is_file() {
                Some(meta.len())
            } else {
                None
            },
        })
    }

    async fn read(&self, path: &Path) -> Result<FileRead> {
        let resolved = self.resolve(path)?;
        let bytes = tokio::fs::read(&resolved)
            .await
            .with_context(|| format!("failed to read file: {}", resolved.display()))?;

        let entry = self.metadata(path).await?;

        Ok(FileRead { entry, bytes })
    }

    async fn write(&self, path: &Path, bytes: &[u8]) -> Result<()> {
        let resolved = self.resolve(path)?;

        // 确保父目录存在
        if let Some(parent) = resolved.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("failed to create parent dir: {}", parent.display()))?;
        }

        tokio::fs::write(&resolved, bytes)
            .await
            .with_context(|| format!("failed to write file: {}", resolved.display()))?;

        Ok(())
    }
}

// ── Tests ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_connector() -> (TempDir, LocalConnector) {
        let tmp = TempDir::new().unwrap();
        let connector = LocalConnector::new(tmp.path().to_path_buf()).unwrap();
        (tmp, connector)
    }

    #[tokio::test]
    async fn new_validates_directory_exists() {
        let result = LocalConnector::new(PathBuf::from("/nonexistent/path"));
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn list_empty_directory() {
        let (_tmp, connector) = setup_connector();
        let entries = connector.list(Path::new("/")).await.unwrap();
        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn write_and_read_file() {
        let (_tmp, connector) = setup_connector();
        let path = Path::new("/test.txt");
        let data = b"hello world";

        connector.write(path, data).await.unwrap();
        let result = connector.read(path).await.unwrap();

        assert_eq!(result.bytes, data);
        assert_eq!(result.entry.name, "test.txt");
        assert_eq!(result.entry.size, Some(11));
    }

    #[tokio::test]
    async fn list_with_files() {
        let (_tmp, connector) = setup_connector();

        connector.write(Path::new("/a.txt"), b"aaa").await.unwrap();
        connector.write(Path::new("/b.txt"), b"bb").await.unwrap();

        let entries = connector.list(Path::new("/")).await.unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[tokio::test]
    async fn metadata_returns_file_type() {
        let (_tmp, connector) = setup_connector();
        connector
            .write(Path::new("/file.txt"), b"data")
            .await
            .unwrap();

        let entry = connector.metadata(Path::new("/file.txt")).await.unwrap();
        assert_eq!(entry.file_type, FileType::File);
        assert_eq!(entry.size, Some(4));
    }

    #[tokio::test]
    async fn path_traversal_blocked() {
        let (_tmp, connector) = setup_connector();
        let result = connector.resolve(Path::new("/../../../etc/passwd"));
        assert!(result.is_err());
    }
}
