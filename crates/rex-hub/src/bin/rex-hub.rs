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
        let config = HubConfig::load(
            cli.config.as_deref(),
            cli.tls_cert.as_deref(),
            cli.tls_key.as_deref(),
        )?;

        let db_path = config.data_dir.join("hub.db");
        std::fs::create_dir_all(&config.data_dir)?;
        let db = Database::new(&db_path)?;

        let secret_key = config.secret_key;
        if secret_key.is_empty() {
            tracing::warn!("secret_key is empty, using default - this is insecure in production");
        }

        let rt = tokio::runtime::Runtime::new()?;
        let static_dir = config.static_dir.clone();
        let data_dir = config.data_dir.clone();
        let app = routes::app_with_static(Arc::new(db), secret_key, static_dir, data_dir);

        if let Some(ref tls) = config.tls {
            tracing::info!(listen = %config.listen, cert = %tls.cert.display(), key = %tls.key.display(), "TLS enabled, serving HTTPS");
            let tls_acceptor = rex_hub::tls::create_tls_acceptor(&tls.cert, &tls.key)?;
            rt.block_on(rex_hub::tls::run_tls_server(
                &config.listen,
                tls_acceptor,
                app,
            ))?;
        } else {
            tracing::info!(listen = %config.listen, "HTTP only, serving without TLS");
            rt.block_on(async {
                let listener = tokio::net::TcpListener::bind(&config.listen).await?;
                axum::serve(listener, app).await?;
                Ok::<(), anyhow::Error>(())
            })?;
        }
    } else {
        app::run_from(args)?;
    }

    Ok(())
}
