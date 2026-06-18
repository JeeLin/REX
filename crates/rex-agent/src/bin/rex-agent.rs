use rex_common::app;

fn main() -> anyhow::Result<()> {
    app::init_tracing();
    tracing::info!("rex-agent starting");
    app::run()
}
