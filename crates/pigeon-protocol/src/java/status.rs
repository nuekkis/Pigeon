//! Status-state packets implementing the server list ping handshake.
//!
//! Packet IDs (serverbound → clientbound from the *server*'s POV):
//!
//! | Direction | ID   | Name                    |
//! |-----------|------|-------------------------|
//! | C → S     | 0x00 | HandshakeInt            |
//! | C → S     | 0x00 | StatusRequest   |
//! | S → C     | 0x00 | StatusResponse |
//! | C → S     | 0x01 | PingRequest         |
//! | S → C     | 0x01 | PongResponse        |
//!
//! Note: `HandshakeInt` is technically the very first packet of every
//! connection (sent in the Handshake state) and is reused below.

use bytes::{Buf, BufMut};
use serde::{Deserialize, Serialize};

use crate::ser::{PacketDecode, PacketEncode, PacketSerError};

// ---------------------------------------------------------------------------
// C → S : Handshake (state = Handshake, id = 0x00)
// ---------------------------------------------------------------------------

/// First packet sent by the client to negotiate the protocol version,
/// server address, port, and the next state to transition to.
#[derive(Debug, Clone)]
pub struct HandshakeInt {
    pub protocol_version: i32,
    pub server_address: String,
    pub server_port: u16,
    pub next_state: NextState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NextState {
    Status = 1,
    Login = 2,
}

impl PacketDecode for HandshakeInt {
    const ID: i32 = 0x00;

    fn decode<B: Buf>(buf: &mut B) -> Result<Self, PacketSerError> {
        let protocol_version = pigeon_codecs::read_var_int(buf)?;
        let server_address = crate::ser::read_string(buf, 255)?;
        if buf.remaining() < 2 {
            return Err(PacketSerError::Underflow);
        }
        let server_port = buf.get_u16();
        let next_state_raw = pigeon_codecs::read_var_int(buf)?;
        let next_state = match next_state_raw {
            1 => NextState::Status,
            2 => NextState::Login,
            _ => return Err(PacketSerError::InvalidValue),
        };
        Ok(Self {
            protocol_version,
            server_address,
            server_port,
            next_state,
        })
    }
}

// ---------------------------------------------------------------------------
// C → S : Status Request (state = Status, id = 0x00 — empty body)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct StatusRequest;

impl PacketDecode for StatusRequest {
    const ID: i32 = 0x00;

    fn decode<B: Buf>(_buf: &mut B) -> Result<Self, PacketSerError> {
        Ok(Self)
    }
}

// ---------------------------------------------------------------------------
// S → C : Status Response (state = Status, id = 0x00)
// ---------------------------------------------------------------------------

/// Server list ping response. Carries a JSON payload matching the legacy
/// "Server List Ping" v1.21 protocol.
#[derive(Debug, Clone, Serialize)]
pub struct StatusResponse {
    /// JSON body identical to the vanilla format.
    pub json_response: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerPingResponse {
    pub version: ServerPingVersion,
    pub players: ServerPingPlayers,
    pub description: pigeon_text::Component,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favicon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enforce_secure_chat: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerPingVersion {
    pub name: String,
    pub protocol: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerPingPlayers {
    pub max: u32,
    pub online: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sample: Vec<ServerPingPlayer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerPingPlayer {
    pub name: String,
    pub id: uuid::Uuid,
}

impl PacketEncode for StatusResponse {
    const ID: i32 = 0x00;

    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<(), PacketSerError> {
        crate::ser::write_string(&self.json_response, buf, 32767)
    }
}

// ---------------------------------------------------------------------------
// C → S : Ping Request (state = Status, id = 0x01)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct PingRequest {
    pub payload: u64,
}

impl PacketDecode for PingRequest {
    const ID: i32 = 0x01;

    fn decode<B: Buf>(buf: &mut B) -> Result<Self, PacketSerError> {
        if buf.remaining() < 8 {
            return Err(PacketSerError::Underflow);
        }
        Ok(Self {
            payload: buf.get_u64(),
        })
    }
}

// ---------------------------------------------------------------------------
// S → C : Pong Response (state = Status, id = 0x01)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct PongResponse {
    pub payload: u64,
}

impl PacketEncode for PongResponse {
    const ID: i32 = 0x01;

    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<(), PacketSerError> {
        if buf.remaining_mut() < 8 {
            return Err(PacketSerError::Overflow);
        }
        buf.put_u64(self.payload);
        Ok(())
    }
}
