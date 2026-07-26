//! Per-connection state machine.
//!
//! A `Connection` owns the TCP stream + codec and progresses through:
//! `Handshake → Status / Login → (Configuration → Play)`.
//!
//! This milestone (M3) wires up Handshake + Status end-to-end so a
//! Minecraft 1.21.11 client can perform a server-list ping and see the
//! configured motd. Login + Configuration + Play handlers are seeded
//! as TODO markers for M4+.

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

/// Protocol version the server presents on status pings.
/// The actual wire id is determined by `pigeon-data::protocol_version()`.
/// For 1.21.11 this still resolves to a placeholder value until we
/// cross-check against `packets.json`; vanilla clients tolerate `-1`
/// by showing "can't connect" but use the version string for display.
const PROTOCOL_VERSION_1_21_11: i32 = 0;

/// Players reported in the status ping. M3 reports `0/0`; the real
/// count is wired in M6 alongside the player registry.
const STATUS_PLAYERS_ONLINE: u32 = 0;

pub struct Connection;

impl Connection {
    pub async fn handle(
        stream: TcpStream,
        config: Arc<ServerConfig>,
        peer: SocketAddr,
    ) -> Result<()> {
        tracing::info!(%peer, "incoming connection");

        let codec = PacketCodec::new();
        let mut framed = Framed::new(stream, codec);

        // Track the current state locally. The codec itself is
        // state-agnostic; routing decisions live here.
        let mut state: ProtocolState = ProtocolState::Handshake;

        loop {
            // Read the next inbound packet from the framed stream.
            let packet = match framed.next().await {
                Some(Ok(packet)) => packet,
                None => {
                    tracing::debug!(%peer, state = state.as_str(), "client closed connection");
                    return Ok(());
                }
                Some(Err(err)) => {
                    tracing::debug!(%peer, state = state.as_str(), %err, "decode error");
                    return Err(anyhow!(err.to_string()));
                }
            };

            tracing::debug!(%peer, state = state.as_str(), id = packet.id, len = packet.payload.len(), "inbound packet");

            // Dispatch based on the current state.
            let reply = match state {
                ProtocolState::Handshake => handle_handshake(packet, &mut state)?,
                ProtocolState::Status => handle_status(packet, &config, &mut state)?,
                ProtocolState::Login => handle_login(packet, &config, &mut state)?,
                ProtocolState::Configuration => {
                    tracing::debug!(%peer, "configuration not wired yet");
                    return Ok(());
                }
                ProtocolState::Play => {
                    tracing::debug!(%peer, "play not wired yet");
                    return Ok(());
                }
            };

            if let Some(encoded) = reply {
                if let Err(err) = framed.send(encoded).await {
                    tracing::debug!(%peer, state = state.as_str(), %err, "send error");
                    return Err(anyhow!(err.to_string()));
                }
            }
        }
    }
}

use futures::SinkExt;
use futures::StreamExt;

/// Result of dispatching one packet to a handler.
type HandlerReply = Option<EncodedPacket>;

/// Decode the Handshake packet, then transition to the next state it
/// requests. No outbound packet is emitted in response.
fn handle_handshake(packet: DecodedPacket, state: &mut ProtocolState) -> Result<HandlerReply> {
    if packet.id != status::HandshakeInt::ID {
        return Err(anyhow!(
            "expected handshake packet (id 0x00) in Handshake state, got 0x{:02x}",
            packet.id
        ));
    }
    let mut cursor = std::io::Cursor::new(packet.payload);
    let handshake =
        status::HandshakeInt::decode(&mut cursor).map_err(|e| anyhow!(e.to_string()))?;

    tracing::info!(
        protocol_version = handshake.protocol_version,
        server_address = %handshake.server_address,
        server_port = handshake.server_port,
        "handshake received"
    );

    *state = match handshake.next_state {
        status::NextState::Status => ProtocolState::Status,
        status::NextState::Login => ProtocolState::Login,
    };

    Ok(None)
}

/// Handle the two-packet Status phase: StatusRequest then PingRequest.
/// After responding to Ping the connection should close.
fn handle_status(
    packet: DecodedPacket,
    config: &ServerConfig,
    state: &mut ProtocolState,
) -> Result<HandlerReply> {
    tracing::debug!(id = packet.id, "status packet received");
    let pid = packet.id;
    let reply = route_status(packet, config, STATUS_PLAYERS_ONLINE)?;
    if reply.is_some() {
        tracing::debug!(id = pid, "status replied");
    }
    let _ = state;
    Ok(reply)
}

/// Login-phase entry. M3 only logs the inbound packet; the full
/// encryption + profile exchange + Login Success / Set Compression /
/// Transition-to-Configuration sequence is wired in M4.
fn handle_login(
    packet: DecodedPacket,
    _config: &ServerConfig,
    state: &mut ProtocolState,
) -> Result<HandlerReply> {
    if packet.id == login::LoginStart::ID {
        let mut cursor = std::io::Cursor::new(packet.payload);
        let login_start =
            login::LoginStart::decode(&mut cursor).map_err(|e| anyhow!(e.to_string()))?;
        tracing::info!(
            username = %login_start.name,
            uuid = %login_start.uuid,
            "login start received (M4 will complete login)"
        );
        // For M3 we simply close the connection. The client will see
        // "Connection lost" until M4 emits Disconnect or LoginSuccess.
        *state = ProtocolState::Login;
        return Ok(None);
    }
    tracing::debug!(
        id = packet.id,
        "ignoring non-LoginStart packet in Login state (M4)"
    );
    Ok(None)
}

/// Builds a server list ping response from `config`.
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
