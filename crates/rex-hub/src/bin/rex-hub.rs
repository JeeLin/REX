use rex_common::app;
use rex_common::Parser;
use rex_hub::config::HubConfig;
use rex_hub::db::Database;
use rex_hub::routes;
use std::sync::Arc;

fn main() -> anyhow::Result<()> {
    app::init_tracing();
    tracing::info!("rex-hub starting");

    let args: Vec<String> = std::env::args().collect();
    let cli = rex_common::cli::Cli::parse_from(&args);

    if cli.worker {
        tracing::info!("worker started");
        let config = HubConfig::load(cli.config.as_deref())?;
        tracing::info!(listen = %config.listen, "starting HTTP server");

        let db_path = config.data_dir.join("hub.db");
        std::fs::create_dir_all(&config.data_dir)?;
        let db = Database::new(&db_path)?;

        let secret_key = config.secret_key;
        if secret_key.is_empty() {
            tracing::warn!("secret_key is empty, using default - this is insecure in production");
        }

        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let listener = tokio::net::TcpListener::bind(&config.listen).await?;
            axum::serve(listener, routes::app(Arc::new(db), secret_key)).await?;
            Ok::<(), anyhow::Error>(())
        })?;
    } else {
        app::run_from(args)?;
    }

    Ok(())
}
