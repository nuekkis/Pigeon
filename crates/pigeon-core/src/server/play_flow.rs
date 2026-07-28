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
use pigeon_protocol::java::play::{self, KeepAlive, KeepAliveAck, LoginPlay};
use pigeon_protocol::ser::{PacketDecode, PacketEncode};
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio_util::codec::Framed;

use futures::SinkExt;
use futures::StreamExt;

/// Drive the play phase for a single connection. Builds and sends the
/// [`LoginPlay`] packet, then enters a keep-alive loop until the
/// client closes the socket.
///
/// `username` / `entity_id` come from the login phase; their full
/// propagation through a player registry arrives in M6. Until then
/// the play loop simply keeps the client attached to verify the
/// boundary.
///
/// The keep-alive period is 15 seconds by default but can be
/// overridden via the `PIGEON_KEEPALIVE_SECS` env var (useful for
/// tests that want to trigger a tick quickly).
pub async fn handle_play(
    framed: &mut Framed<TcpStream, PacketCodec>,
    config: &ServerConfig,
    peer: SocketAddr,
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

    // 1b. S → C: PlayerInfoUpdate — admit the joining player to the
    //     tab list. M6 wires the broadcast to *other* players; here we
    //     only inform the joiner so it sees itself listed.
    let player_info = play::PlayerInfoUpdate {
        actions: play::PlayerInfoActions::empty()
            .with(play::PlayerInfoActions::ADD_PLAYER)
            .with(play::PlayerInfoActions::UPDATE_GAMEMODE)
            .with(play::PlayerInfoActions::UPDATE_LISTED),
        entries: vec![play::PlayerInfoEntry {
            uuid: player_uuid,
            name: Some(username.clone()),
            has_chat_session: None,
            gamemode: Some(1),
            listed: Some(true),
            latency: None,
            has_display_name: None,
        }],
    };
    write_packet(framed, &player_info, peer).await?;

    // 2. Periodic keep-alive loop. We tolerate any inbound packet here
    //    ("movement"), only acting on `KeepAliveAck` and silently
    //    logging unknown ids until M6+ plumbs more handlers.
    let mut next_keep_alive = Instant::now() + keep_alive_period;
    loop {
        // Calculate how long we should wait for the next inbound
        // packet before issuing the next keep-alive.
        let wait = next_keep_alive
            .checked_duration_since(Instant::now())
            .unwrap_or(Duration::ZERO);

        match tokio::time::timeout(wait, framed.next()).await {
            Ok(Some(Ok(packet))) => {
                let id = packet.id;
                tracing::debug!(%peer, id, "play inbound packet");
                if id == <KeepAliveAck as PacketDecode>::ID {
                    if let Ok(ack) = KeepAliveAck::decode(&mut std::io::Cursor::new(packet.payload))
                    {
                        tracing::debug!(%peer, payload = ack.payload, "keep-alive ack received");
                    }
                }
                // Any other packet is acknowledged-but-ignored.
            }
            Ok(Some(Err(err))) => {
                tracing::debug!(%peer, %err, "decode error during play");
                return Err(anyhow!(err.to_string()));
            }
            Ok(None) => {
                tracing::info!(%peer, "client closed play connection");
                return Ok(());
            }
            Err(_) => {
                // Keep-alive tick elapsed. Send the next KeepAlive.
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
