/// 版本信息模块 — 编译时嵌入版本号、Git commit、构建时间
use serde::Serialize;

/// 当前版本（编译时嵌入）
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Git commit hash（编译时嵌入，需要 build.rs）
pub const GIT_COMMIT: &str = env!("GIT_COMMIT_HASH");

/// 构建时间
pub const BUILD_TIME: &str = env!("BUILD_TIME");

/// 版本信息响应
#[derive(Debug, Clone, Serialize)]
pub struct VersionInfo {
    pub version: String,
    pub git_commit: String,
    pub build_time: String,
    pub rust_version: String,
}

impl VersionInfo {
    /// 返回当前版本信息
    pub fn current() -> Self {
        Self {
            version: VERSION.to_string(),
            git_commit: GIT_COMMIT.to_string(),
            build_time: BUILD_TIME.to_string(),
            rust_version: env!("CARGO_PKG_RUST_VERSION", "unknown").to_string(),
        }
    }
}

/// 比较两个语义化版本号，返回 latest 是否比 current 新
pub fn is_newer(current: &str, latest: &str) -> Option<bool> {
    let parse = |v: &str| -> Option<(u32, u32, u32)> {
        let v = v.strip_prefix('v').unwrap_or(v);
        let parts: Vec<&str> = v.split('.').collect();
        if parts.len() < 3 {
            return None;
        }
        let major = parts[0].parse::<u32>().ok()?;
        let minor = parts[1].parse::<u32>().ok()?;
        let patch = parts[2].parse::<u32>().ok()?;
        Some((major, minor, patch))
    };

    let c = parse(current)?;
    let l = parse(latest)?;

    Some(l > c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_info_current() {
        let info = VersionInfo::current();
        assert_eq!(info.version, VERSION);
        assert_eq!(info.git_commit, GIT_COMMIT);
        assert_eq!(info.build_time, BUILD_TIME);
    }

    #[test]
    fn is_newer_major() {
        assert_eq!(is_newer("0.1.0", "1.0.0"), Some(true));
        assert_eq!(is_newer("1.0.0", "0.1.0"), Some(false));
    }

    #[test]
    fn is_newer_minor() {
        assert_eq!(is_newer("0.1.0", "0.2.0"), Some(true));
        assert_eq!(is_newer("0.2.0", "0.1.0"), Some(false));
    }

    #[test]
    fn is_newer_patch() {
        assert_eq!(is_newer("0.1.0", "0.1.1"), Some(true));
        assert_eq!(is_newer("0.1.1", "0.1.0"), Some(false));
    }

    #[test]
    fn is_newer_equal() {
        assert_eq!(is_newer("0.1.0", "0.1.0"), Some(false));
    }

    #[test]
    fn is_newer_with_v_prefix() {
        assert_eq!(is_newer("v0.1.0", "v0.2.0"), Some(true));
        assert_eq!(is_newer("v1.0.0", "v0.9.0"), Some(false));
    }

    #[test]
    fn is_newer_invalid() {
        assert_eq!(is_newer("abc", "0.1.0"), None);
        assert_eq!(is_newer("0.1", "0.1.0"), None);
    }
}
