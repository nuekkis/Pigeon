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
//!
//! Wire ids are kept in sync with `pigeon-data`'s embedded `packets.json`
//! report — see [`crate::java::ids::status`] for the canonical resource
//! locations and `cargo test -p pigeon-protocol` for the regression check.

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
    /// `minecraft:intention` — the only Handshake-state packet (verifiable via
    /// [`crate::java::ids`]).
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

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    /// Helper: decode a packet body from a slice.
    fn decode_from_bytes<T: PacketDecode>(bytes: &[u8]) -> Result<T, PacketSerError> {
        let mut buf = bytes;
        T::decode(&mut buf)
    }

    #[test]
    fn handshake_decodes_status_intention() {
        // VarInt(765) + String("127.0.0.1") + u16(25565 BE) + VarInt(1=status)
        let mut wire = Vec::new();
        pigeon_codecs::write_var_int(765, &mut wire).unwrap();
        let mut s = BytesMut::new();
        crate::ser::write_string("127.0.0.1", &mut s, 255).unwrap();
        wire.extend_from_slice(&s);
        wire.push(0x63); // 25565 split: high byte 0x63 (99)
        wire.push(0xDD); // low byte 0xDD
        pigeon_codecs::write_var_int(1, &mut wire).unwrap();
        let decoded = decode_from_bytes::<HandshakeInt>(&wire).expect("decode must succeed");
        assert_eq!(decoded.protocol_version, 765);
        assert_eq!(decoded.server_address, "127.0.0.1");
        assert_eq!(decoded.server_port, 25565);
        assert_eq!(decoded.next_state, NextState::Status);
    }

    #[test]
    fn handshake_decodes_login_intention() {
        // VarInt(-3) + String("localhost") + u16(25565 BE) + VarInt(2=login)
        let mut wire = Vec::new();
        pigeon_codecs::write_var_int(-3, &mut wire).unwrap();
        let mut s = BytesMut::new();
        crate::ser::write_string("localhost", &mut s, 255).unwrap();
        wire.extend_from_slice(&s);
        wire.push(0x63);
        wire.push(0xDD);
        pigeon_codecs::write_var_int(2, &mut wire).unwrap();
        let decoded = decode_from_bytes::<HandshakeInt>(&wire).expect("decode must succeed");
        assert_eq!(decoded.protocol_version, -3);
        assert_eq!(decoded.server_address, "localhost");
        assert_eq!(decoded.next_state, NextState::Login);
    }

    #[test]
    fn handshake_rejects_unknown_next_state() {
        let mut wire = Vec::new();
        pigeon_codecs::write_var_int(1, &mut wire).unwrap();
        let mut s = BytesMut::new();
        crate::ser::write_string("h", &mut s, 255).unwrap();
        wire.extend_from_slice(&s);
        wire.push(0);
        wire.push(0);
        pigeon_codecs::write_var_int(99, &mut wire).unwrap();
        let err =
            decode_from_bytes::<HandshakeInt>(&wire).expect_err("must reject invalid next_state");
        assert!(matches!(err, PacketSerError::InvalidValue), "got {err:?}");
    }

    #[test]
    fn status_request_decodes_empty_body() {
        let decoded = decode_from_bytes::<StatusRequest>(&[]).expect("decode must succeed");
        let _ = decoded;
    }

    #[test]
    fn status_response_encodes_json_payload() {
        let original = StatusResponse {
            json_response: "{\"version\":{\"name\":\"1.21.11\",\"protocol\":765}}".to_string(),
        };
        let mut buf = BytesMut::new();
        PacketEncode::encode(&original, &mut buf).expect("encode must succeed");
        let bytes = buf.freeze();
        assert!(!bytes.is_empty(), "body must not be empty");
        // The body is a VarInt prefix (length) followed by the UTF-8 string.
        let mut reader = bytes.as_ref();
        let s = crate::ser::read_string(&mut reader, 32767).expect("must read string back");
        assert_eq!(s, original.json_response);
        assert!(reader.is_empty(), "no trailing bytes after string");
    }

    #[test]
    fn ping_request_decodes_payload() {
        let payload: u64 = 0xDEADBEEFCAFEBABE;
        let wire = payload.to_be_bytes();
        let decoded = decode_from_bytes::<PingRequest>(&wire).expect("decode must succeed");
        assert_eq!(decoded.payload, payload);
    }

    #[test]
    fn ping_request_rejects_short_body() {
        let err = decode_from_bytes::<PingRequest>(&[0, 0, 0]).expect_err("short body must fail");
        assert!(matches!(err, PacketSerError::Underflow), "got {err:?}");
    }

    #[test]
    fn pong_response_encodes_payload_be() {
        let original = PongResponse {
            payload: 0x0102030405060708,
        };
        let mut buf = BytesMut::new();
        PacketEncode::encode(&original, &mut buf).expect("encode must succeed");
        let bytes = buf.freeze();
        assert_eq!(bytes.len(), 8, "pong body must be exactly 8 bytes");
        assert_eq!(&*bytes, &original.payload.to_be_bytes());
    }

    #[test]
    fn status_table_wire_ids_match_data_report() {
        // Cross-check the hardcoded ID consts against the canonical
        // resource locations in `pigeon-data`'s embedded report.
        assert_eq!(
            pigeon_data::packets::serverbound_id(
                "status",
                crate::java::ids::status::STATUS_REQUEST
            ),
            Some(StatusRequest::ID),
        );
        assert_eq!(
            pigeon_data::packets::clientbound_id(
                "status",
                crate::java::ids::status::STATUS_RESPONSE
            ),
            Some(StatusResponse::ID),
        );
        assert_eq!(
            pigeon_data::packets::serverbound_id("status", crate::java::ids::status::PING_REQUEST),
            Some(PingRequest::ID),
        );
        assert_eq!(
            pigeon_data::packets::clientbound_id("status", crate::java::ids::status::PONG_RESPONSE),
            Some(PongResponse::ID),
        );
        // The HandshakeIntention packet lives in the handshake phase.
        assert_eq!(
            pigeon_data::packets::serverbound_id(
                "handshake",
                crate::java::ids::handshake::INTENTION
            ),
            Some(HandshakeInt::ID),
        );
    }
}
