use anyhow::Result;

pub mod server;

pub fn run() -> Result<()> {
    init_tracing();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    runtime.block_on(async_main())
}

fn init_tracing() {
    let _ = tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init();
    eprintln!("[pigeon] tracing initialized");
}

async fn async_main() -> Result<()> {
    tracing::info!("Pigeon server starting up");
    server::Server::new().await?.run().await
}
