//! REX Agent 入口 — supervisor + worker 进程模型。
//!
//! PID 1 = supervisor（启动 worker、监控退出、处理更新替换）
//! Worker = 实际业务逻辑（WebSocket 连接、资源代理）
//!
//! REX_WORKER=1 → 运行 worker
//! 否则 → 运行 supervisor

mod agent_file;
mod agent_redis;
mod agent_sql;
mod agent_ssh;
mod agent_ws;
mod supervisor;
mod updater;

fn main() {
    if std::env::var("REX_WORKER").is_ok() {
        worker_main();
    } else {
        crate::supervisor::run_supervisor();
    }
}

fn worker_main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("info".parse().unwrap()),
        )
        .init();

    tracing::info!(
        name = "REX Agent",
        version = env!("CARGO_PKG_VERSION"),
        status = "worker starting"
    );

    let config = match agent_ws::AgentConfig::from_env() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "failed to load agent config");
            eprintln!("Error: {e}");
            eprintln!("Required environment variables: REX_HUB_URL, REX_AGENT_TOKEN");
            std::process::exit(1);
        }
    };

    tracing::info!(
        hub_url = %config.hub_url,
        auto_update = config.auto_update,
        "agent configured"
    );

    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    rt.block_on(async {
        agent_ws::run_agent(config).await;
    });
}
