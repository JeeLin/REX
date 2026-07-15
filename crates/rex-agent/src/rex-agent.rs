//! REX Agent 入口 — supervisor + worker 进程模型。
//!
//! 2.0 重设计骨架。后续里程碑逐步实现 WebSocket 反向隧道 / Agent 代理逻辑。

use tracing_subscriber::EnvFilter;

fn main() {
    // supervisor 模式：开发阶段直接调用 worker（不 fork 子进程）
    if std::env::var("REX_WORKER").is_err() {
        std::env::set_var("REX_WORKER", "1");
        worker_main();
    } else {
        worker_main();
    }
}

fn worker_main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .init();

    tracing::info!(
        name = "REX Agent",
        version = env!("CARGO_PKG_VERSION"),
        status = "starting"
    );

    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    rt.block_on(async {
        tracing::info!("worker runtime ready — agent logic will go here");
        // 占位：后续实现 WebSocket 反向隧道
        tokio::signal::ctrl_c().await.ok();
        tracing::info!("shutting down");
    });
}
