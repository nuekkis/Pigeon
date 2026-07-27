//! Play-state packets.
//!
//! The Play state is the gameplay phase: full chunk streaming, entity
//! updates, player movement, chat, … 1.21.11 ships 139 clientbound and
//! 66 serverbound packets here. This module starts with just the
//! packets necessary for the play-boundary handshake and a working
//! keep-alive loop; later milestones add chunks/entities/world.
//!
//! ### Clientbound (S → C) — minimal M5 subset
//!
//! | ID | Canonical name         | Rust type     |
//! |----|-------------------------|---------------|
//! | 32 | `minecraft:disconnect` | `Disconnect`  |
//! | 43 | `minecraft:keep_alive`  | `KeepAlive`   |
//! | 48 | `minecraft:login`      | `LoginPlay`   |
//!
//! ### Serverbound (C → S) — minimal M5 subset
//!
//! | ID | Canonical name             | Rust type      |
//! |----|-----------------------------|----------------|
//! | 27 | `minecraft:keep_alive`     | `KeepAliveAck` |
//!
//! Wire ids are kept in sync with `pigeon-data`'s embedded `packets.json`
//! report — see [`crate::java::ids::play`] and the regression tests in
//! [`crate::java::ids`].

use bytes::{Buf, BufMut};
use pigeon_nbt::{Nbt, NbtCompound};

use crate::java::ids;
use crate::ser::{PacketDecode, PacketEncode, PacketSerError};

// ===========================================================================
// Clientbound (S → C)
// ===========================================================================

// ---------------------------------------------------------------------------
// S → C : Login (Play) — id 48
// ---------------------------------------------------------------------------

/// `ClientboundLoginPacket` — sent by the server immediately after the
/// client acknowledges `FinishConfiguration`.
///
/// The minimal subset of the 1.21.11 wire format implemented here:
///
///   - i32   entity_id              (VarInt)
///   - bool  is_hardcore
///   - VarInt  gamemode (0=creative,1=survival,2=adventure,3=spectator)
///   - i8    previous_gamemode (-1 = none)
///   - Vec<String> dimension_names (each ≤ 32767 UTF-8 chars)
///   - Nbt   registry_payload (vanilla `minecraft:dimension_type` codec)
///   - VarStr world_name (`minecraft:overworld` for first join)
///   - i64   seed_hash
///   - VarInt max_players
///   - VarInt view_distance (server-side chunk radius, 2..32)
///   - VarInt simulation_distance (server-side sim radius, 0..32)
///   - bool  reduced_debug_info
///   - bool  show_death_screen
///   - bool  do_limited_crafting
///
/// Per-entry NBT for the dimension type codec is **stubbed** for the
/// M5 boundary: the packet still parses cleanly, but until M5.5 wraps
/// `DimensionTypeRegistry::vanilla()` the only dimension advertised
/// is a hardcoded Overworld-shaped entry.
#[derive(Debug, Clone)]
pub struct LoginPlay {
    pub entity_id: i32,
    pub is_hardcore: bool,
    pub gamemode: i8,
    pub previous_gamemode: i8,
    pub dimension_names: Vec<String>,
    /// Root NBT tag carrying `minecraft:dimension_type` and other
    /// `minecraft:*` registry payloads the client needs to spawn the
    /// player. The structure mirrors vanilla's `RegistryCodec`
    /// payload but is **not** auto-generated from `pigeon-data` yet.
    pub registry_payload: Nbt,
    pub world_name: String,
    /// 64-bit world seed (hash) — arbitrary for the M5 stub.
    pub seed_hash: i64,
    pub max_players: i32,
    pub view_distance: i32,
    pub simulation_distance: i32,
    pub reduced_debug_info: bool,
    pub show_death_screen: bool,
    pub do_limited_crafting: bool,
}

impl PacketEncode for LoginPlay {
    const ID: i32 = 0x30; // 48 — matches `minecraft:login` in packets.json

    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<(), PacketSerError> {
        pigeon_codecs::write_var_int(self.entity_id, buf)?;
        buf.put_u8(u8::from(self.is_hardcore));
        pigeon_codecs::write_var_int(self.gamemode as i32, buf)?;
        buf.put_i8(self.previous_gamemode);
        pigeon_codecs::write_var_int(self.dimension_names.len() as i32, buf)?;
        for name in &self.dimension_names {
            crate::ser::write_string(name, buf, 32767)?;
        }
        pigeon_nbt::NbtWriter::new(buf)
            .write_root(&self.registry_payload)
            .map_err(|_| PacketSerError::Overflow)?;
        crate::ser::write_string(&self.world_name, buf, 32767)?;
        buf.put_i64(self.seed_hash);
        pigeon_codecs::write_var_int(self.max_players, buf)?;
        pigeon_codecs::write_var_int(self.view_distance, buf)?;
        pigeon_codecs::write_var_int(self.simulation_distance, buf)?;
        buf.put_u8(u8::from(self.reduced_debug_info));
        buf.put_u8(u8::from(self.show_death_screen));
        buf.put_u8(u8::from(self.do_limited_crafting));
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// S → C : KeepAlive (Play) — id 43
// ---------------------------------------------------------------------------

/// Server keep-alive ping in the Play phase. The client must reply with
/// [`KeepAliveAck`] carrying the same `payload`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeepAlive {
    /// Arbitrary long, usually derived from the server clock.
    pub payload: i64,
}

impl PacketEncode for KeepAlive {
    const ID: i32 = 0x2B; // 43

    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<(), PacketSerError> {
        buf.put_i64(self.payload);
        Ok(())
    }
}

impl PacketDecode for KeepAlive {
    const ID: i32 = 0x2B;

    fn decode<B: Buf>(buf: &mut B) -> Result<Self, PacketSerError> {
        if buf.remaining() < 8 {
            return Err(PacketSerError::Underflow);
        }
        Ok(Self {
            payload: buf.get_i64(),
        })
    }
}

// ---------------------------------------------------------------------------
// S → C : Disconnect (Play) — id 32
// ---------------------------------------------------------------------------

/// Disconnect the client from play state with a chat-component reason.
#[derive(Debug, Clone)]
pub struct Disconnect {
    /// JSON-encoded chat component, e.g. `{"text":"kicked"}`.
    pub reason_json: String,
}

impl PacketEncode for Disconnect {
    const ID: i32 = 0x20; // 32

    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<(), PacketSerError> {
        crate::ser::write_string(&self.reason_json, buf, 32767)
    }
}

// ===========================================================================
// Serverbound (C → S)
// ===========================================================================

// ---------------------------------------------------------------------------
// C → S : KeepAliveAck (Play) — id 27
// ---------------------------------------------------------------------------

/// Client reply to a [`KeepAlive`] ping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeepAliveAck {
    /// Must match the payload sent by the server in the last play-phase
    /// [`KeepAlive`] packet.
    pub payload: i64,
}

impl PacketDecode for KeepAliveAck {
    const ID: i32 = 0x1B; // 27

    fn decode<B: Buf>(buf: &mut B) -> Result<Self, PacketSerError> {
        if buf.remaining() < 8 {
            return Err(PacketSerError::Underflow);
        }
        Ok(Self {
            payload: buf.get_i64(),
        })
    }
}

impl PacketEncode for KeepAliveAck {
    const ID: i32 = 0x1B;

    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<(), PacketSerError> {
        buf.put_i64(self.payload);
        Ok(())
    }
}

// ===========================================================================
// Helpers
// ===========================================================================

/// Build the minimal vanilla-shaped NBT root used as the
/// `registry_payload` field of [`LoginPlay`]. Returns a `minecraft:root`
/// containing a `minecraft:dimension_type` registry with a single
/// (`minecraft:overworld`) hardcoded entry sufficient for the M5 boundary.
///
/// The NBT layout mirrors the `RegistryCodec` shape used by vanilla for
/// `LoginPlay#registry`:
///
/// ```text
/// TAG_Compound("") {
///   "minecraft:dimension_type": TAG_Compound {
///     "type": "minecraft:dimension_type",
///     "value": TAG_List [
///       TAG_Compound {
///         "id": 0,
///         "name": "minecraft:overworld",
///         "element": TAG_Compound {
///           "has_skylight": 1,
///           "has_ceiling": 0,
///           "ultrawarm": 0,
///           "natural": 1,
///           "coordinate_scale": 1.0,
///           "has_raids": 1,
///           "respawn_anchor_works": 0,
///           "bed_works": 1,
///           "effects": "minecraft:overworld",
///           "min_y": -64,
///           "height": 384,
///           "logical_height": 384,
///           "infiniburn": "#minecraft:infiniburn_overworld",
///           "piglin_safe": 0,
///           "fixed_time": (absent)
///         }
///       }
///     ]
///   }
/// }
/// ```
pub fn stub_overworld_registry() -> Nbt {
    use pigeon_nbt::{NbtList, NbtTag, NbtValue};

    let mut element = NbtCompound::new();
    element.insert("has_skylight", NbtValue::Byte(1));
    element.insert("has_ceiling", NbtValue::Byte(0));
    element.insert("ultrawarm", NbtValue::Byte(0));
    element.insert("natural", NbtValue::Byte(1));
    element.insert("coordinate_scale", NbtValue::Float(1.0));
    element.insert("has_raids", NbtValue::Byte(1));
    element.insert("respawn_anchor_works", NbtValue::Byte(0));
    element.insert("bed_works", NbtValue::Byte(1));
    element.insert(
        "effects",
        NbtValue::String("minecraft:overworld".to_string()),
    );
    element.insert("min_y", NbtValue::Int(-64));
    element.insert("height", NbtValue::Int(384));
    element.insert("logical_height", NbtValue::Int(384));
    element.insert(
        "infiniburn",
        NbtValue::String("#minecraft:infiniburn_overworld".to_string()),
    );
    element.insert("piglin_safe", NbtValue::Byte(0));

    let mut entry = NbtCompound::new();
    entry.insert("id", NbtValue::Int(0));
    entry.insert("name", NbtValue::String("minecraft:overworld".to_string()));
    entry.insert("element", NbtValue::Compound(element));

    let mut list = NbtList::new(NbtTag::Compound);
    list.elements.push(NbtValue::Compound(entry));

    let mut dim_type = NbtCompound::new();
    dim_type.insert(
        "type",
        NbtValue::String("minecraft:dimension_type".to_string()),
    );
    dim_type.insert("value", NbtValue::List(list));

    let root = {
        let mut r = NbtCompound::new();
        r.insert("minecraft:dimension_type", NbtValue::Compound(dim_type));
        r
    };

    Nbt::new("", root)
}

/// Resolve the wire id of the `LoginPlay` packet from `packets.json`.
/// Used by callers that prefer not to hardcode `0x30` (e.g. M5 driver).
pub fn login_id() -> i32 {
    ids::clientbound("play", ids::play::LOGIN)
}

/// Resolve the wire id of the clientbound `KeepAlive` packet.
pub fn keep_alive_id() -> i32 {
    ids::clientbound("play", ids::play::KEEP_ALIVE)
}

/// Resolve the wire id of the play-state `Disconnect` packet.
pub fn disconnect_id() -> i32 {
    ids::clientbound("play", ids::play::DISCONNECT)
}

/// Resolve the wire id of the serverbound `KeepAliveAck` packet.
pub fn keep_alive_ack_id() -> i32 {
    ids::serverbound("play", ids::play::KEEP_ALIVE_SB)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_wire_ids() {
        // Statistical guards against silent renumbering in vanilla
        // data. If `packets.json` changes, these tests blow up loudly
        // and we revisit the wire ids across the play module.
        assert_eq!(login_id(), 0x30);
        assert_eq!(keep_alive_id(), 0x2B);
        assert_eq!(disconnect_id(), 0x20);
        assert_eq!(keep_alive_ack_id(), 0x1B);
        // Sanity check against the typed `ID` consts as well.
        assert_eq!(<LoginPlay as PacketEncode>::ID, keep_alive_id() + 5); // 0x30 vs 0x2B
        assert_eq!(<KeepAlive as PacketEncode>::ID, keep_alive_id());
        assert_eq!(<KeepAliveAck as PacketDecode>::ID, keep_alive_ack_id());
        assert_eq!(<Disconnect as PacketEncode>::ID, disconnect_id());
    }

    #[test]
    fn keep_alive_roundtrip() {
        let pkt = KeepAlive {
            payload: 0x0123_4567_89AB_CDEF,
        };
        let mut buf = bytes::BytesMut::new();
        pkt.encode(&mut buf).unwrap();
        let mut cursor = std::io::Cursor::new(buf.freeze());
        let decoded = KeepAlive::decode(&mut cursor).unwrap();
        assert_eq!(pkt, decoded);
    }

    #[test]
    fn keep_alive_ack_roundtrip() {
        let pkt = KeepAliveAck { payload: -1 };
        let mut buf = bytes::BytesMut::new();
        pkt.encode(&mut buf).unwrap();
        let mut cursor = std::io::Cursor::new(buf.freeze());
        let decoded = KeepAliveAck::decode(&mut cursor).unwrap();
        assert_eq!(pkt, decoded);
    }

    #[test]
    fn disconnect_encode_matches_shape() {
        let pkt = Disconnect {
            reason_json: r#"{"text":"bye"}"#.to_string(),
        };
        let mut buf = bytes::BytesMut::new();
        pkt.encode(&mut buf).unwrap();
        // Layout: VarStr(reason_json) = VarInt(14) + 14 bytes.
        let bytes = buf.freeze();
        assert_eq!(bytes[0], 14); // length prefix
        assert_eq!(&bytes[1..], b"{\"text\":\"bye\"}");
    }

    #[test]
    fn login_play_encodes_overworld_payload() {
        let registry = stub_overworld_registry();
        let pkt = LoginPlay {
            entity_id: 42,
            is_hardcore: false,
            gamemode: 1, // survival
            previous_gamemode: -1,
            dimension_names: vec![
                "minecraft:overworld".to_string(),
                "minecraft:the_nether".to_string(),
                "minecraft:the_end".to_string(),
            ],
            registry_payload: registry,
            world_name: "minecraft:overworld".to_string(),
            seed_hash: 0,
            max_players: 20,
            view_distance: 10,
            simulation_distance: 10,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
        };

        let mut buf = bytes::BytesMut::new();
        let result = pkt.encode(&mut buf);
        assert!(result.is_ok(), "encode failed: {:?}", result);
        // Sanity: at least the scalar prefix should be present.
        // VarInt(42) + u8(0) + VarInt(1) + i8(-1) + VarInt(3) + ...
        assert!(buf.len() > 20);
    }
}
