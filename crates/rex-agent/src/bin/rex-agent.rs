use rex_agent::{
    client,
    config::AgentConfig,
    identity::AgentIdentity,
    log_collector::{self, LogCollector},
    ws::AgentWs,
};
use rex_common::{app, tls_client, Parser};

const AGENT_VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let cli = rex_common::cli::Cli::parse_from(&args);

    if cli.worker {
        // Worker mode: init tracing with log collector
        let log_collector = LogCollector::new();
        log_collector::init_tracing_with_collector(log_collector.clone());

        tracing::info!("agent worker started");
        let mut config = AgentConfig::load(cli.config.as_deref())?;
        tracing::info!(
            server = %config.server,
            name = %config.name,
            data_dir = %config.data_dir.display(),
            "agent config loaded"
        );

        // Merge TLS config: CLI args > env vars > config file
        if let Some(ref cert_path) = cli.ca_cert {
            config.tls.ca_cert = Some(std::path::PathBuf::from(cert_path));
        }
        if cli.insecure {
            config.tls.insecure = true;
        }

        // Build TLS clients
        let ca_cert_path = config.tls.ca_cert.as_deref();
        let insecure = config.tls.insecure;

        let identity = AgentIdentity::load_or_create(&config.data_dir)?;
        tracing::info!(agent_id = %identity.id, "agent identity loaded");

        // Check if this is an update-pending startup
        if std::env::var("REX_UPDATE_PENDING").is_ok() {
            tracing::info!("update pending mode, performing health check");
            let http_client = tls_client::build_reqwest_client(ca_cert_path, insecure)?;
            let rt = tokio::runtime::Runtime::new()?;
            let result = rt.block_on(async {
                let hub_client = client::HubClient::with_client(&config.server, http_client);
                let (os, arch, hostname, os_version) = client::platform_info();
                let req = client::RegisterRequest {
                    id: identity.id.clone(),
                    token: config.token.clone(),
                    name: config.name.clone(),
                    version: AGENT_VERSION.to_string(),
                    sha256: String::new(),
                    os,
                    arch,
                    hostname,
                    os_version,
                };
                hub_client.register(&req).await
            });
            match result {
                Ok(_) => {
                    tracing::info!("health check passed, update committed");
                    std::process::exit(0);
                }
                Err(e) => {
                    tracing::error!(error = %e, "health check failed");
                    std::process::exit(11);
                }
            }
        }

        // Build clients for normal operation
        let http_client = tls_client::build_reqwest_client(ca_cert_path, insecure)?;
        let ws_connector = tls_client::build_ws_connector(ca_cert_path, insecure)?;

        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(run_agent(config, identity, log_collector, http_client, ws_connector))?;
    } else {
        // Supervisor mode: use update supervisor with data_dir
        let config_path = cli.config.clone();
        let data_dir = AgentConfig::load(config_path.as_deref())
            .map(|c| c.data_dir)
            .unwrap_or_else(|_| std::path::PathBuf::from("./data"));

        app::run_update_supervisor_from_args(args, data_dir)?;
    }

    Ok(())
}

async fn run_agent(
    config: AgentConfig,
    identity: AgentIdentity,
    log_collector: LogCollector,
    http_client: reqwest::Client,
    ws_connector: tokio_tungstenite::Connector,
) -> anyhow::Result<()> {
    let (os, arch, hostname, os_version) = client::platform_info();

    // Register with Hub using custom TLS client
    tracing::info!("registering with hub");
    let hub_client = client::HubClient::with_client(&config.server, http_client);
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

    // Rebuild HTTP client for WS (HubClient consumed it)
    let ca_cert_path = config.tls.ca_cert.as_deref();
    let insecure = config.tls.insecure;
    let http_client = tls_client::build_reqwest_client(ca_cert_path, insecure)?;

    // Start WebSocket heartbeat with custom TLS
    tracing::info!("connecting websocket");
    let ws = AgentWs::with_tls(
        hub_client.websocket_url(),
        identity.id,
        config.token.clone(),
        AGENT_VERSION.to_string(),
        config.update.auto_update,
        config.data_dir,
        log_collector.clone(),
        ws_connector,
        http_client,
    );
    ws.run().await
}
