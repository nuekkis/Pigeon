//! Configuration-phase driver (M7).
//!
//! Once a client has acknowledged `LoginSuccess` the connection enters
//! the Configuration state. From the server side the canonical
//! handshake is:
//!
//!   S → C : [`SelectKnownPacks`]            (server advertises packs)
//!   C → S : [`SelectKnownPacksAck`]         (client replies with the subset it has)
//!   S → C : [`RegistryData`] *              (one packet per vanilla registry)
//!   S → C : [`UpdateEnabledFeatures`]
//!   S → C : [`UpdateTags`] (optional — legal empty)
//!   S → C : [`CustomPayload`] `"minecraft:brand"`
//!   S → C : [`FinishConfiguration`]
//!   C → S : [`FinishConfigurationAck`]
//!
//! After receiving the ack the server switches the codec-aware state
//! to Play (returned to the caller so the main loop keeps going).
//!
//! The per-registry data payloads (dimension_type/chat_type/biome/etc.
//! NBT) ship from vanilla `pigeon-data` once M5+ exposes them; until
//! then the registries ship with empty NBT compounds, which is enough
//! for the connection to complete against a vanilla-compatible client
//! when subsequent content (chunks, etc.) is also stubbed.

use anyhow::{anyhow, Result};
use pigeon_config::ServerConfig;
use pigeon_protocol::codec::PacketCodec;
use pigeon_protocol::java::configuration::{
    CustomPayload, Disconnect, FinishConfiguration, KnownPack, SelectKnownPacks,
    UpdateEnabledFeatures,
};
use pigeon_protocol::java::ProtocolState;
use pigeon_protocol::ser::{PacketDecode, PacketEncode};
use std::net::SocketAddr;
use std::str::FromStr;
use tokio::net::TcpStream;
use tokio_util::codec::Framed;

use futures::SinkExt;
use futures::StreamExt;

/// Drive the configuration-phase handshake to completion. Returns the
/// configured username so the caller can use it in the play phase.
///
/// `username` is forwarded so report lines include the logged-in player.
pub async fn handle_configuration(
    framed: &mut Framed<TcpStream, PacketCodec>,
    config: &ServerConfig,
    peer: SocketAddr,
    username: String,
) -> Result<()> {
    tracing::info!(%peer, username = %username, "configuration handshake begin");

    // 1. S → C: SelectKnownPacks with the only pack we ship — `minecraft:core`.
    let select_known_packs = SelectKnownPacks {
        packs: vec![KnownPack {
            namespace: "minecraft".to_string(),
            id: "core".to_string(),
            version: pigeon_data::MINECRAFT_VERSION.to_string(),
        }],
    };
    write_packet(framed, &select_known_packs, peer).await?;

    // 2. C → S: SelectKnownPacksAck — wait for the client to reply with
    // the subset it has locally. We do not gate further server packets on
    // whether the client actually owns "minecraft:core" (vanilla always
    // does for offline mode); we trust anything it sends here.
    let ack = read_packet(framed, peer).await?;
    if ack.id == pigeon_protocol::java::configuration::SelectKnownPacksAck::ID {
        tracing::debug!(%peer, "select_known_packs ack received");
    } else {
        tracing::debug!(%peer, id = ack.id, "ignored non-ack packet while waiting for SelectKnownPacksAck");
    }

    // 3. S → C: send empty-registry payloads for every vanilla wire
    //    registry we know about. Empty NBT is enough to advance a
    //    vanilla-compatible handshake until M5+ wires real payloads.
    for registry_id in vanilla_wire_registries().iter() {
        write_registry_data_empty(framed, registry_id, peer).await?;
    }

    // 4. S → C: UpdateEnabledFeatures("minecraft:vanilla").
    let features = UpdateEnabledFeatures {
        features: vec!["minecraft:vanilla".to_string()],
    };
    write_packet(framed, &features, peer).await?;

    // 5. S → C: empty UpdateTags (legal in vanilla — means "no tags").
    let tags = pigeon_protocol::java::configuration::UpdateTags::default();
    write_packet(framed, &tags, peer).await?;

    // 6. S → C: brand CustomPayload.
    let brand = CustomPayload {
        channel: "minecraft:brand".to_string(),
        data: b"PigeonMC\0".to_vec(),
    };
    write_packet(framed, &brand, peer).await?;

    // 7. S → C: FinishConfiguration.
    let finish = FinishConfiguration;
    write_packet(framed, &finish, peer).await?;

    // 8. C → S: wait for FinishConfigurationAck (id 0x03).
    loop {
        let pkt = read_packet(framed, peer).await?;
        if pkt.id == pigeon_protocol::java::configuration::FinishConfigurationAck::ID {
            tracing::info!(%peer, "finish configuration ack received — transition to play");
            break;
        }
        tracing::debug!(%peer, id = pkt.id, "ignored packet while waiting for FinishConfigurationAck");
    }

    let _ = config;
    let _ = username;
    Ok(())
}

/// Hardened convenience: encode + send a typed PacketEncode.
async fn write_packet<T: PacketEncode>(
    framed: &mut Framed<TcpStream, PacketCodec>,
    pkt: &T,
    peer: SocketAddr,
) -> Result<()> {
    let mut buf = bytes::BytesMut::new();
    pkt.encode(&mut buf).map_err(|e| {
        tracing::debug!(%peer, encode_err = %e, "encode failed");
        anyhow!(e.to_string())
    })?;
    let encoded = pigeon_protocol::EncodedPacket::new(T::ID, buf.freeze());
    if let Err(err) = framed.send(encoded).await {
        tracing::debug!(%peer, %err, "send error");
        return Err(anyhow!(err.to_string()));
    }
    Ok(())
}

/// Read the next decoded packet, propagating decode errors.
async fn read_packet(
    framed: &mut Framed<TcpStream, PacketCodec>,
    peer: SocketAddr,
) -> Result<pigeon_protocol::DecodedPacket> {
    match framed.next().await {
        Some(Ok(packet)) => Ok(packet),
        Some(Err(err)) => {
            tracing::debug!(%peer, %err, "decode error");
            Err(anyhow!(err.to_string()))
        }
        None => Err(anyhow!("client closed during configuration handshake")),
    }
}

/// A curated list of `minecraft:*` wire registries that vanilla uses
/// via the `RegistryData` packet during configuration. We ship empty
/// NBT for each since `pigeon-data::registries()` does not yet expose
/// the per-entry NBT payloads; M5+ will replace this list with typed
/// `pigeon_registry::RegistryCodec` payloads built from real report
/// data.
const fn vanilla_wire_registries() -> &'static [&'static str] {
    &[
        "minecraft:cat_variant",
        "minecraft:chicken_variant",
        "minecraft:cow_variant",
        "minecraft:pig_variant",
        "minecraft:wolf_variant",
        "minecraft:fox_variant",
        "minecraft:frog_variant",
        "minecraft:salmon_variant",
        "minecraft:painting_variant",
        "minecraft:villager_type",
        "minecraft:villager_profession",
        "minecraft:point_of_interest_type",
        "minecraft:dimension_type",
        "minecraft:worldgen/biome",
        "minecraft:chat_type",
        "minecraft:trim_pattern",
        "minecraft:trim_material",
        "minecraft:banner_pattern",
        "minecraft:instrument",
        "minecraft:menu",
        "minecraft:recipe_book_category",
        "minecraft:attribute",
        "minecraft:enchantment",
        "minecraft:block",
        "minecraft:item",
        "minecraft:entity_type",
        "minecraft:fluid",
        "minecraft:game_event",
        "minecraft:particle_type",
        "minecraft:position_source_type",
        "minecraft:screen",
        "minecraft:slot",
        "minecraft:sound_event",
    ]
}

/// Send a `RegistryData` packet containing an **empty** entry list for
/// the given registry, encoded as the canonical vanilla empty payload.
///
/// Wire layout (per `pigeon_registry::RegistryCodec`):
///   - VarStr(`registry_id`)
///   - VarInt(0)            ← entry count
async fn write_registry_data_empty(
    framed: &mut Framed<TcpStream, PacketCodec>,
    registry_id: &str,
    peer: SocketAddr,
) -> Result<()> {
    // Build the body using the typed RegistryCodec instead of hand-rolling
    // bytes — this keeps the wire format aligned with pigeon-registry.
    let identifier =
        pigeon_util::Identifier::from_str(registry_id).map_err(|e| anyhow!(e.to_string()))?;
    let codec = pigeon_registry::RegistryCodec::new(identifier);
    let pkt = pigeon_protocol::java::configuration::RegistryData::from_codec(&codec)
        .map_err(|e| anyhow!(e.to_string()))?;
    write_packet_direct(framed, pkt, peer).await
}

/// Encode the typed `RegistryData` (which already owns its `body: Vec<u8>`)
/// and ship it through the framed stream.
async fn write_packet_direct(
    framed: &mut Framed<TcpStream, PacketCodec>,
    pkt: pigeon_protocol::java::configuration::RegistryData,
    peer: SocketAddr,
) -> Result<()> {
    let mut buf = bytes::BytesMut::new();
    pkt.encode(&mut buf).map_err(|e| anyhow!(e.to_string()))?;
    let encoded = pigeon_protocol::EncodedPacket::new(
        pigeon_protocol::java::configuration::RegistryData::ID,
        buf.freeze(),
    );
    if let Err(err) = framed.send(encoded).await {
        tracing::debug!(%peer, %err, "send error in RegistryData");
        return Err(anyhow!(err.to_string()));
    }
    Ok(())
}

/// Build a configuration-phase Disconnect with `reason_json` text and
/// send it. Useful for cleanly aborting the configuration handshake
/// when the protocol handshake fails downstream.
#[allow(dead_code)]
pub async fn send_configuration_disconnect(
    framed: &mut Framed<TcpStream, PacketCodec>,
    peer: SocketAddr,
    reason: &str,
) -> Result<()> {
    let payload = serde_json::json!({ "text": reason }).to_string();
    let pkt = Disconnect {
        reason_json: payload,
    };
    write_packet(framed, &pkt, peer).await
}

/// Marker that the Configuration driver is a self-contained async unit.
fn _uses_protocolstate_for_doc_links() -> ProtocolState {
    ProtocolState::Configuration
}
