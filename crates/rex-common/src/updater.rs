/// GitHub Releases 更新检查器 + 下载 + SHA256 校验
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// GitHub Release 信息
#[derive(Debug, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub name: String,
    pub published_at: String,
    pub body: Option<String>,
    pub assets: Vec<GitHubAsset>,
}

/// GitHub Release Asset
#[derive(Debug, Deserialize)]
pub struct GitHubAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
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

/// 下载进度回调
pub type ProgressCallback = Box<dyn Fn(u32) + Send + Sync>;

/// 更新下载源
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum UpdateSource {
    #[default]
    GitHub,
    Hub,
}


/// 更新检查器
pub struct UpdateChecker {
    repo: String,
    current_version: String,
}

impl UpdateChecker {
    /// 创建新的更新检查器
    pub fn new(repo: &str, current_version: &str) -> Self {
        Self {
            repo: repo.to_string(),
            current_version: current_version.to_string(),
        }
    }

    /// 获取 HTTP 客户端
    fn client() -> reqwest::Client {
        reqwest::Client::builder()
            .user_agent("rex-hub-update-checker")
            .build()
            .unwrap_or_default()
    }

    /// 检查是否有新版本
    pub async fn check_for_update(&self) -> UpdateStatus {
        let url = format!("https://api.github.com/repos/{}/releases/latest", self.repo);

        let resp = match Self::client().get(&url).send().await {
            Ok(r) => r,
            Err(e) => return UpdateStatus::CheckFailed(format!("网络错误: {e}")),
        };

        if !resp.status().is_success() {
            return UpdateStatus::CheckFailed(format!("GitHub API 返回 {}", resp.status()));
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

    /// 确定当前平台的 asset 文件名前缀
    pub fn platform_asset_prefix() -> &'static str {
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        {
            "rex-linux-x86_64"
        }
        #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
        {
            "rex-linux-aarch64"
        }
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        {
            "rex-macos-x86_64"
        }
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            "rex-macos-aarch64"
        }
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        {
            "rex-windows-x86_64.exe"
        }
        #[cfg(not(any(
            all(target_os = "linux", target_arch = "x86_64"),
            all(target_os = "linux", target_arch = "aarch64"),
            all(target_os = "macos", target_arch = "x86_64"),
            all(target_os = "macos", target_arch = "aarch64"),
            all(target_os = "windows", target_arch = "x86_64"),
        )))]
        {
            "rex-unknown"
        }
    }

    /// 当前平台 os 名称
    pub fn current_os() -> &'static str {
        #[cfg(target_os = "linux")]
        {
            "linux"
        }
        #[cfg(target_os = "macos")]
        {
            "darwin"
        }
        #[cfg(target_os = "windows")]
        {
            "windows"
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            "unknown"
        }
    }

    /// 当前平台 arch 名称
    pub fn current_arch() -> &'static str {
        #[cfg(target_arch = "x86_64")]
        {
            "amd64"
        }
        #[cfg(target_arch = "aarch64")]
        {
            "arm64"
        }
        #[cfg(target_arch = "arm")]
        {
            "armv7l"
        }
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64", target_arch = "arm")))]
        {
            "unknown"
        }
    }

    /// 从 Hub 下载 Agent 二进制
    pub async fn download_from_hub(
        hub_url: &str,
        token: &str,
        data_dir: &Path,
        on_progress: Option<ProgressCallback>,
    ) -> anyhow::Result<PathBuf> {
        let os = Self::current_os();
        let arch = Self::current_arch();
        let url = format!("{hub_url}/api/agent/download?os={os}&arch={arch}");

        let resp = Self::client().get(&url).bearer_auth(token).send().await?;

        if !resp.status().is_success() {
            anyhow::bail!("Hub 下载失败: HTTP {}", resp.status());
        }

        // 读取 SHA256 校验和
        let expected_sha256 = resp
            .headers()
            .get("X-Agent-SHA256")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let total = resp.content_length().unwrap_or(0);
        let mut downloaded: u64 = 0;
        let mut bytes = Vec::with_capacity(total as usize);

        let mut stream = resp.bytes_stream();
        use futures_util::StreamExt;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            downloaded += chunk.len() as u64;
            bytes.extend_from_slice(&chunk);
            if let Some(ref cb) = on_progress {
                let percent = if total > 0 {
                    (downloaded * 100 / total) as u32
                } else {
                    0
                };
                cb(percent);
            }
        }

        // SHA256 校验
        if let Some(ref expected) = expected_sha256 {
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            let actual = format!("{:x}", hasher.finalize());
            if actual != *expected {
                anyhow::bail!("SHA256 校验失败: 期望 {expected}, 实际 {actual}");
            }
        }

        // 写入 staging 目录
        let staging_dir = data_dir.join("updates").join("staging");
        std::fs::create_dir_all(&staging_dir)?;

        let filename = format!("agent-{os}-{arch}");
        let staged_path = staging_dir.join(&filename);
        std::fs::write(&staged_path, &bytes)?;

        // chmod +x (Unix)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&staged_path)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&staged_path, perms)?;
        }

        Ok(staged_path)
    }

    /// 下载新版本二进制到 staging 目录
    pub async fn download_update(
        &self,
        data_dir: &Path,
        on_progress: Option<ProgressCallback>,
    ) -> anyhow::Result<PathBuf> {
        // 获取 latest release
        let url = format!("https://api.github.com/repos/{}/releases/latest", self.repo);
        let release: GitHubRelease = Self::client().get(&url).send().await?.json().await?;

        let prefix = Self::platform_asset_prefix();
        let asset = release
            .assets
            .iter()
            .find(|a| a.name.starts_with(prefix))
            .ok_or_else(|| anyhow::anyhow!("未找到平台 {} 对应的二进制", prefix))?;

        // 创建 staging 目录
        let staging_dir = data_dir.join("updates").join("staging");
        std::fs::create_dir_all(&staging_dir)?;

        let staged_path = staging_dir.join(&asset.name);

        // 下载
        let resp = Self::client()
            .get(&asset.browser_download_url)
            .send()
            .await?;

        let total = asset.size;
        let mut downloaded: u64 = 0;
        let mut bytes = Vec::with_capacity(total as usize);

        let mut stream = resp.bytes_stream();
        use futures_util::StreamExt;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            downloaded += chunk.len() as u64;
            bytes.extend_from_slice(&chunk);
            if let Some(ref cb) = on_progress {
                let percent = if total > 0 {
                    (downloaded * 100 / total) as u32
                } else {
                    0
                };
                cb(percent);
            }
        }

        std::fs::write(&staged_path, &bytes)?;

        // chmod +x (Unix)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&staged_path)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&staged_path, perms)?;
        }

        Ok(staged_path)
    }

    /// 备份当前二进制到 rollback 目录
    pub fn backup_current(data_dir: &Path) -> anyhow::Result<PathBuf> {
        let rollback_dir = data_dir.join("updates").join("rollback");
        std::fs::create_dir_all(&rollback_dir)?;

        let current_exe = std::env::current_exe()?;
        let filename = current_exe
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("无法获取当前二进制文件名"))?;

        let rollback_path = rollback_dir.join(filename);

        // 如果已存在备份，先删除
        if rollback_path.exists() {
            std::fs::remove_file(&rollback_path)?;
        }

        std::fs::copy(&current_exe, &rollback_path)?;

        Ok(rollback_path)
    }
}

/// 计算文件 SHA256
pub fn sha256_file(path: &Path) -> anyhow::Result<String> {
    let bytes = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

/// 下载并验证 SHA256SUMS
pub async fn verify_download(
    binary_path: &Path,
    checksums_url: &str,
    expected_filename: &str,
) -> anyhow::Result<bool> {
    let resp = reqwest::get(checksums_url).await?;
    if !resp.status().is_success() {
        anyhow::bail!("下载 checksums 失败: {}", resp.status());
    }
    let checksums_text = resp.text().await?;

    // 解析 checksums 文件，找到对应文件的 hash
    for line in checksums_text.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() == 2 && parts[1] == expected_filename {
            let expected_hash = parts[0];
            let actual_hash = sha256_file(binary_path)?;
            return Ok(actual_hash == expected_hash);
        }
    }

    anyhow::bail!("checksums 中未找到 {}", expected_filename)
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
    fn platform_asset_prefix_is_nonempty() {
        let prefix = UpdateChecker::platform_asset_prefix();
        assert!(!prefix.is_empty());
    }

    #[test]
    fn sha256_file_works() {
        use std::io::Write;
        let tmp = tempfile::NamedTempFile::new().unwrap();
        tmp.as_file().write_all(b"hello world").unwrap();
        let hash = sha256_file(tmp.path()).unwrap();
        // SHA256("hello world") = b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn backup_current_creates_file() {
        let tmp = tempfile::tempdir().unwrap();
        let result = UpdateChecker::backup_current(tmp.path());
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.exists());
    }

    #[test]
    fn current_os_is_nonempty() {
        assert!(!UpdateChecker::current_os().is_empty());
    }

    #[test]
    fn current_arch_is_nonempty() {
        assert!(!UpdateChecker::current_arch().is_empty());
    }

    #[test]
    fn update_source_default_is_github() {
        assert_eq!(UpdateSource::default(), UpdateSource::GitHub);
    }

    #[test]
    fn update_source_deserialize() {
        let github: UpdateSource = serde_json::from_str("\"github\"").unwrap();
        assert_eq!(github, UpdateSource::GitHub);
        let hub: UpdateSource = serde_json::from_str("\"hub\"").unwrap();
        assert_eq!(hub, UpdateSource::Hub);
    }
}
