//! Per-connection state machine.
//!
//! A `Connection` owns the TCP stream + codec and progresses through:
//! `Handshake → Status / Login → (Configuration → Play)`.
//!
//! Milestone M3 wires up status + login dispatch helpers. The actual
//! packet IO loop (reading from `Framed`, routing to handlers) will be
//! completed in M3.2–3.4 alongside the connection-state enum.

use anyhow::{anyhow, Result};
use pigeon_config::ServerConfig;
use pigeon_protocol::codec::PacketCodec;
use pigeon_protocol::java::{login, status, ProtocolState};
use pigeon_protocol::ser::{PacketDecode, PacketEncode};
use pigeon_protocol::{DecodedPacket, EncodedPacket};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_util::codec::Framed;

const PROTOCOL_VERSION_1_21_11: i32 = 0;
#[allow(dead_code)]
const SUPPORTED_PROTOCOL: i32 = -1;

pub struct Connection;

impl Connection {
    pub async fn handle(
        _stream: TcpStream,
        config: Arc<ServerConfig>,
        peer: SocketAddr,
    ) -> Result<()> {
        tracing::info!(%peer, "incoming connection");
        let _ = config;
        // Connection state machine arrives in M3.2–M3.4. For now we
        // only verify the codec and packet types wire up.
        Ok(())
    }
}

/// Builds a server list ping response from `config`.
#[allow(dead_code)]
pub fn build_status_response(config: &ServerConfig, players_online: u32) -> status::StatusResponse {
    use pigeon_text::Component;
    let description = Component::text(format!("{}\n{}", config.motd.line1, config.motd.line2));
    let response = status::ServerPingResponse {
        version: status::ServerPingVersion {
            name: "1.21.11".to_string(),
            protocol: PROTOCOL_VERSION_1_21_11,
        },
        players: status::ServerPingPlayers {
            max: config.server.max_players,
            online: players_online,
            sample: Vec::new(),
        },
        description,
        favicon: config.motd.favicon.clone(),
        enforce_secure_chat: Some(false),
    };
    let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string());
    status::StatusResponse {
        json_response: json,
    }
}

/// Routes a decoded packet in the Status state to its reply (if any).
#[allow(dead_code)]
pub fn route_status(
    packet: DecodedPacket,
    config: &ServerConfig,
    players_online: u32,
) -> Result<Option<EncodedPacket>> {
    match packet.id {
        status::StatusRequest::ID => {
            let response = build_status_response(config, players_online);
            let mut buf = bytes::BytesMut::new();
            response
                .encode(&mut buf)
                .map_err(|e| anyhow!(e.to_string()))?;
            Ok(Some(EncodedPacket::new(
                status::StatusResponse::ID,
                buf.freeze(),
            )))
        }
        status::PingRequest::ID => {
            let mut reader = std::io::Cursor::new(packet.payload);
            let req =
                status::PingRequest::decode(&mut reader).map_err(|e| anyhow!(e.to_string()))?;
            let mut buf = bytes::BytesMut::new();
            let resp = status::PongResponse {
                payload: req.payload,
            };
            resp.encode(&mut buf).map_err(|e| anyhow!(e.to_string()))?;
            Ok(Some(EncodedPacket::new(
                status::PongResponse::ID,
                buf.freeze(),
            )))
        }
        _ => Ok(None),
    }
}

/// Routes a Login Start packet. Returns `Some(LoginStart)` if the inbound
/// packet is a login start, `None` otherwise.
#[allow(dead_code)]
pub fn route_login(packet: DecodedPacket) -> Result<Option<login::LoginStart>> {
    if packet.id != login::LoginStart::ID {
        return Ok(None);
    }
    let mut reader = std::io::Cursor::new(packet.payload);
    let login_start = login::LoginStart::decode(&mut reader).map_err(|e| anyhow!(e.to_string()))?;
    Ok(Some(login_start))
}

/// Returns the next state given the handshake's `next_state` field.
#[allow(dead_code)]
pub fn next_state_from(next: status::NextState) -> ProtocolState {
    match next {
        status::NextState::Status => ProtocolState::Status,
        status::NextState::Login => ProtocolState::Login,
    }
}

/// Marker so callers wanting a typed `Framed` handle keep referring to the
/// same codec facade as the implementation evolves.
#[allow(dead_code)]
pub type WireConnection = Framed<TcpStream, PacketCodec>;
