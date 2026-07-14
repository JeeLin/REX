//! REX Hub 入口 — supervisor + worker 进程模型。
//!
//! 2.0 重设计骨架。后续里程碑逐步实现 HTTP server / WebSocket / 静态资源托管。

use tracing_subscriber::EnvFilter;

fn main() {
    // supervisor 模式：直接调用 worker（开发阶段不 fork 子进程）
    // 后续实现：PID 1 = supervisor，fork worker 子进程，监控存活/替换/回滚
    if std::env::var("REX_WORKER").is_err() {
        // supervisor 角色：设置环境变量后启动 worker
        std::env::set_var("REX_WORKER", "1");
        // 开发阶段直接调用 worker_main，不 fork
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
        name = "REX Hub",
        version = env!("CARGO_PKG_VERSION"),
        status = "starting"
    );

    // worker 主循环：启动 tokio runtime，后续监听 HTTP 端口
    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    rt.block_on(async {
        tracing::info!("worker runtime ready — HTTP server will bind here");
        // 占位：后续 M1 实现 axum server
        tokio::signal::ctrl_c().await.ok();
        tracing::info!("shutting down");
    });
}
