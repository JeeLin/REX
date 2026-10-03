//! 日志轮转初始化：hub / agent worker 共用。

use std::path::PathBuf;

/// 构建滚动日志 appender：滚动写入 `REX_LOG_DIR`（默认 `data/logs/`），
/// 按小时切分 + 旧日志清理，`REX_LOG_MAX_FILES` 控制保留个数（默认 7）。
///
/// 返回 `(appender, log_dir, max_log_files)` 供 worker 初始化订阅器与启动日志。
pub fn rolling_appender(
    filename_prefix: &str,
) -> (
    tracing_appender::rolling::RollingFileAppender,
    PathBuf,
    usize,
) {
    let data_dir = std::env::var("REX_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| crate::config::default_data_dir());
    let log_dir = std::env::var("REX_LOG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| data_dir.join("logs"));
    std::fs::create_dir_all(&log_dir).ok();
    let max_log_files: usize = std::env::var("REX_LOG_MAX_FILES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(7);
    let appender = tracing_appender::rolling::RollingFileAppender::builder()
        .rotation(tracing_appender::rolling::Rotation::HOURLY)
        .filename_prefix(filename_prefix)
        .max_log_files(max_log_files)
        .build(&log_dir)
        .expect("failed to init rolling log appender");
    (appender, log_dir, max_log_files)
}
