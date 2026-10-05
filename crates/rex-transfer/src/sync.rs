//! 目录同步数据模型 — v0.92.0 (`docs/PRODUCT.md` §3.8 文件夹同步五要素)。
//!
//! 本模块只承载**数据模型与枚举**（方向 / 比较依据 / 掩码 / 孤儿策略 / 计划动作），
//! 供 rex-hub 持久化 + REST 序列化 + 前端类型共享；diff 纯函数与引擎执行在
//! 任务 2 落地（`diff` / 掩码匹配 / apply）。

use serde::{Deserialize, Serialize};

/// 同步方向：源端（右键选中的目录）→ 目标端（对面面板）为 `Upload`。
///
/// 与 Xftp 双面板语义对齐：`Upload` = 源 → 目标，`Download` = 目标 → 源，
/// `Bidirectional` = 双向（较新者为准）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncDirection {
    #[default]
    Upload,
    Download,
    Bidirectional,
}

impl SyncDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Upload => "upload",
            Self::Download => "download",
            Self::Bidirectional => "bidirectional",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "upload" => Self::Upload,
            "download" => Self::Download,
            "bidirectional" => Self::Bidirectional,
            _ => return None,
        })
    }
}

impl std::fmt::Display for SyncDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 比较依据：`Size` 仅比大小，`ModifiedTime` 大小 + 修改时间。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompareBasis {
    Size,
    #[default]
    #[serde(rename = "modified_time")]
    ModifiedTime,
}

impl CompareBasis {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Size => "size",
            Self::ModifiedTime => "modified_time",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "size" => Self::Size,
            "modified_time" => Self::ModifiedTime,
            _ => return None,
        })
    }
}

/// 同步任务选项（创建 / 预览共用的请求参数）。
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SyncOptions {
    #[serde(default)]
    pub direction: SyncDirection,
    #[serde(default)]
    pub compare: CompareBasis,
    /// 包含掩码（glob，空 = 全部）。
    #[serde(default)]
    pub include: Vec<String>,
    /// 排除掩码（glob，优先级高于 include）。
    #[serde(default)]
    pub exclude: Vec<String>,
    /// 单向同步时删除目标侧（`Download` 为源侧）多余项。
    #[serde(default)]
    pub delete_orphans: bool,
}

/// 单条同步动作类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncActionKind {
    /// 目标缺失 / 单向更新：复制。
    Copy,
    /// 孤儿删除（`delete_orphans`）。
    Delete,
    /// 双向冲突：两侧均已修改，按「较新者为准」处理。
    Conflict,
}

/// 动作作用方向：`ToTarget` = 源 → 目标，`ToSource` = 目标 → 源。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncActionDir {
    ToTarget,
    ToSource,
}

/// 计划中的单条动作（`POST /api/files/sync/preview` 直接返回给前端渲染）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncAction {
    /// 相对同步根的路径（不含根前缀，正斜杠分隔）。
    pub rel_path: String,
    pub action: SyncActionKind,
    pub dir: SyncActionDir,
    /// 文件大小（删除动作为目标侧文件大小）。
    pub size: u64,
    /// 源侧修改时间（Unix 秒）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_mtime: Option<i64>,
    /// 目标侧修改时间（Unix 秒）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_mtime: Option<i64>,
}

/// 计划汇总（前端预览徽标：N 复制 · M 删除 · K 冲突）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SyncSummary {
    pub copies: u64,
    pub deletes: u64,
    pub conflicts: u64,
    /// 待复制字节总量（不含删除项），驱动同步进度条。
    pub total_bytes: u64,
}

/// 一次同步计划：动作清单 + 汇总。空计划 = 两侧已是最新。
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SyncPlan {
    pub actions: Vec<SyncAction>,
    pub summary: SyncSummary,
}

impl SyncPlan {
    /// 由动作清单重建汇总（保证 summary 与 actions 一致）。
    pub fn from_actions(mut actions: Vec<SyncAction>) -> Self {
        actions.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
        let mut summary = SyncSummary::default();
        for a in &actions {
            match a.action {
                SyncActionKind::Copy => {
                    summary.copies += 1;
                    summary.total_bytes += a.size;
                }
                SyncActionKind::Delete => summary.deletes += 1,
                SyncActionKind::Conflict => {
                    summary.conflicts += 1;
                    summary.total_bytes += a.size;
                }
            }
        }
        Self { actions, summary }
    }
}

/// diff 的输入条目：一侧目录树中的单个文件（目录不进计划，落盘时按需 mkdir）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncEntry {
    /// 相对同步根的路径。
    pub rel_path: String,
    pub size: u64,
    /// 修改时间（Unix 秒），无法解析时为 `None`。
    pub mtime: Option<i64>,
}

impl SyncEntry {
    pub fn new(rel_path: impl Into<String>, size: u64, mtime: Option<i64>) -> Self {
        Self {
            rel_path: rel_path.into(),
            size,
            mtime,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direction_round_trip() {
        assert_eq!(SyncDirection::default(), SyncDirection::Upload);
        for d in [
            SyncDirection::Upload,
            SyncDirection::Download,
            SyncDirection::Bidirectional,
        ] {
            let json = serde_json::to_string(&d).unwrap();
            let back: SyncDirection = serde_json::from_str(&json).unwrap();
            assert_eq!(back, d);
            assert_eq!(SyncDirection::from_str(d.as_str()), Some(d));
            assert_eq!(d.to_string(), d.as_str());
        }
        assert_eq!(SyncDirection::from_str("nope"), None);
    }

    #[test]
    fn compare_basis_round_trip() {
        assert_eq!(CompareBasis::default(), CompareBasis::ModifiedTime);
        assert_eq!(
            serde_json::to_string(&CompareBasis::ModifiedTime).unwrap(),
            "\"modified_time\""
        );
        let back: CompareBasis = serde_json::from_str("\"size\"").unwrap();
        assert_eq!(back, CompareBasis::Size);
        assert_eq!(
            CompareBasis::from_str("modified_time"),
            Some(CompareBasis::ModifiedTime)
        );
        assert_eq!(CompareBasis::from_str("hash"), None);
    }

    #[test]
    fn sync_options_defaults_and_serde() {
        let opts = SyncOptions::default();
        assert_eq!(opts.direction, SyncDirection::Upload);
        assert_eq!(opts.compare, CompareBasis::ModifiedTime);
        assert!(opts.include.is_empty());
        assert!(opts.exclude.is_empty());
        assert!(!opts.delete_orphans);

        let json = serde_json::to_string(&opts).unwrap();
        let back: SyncOptions = serde_json::from_str(&json).unwrap();
        assert_eq!(back, opts);

        // 缺省字段按 default 反序列化（前端只传部分字段也合法）
        let partial: SyncOptions = serde_json::from_str(r#"{"delete_orphans":true}"#).unwrap();
        assert_eq!(partial.direction, SyncDirection::Upload);
        assert!(partial.delete_orphans);
    }

    #[test]
    fn sync_plan_from_actions_sorts_and_summarizes() {
        let actions = vec![
            SyncAction {
                rel_path: "b.txt".into(),
                action: SyncActionKind::Delete,
                dir: SyncActionDir::ToTarget,
                size: 10,
                source_mtime: None,
                target_mtime: Some(7),
            },
            SyncAction {
                rel_path: "a.txt".into(),
                action: SyncActionKind::Copy,
                dir: SyncActionDir::ToTarget,
                size: 100,
                source_mtime: Some(1),
                target_mtime: None,
            },
            SyncAction {
                rel_path: "c.txt".into(),
                action: SyncActionKind::Conflict,
                dir: SyncActionDir::ToSource,
                size: 5,
                source_mtime: Some(2),
                target_mtime: Some(3),
            },
        ];
        let plan = SyncPlan::from_actions(actions);
        let paths: Vec<&str> = plan.actions.iter().map(|a| a.rel_path.as_str()).collect();
        assert_eq!(paths, vec!["a.txt", "b.txt", "c.txt"]);
        assert_eq!(plan.summary.copies, 1);
        assert_eq!(plan.summary.deletes, 1);
        assert_eq!(plan.summary.conflicts, 1);
        assert_eq!(plan.summary.total_bytes, 105);
    }

    #[test]
    fn sync_action_kind_round_trip() {
        for k in [
            SyncActionKind::Copy,
            SyncActionKind::Delete,
            SyncActionKind::Conflict,
        ] {
            let json = serde_json::to_string(&k).unwrap();
            let back: SyncActionKind = serde_json::from_str(&json).unwrap();
            assert_eq!(back, k);
        }
        assert!(serde_json::from_str::<SyncActionKind>("\"nope\"").is_err());
    }

    #[test]
    fn sync_action_serde_uses_snake_case_dir() {
        let a = SyncAction {
            rel_path: "x".into(),
            action: SyncActionKind::Conflict,
            dir: SyncActionDir::ToSource,
            size: 3,
            source_mtime: None,
            target_mtime: None,
        };
        let json = serde_json::to_string(&a).unwrap();
        assert!(json.contains("\"dir\":\"to_source\""));
        let back: SyncAction = serde_json::from_str(&json).unwrap();
        assert_eq!(back, a);
    }
}
