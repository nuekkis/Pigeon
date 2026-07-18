//! Core server logic: TCP accept loop + per-connection state machine.

mod connection;

use anyhow::Result;
use pigeon_config::ServerConfig;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

pub use connection::Connection;

/// Top-level server handle.
pub struct Server {
    config: Arc<ServerConfig>,
    listener: TcpListener,
}

impl Server {
    pub async fn new() -> Result<Self> {
        let config = Arc::new(pigeon_config::load_default()?);
        let addr: SocketAddr =
            format!("{}:{}", config.network.bind_address, config.network.port).parse()?;
        tracing::info!(%addr, "starting Pigeon server");
        let listener = TcpListener::bind(addr).await?;
        Ok(Self { config, listener })
    }

    pub async fn run(self) -> Result<()> {
        let config = self.config.clone();
        loop {
            let (stream, peer) = match self.listener.accept().await {
                Ok(pair) => pair,
                Err(err) => {
                    tracing::error!(%err, "accept failed");
                    continue;
                }
            };
            tracing::debug!(%peer, "incoming connection");
            let cfg = config.clone();
            tokio::spawn(async move {
                if let Err(err) = Connection::handle(stream, cfg, peer).await {
                    tracing::debug!(%peer, %err, "connection closed");
                }
            });
        }
    }
}

/// Maximum number of bytes the server will read from any single handshake
/// packet before dropping the connection (vanilla uses 2 MiB).
pub const MAX_HANDSHAKE_SIZE: usize = 2 * 1024 * 1024;
