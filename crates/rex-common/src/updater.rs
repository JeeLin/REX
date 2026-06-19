/// GitHub Releases 更新检查器
use serde::Deserialize;

/// GitHub Release 信息
#[derive(Debug, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub name: String,
    pub published_at: String,
    pub body: Option<String>,
}

/// 版本比较结果
#[derive(Debug, Clone, PartialEq)]
pub enum UpdateStatus {
    /// 已是最新版本
    UpToDate,
    /// 发现新版本
    UpdateAvailable {
        current: String,
        latest: String,
        release_notes: String,
        published_at: String,
    },
    /// 检查失败
    CheckFailed(String),
}

/// 更新检查器
pub struct UpdateChecker {
    repo: String,
    current_version: String,
}

impl UpdateChecker {
    /// 创建新的更新检查器
    ///
    /// - `repo`: GitHub 仓库，格式为 "owner/repo"
    /// - `current_version`: 当前版本号，如 "0.1.0" 或 "v0.1.0"
    pub fn new(repo: &str, current_version: &str) -> Self {
        Self {
            repo: repo.to_string(),
            current_version: current_version.to_string(),
        }
    }

    /// 检查是否有新版本
    pub async fn check_for_update(&self) -> UpdateStatus {
        let url = format!(
            "https://api.github.com/repos/{}/releases/latest",
            self.repo
        );

        let client = reqwest::Client::builder()
            .user_agent("rex-hub-update-checker")
            .build()
            .unwrap_or_default();

        let resp = match client.get(&url).send().await {
            Ok(r) => r,
            Err(e) => return UpdateStatus::CheckFailed(format!("网络错误: {e}")),
        };

        if !resp.status().is_success() {
            return UpdateStatus::CheckFailed(format!(
                "GitHub API 返回 {}",
                resp.status()
            ));
        }

        let release: GitHubRelease = match resp.json().await {
            Ok(r) => r,
            Err(e) => return UpdateStatus::CheckFailed(format!("解析失败: {e}")),
        };

        let latest = release.tag_name.trim_start_matches('v').to_string();
        let current = self.current_version.trim_start_matches('v').to_string();

        match crate::version::is_newer(&current, &latest) {
            Some(true) => UpdateStatus::UpdateAvailable {
                current,
                latest,
                release_notes: release.body.unwrap_or_default(),
                published_at: release.published_at,
            },
            Some(false) => UpdateStatus::UpToDate,
            None => UpdateStatus::CheckFailed(format!(
                "无法比较版本: current={current}, latest={latest}"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_checker_new() {
        let checker = UpdateChecker::new("test/repo", "0.1.0");
        assert_eq!(checker.repo, "test/repo");
        assert_eq!(checker.current_version, "0.1.0");
    }

    #[test]
    fn update_checker_strips_v_prefix() {
        let checker = UpdateChecker::new("test/repo", "v0.1.0");
        assert_eq!(checker.current_version, "v0.1.0");
    }
}
