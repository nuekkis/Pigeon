//! Protocol packet ids parsed from the embedded `packets.json` report.
//!
//! Mojang's data generator emits ids grouped by phase and direction. This
//! module exposes a typed view over that report so that `pigeon-protocol`
//! can wire ids to packet types without keeping its own hand-curated table.
//!
//! The actual protocol *version* (e.g. the integer reneged during
//! handshake) is **not** part of this report; `pigeon-protocol` carries
//! that constant.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::raw;

pub type PacketReport = BTreeMap<String, Phase>;

#[derive(Debug, Clone, Deserialize)]
pub struct Phase {
    #[serde(default)]
    pub clientbound: BTreeMap<String, PacketEntry>,
    #[serde(default)]
    pub serverbound: BTreeMap<String, PacketEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PacketEntry {
    pub protocol_id: i32,
}

static PACKETS: OnceLock<PacketReport> = OnceLock::new();

/// Returns the parsed `packets.json` report.
pub fn packets() -> &'static PacketReport {
    PACKETS.get_or_init(|| {
        serde_json::from_str(raw::PACKETS_JSON).expect("embedded packets.json must be valid")
    })
}

/// Returns the named phase (e.g. `"play"`).
pub fn phase_named(name: &str) -> Option<&'static Phase> {
    packets().get(name)
}

/// Looks up the clientbound packet id for a given phase + resource location.
pub fn clientbound_id(phase: &str, packet: &str) -> Option<i32> {
    phase_named(phase).and_then(|p| p.clientbound.get(packet).map(|e| e.protocol_id))
}

/// Looks up the serverbound packet id for a given phase + resource location.
pub fn serverbound_id(phase: &str, packet: &str) -> Option<i32> {
    phase_named(phase).and_then(|p| p.serverbound.get(packet).map(|e| e.protocol_id))
}

/// Aggregates the total packet count across every phase and direction.
pub fn count() -> usize {
    packets()
        .values()
        .map(|p| p.clientbound.len() + p.serverbound.len())
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_without_panicking() {
        assert!(count() > 200, "expected 1.21.11 to define >200 packets");
        for phase_name in ["handshake", "status", "login", "configuration", "play"] {
            assert!(
                phase_named(phase_name).is_some(),
                "phase {phase_name} must be present"
            );
        }
    }

    #[test]
    fn play_keep_alive_ids_are_present() {
        assert!(clientbound_id("play", "minecraft:keep_alive").is_some());
        assert!(serverbound_id("play", "minecraft:keep_alive").is_some());
    }
}
