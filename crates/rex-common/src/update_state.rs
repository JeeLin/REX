/// 更新状态模型 + 原子读写
use serde::{Deserialize, Serialize};
use std::path::Path;

/// 更新阶段
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum UpdatePhase {
    /// 无更新，正常运行
    Idle,
    /// worker 已下载新版本并写入状态，请求 supervisor 重启
    Requested,
    /// supervisor 正在替换二进制并启动新版 worker
    StartingNew,
    /// 新版本健康检查通过，更新完成，删除旧备份
    Committed,
    /// 新版本健康检查失败，supervisor 正在恢复旧版
    RollingBack,
    /// 回滚完成，旧版 worker 正在运行
    RolledBack,
    /// 连续 3 次启动失败，更新终止，保留旧版
    Failed,
}

/// 更新状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateState {
    pub phase: UpdatePhase,
    pub target_version: String,
    pub old_version: String,
    pub staged_path: String,
    pub rollback_path: String,
    pub attempt: u32,
}

impl Default for UpdateState {
    fn default() -> Self {
        Self {
            phase: UpdatePhase::Idle,
            target_version: String::new(),
            old_version: String::new(),
            staged_path: String::new(),
            rollback_path: String::new(),
            attempt: 0,
        }
    }
}

impl UpdateState {
    /// 读取状态文件，不存在或解析失败返回默认 Idle
    pub fn read(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// 原子写入状态文件（先写 .tmp 再 fsync 再 rename）
    pub fn write(&self, path: &Path) -> anyhow::Result<()> {
        let tmp_path = path.with_extension("json.tmp");
        let json = serde_json::to_string_pretty(self)?;

        // 写入临时文件
        std::fs::write(&tmp_path, &json)?;

        // fsync 确保数据落盘
        let file = std::fs::File::open(&tmp_path)?;
        file.sync_all()?;

        // 原子 rename
        std::fs::rename(&tmp_path, path)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn default_state_is_idle() {
        let state = UpdateState::default();
        assert_eq!(state.phase, UpdatePhase::Idle);
        assert!(state.target_version.is_empty());
        assert_eq!(state.attempt, 0);
    }

    #[test]
    fn write_and_read_roundtrip() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().with_extension("json");

        let state = UpdateState {
            phase: UpdatePhase::Requested,
            target_version: "0.2.0".to_string(),
            old_version: "0.1.0".to_string(),
            staged_path: "/tmp/rex-new".to_string(),
            rollback_path: "/tmp/rex-old".to_string(),
            attempt: 1,
        };

        state.write(&path).unwrap();
        let loaded = UpdateState::read(&path);

        assert_eq!(loaded.phase, UpdatePhase::Requested);
        assert_eq!(loaded.target_version, "0.2.0");
        assert_eq!(loaded.old_version, "0.1.0");
        assert_eq!(loaded.attempt, 1);
    }

    #[test]
    fn read_missing_file_returns_default() {
        let path = Path::new("/tmp/nonexistent_update_state_test.json");
        let state = UpdateState::read(path);
        assert_eq!(state.phase, UpdatePhase::Idle);
    }

    #[test]
    fn read_corrupt_file_returns_default() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().with_extension("json");
        std::fs::write(&path, "not valid json {{{").unwrap();

        let state = UpdateState::read(&path);
        assert_eq!(state.phase, UpdatePhase::Idle);
    }
}
