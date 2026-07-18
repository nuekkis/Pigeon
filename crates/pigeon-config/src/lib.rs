pub mod schema;

pub use schema::ServerConfig;

use anyhow::Result;

pub fn load_default() -> Result<ServerConfig> {
    ServerConfig::default_config()
}
