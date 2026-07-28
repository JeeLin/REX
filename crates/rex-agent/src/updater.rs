//! Agent 更新处理器 — 下载新二进制、SHA256 校验、写 update-state.json。
//!
//! Worker 调用此模块执行更新，完成后 Worker 退出（exit code 42），
//! 由 Supervisor 读取 update-state.json 完成实际的二进制替换。

use std::path::PathBuf;

use rex_common::update::{UpdateCommand, UpdatePhase, UpdateProgress};
use tokio::io::AsyncWriteExt;

/// 向 Hub 报告更新进度的回调
pub type ProgressReporter =
    Box<dyn Fn(UpdateProgress) -> futures_util::future::BoxFuture<'static, ()> + Send + Sync>;

/// 执行 Agent 更新流程
///
/// 1. 下载新二进制到 `{current_exe}.tmp`
/// 2. SHA256 校验
/// 3. 写 `update-state.json`（Supervisor 读取后完成替换）
///
/// 成功后调用方应 `std::process::exit(42)` 让 Supervisor 接管。
pub async fn run_update(
    cmd: UpdateCommand,
    current_exe: PathBuf,
    report: &ProgressReporter,
) -> Result<(), UpdateError> {
    let tmp_path = current_exe.with_extension("tmp");

    // 1. 下载
    report(UpdateProgress {
        phase: UpdatePhase::Downloading,
        progress: 0.0,
        error: None,
    })
    .await;

    let bytes = download(&cmd, report).await?;

    // 2. 校验 SHA256
    report(UpdateProgress {
        phase: UpdatePhase::Verifying,
        progress: 0.0,
        error: None,
    })
    .await;

    if !cmd.sha256.is_empty() {
        let hash = rex_common::update::sha256_hex(&bytes);
        if hash != cmd.sha256 {
            let err = format!("SHA256 mismatch: expected {}, got {}", cmd.sha256, hash);
            report(UpdateProgress {
                phase: UpdatePhase::Error,
                progress: 0.0,
                error: Some(err.clone()),
            })
            .await;
            return Err(UpdateError::Sha256Mismatch(err));
        }
    }

    // 3. 写入临时文件
    report(UpdateProgress {
        phase: UpdatePhase::Replacing,
        progress: 0.0,
        error: None,
    })
    .await;

    let mut file = tokio::fs::File::create(&tmp_path)
        .await
        .map_err(|e| UpdateError::Io(e.to_string()))?;
    file.write_all(&bytes)
        .await
        .map_err(|e| UpdateError::Io(e.to_string()))?;
    file.flush()
        .await
        .map_err(|e| UpdateError::Io(e.to_string()))?;
    drop(file);

    // 设置可执行权限（Unix）
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        tokio::fs::set_permissions(&tmp_path, perms)
            .await
            .map_err(|e| UpdateError::Io(e.to_string()))?;
    }

    // 4. 写 update-state.json
    let state_file = current_exe.with_extension("update-state.json");
    let state = rex_common::update::UpdateStateFile {
        phase: UpdatePhase::Requested,
        target_version: cmd.version.clone(),
        old_version: rex_common::APP_VERSION.to_string(),
        staged_path: tmp_path.to_string_lossy().into_owned(),
        rollback_path: current_exe.with_extension("bak").to_string_lossy().into_owned(),
        sha256: cmd.sha256.clone(),
        attempt: 0,
    };
    let state_json =
        serde_json::to_string_pretty(&state).map_err(|e| UpdateError::Io(e.to_string()))?;
    tokio::fs::write(&state_file, state_json)
        .await
        .map_err(|e| UpdateError::Io(e.to_string()))?;

    report(UpdateProgress {
        phase: UpdatePhase::Restarting,
        progress: 1.0,
        error: None,
    })
    .await;

    tracing::info!(
        target_version = %cmd.version,
        tmp_path = %tmp_path.display(),
        "update prepared, exiting for supervisor to apply"
    );

    Ok(())
}

/// 下载二进制（优先 Hub，fallback GitHub）
async fn download(cmd: &UpdateCommand, report: &ProgressReporter) -> Result<Vec<u8>, UpdateError> {
    match try_download(&cmd.download_url, report).await {
        Ok(bytes) => Ok(bytes),
        Err(e) => {
            tracing::warn!("primary download failed: {e}, trying fallback");
            try_download(&cmd.fallback_url, report).await
        }
    }
}

async fn try_download(url: &str, report: &ProgressReporter) -> Result<Vec<u8>, UpdateError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| UpdateError::Io(e.to_string()))?;

    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| UpdateError::Download(e.to_string()))?;

    let total = resp.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;

    let mut bytes = Vec::new();
    let mut stream = resp.bytes_stream();

    use futures_util::StreamExt;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| UpdateError::Download(e.to_string()))?;
        downloaded += chunk.len() as u64;
        bytes.extend_from_slice(&chunk);

        if total > 0 {
            let progress = downloaded as f64 / total as f64;
            report(UpdateProgress {
                phase: UpdatePhase::Downloading,
                progress,
                error: None,
            })
            .await;
        }
    }

    Ok(bytes)
}

#[derive(Debug)]
pub enum UpdateError {
    Download(String),
    Sha256Mismatch(String),
    Io(String),
}

impl std::fmt::Display for UpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Download(e) => write!(f, "download error: {e}"),
            Self::Sha256Mismatch(e) => write!(f, "{e}"),
            Self::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_sha256_hex() {
        let hash = rex_common::update::sha256_hex(b"hello");
        assert_eq!(
            hash,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }
}
