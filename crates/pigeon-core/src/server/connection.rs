//! Per-connection state machine.
//!
//! A `Connection` owns the TCP stream + codec and progresses through:
//! `Handshake → Status / Login → (Configuration → Play)`.
//!
//! M3 wires up Handshake + Status end-to-end so a Minecraft 1.21.11
//! client can perform a server-list ping and see the configured motd.
//! M4 adds the offline (online-mode=false) Login sequence:
//! `LoginStart → SetCompression (optional) → LoginSuccess →
//! LoginAcknowledged → Configuration`. The encryption request path
//! (online-mode=true) waits on the Mojang session-server integration
//! that lives in a later milestone.

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

use futures::SinkExt;
use futures::StreamExt;

/// Protocol version the server presents on status pings. The real
/// value for 1.21.11 is determined by `pigeon-data::protocol_version()`;
/// the placeholder here is replaced once the data report cross-check
/// for the live `packets.json` lands.
const PROTOCOL_VERSION_1_21_11: i32 = 0;

/// Drop-in helper to push an `EncodedPacket` built from an `id` + raw
/// body through the framed stream, returning any send error.
async fn send_packet(
    framed: &mut Framed<TcpStream, PacketCodec>,
    encoded: EncodedPacket,
    peer: SocketAddr,
    state: ProtocolState,
) -> Result<()> {
    if let Err(err) = framed.send(encoded).await {
        tracing::debug!(%peer, state = state.as_str(), %err, "send error");
        return Err(anyhow!(err.to_string()));
    }
    Ok(())
}

/// Encode a typed `PacketEncode` packet into a fresh `EncodedPacket`.
fn encode_packet<T: PacketEncode>(pkt: &T, peer: SocketAddr) -> Result<EncodedPacket> {
    let mut buf = bytes::BytesMut::new();
    pkt.encode(&mut buf).map_err(|e| {
        tracing::debug!(%peer, encode_err = %e, "encode failed");
        anyhow!(e.to_string())
    })?;
    Ok(EncodedPacket::new(T::ID, buf.freeze()))
}

pub struct Connection;

impl Connection {
    pub async fn handle(
        stream: TcpStream,
        config: Arc<ServerConfig>,
        peer: SocketAddr,
        players: super::player_registry::PlayerRegistryHandle,
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

            match state {
                ProtocolState::Handshake => {
                    handle_handshake(packet, &mut state)?;
                }
                ProtocolState::Status => {
                    let online = players.online() as u32;
                    if let Some(encoded) = handle_status(packet, &config, online)? {
                        send_packet(&mut framed, encoded, peer, state).await?;
                    }
                }
                ProtocolState::Login => {
                    // The login sequence spans multiple packets and may
                    // need to flip compression on the codec mid-flow, so
                    // it gets its own async sub-handler that finishes
                    // when the client either transitions to Configuration
                    // or disconnects. It also returns the username so the
                    // configuration/play phases can carry it forward.
                    let (next, record) =
                        handle_login(packet, &config, &mut framed, peer, &players).await?;
                    state = next;
                    if state == ProtocolState::Configuration {
                        // Drive the configuration handshake to completion.
                        if let Err(err) = super::configuration_flow::handle_configuration(
                            &mut framed,
                            &config,
                            peer,
                            record
                                .as_ref()
                                .map(|r| r.username.clone())
                                .unwrap_or_default(),
                        )
                        .await
                        {
                            tracing::debug!(%peer, %err, "configuration handshake failed");
                            return Err(err);
                        }
                        // Configuration succeeded — drive the play phase.
                        let record =
                            record.expect("configuration path requires an admitted player record");
                        let entity_id = record.entity_id;
                        let player_uuid = record.uuid;
                        if let Err(err) = super::play_flow::handle_play(
                            &mut framed,
                            &config,
                            peer,
                            record.username,
                            entity_id,
                            player_uuid,
                        )
                        .await
                        {
                            tracing::debug!(%peer, %err, "play phase ended with err");
                            // Player departed — remove from registry.
                            players.depart(&player_uuid);
                            return Err(err);
                        }
                        tracing::info!(%peer, "play phase complete");
                        players.depart(&player_uuid);
                        return Ok(());
                    }
                }
                ProtocolState::Configuration => {
                    // Reached only if a non-Login path somehow lands
                    // here; treat as terminal.
                    tracing::debug!(%peer, "unexpected configuration state after non-login path");
                    return Ok(());
                }
                ProtocolState::Play => {
                    // `handle_play` is invoked synchronously after the
                    // configuration handshake from the Login branch, so
                    // we never expect to dispatch an inbound packet
                    // directly into Play here. If we do, log and close.
                    tracing::debug!(%peer, "play packet reached outer dispatcher");
                    return Ok(());
                }
            }
        }
    }
}

/// Decode the Handshake packet, then transition to the next state it
/// requests. No outbound packet is emitted in response.
fn handle_handshake(packet: DecodedPacket, state: &mut ProtocolState) -> Result<()> {
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

    Ok(())
}

/// Handle the two-packet Status phase: StatusRequest then PingRequest.
fn handle_status(
    packet: DecodedPacket,
    config: &ServerConfig,
    players_online: u32,
) -> Result<Option<EncodedPacket>> {
    route_status(packet, config, players_online)
}

/// Login-phase driver. Handles the offline (online-mode=false) flow:
///
/// 1. read `LoginStart`
/// 2. send `SetCompression` (if `config.network.compression_threshold >= 0`) and
///    flip the codec's compression on
/// 3. send `LoginSuccess` carrying the player's name + uuid
/// 4. read `LoginAcknowledged` from the client
/// 5. return `ProtocolState::Configuration`
///
/// Online-mode=true will require the `EncryptionRequest`/`EncryptionResponse`
/// round-trip + Mojang `sessionserver/minecraft/join` lookup, which lands
/// alongside the player profile cache in M6+.
async fn handle_login(
    first_packet: DecodedPacket,
    config: &ServerConfig,
    framed: &mut Framed<TcpStream, PacketCodec>,
    peer: SocketAddr,
    players: &super::player_registry::PlayerRegistryHandle,
) -> Result<(ProtocolState, Option<super::player_registry::PlayerRecord>)> {
    // Step 1: must be a LoginStart.
    if first_packet.id != login::LoginStart::ID {
        tracing::debug!(%peer, id = first_packet.id, "expected login start");
        // Send a login disconnect and bail.
        let _ = send_login_disconnect(framed, peer, "Expected LoginStart.").await;
        return Ok((ProtocolState::Login, None));
    }
    let mut cursor = std::io::Cursor::new(first_packet.payload);
    let login_start = login::LoginStart::decode(&mut cursor).map_err(|e| anyhow!(e.to_string()))?;
    tracing::info!(
        username = %login_start.name,
        uuid = %login_start.uuid,
        online_mode = config.login.online_mode,
        "login start received"
    );

    // For now only the offline path is implemented. When online_mode is
    // enabled the connection is dropped with a clear log line until the
    // Mojang sessionserver integration lands.
    if config.login.online_mode {
        tracing::warn!(%peer, "online-mode=true is not supported yet (M6+); disconnecting");
        let _ = send_login_disconnect(
            framed,
            peer,
            "PigeonMC online-mode is not yet implemented. Set online_mode=false in your config.",
        )
        .await;
        return Ok((ProtocolState::Login, None));
    }

    // Step 2: optional SetCompression. We always emit it when the
    // configured threshold is non-negative, then flip the codec so all
    // subsequent frames are subject to the threshold.
    let threshold = config.network.compression_threshold;
    if threshold >= 0 {
        tracing::debug!(%peer, threshold, "sending set compression");
        let pkt = login::SetCompression { threshold };
        let enc = encode_packet(&pkt, peer)?;
        send_packet(framed, enc, peer, ProtocolState::Login).await?;
        framed.codec_mut().set_compression(threshold);
    }

    // Step 3: LoginSuccess with the player's name + uuid (no Mojang
    // properties for offline mode).
    tracing::info!(%peer, username = %login_start.name, "login success sent (offline)");
    let success = login::LoginSuccess {
        uuid: login_start.uuid,
        username: login_start.name.clone(),
        properties: Vec::new(),
    };
    let enc = encode_packet(&success, peer)?;
    send_packet(framed, enc, peer, ProtocolState::Login).await?;

    // Step 4: wait for the client to acknowledge by sending
    // `LoginAcknowledged` (id 0x03 in login state). The client may also
    // send a LoginPluginResponse (id 0x02) or CookieResponse (id 0x04)
    // if a plugin request was outstanding — we just log and keep
    // reading until we see the ack.
    loop {
        let next = match framed.next().await {
            Some(Ok(packet)) => packet,
            Some(Err(err)) => {
                tracing::debug!(%peer, %err, "decode error while awaiting LoginAcknowledged");
                return Err(anyhow!(err.to_string()));
            }
            None => {
                tracing::debug!(%peer, "client closed during login ack wait");
                return Ok((ProtocolState::Login, None));
            }
        };
        tracing::debug!(%peer, id = next.id, "post-login packet");
        if next.id == login::LoginAcknowledged::ID {
            tracing::info!(%peer, "login acknowledged — transition to configuration");
            // Admit the player to the registry; the returned record
            // carries the assigned entity_id + default gamemode.
            let entity_id = next_entity_id();
            let record = players.admit(login_start.uuid, login_start.name, entity_id);
            tracing::info!(%peer, uuid = %record.uuid, entity_id, "player admitted to registry");
            return Ok((ProtocolState::Configuration, Some(record)));
        }
        // Any other packet: log + keep waiting.
        tracing::debug!(%peer, id = next.id, "ignoring while awaiting LoginAcknowledged");
    }
}

/// Send a `DisconnectLogin` carrying `reason_json` (a Minecraft chat JSON
/// string) and flush.
async fn send_login_disconnect(
    framed: &mut Framed<TcpStream, PacketCodec>,
    peer: SocketAddr,
    reason: &str,
) -> Result<()> {
    tracing::info!(%peer, reason, "login disconnect");
    let payload = serde_json::json!({ "text": reason }).to_string();
    let pkt = login::DisconnectLogin {
        reason_json: payload,
    };
    let enc = encode_packet(&pkt, peer)?;
    send_packet(framed, enc, peer, ProtocolState::Login).await
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

/// Process-global allocator for the player entity ids passed to
/// `LoginPlay`. A real player registry (M6) will own this, but the
/// simple counter is enough to keep play-side semantics consistent.
static NEXT_ENTITY_ID: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(1);

fn next_entity_id() -> i32 {
    NEXT_ENTITY_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}
