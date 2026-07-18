use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub server: ServerSection,
    pub network: NetworkSection,
    pub motd: MotdSection,
    pub login: LoginSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSection {
    pub name: String,
    pub max_players: u32,
    pub default_gamemode: String,
    pub view_distance: u32,
    pub simulation_distance: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSection {
    pub bind_address: String,
    pub port: u16,
    pub compression_threshold: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotdSection {
    pub line1: String,
    pub line2: String,
    pub favicon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginSection {
    pub online_mode: bool,
    pub prevent_proxy_connections: bool,
}

impl ServerConfig {
    pub fn default_config() -> anyhow::Result<Self> {
        Ok(Self::default())
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            server: ServerSection {
                name: "Pigeon Server".to_string(),
                max_players: 20,
                default_gamemode: "survival".to_string(),
                view_distance: 10,
                simulation_distance: 10,
            },
            network: NetworkSection {
                bind_address: "0.0.0.0".to_string(),
                port: 25565,
                compression_threshold: 256,
            },
            motd: MotdSection {
                line1: "A Pigeon Server".to_string(),
                line2: "Powered by PigeonMC".to_string(),
                favicon: None,
            },
            login: LoginSection {
                online_mode: true,
                prevent_proxy_connections: false,
            },
        }
    }
}
