//! Play-phase driver (M5 boundary).
//!
//! After the configuration handshake completes (`FinishConfigurationAck`
//! received) the server transitions the framed connection into the
//! Play state. The minimal M5 flow is:
//!
//!   S → C : [`LoginPlay`]               (player → world binding)
//!   looping:
//!     S → C : [`KeepAlive`]              (every ~15s tick)
//!     C → S : [`KeepAliveAck`]           (within 15s or kick)
//!     C → S : any play packet           (movement, chat, commands, …)
//!
//! The driver keeps the player in this loop until either side closes
//! the connection or a [`Disconnect`](crate::play::Disconnect) is
//! sent. The loop bounds the implementation surface for now: full
//! chunk streaming / worldgen / entity simulation arrives in later
//! milestones.

use anyhow::{anyhow, Result};
use pigeon_config::ServerConfig;
use pigeon_protocol::codec::PacketCodec;
use pigeon_protocol::java::play::{
    self, KeepAlive, KeepAliveAck, LoginPlay, PlayerInfoActions, PlayerInfoEntry, PlayerInfoUpdate,
};
use pigeon_protocol::ser::{PacketDecode, PacketEncode};
use pigeon_protocol::EncodedPacket;
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio_util::codec::Framed;

use futures::SinkExt;
use futures::StreamExt;

use super::player_registry::{PlayerRecord, PlayerRegistryHandle};

/// Build a `PlayerInfoUpdate` packet describing a single record (the
/// joiner) — used for both the joiner's own self-info and the
/// broadcast sent to every other online peer.
fn build_self_player_info(
    uuid: uuid::Uuid,
    name: &str,
    gamemode: i32,
    listed: bool,
) -> PlayerInfoUpdate {
    PlayerInfoUpdate {
        actions: PlayerInfoActions::empty()
            .with(PlayerInfoActions::ADD_PLAYER)
            .with(PlayerInfoActions::UPDATE_GAMEMODE)
            .with(PlayerInfoActions::UPDATE_LISTED),
        entries: vec![PlayerInfoEntry {
            uuid,
            name: Some(name.to_string()),
            has_chat_session: None,
            gamemode: Some(gamemode),
            listed: Some(listed),
            latency: None,
            has_display_name: None,
        }],
    }
}

/// Build a single `PlayerInfoUpdate` containing every record in
/// `records`. The actions bitmask covers the union of information
/// attached per entry. Returns `None` when `records` is empty.
fn build_snapshot_player_info(records: &[PlayerRecord]) -> Option<PlayerInfoUpdate> {
    if records.is_empty() {
        return None;
    }
    let entries = records
        .iter()
        .map(|r| PlayerInfoEntry {
            uuid: r.uuid,
            name: Some(r.username.clone()),
            has_chat_session: None,
            gamemode: Some(r.gamemode),
            listed: Some(r.listed),
            latency: Some(r.latency),
            has_display_name: None,
        })
        .collect();
    Some(PlayerInfoUpdate {
        actions: PlayerInfoActions::empty()
            .with(PlayerInfoActions::ADD_PLAYER)
            .with(PlayerInfoActions::UPDATE_GAMEMODE)
            .with(PlayerInfoActions::UPDATE_LISTED)
            .with(PlayerInfoActions::UPDATE_LATENCY),
        entries,
    })
}

/// Encode a typed `PacketEncode` packet into a fresh `EncodedPacket`.
fn encode_typed<T: PacketEncode>(pkt: &T, peer: SocketAddr) -> Result<EncodedPacket> {
    let mut buf = bytes::BytesMut::new();
    pkt.encode(&mut buf).map_err(|e| {
        tracing::debug!(%peer, encode_err = %e, "encode failed");
        anyhow!(e.to_string())
    })?;
    Ok(EncodedPacket::new(T::ID, buf.freeze()))
}

/// Drive the play phase for a single connection. Builds and sends the
/// [`LoginPlay`] packet, registers the joiner with the player registry
/// (broadcasting their `PlayerInfoUpdate` to every other online peer
/// and emitting a snapshot of every online peer back to the joiner),
/// then enters a keep-alive + broadcast-relay loop until the client
/// closes the socket.
///
/// `username` / `entity_id` come from the login phase; M6.5 threads the
/// `PlayerRegistryHandle` through so joining players are announced to
/// already-connected peers.
///
/// The keep-alive period is 15 seconds by default but can be
/// overridden via the `PIGEON_KEEPALIVE_SECS` env var (useful for
/// tests that want to trigger a tick quickly).
pub async fn handle_play(
    framed: &mut Framed<TcpStream, PacketCodec>,
    config: &ServerConfig,
    peer: SocketAddr,
    players: &PlayerRegistryHandle,
    username: String,
    entity_id: i32,
    player_uuid: uuid::Uuid,
) -> Result<()> {
    tracing::info!(%peer, username = %username, "play phase begin");

    let keep_alive_period = std::env::var("PIGEON_KEEPALIVE_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or(Duration::from_secs(15));

    // 1. S → C: LoginPlay with a hardcoded minimal overworld layout.
    let login = build_login_play(config, entity_id);
    write_packet(framed, &login, peer).await?;

    // 2. Build the joiner's PlayerInfoUpdate payload once — both the
    //    joiner and every other online peer receive this exact packet.
    let self_info = build_self_player_info(player_uuid, &username, 1, true);
    let joiner_packet = encode_typed(&self_info, peer)?;

    // 3. Attach the outbound broadcast channel to the registry — this
    //    fans the joiner's PlayerInfoUpdate out to every other online
    //    player and returns a snapshot of every other player so the
    //    joiner can populate its tab list.
    let (mut rx, snapshot_records) = players.attach_and_snapshot(player_uuid, joiner_packet);
    let snapshot_encoded = match build_snapshot_player_info(&snapshot_records) {
        Some(ref pkt) => Some(encode_typed(pkt, peer)?),
        None => None,
    };

    // 4. Tell the joiner about itself.
    write_packet(framed, &self_info, peer).await?;

    // 5. Tell the joiner about everyone else (if any).
    if let Some(encoded) = snapshot_encoded {
        write_encoded(framed, encoded, peer).await?;
    }

    // 6. Loop: keep-alive + broadcast relay + inbound packets.
    let mut next_keep_alive = Instant::now() + keep_alive_period;
    loop {
        let wait = next_keep_alive
            .checked_duration_since(Instant::now())
            .unwrap_or(Duration::ZERO);

        tokio::select! {
            // Inbound packet — keep-alive ack or movement/etc.
            packet = framed.next() => match packet {
                Some(Ok(packet)) => {
                    let id = packet.id;
                    tracing::debug!(%peer, id, "play inbound packet");
                    if id == <KeepAliveAck as PacketDecode>::ID {
                        if let Ok(ack) = KeepAliveAck::decode(
                            &mut std::io::Cursor::new(packet.payload),
                        ) {
                            tracing::debug!(%peer, payload = ack.payload, "keep-alive ack received");
                        }
                    }
                }
                Some(Err(err)) => {
                    tracing::debug!(%peer, %err, "decode error during play");
                    return Err(anyhow!(err.to_string()));
                }
                None => {
                    tracing::info!(%peer, "client closed play connection");
                    return Ok(());
                }
            },
            // Broadcast relay — forward any packet pushed by another
            // connection's admission flow directly to this client.
            Some(packet) = rx.recv() => {
                if let Err(err) = write_encoded(framed, packet, peer).await {
                    tracing::debug!(%peer, %err, "broadcast send failed");
                    return Err(err);
                }
            }
            // Periodic keep-alive.
            _ = tokio::time::sleep(wait) => {
                let payload = chrono_like_millis();
                let pkt = KeepAlive { payload };
                write_packet(framed, &pkt, peer).await?;
                next_keep_alive = Instant::now() + keep_alive_period;
            }
        }
    }
}

/// Build a vanilla-shaped `LoginPlay` packet. The overworld registry is
/// produced by [`pigeon_protocol::java::play::stub_overworld_registry`];
/// until M5.5 swaps that out for a real `pigeon-data`-backed codec the
/// player only sees the overworld.
fn build_login_play(config: &ServerConfig, entity_id: i32) -> LoginPlay {
    LoginPlay {
        entity_id,
        is_hardcore: config.login.online_mode,
        gamemode: 1, // survival
        previous_gamemode: -1,
        dimension_names: vec![
            "minecraft:overworld".to_string(),
            "minecraft:the_nether".to_string(),
            "minecraft:the_end".to_string(),
        ],
        registry_payload: play::stub_overworld_registry(),
        world_name: "minecraft:overworld".to_string(),
        seed_hash: 0,
        max_players: config.server.max_players as i32,
        view_distance: config.server.view_distance as i32,
        simulation_distance: config.server.simulation_distance as i32,
        reduced_debug_info: false,
        show_death_screen: true,
        do_limited_crafting: false,
    }
}

async fn write_packet<T: PacketEncode>(
    framed: &mut Framed<TcpStream, PacketCodec>,
    pkt: &T,
    peer: SocketAddr,
) -> Result<()> {
    let mut buf = bytes::BytesMut::new();
    pkt.encode(&mut buf).map_err(|e| anyhow!(e.to_string()))?;
    let encoded = pigeon_protocol::EncodedPacket::new(T::ID, buf.freeze());
    if let Err(err) = framed.send(encoded).await {
        tracing::debug!(%peer, %err, "send error during play");
        return Err(anyhow!(err.to_string()));
    }
    Ok(())
}

/// Forward an already-encoded packet (such as a broadcast relay)
/// directly through the framed sink without re-encoding it.
async fn write_encoded(
    framed: &mut Framed<TcpStream, PacketCodec>,
    packet: EncodedPacket,
    peer: SocketAddr,
) -> Result<()> {
    if let Err(err) = framed.send(packet).await {
        tracing::debug!(%peer, %err, "send error during play (encoded)");
        return Err(anyhow!(err.to_string()));
    }
    Ok(())
}

/// Best-effort UTC milliseconds, used as the keep-alive payload.
/// Avoids pulling in `chrono` at the play layer.
fn chrono_like_millis() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| d.as_millis().try_into().ok())
        .unwrap_or(0)
}
