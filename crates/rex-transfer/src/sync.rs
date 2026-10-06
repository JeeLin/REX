//! 目录同步数据模型 + diff 纯函数 — v0.92.0
//! (`docs/PRODUCT.md` §3.8 文件夹同步五要素)。
//!
//! 本模块承载**数据模型与枚举**（方向 / 比较依据 / 掩码 / 孤儿策略 / 计划动作），
//! 供 rex-hub 持久化 + REST 序列化 + 前端类型共享；同时承载**纯函数** [`diff`]、
//! 掩码匹配与比较依据判定（[`needs_copy`]）。引擎的 apply（递归扫描 / 直连搬运 /
//! 删除）落在 `rex-hub::sync_coordinator`，数据不经过浏览器。

use std::collections::BTreeMap;

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

// ---------------------------------------------------------------------------
// 掩码匹配（include / exclude glob）
// ---------------------------------------------------------------------------

/// glob 段匹配：`*` 匹配任意非 `/` 序列，`?` 匹配单个非 `/` 字符。
fn glob_segment_match(pattern: &str, segment: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let s: Vec<char> = segment.chars().collect();
    // dp[i][j]：pattern[..i] 是否匹配 segment[..j]
    let mut dp = vec![vec![false; s.len() + 1]; p.len() + 1];
    dp[0][0] = true;
    for i in 1..=p.len() {
        if p[i - 1] == '*' {
            dp[i][0] = dp[i - 1][0];
        }
    }
    for i in 1..=p.len() {
        for j in 1..=s.len() {
            dp[i][j] = match p[i - 1] {
                '*' => dp[i - 1][j] || dp[i][j - 1],
                '?' => dp[i - 1][j - 1],
                c => dp[i - 1][j - 1] && c == s[j - 1],
            };
        }
    }
    dp[p.len()][s.len()]
}

/// 路径级 glob：段序列匹配，**`**` 段跨任意层级**（含 0 层）。
fn glob_path_match(pattern: &str, path: &str) -> bool {
    let pat: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
    let seg: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    // dp[i][j]：pattern[..i] 是否匹配 path 前 j 段
    let mut dp = vec![vec![false; seg.len() + 1]; pat.len() + 1];
    dp[0][0] = true;
    for i in 1..=pat.len() {
        if pat[i - 1] == "**" {
            // `**` 匹配 0..=j 段
            for j in 0..=seg.len() {
                dp[i][j] = dp[i - 1][j];
                if j > 0 {
                    dp[i][j] = dp[i][j] || dp[i][j - 1];
                }
            }
        } else {
            for j in 1..=seg.len() {
                dp[i][j] = dp[i - 1][j - 1] && glob_segment_match(pat[i - 1], seg[j - 1]);
            }
        }
    }
    dp[pat.len()][seg.len()]
}

/// 单个掩码是否命中 `rel_path`。
///
/// 语义（对齐 Xftp / rsync 掩码习惯）：
/// - 空掩码忽略；
/// - 无 `/` 的掩码匹配**任意层级**的文件名（`*.log` 命中 `a/b/c.log`）；
/// - 目录掩码命中其下全部内容（`node_modules` 命中 `node_modules/x/y.js`）。
pub fn mask_matches(rel_path: &str, mask: &str) -> bool {
    let mask = mask.trim().trim_matches('/');
    if mask.is_empty() || rel_path.is_empty() {
        return false;
    }
    if glob_path_match(mask, rel_path) {
        return true;
    }
    // 无 `/` 的掩码：先试整条路径，再逐段试（`node_modules`、`*.log`）
    if !mask.contains('/') && rel_path.split('/').any(|seg| glob_segment_match(mask, seg)) {
        return true;
    }
    // 目录掩码：命中任一祖先目录即命中其下内容
    let mut prefix = rel_path;
    while let Some(idx) = prefix.rfind('/') {
        prefix = &prefix[..idx];
        if glob_path_match(mask, prefix) {
            return true;
        }
    }
    false
}

/// 掩码过滤：exclude 优先于 include；`include` 为空 = 全选。
pub fn is_included(rel_path: &str, opts: &SyncOptions) -> bool {
    if opts.exclude.iter().any(|m| mask_matches(rel_path, m)) {
        return false;
    }
    if opts.include.is_empty() {
        return true;
    }
    opts.include.iter().any(|m| mask_matches(rel_path, m))
}

// ---------------------------------------------------------------------------
// 比较依据
// ---------------------------------------------------------------------------

/// 目标侧是否需要被源侧内容覆盖（源缺失除外，那由 diff 单独处理）。
///
/// - [`CompareBasis::Size`]：仅比大小；
/// - [`CompareBasis::ModifiedTime`]：大小或修改时间不同即需复制；任一侧时间
///   未知（部分 SFTP/S3 列表不返回可解析时间）时退化为仅比大小。
pub fn needs_copy(source: &SyncEntry, target: &SyncEntry, basis: CompareBasis) -> bool {
    if source.size != target.size {
        return true;
    }
    match basis {
        CompareBasis::Size => false,
        CompareBasis::ModifiedTime => match (source.mtime, target.mtime) {
            (Some(s), Some(t)) => s != t,
            _ => false,
        },
    }
}

/// 双向同步冲突时哪一侧较新（`ToTarget` = 源较新，`ToSource` = 目标较新）。
///
/// 时间为未知或相等时以源侧为准（源是右键发起的选中项）。
pub fn newer_side(source: &SyncEntry, target: &SyncEntry) -> SyncActionDir {
    match (source.mtime, target.mtime) {
        (Some(s), Some(t)) if t > s => SyncActionDir::ToSource,
        _ => SyncActionDir::ToTarget,
    }
}

// ---------------------------------------------------------------------------
// diff 纯函数
// ---------------------------------------------------------------------------

/// compare → diff：对比两侧文件清单生成 [`SyncPlan`]。
///
/// 掩码在两侧同时生效——被掩码排除的条目在两侧都不可见，因此既不会被复制，
/// 也不会被判为孤儿而删除。
///
/// - [`SyncDirection::Upload`]：源为准 → 目标；目标侧多余项为孤儿（删除落目标侧）；
/// - [`SyncDirection::Download`]：目标为准 → 源；源侧多余项为孤儿（删除落源侧）；
/// - [`SyncDirection::Bidirectional`]：两侧都保留 → 无孤儿删除；仅单侧存在时复制
///   到对侧，两侧都存在且按比较依据不同则标记 [`SyncActionKind::Conflict`]，
///   方向取「较新者为准」。
///
/// 目录不进计划（仅文件进计划），落盘时由 apply 按需 mkdir。
pub fn diff(source: &[SyncEntry], target: &[SyncEntry], opts: &SyncOptions) -> SyncPlan {
    let src: BTreeMap<&str, &SyncEntry> = source
        .iter()
        .filter(|e| is_included(&e.rel_path, opts))
        .map(|e| (e.rel_path.as_str(), e))
        .collect();
    let tgt: BTreeMap<&str, &SyncEntry> = target
        .iter()
        .filter(|e| is_included(&e.rel_path, opts))
        .map(|e| (e.rel_path.as_str(), e))
        .collect();

    let mut actions: Vec<SyncAction> = Vec::new();

    match opts.direction {
        SyncDirection::Upload => {
            for (path, s) in &src {
                match tgt.get(path) {
                    None => actions.push(copy_action(s, None, SyncActionDir::ToTarget)),
                    Some(t) if needs_copy(s, t, opts.compare) => {
                        actions.push(copy_action(s, Some(t), SyncActionDir::ToTarget))
                    }
                    Some(_) => {}
                }
            }
            if opts.delete_orphans {
                for (path, t) in &tgt {
                    if !src.contains_key(path) {
                        actions.push(delete_action(t, SyncActionDir::ToTarget));
                    }
                }
            }
        }
        SyncDirection::Download => {
            for (path, t) in &tgt {
                match src.get(path) {
                    None => actions.push(copy_action(t, None, SyncActionDir::ToSource)),
                    Some(s) if needs_copy(t, s, opts.compare) => {
                        actions.push(copy_action(t, Some(s), SyncActionDir::ToSource))
                    }
                    Some(_) => {}
                }
            }
            if opts.delete_orphans {
                for (path, s) in &src {
                    if !tgt.contains_key(path) {
                        actions.push(delete_action(s, SyncActionDir::ToSource));
                    }
                }
            }
        }
        SyncDirection::Bidirectional => {
            for (path, s) in &src {
                match tgt.get(path) {
                    None => actions.push(copy_action(s, None, SyncActionDir::ToTarget)),
                    Some(t) if needs_copy(s, t, opts.compare) => {
                        let dir = newer_side(s, t);
                        actions.push(SyncAction {
                            rel_path: (*path).to_string(),
                            action: SyncActionKind::Conflict,
                            dir,
                            size: if dir == SyncActionDir::ToTarget {
                                s.size
                            } else {
                                t.size
                            },
                            source_mtime: s.mtime,
                            target_mtime: t.mtime,
                        })
                    }
                    Some(_) => {}
                }
            }
            for (path, t) in &tgt {
                if !src.contains_key(path) {
                    actions.push(copy_action(t, None, SyncActionDir::ToSource));
                }
            }
        }
    }

    SyncPlan::from_actions(actions)
}

/// 构造复制动作：`from` 为被复制方（提供 size / mtime），`other` 为对侧
/// （缺失时对侧 mtime 为 `None`）。`dir` 指明复制方向。
fn copy_action(from: &SyncEntry, other: Option<&SyncEntry>, dir: SyncActionDir) -> SyncAction {
    let (source_mtime, target_mtime) = match dir {
        SyncActionDir::ToTarget => (from.mtime, other.and_then(|e| e.mtime)),
        SyncActionDir::ToSource => (other.and_then(|e| e.mtime), from.mtime),
    };
    SyncAction {
        rel_path: from.rel_path.clone(),
        action: SyncActionKind::Copy,
        dir,
        size: from.size,
        source_mtime,
        target_mtime,
    }
}

fn delete_action(entry: &SyncEntry, dir: SyncActionDir) -> SyncAction {
    let (source_mtime, target_mtime) = match dir {
        SyncActionDir::ToTarget => (None, entry.mtime),
        SyncActionDir::ToSource => (entry.mtime, None),
    };
    SyncAction {
        rel_path: entry.rel_path.clone(),
        action: SyncActionKind::Delete,
        dir,
        size: entry.size,
        source_mtime,
        target_mtime,
    }
}

/// `rel_path` 的祖先目录（由浅到深），供 apply 按需 mkdir。
pub fn ancestor_dirs(rel_path: &str) -> Vec<String> {
    let mut dirs = Vec::new();
    let mut prefix = rel_path;
    while let Some(idx) = prefix.rfind('/') {
        prefix = &prefix[..idx];
        if prefix.is_empty() {
            break;
        }
        dirs.push(prefix.to_string());
    }
    dirs.reverse();
    dirs
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

    // -------------------------------------------------------------------
    // 掩码匹配
    // -------------------------------------------------------------------

    #[test]
    fn mask_matches_basics() {
        // 无 `/` 掩码匹配任意层级文件名
        assert!(mask_matches("a.log", "*.log"));
        assert!(mask_matches("logs/deep/a.log", "*.log"));
        assert!(!mask_matches("a.txt", "*.log"));
        // `?` 匹配单字符
        assert!(mask_matches("a1.txt", "a?.txt"));
        assert!(!mask_matches("a12.txt", "a?.txt"));
        // 空掩码永不命中
        assert!(!mask_matches("a.txt", ""));
        assert!(!mask_matches("a.txt", "   "));
        assert!(!mask_matches("", "*.txt"));
    }

    #[test]
    fn mask_matches_directory_and_double_star() {
        // 目录掩码命中其下内容
        assert!(mask_matches("node_modules/pkg/index.js", "node_modules"));
        assert!(!mask_matches("src/node_modules_helper.js", "node_modules"));
        // `**` 跨层级
        assert!(mask_matches("src/a/b/c.rs", "src/**/*.rs"));
        assert!(!mask_matches("src/a/c.txt", "src/**/*.rs"));
        // `**` 也匹配 0 层
        assert!(mask_matches("top.rs", "**/*.rs"));
        assert!(mask_matches("a/b/c", "a/**"));
    }

    #[test]
    fn is_included_respects_exclude_priority_and_include_allowlist() {
        let opts = SyncOptions {
            include: vec!["*.rs".into()],
            exclude: vec!["target/**".into()],
            ..Default::default()
        };
        assert!(is_included("src/main.rs", &opts));
        assert!(!is_included("src/main.txt", &opts), "include allowlist");
        assert!(
            !is_included("target/debug/build.rs", &opts),
            "exclude wins over include"
        );

        // include 为空 = 全选（仅受 exclude 约束）
        let only_exclude = SyncOptions {
            exclude: vec!["*.tmp".into()],
            ..Default::default()
        };
        assert!(is_included("a/b.txt", &only_exclude));
        assert!(!is_included("a/b.tmp", &only_exclude));
        assert!(is_included("a/b.txt", &SyncOptions::default()));
    }

    #[test]
    fn ancestor_dirs_returns_shallow_to_deep() {
        assert_eq!(
            ancestor_dirs("a/b/c/d.txt"),
            vec!["a".to_string(), "a/b".to_string(), "a/b/c".to_string()]
        );
        assert!(ancestor_dirs("top.txt").is_empty());
    }

    // -------------------------------------------------------------------
    // needs_copy / newer_side
    // -------------------------------------------------------------------

    #[test]
    fn needs_copy_respects_compare_basis() {
        let src = SyncEntry::new("a", 10, Some(100));
        let same = SyncEntry::new("a", 10, Some(100));
        let diff_time = SyncEntry::new("a", 10, Some(50));
        let diff_size = SyncEntry::new("a", 11, Some(100));

        assert!(!needs_copy(&src, &same, CompareBasis::ModifiedTime));
        assert!(!needs_copy(&src, &same, CompareBasis::Size));
        assert!(needs_copy(&src, &diff_time, CompareBasis::ModifiedTime));
        assert!(!needs_copy(&src, &diff_time, CompareBasis::Size));
        assert!(needs_copy(&src, &diff_size, CompareBasis::Size));

        // 时间未知 → 退化为仅比大小
        let no_time = SyncEntry::new("a", 10, None);
        assert!(!needs_copy(&src, &no_time, CompareBasis::ModifiedTime));
        assert!(needs_copy(
            &SyncEntry::new("a", 12, None),
            &no_time,
            CompareBasis::Size
        ));
    }

    #[test]
    fn newer_side_prefers_newer_mtime_source_wins_ties() {
        let src = SyncEntry::new("a", 1, Some(200));
        let tgt = SyncEntry::new("a", 1, Some(100));
        assert_eq!(newer_side(&src, &tgt), SyncActionDir::ToTarget);
        assert_eq!(newer_side(&tgt, &src), SyncActionDir::ToSource);

        // 时间未知 → 以源侧为准
        let unknown = SyncEntry::new("a", 1, None);
        assert_eq!(newer_side(&src, &unknown), SyncActionDir::ToTarget);
    }

    // -------------------------------------------------------------------
    // diff
    // -------------------------------------------------------------------

    fn entries(list: &[(&str, u64, Option<i64>)]) -> Vec<SyncEntry> {
        list.iter()
            .map(|(p, s, m)| SyncEntry::new(*p, *s, *m))
            .collect()
    }

    fn plan_of(plan: &SyncPlan) -> Vec<(&str, SyncActionKind, SyncActionDir)> {
        plan.actions
            .iter()
            .map(|a| (a.rel_path.as_str(), a.action, a.dir))
            .collect()
    }

    #[test]
    fn diff_upload_copies_new_and_updates_changed() {
        let source = entries(&[
            ("new.txt", 10, Some(1)),
            ("changed.txt", 20, Some(5)),
            ("same.txt", 30, Some(9)),
        ]);
        let target = entries(&[("changed.txt", 20, Some(1)), ("same.txt", 30, Some(9))]);
        let opts = SyncOptions::default();

        let plan = diff(&source, &target, &opts);
        assert_eq!(
            plan_of(&plan),
            vec![
                ("changed.txt", SyncActionKind::Copy, SyncActionDir::ToTarget),
                ("new.txt", SyncActionKind::Copy, SyncActionDir::ToTarget),
            ]
        );
        assert_eq!(plan.summary.copies, 2);
        assert_eq!(plan.summary.total_bytes, 30);
    }

    #[test]
    fn diff_upload_size_basis_ignores_time_only_difference() {
        let source = entries(&[("a.txt", 10, Some(500))]);
        let target = entries(&[("a.txt", 10, Some(1))]);
        let opts = SyncOptions {
            compare: CompareBasis::Size,
            ..Default::default()
        };
        assert!(diff(&source, &target, &opts).actions.is_empty());
    }

    #[test]
    fn diff_delete_orphans_only_when_enabled() {
        let source = entries(&[("keep.txt", 1, None)]);
        let target = entries(&[("keep.txt", 1, None), ("orphan.txt", 5, Some(3))]);

        let no_orphans = SyncOptions::default();
        assert!(diff(&source, &target, &no_orphans).actions.is_empty());

        let with_orphans = SyncOptions {
            delete_orphans: true,
            ..Default::default()
        };
        let plan = diff(&source, &target, &with_orphans);
        assert_eq!(
            plan_of(&plan),
            vec![(
                "orphan.txt",
                SyncActionKind::Delete,
                SyncActionDir::ToTarget
            )]
        );
        assert_eq!(plan.summary.deletes, 1);
        assert_eq!(plan.summary.total_bytes, 0, "deletes carry no bytes");
    }

    #[test]
    fn diff_masks_apply_to_both_sides_so_excluded_files_are_untouched() {
        let source = entries(&[("a.txt", 1, None), ("skip.log", 2, None)]);
        let target = entries(&[("a.txt", 9, Some(1))]);
        let opts = SyncOptions {
            exclude: vec!["*.log".into()],
            ..Default::default()
        };

        let plan = diff(&source, &target, &opts);
        assert_eq!(
            plan_of(&plan),
            vec![("a.txt", SyncActionKind::Copy, SyncActionDir::ToTarget)]
        );
    }

    #[test]
    fn diff_excluded_target_file_is_not_treated_as_orphan() {
        // 目标侧被 exclude 的文件在两侧都不可见 → 不复制也不删除
        let source = entries(&[]);
        let target = entries(&[("vendor/big.js", 100, None)]);
        let opts = SyncOptions {
            exclude: vec!["vendor/**".into()],
            delete_orphans: true,
            ..Default::default()
        };
        let plan = diff(&source, &target, &opts);
        assert!(
            plan.actions.is_empty(),
            "excluded target files must survive orphan deletion"
        );
    }

    #[test]
    fn diff_include_allowlist_filters_copies() {
        let source = entries(&[("a.rs", 1, None), ("b.txt", 2, None)]);
        let target = entries(&[]);
        let opts = SyncOptions {
            include: vec!["*.rs".into()],
            ..Default::default()
        };
        let plan = diff(&source, &target, &opts);
        assert_eq!(
            plan_of(&plan),
            vec![("a.rs", SyncActionKind::Copy, SyncActionDir::ToTarget)]
        );
    }

    #[test]
    fn diff_download_mirrors_direction_and_orphan_side() {
        let source = entries(&[("old.txt", 1, Some(1))]);
        let target = entries(&[("new.txt", 2, Some(1)), ("old.txt", 1, Some(9))]);
        let opts = SyncOptions {
            direction: SyncDirection::Download,
            delete_orphans: true,
            ..Default::default()
        };

        let plan = diff(&source, &target, &opts);
        assert_eq!(
            plan_of(&plan),
            vec![
                ("new.txt", SyncActionKind::Copy, SyncActionDir::ToSource),
                ("old.txt", SyncActionKind::Copy, SyncActionDir::ToSource),
            ]
        );
        assert_eq!(plan.summary.copies, 2);
    }

    #[test]
    fn diff_bidirectional_marks_conflicts_and_never_deletes() {
        let source = entries(&[
            ("only-src.txt", 3, Some(10)),
            ("src-newer.txt", 4, Some(100)),
            ("tgt-newer.txt", 5, Some(10)),
            ("same.txt", 6, Some(50)),
        ]);
        let target = entries(&[
            ("only-tgt.txt", 7, Some(10)),
            ("src-newer.txt", 4, Some(10)),
            ("tgt-newer.txt", 55, Some(100)),
            ("same.txt", 6, Some(50)),
        ]);
        let opts = SyncOptions {
            direction: SyncDirection::Bidirectional,
            delete_orphans: true,
            ..Default::default()
        };

        let plan = diff(&source, &target, &opts);
        assert_eq!(
            plan_of(&plan),
            vec![
                (
                    "only-src.txt",
                    SyncActionKind::Copy,
                    SyncActionDir::ToTarget
                ),
                (
                    "only-tgt.txt",
                    SyncActionKind::Copy,
                    SyncActionDir::ToSource
                ),
                (
                    "src-newer.txt",
                    SyncActionKind::Conflict,
                    SyncActionDir::ToTarget
                ),
                (
                    "tgt-newer.txt",
                    SyncActionKind::Conflict,
                    SyncActionDir::ToSource
                ),
            ]
        );
        assert_eq!(plan.summary.conflicts, 2);
        assert_eq!(plan.summary.deletes, 0, "bidirectional keeps both sides");
        // 冲突动作的 size 取「较新者」一侧
        let src_newer = plan
            .actions
            .iter()
            .find(|a| a.rel_path == "src-newer.txt")
            .unwrap();
        assert_eq!(src_newer.size, 4);
        let tgt_newer = plan
            .actions
            .iter()
            .find(|a| a.rel_path == "tgt-newer.txt")
            .unwrap();
        assert_eq!(tgt_newer.size, 55);
    }

    #[test]
    fn diff_empty_sides_yield_empty_plan() {
        let opts = SyncOptions {
            delete_orphans: true,
            ..Default::default()
        };
        assert!(diff(&[], &[], &opts).actions.is_empty());
        assert_eq!(diff(&[], &[], &opts).summary, SyncSummary::default());
    }

    #[test]
    fn diff_download_delete_orphan_only_when_enabled() {
        let source = entries(&[("orphan.txt", 5, Some(3))]);
        let target = entries(&[]);
        let base = SyncOptions {
            direction: SyncDirection::Download,
            ..Default::default()
        };
        assert!(diff(&source, &target, &base).actions.is_empty());

        let with_orphans = SyncOptions {
            delete_orphans: true,
            ..base.clone()
        };
        assert_eq!(
            plan_of(&diff(&source, &target, &with_orphans)),
            vec![(
                "orphan.txt",
                SyncActionKind::Delete,
                SyncActionDir::ToSource
            )]
        );
    }
}
