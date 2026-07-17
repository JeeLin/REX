//! REX Agent 入口 — supervisor + worker 进程模型。

mod agent_ws;

use tracing_subscriber::EnvFilter;

fn main() {
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

    let config = match agent_ws::AgentConfig::from_env() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "failed to load agent config");
            eprintln!("Error: {e}");
            eprintln!("Required environment variables: REX_HUB_URL, REX_AGENT_TOKEN, REX_AGENT_ID");
            std::process::exit(1);
        }
    };

    tracing::info!(
        hub_url = %config.hub_url,
        "agent configured"
    );

    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    rt.block_on(async {
        agent_ws::run_agent(config).await;
    });
}
