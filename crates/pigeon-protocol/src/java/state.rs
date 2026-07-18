use serde::{Deserialize, Serialize};

/// The four top-level states of a Minecraft Java connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProtocolState {
    /// Initial handshake: client selects the next state (status or login).
    Handshake,
    /// Server list ping + pong.
    Status,
    /// Login sequence: encryption, compression, profile exchange.
    Login,
    /// Configuration phase: registry sync, brand, resource pack prompt.
    Configuration,
    /// Play phase: gameplay packets.
    Play,
}

impl ProtocolState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Handshake => "handshake",
            Self::Status => "status",
            Self::Login => "login",
            Self::Configuration => "configuration",
            Self::Play => "play",
        }
    }
}

impl Default for ProtocolState {
    fn default() -> Self {
        Self::Handshake
    }
}
