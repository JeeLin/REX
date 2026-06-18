use rex_agent::{client, config::AgentConfig, identity::AgentIdentity, ws::AgentWs};
use rex_common::app;
use rex_common::Parser;

const AGENT_VERSION: &str = "0.1.0";

fn main() -> anyhow::Result<()> {
    app::init_tracing();
    tracing::info!("rex-agent starting");

    let args: Vec<String> = std::env::args().collect();
    let cli = rex_common::cli::Cli::parse_from(&args);

    if cli.worker {
        tracing::info!("agent worker started");
        let config = AgentConfig::load(cli.config.as_deref())?;
        tracing::info!(
            server = %config.server,
            name = %config.name,
            data_dir = %config.data_dir.display(),
            "agent config loaded"
        );

        let identity = AgentIdentity::load_or_create(&config.data_dir)?;
        tracing::info!(agent_id = %identity.id, "agent identity loaded");

        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(run_agent(config, identity))?;
    } else {
        app::run_from(args)?;
    }

    Ok(())
}

async fn run_agent(config: AgentConfig, identity: AgentIdentity) -> anyhow::Result<()> {
    let (os, arch, hostname, os_version) = client::platform_info();

    // Register with Hub
    tracing::info!("registering with hub");
    let hub_client = client::HubClient::new(&config.server);
    let register_req = client::RegisterRequest {
        id: identity.id.clone(),
        token: config.token.clone(),
        name: config.name.clone(),
        version: AGENT_VERSION.to_string(),
        sha256: String::new(),
        os: os.clone(),
        arch: arch.clone(),
        hostname: hostname.clone(),
        os_version: os_version.clone(),
    };

    match hub_client.register(&register_req).await {
        Ok(resp) => {
            tracing::info!(
                agent_id = %resp.id,
                environment_id = %resp.environment_id,
                "registered successfully"
            );
        }
        Err(e) => {
            tracing::error!(error = %e, "failed to register with hub");
            return Err(e);
        }
    }

    // Start WebSocket heartbeat
    tracing::info!("connecting websocket");
    let ws = AgentWs::new(
        hub_client.websocket_url(),
        identity.id,
        config.token,
        AGENT_VERSION.to_string(),
    );
    ws.run().await
}
