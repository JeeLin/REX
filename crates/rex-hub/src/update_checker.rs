//! Hub 自动更新检查器 — 从 GitHub Release 下载新版二进制并触发 supervisor 替换。

use std::path::PathBuf;

use rex_common::update::{sha256_hex, UpdatePhase, UpdateStateFile};

/// GitHub Release 信息
#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub version: String,
    pub download_url: String,
    pub sha256: String,
}

/// 更新检查器
pub struct UpdateChecker {
    pub current_version: String,
    pub github_owner: String,
    pub github_repo: String,
    pub data_dir: PathBuf,
}

impl UpdateChecker {
    pub fn new(
        current_version: &str,
        github_owner: &str,
        github_repo: &str,
        data_dir: PathBuf,
    ) -> Self {
        Self {
            current_version: current_version.to_string(),
            github_owner: github_owner.to_string(),
            github_repo: github_repo.to_string(),
            data_dir,
        }
    }

    /// 从环境变量或默认值创建
    pub fn from_env(data_dir: PathBuf) -> Self {
        let owner = std::env::var("REX_UPDATE_GITHUB_OWNER").unwrap_or_else(|_| "JeeLin".into());
        let repo = std::env::var("REX_UPDATE_GITHUB_REPO").unwrap_or_else(|_| "REX".into());
        Self::new(rex_common::APP_VERSION, &owner, &repo, data_dir)
    }

    /// 检查 GitHub 是否有新版本
    pub async fn check_for_update(&self) -> Result<Option<UpdateInfo>, String> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            self.github_owner, self.github_repo
        );

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent(format!("rex-hub/{}", self.current_version))
            .build()
            .map_err(|e| format!("failed to create HTTP client: {e}"))?;

        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("failed to check for update: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("GitHub API returned status {}", resp.status()));
        }

        let release: GitHubRelease = resp
            .json()
            .await
            .map_err(|e| format!("failed to parse release JSON: {e}"))?;

        let latest_version = release.tag_name.trim_start_matches('v');

        // 比较语义化版本号：仅当 latest 比当前版本更新时才更新，
        // 避免「Latest」被标成更旧版本时把已更新的用户降级。
        if compare_version(latest_version, &self.current_version) != std::cmp::Ordering::Greater {
            return Ok(None);
        }

        // 查找当前平台的二进制和 SHA256
        let (os_raw, arch) = get_platform();
        // darwin → mac（Release 资源命名用 mac，不是 darwin）
        let os = if os_raw == "darwin" { "mac" } else { os_raw };
        let binary_name = format!("rex-hub-{os}-{arch}");
        let sha256_name = "rex-hub-SHA256SUMS".to_string();

        let download_url = release
            .assets
            .iter()
            .find(|a| a.name == binary_name)
            .map(|a| a.browser_download_url.clone())
            .ok_or_else(|| format!("binary {binary_name} not found in release"))?;

        let sha256_url = release
            .assets
            .iter()
            .find(|a| a.name == sha256_name)
            .map(|a| a.browser_download_url.clone());

        // 下载 SHA256SUMS
        let sha256 = if let Some(url) = sha256_url {
            let resp = client
                .get(&url)
                .send()
                .await
                .map_err(|e| format!("failed to download SHA256SUMS: {e}"))?;
            let text = resp
                .text()
                .await
                .map_err(|e| format!("failed to read SHA256SUMS: {e}"))?;
            parse_sha256sums(&text, &binary_name)
                .ok_or_else(|| format!("{binary_name} not found in SHA256SUMS"))?
        } else {
            // 无 SHA256SUMS 文件，返回空（后续下载时计算）
            String::new()
        };

        Ok(Some(UpdateInfo {
            version: latest_version.to_string(),
            download_url,
            sha256,
        }))
    }

    /// 下载新版二进制、校验 SHA256、写入 update-state.json。
    pub async fn download_and_stage(&self, info: &UpdateInfo) -> Result<(), String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300)) // 5 分钟下载超时
            .build()
            .map_err(|e| format!("failed to create HTTP client: {e}"))?;

        // 下载二进制
        tracing::info!(
            action = "UPDATE_DOWNLOAD",
            version = %info.version,
            url = %info.download_url,
            "downloading new binary"
        );

        let resp = client
            .get(&info.download_url)
            .send()
            .await
            .map_err(|e| format!("failed to start download: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("download returned status {}", resp.status()));
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| format!("failed to read download: {e}"))?;

        // 校验 SHA256
        if !info.sha256.is_empty() {
            let actual = sha256_hex(&bytes);
            if actual != info.sha256 {
                return Err(format!(
                    "SHA256 mismatch: expected {}, got {}",
                    info.sha256, actual
                ));
            }
            tracing::info!(action = "UPDATE_VERIFY", "SHA256 verification passed");
        }

        // 写入 staging 路径
        let staging_dir = self.data_dir.join("update");
        let _ = std::fs::create_dir_all(&staging_dir);

        let ext = if cfg!(windows) { ".exe" } else { "" };
        let staging_path = staging_dir.join(format!("rex-hub.v{}{ext}", info.version));

        std::fs::write(&staging_path, &bytes)
            .map_err(|e| format!("failed to write staged binary: {e}"))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&staging_path, std::fs::Permissions::from_mode(0o755));
        }

        // 写入 update-state.json
        let rollback_dir = self.data_dir.join("update/rollback");
        let _ = std::fs::create_dir_all(&rollback_dir);

        let ext = if cfg!(windows) { ".exe" } else { "" };
        let rollback_path = rollback_dir.join(format!("rex-hub.v{}{ext}", self.current_version));

        let state = UpdateStateFile {
            phase: UpdatePhase::Requested,
            target_version: info.version.clone(),
            old_version: self.current_version.clone(),
            staged_path: staging_path.to_string_lossy().into_owned(),
            rollback_path: rollback_path.to_string_lossy().into_owned(),
            sha256: info.sha256.clone(),
            attempt: 0,
        };

        let state_path = self.data_dir.join("update-state.json");
        let tmp_path = state_path.with_extension("json.tmp");
        let json = serde_json::to_string_pretty(&state)
            .map_err(|e| format!("failed to serialize update state: {e}"))?;
        std::fs::write(&tmp_path, &json)
            .map_err(|e| format!("failed to write update state tmp: {e}"))?;
        std::fs::rename(&tmp_path, &state_path)
            .map_err(|e| format!("failed to rename update state: {e}"))?;

        tracing::info!(
            action = "UPDATE_STAGED",
            version = %info.version,
            "update staged, worker will exit to trigger supervisor replacement"
        );

        Ok(())
    }
}

/// 获取当前平台标识
fn get_platform() -> (&'static str, &'static str) {
    let os = if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "unknown"
    };

    let arch = if cfg!(target_arch = "x86_64") {
        "amd64"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else if cfg!(target_arch = "arm") {
        "armv7l"
    } else {
        "unknown"
    };

    (os, arch)
}

/// 从 SHA256SUMS 文件内容中解析指定文件的哈希值。
fn parse_sha256sums(content: &str, filename: &str) -> Option<String> {
    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() == 2 && parts[1] == filename {
            return Some(parts[0].to_string());
        }
    }
    None
}

/// 后台定期检查更新的任务。
///
/// 每 6 小时检查一次 GitHub Release，发现新版本后下载并写入 update-state.json，
/// 然后以 exit(10) 退出，由 supervisor 执行二进制替换。
pub async fn background_update_task(data_dir: PathBuf) {
    let checker = UpdateChecker::from_env(data_dir);

    loop {
        // 检查是否处于更新验证阶段
        if std::env::var("REX_UPDATE_PENDING").is_ok() {
            // 新版 worker 启动后，只执行健康检查，不检查更新
            tracing::info!(
                action = "UPDATE_SKIP",
                "update pending, skipping update check"
            );
            return;
        }

        // 等待 5 分钟（给服务启动时间）
        tokio::time::sleep(std::time::Duration::from_secs(5 * 60)).await;

        match checker.check_for_update().await {
            Ok(Some(info)) => {
                tracing::info!(
                    action = "UPDATE_AVAILABLE",
                    current = %checker.current_version,
                    latest = %info.version,
                    "new version available"
                );

                if let Err(e) = checker.download_and_stage(&info).await {
                    tracing::error!(action = "UPDATE_FAILED", error = %e, "failed to stage update");
                    continue;
                }

                // 更新已暂存，设置环境变量让 main loop 检测后优雅退出
                tracing::info!(action = "UPDATE_EXIT", "update staged, setting exit flag");
                // 设置退出标志，由 main loop 检测后调用 std::process::exit(10)
                std::env::set_var("REX_UPDATE_READY", "1");
                return;
            }
            Ok(None) => {
                tracing::debug!(action = "UPDATE_CHECK", "already on latest version");
            }
            Err(e) => {
                tracing::warn!(action = "UPDATE_CHECK_FAILED", error = %e, "failed to check for update");
            }
        }

        // 检查间隔 6 小时
        tokio::time::sleep(std::time::Duration::from_secs(6 * 60 * 60)).await;
    }
}

/// 读取当前更新状态
pub fn read_update_status(data_dir: &std::path::Path) -> Option<UpdateStateFile> {
    let path = data_dir.join("update-state.json");
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

/// 比较两个语义化版本号（形如 `v0.68.0` / `0.65.4` / `1.10.0`），按 `.` 分段逐段比较。
/// 前缀 `v` 会被忽略；缺省小版本段视为 0；空版本视为最小。
/// 返回 `Ordering::Greater` 表示 `a` 比 `b` 新。
fn compare_version(a: &str, b: &str) -> std::cmp::Ordering {
    let parse = |s: &str| -> Vec<u32> {
        s.trim_start_matches('v')
            .split('.')
            .map(|seg| seg.parse::<u32>().unwrap_or(0))
            .collect()
    };
    let av = parse(a);
    let bv = parse(b);
    let len = av.len().max(bv.len());
    for i in 0..len {
        let x = av.get(i).copied().unwrap_or(0);
        let y = bv.get(i).copied().unwrap_or(0);
        match x.cmp(&y) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }
    std::cmp::Ordering::Equal
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sha256sums() {
        let content = "abc123def456  rex-hub-linux-amd64\n789xyz000  rex-hub-darwin-arm64\n";
        assert_eq!(
            parse_sha256sums(content, "rex-hub-linux-amd64"),
            Some("abc123def456".into())
        );
        assert_eq!(parse_sha256sums(content, "rex-hub-windows-amd64"), None);
    }

    #[test]
    fn test_get_platform() {
        let (os, arch) = get_platform();
        assert!(!os.is_empty());
        assert!(!arch.is_empty());
    }

    #[test]
    fn test_compare_version() {
        // 逐段比较，而非字符串比较
        assert_eq!(
            compare_version("0.68.0", "0.65.4"),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            compare_version("0.65.4", "0.68.0"),
            std::cmp::Ordering::Less
        );
        assert_eq!(compare_version("1.2.0", "1.10.0"), std::cmp::Ordering::Less);
        assert_eq!(
            compare_version("1.10.0", "1.2.0"),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            compare_version("0.68.0", "0.68.0"),
            std::cmp::Ordering::Equal
        );
        // 前缀比较
        assert_eq!(compare_version("1.0", "1.0.1"), std::cmp::Ordering::Less);
        assert_eq!(compare_version("1.0.1", "1.0"), std::cmp::Ordering::Greater);
        // 前导 v 不影响
        assert_eq!(
            compare_version("v0.68.0", "0.68.0"),
            std::cmp::Ordering::Equal
        );
        // 空版本视为最小
        assert_eq!(compare_version("0.68.0", ""), std::cmp::Ordering::Greater);
    }
}

/// GitHub Release API 响应结构
#[derive(Debug, serde::Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, serde::Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}
