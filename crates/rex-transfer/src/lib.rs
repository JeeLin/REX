pub mod executor;
pub mod local;
pub mod sftp;
pub mod task;

use std::path::{Path, PathBuf};

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ── 文件类型 ──────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileType {
    File,
    Directory,
}

// ── 文件条目 ──────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub file_type: FileType,
    pub size: Option<u64>,
}

// ── 读取结果 ──────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileRead {
    pub entry: FileEntry,
    pub bytes: Vec<u8>,
}

// ── 写入请求 ──────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileWrite {
    pub path: PathBuf,
    pub bytes: Vec<u8>,
}

// ── FileConnector trait ────────────────────────────────

#[async_trait]
pub trait FileConnector: Send + Sync {
    /// connector 名称，用于展示
    fn connector_name(&self) -> &'static str;

    /// 列出目录内容
    async fn list(&self, path: &Path) -> Result<Vec<FileEntry>>;

    /// 获取文件/目录元数据
    async fn metadata(&self, path: &Path) -> Result<FileEntry>;

    /// 读取文件内容
    async fn read(&self, path: &Path) -> Result<FileRead>;

    /// 写入文件内容
    async fn write(&self, path: &Path, bytes: &[u8]) -> Result<()>;

    /// 创建目录
    async fn mkdir(&self, path: &Path) -> Result<()>;

    /// 删除文件或目录
    async fn delete(&self, path: &Path) -> Result<()>;

    /// 重命名/移动
    async fn rename(&self, from: &Path, to: &Path) -> Result<()>;
}

// ── Tests ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_type_serializes_as_expected() {
        let file = FileType::File;
        let dir = FileType::Directory;
        assert_eq!(serde_json::to_string(&file).unwrap(), "\"file\"");
        assert_eq!(serde_json::to_string(&dir).unwrap(), "\"directory\"");
    }

    #[test]
    fn file_entry_roundtrips_json() {
        let entry = FileEntry {
            name: "test.txt".to_string(),
            path: PathBuf::from("/home/user/test.txt"),
            file_type: FileType::File,
            size: Some(1024),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: FileEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry, parsed);
    }

    #[test]
    fn file_read_carries_entry_and_bytes() {
        let entry = FileEntry {
            name: "data.bin".to_string(),
            path: PathBuf::from("/data.bin"),
            file_type: FileType::File,
            size: Some(4),
        };
        let read = FileRead {
            entry: entry.clone(),
            bytes: vec![0x01, 0x02, 0x03, 0x04],
        };
        assert_eq!(read.entry, entry);
        assert_eq!(read.bytes.len(), 4);
    }

    #[test]
    fn file_write_carries_path_and_bytes() {
        let write = FileWrite {
            path: PathBuf::from("/output/result.txt"),
            bytes: b"hello".to_vec(),
        };
        assert_eq!(write.path, PathBuf::from("/output/result.txt"));
        assert_eq!(write.bytes, b"hello");
    }

    #[test]
    fn file_connector_trait_is_object_safe() {
        // 编译时验证 trait 可以作为 trait object 使用
        fn _assert_object_safe(_: &dyn FileConnector) {}
    }
}
