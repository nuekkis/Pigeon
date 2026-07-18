use anyhow::Result;

pub mod server;

pub fn run() -> Result<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    runtime.block_on(async_main())
}

async fn async_main() -> Result<()> {
    tracing::info!("Pigeon server starting up");
    server::Server::new().await?.run().await
}
