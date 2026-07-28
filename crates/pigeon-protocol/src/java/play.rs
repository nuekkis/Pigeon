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
// Clientbound (S → C) — Player Info (1.21.x)
// ===========================================================================

/// Bitmask actions packed into the `PlayerInfoUpdate` header.
///
/// Mojang order — actions are emitted in this exact ascending bit order:
///   bit 0 — ADD_PLAYER         (name + properties)
///   bit 1 — INITIALIZE_CHAT    (session signature)
///   bit 2 — UPDATE_GAMEMODE
///   bit 3 — UPDATE_LISTED
///   bit 4 — UPDATE_LATENCY
///   bit 5 — UPDATE_DISPLAY_NAME
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerInfoActions(pub u8);

impl PlayerInfoActions {
    pub const ADD_PLAYER: u8 = 1 << 0;
    pub const INITIALIZE_CHAT: u8 = 1 << 1;
    pub const UPDATE_GAMEMODE: u8 = 1 << 2;
    pub const UPDATE_LISTED: u8 = 1 << 3;
    pub const UPDATE_LATENCY: u8 = 1 << 4;
    pub const UPDATE_DISPLAY_NAME: u8 = 1 << 5;

    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn with(mut self, flag: u8) -> Self {
        self.0 |= flag;
        self
    }
}

/// Per-player entry payload for [`PlayerInfoUpdate`]. Each `Option<_>`
/// is read/written only if the matching action bit is set.
#[derive(Debug, Clone)]
pub struct PlayerInfoEntry {
    /// The player's uuid (16 raw bytes).
    pub uuid: uuid::Uuid,
    /// ADD_PLAYER data.
    pub name: Option<String>,
    /// INITIALIZE_CHAT data — when `Some(false)` indicates the player
    /// has no chat session; when `Some(true)` the inner value carries
    /// the (yet-unimplemented) session signature. We currently only
    /// ship the no-signature case.
    pub has_chat_session: Option<bool>,
    /// UPDATE_GAMEMODE data (0 = survival, 1 = creative, …).
    pub gamemode: Option<i32>,
    /// UPDATE_LISTED data (tab-list visibility).
    pub listed: Option<bool>,
    /// UPDATE_LATENCY data (milliseconds, -1 = unknown).
    pub latency: Option<i32>,
    /// UPDATE_DISPLAY_NAME data (`true` followed by an NBT component).
    /// The component body itself is *not* emitted by this struct yet —
    /// when present it will be `Some(true)` for stubbing purposes.
    pub has_display_name: Option<bool>,
}

/// `ClientboundPlayerInfoUpdatePacket` (id 68) — push per-player
/// updates to the tab list. The 1.21.x wire layout is:
///
///   VarInt(actions mask) | VarInt(player count) |
///     for each player:
///       UUID (16 bytes) + per-action payload
///
/// where the per-action payload is included iff its action bit is set.
#[derive(Debug, Clone)]
pub struct PlayerInfoUpdate {
    pub actions: PlayerInfoActions,
    pub entries: Vec<PlayerInfoEntry>,
}

impl PacketEncode for PlayerInfoUpdate {
    const ID: i32 = 0x44; // 68

    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<(), PacketSerError> {
        pigeon_codecs::write_var_int(self.actions.0 as i32, buf)?;
        pigeon_codecs::write_var_int(self.entries.len() as i32, buf)?;
        for entry in &self.entries {
            // UUID — 16 raw bytes.
            buf.put_slice(entry.uuid.as_bytes());

            if self.actions.0 & PlayerInfoActions::ADD_PLAYER != 0 {
                let name = entry.name.as_ref().ok_or(PacketSerError::InvalidValue)?;
                crate::ser::write_string(name, buf, 16)?;
                // Properties (PropertyArray) — minimal: 0 entries.
                pigeon_codecs::write_var_int(0, buf)?;
            }
            if self.actions.0 & PlayerInfoActions::INITIALIZE_CHAT != 0 {
                // bool has_signature(false) — no chat session data emitted.
                buf.put_u8(u8::from(entry.has_chat_session.unwrap_or(false)));
            }
            if self.actions.0 & PlayerInfoActions::UPDATE_GAMEMODE != 0 {
                let gm = entry.gamemode.unwrap_or(0);
                pigeon_codecs::write_var_int(gm, buf)?;
            }
            if self.actions.0 & PlayerInfoActions::UPDATE_LISTED != 0 {
                buf.put_u8(u8::from(entry.listed.unwrap_or(false)));
            }
            if self.actions.0 & PlayerInfoActions::UPDATE_LATENCY != 0 {
                pigeon_codecs::write_var_int(entry.latency.unwrap_or(-1), buf)?;
            }
            if self.actions.0 & PlayerInfoActions::UPDATE_DISPLAY_NAME != 0 {
                buf.put_u8(u8::from(entry.has_display_name.unwrap_or(false)));
            }
        }
        Ok(())
    }
}

/// `ClientboundPlayerInfoRemovePacket` (id 67) — remove the listed
/// players by uuid.
///
/// Wire layout: `VarInt(count) | UUID(16 bytes) * count`.
#[derive(Debug, Clone)]
pub struct PlayerInfoRemove {
    pub uuids: Vec<uuid::Uuid>,
}

impl PacketEncode for PlayerInfoRemove {
    const ID: i32 = 0x43; // 67

    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<(), PacketSerError> {
        pigeon_codecs::write_var_int(self.uuids.len() as i32, buf)?;
        for uuid in &self.uuids {
            buf.put_slice(uuid.as_bytes());
        }
        Ok(())
    }
}

/// Resolve the wire id of the `PlayerInfoUpdate` packet from `packets.json`.
pub fn player_info_update_id() -> i32 {
    ids::clientbound("play", ids::play::PLAYER_INFO_UPDATE)
}

/// Resolve the wire id of the `PlayerInfoRemove` packet from `packets.json`.
pub fn player_info_remove_id() -> i32 {
    ids::clientbound("play", ids::play::PLAYER_INFO_REMOVE)
}

// ===========================================================================
// Serverbound (C → S) — Movement (1.21.x minimal subset)
// ===========================================================================

/// Common fields for the four `move_player_*` packets. The flag bits
/// distinguish the variants on the wire (only changed fields are sent).
#[derive(Debug, Clone, PartialEq)]
pub struct MovePlayer {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
}

/// `ServerboundMovePlayerPosPacket` (id 29) — position changed.
#[derive(Debug, Clone, PartialEq)]
pub struct MovePlayerPos(pub MovePlayer);

impl PacketDecode for MovePlayerPos {
    const ID: i32 = 0x1D; // 29

    fn decode<B: Buf>(buf: &mut B) -> Result<Self, PacketSerError> {
        let x = buf.get_f64();
        let y = buf.get_f64();
        let z = buf.get_f64();
        let on_ground = buf.get_u8() != 0;
        Ok(Self(MovePlayer {
            x,
            y,
            z,
            yaw: 0.0,
            pitch: 0.0,
            on_ground,
        }))
    }
}

/// `ServerboundMovePlayerPosRotPacket` (id 30) — position + rotation changed.
#[derive(Debug, Clone, PartialEq)]
pub struct MovePlayerPosRot(pub MovePlayer);

impl PacketDecode for MovePlayerPosRot {
    const ID: i32 = 0x1E; // 30

    fn decode<B: Buf>(buf: &mut B) -> Result<Self, PacketSerError> {
        let x = buf.get_f64();
        let y = buf.get_f64();
        let z = buf.get_f64();
        let yaw = buf.get_f32();
        let pitch = buf.get_f32();
        let on_ground = buf.get_u8() != 0;
        Ok(Self(MovePlayer {
            x,
            y,
            z,
            yaw,
            pitch,
            on_ground,
        }))
    }
}

/// `ServerboundMovePlayerRotPacket` (id 31) — rotation changed.
#[derive(Debug, Clone, PartialEq)]
pub struct MovePlayerRot(pub MovePlayer);

impl PacketDecode for MovePlayerRot {
    const ID: i32 = 0x1F; // 31

    fn decode<B: Buf>(buf: &mut B) -> Result<Self, PacketSerError> {
        let yaw = buf.get_f32();
        let pitch = buf.get_f32();
        let on_ground = buf.get_u8() != 0;
        Ok(Self(MovePlayer {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            yaw,
            pitch,
            on_ground,
        }))
    }
}

/// `ServerboundMovePlayerStatusOnlyPacket` (id 32) — only on_ground flag changed.
#[derive(Debug, Clone, PartialEq)]
pub struct MovePlayerStatusOnly(pub MovePlayer);

impl PacketDecode for MovePlayerStatusOnly {
    const ID: i32 = 0x20; // 32

    fn decode<B: Buf>(buf: &mut B) -> Result<Self, PacketSerError> {
        let on_ground = buf.get_u8() != 0;
        Ok(Self(MovePlayer {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            yaw: 0.0,
            pitch: 0.0,
            on_ground,
        }))
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

    #[test]
    fn player_info_update_encodes_add_listed() {
        let uuid = uuid::Uuid::from_u128(0x0123_4567_89ab_cdef_0123_4567_89ab_cdef);
        let pkt = PlayerInfoUpdate {
            actions: PlayerInfoActions::empty()
                .with(PlayerInfoActions::ADD_PLAYER)
                .with(PlayerInfoActions::UPDATE_GAMEMODE)
                .with(PlayerInfoActions::UPDATE_LISTED),
            entries: vec![PlayerInfoEntry {
                uuid,
                name: Some("PigeonTest".to_string()),
                has_chat_session: None,
                gamemode: Some(1),
                listed: Some(true),
                latency: None,
                has_display_name: None,
            }],
        };

        let mut buf = bytes::BytesMut::new();
        let result = pkt.encode(&mut buf);
        assert!(result.is_ok(), "encode failed: {:?}", result);
        let bytes = buf.freeze();

        // Layout:
        //   VarInt(actions=0b001_0101=21) | VarInt(1)=1 | UUID(16) |
        //   [ADD_PLAYER] VarStr(10)="PigeonTest" + VarInt(0) |
        //   [UPDATE_GAMEMODE] VarInt(1) |
        //   [UPDATE_LISTED] u8(1)
        // ~ 1+1+16+1+1+10+1+1+1+1 = ~34
        assert!(bytes.len() >= 30, "buf too short: {}", bytes.len());

        // Sanity-check the first byte (action mask VarInt = 13).
        // bits: ADD_PLAYER(1) + UPDATE_GAMEMODE(4) + UPDATE_LISTED(8) = 13.
        assert_eq!(bytes[0], 13);
    }

    #[test]
    fn player_info_remove_encodes_uuids() {
        let pkt = PlayerInfoRemove {
            uuids: vec![
                uuid::Uuid::from_u128(0x0123_4567_89ab_cdef_0123_4567_89ab_cdef),
                uuid::Uuid::from_u128(0x1111_1111_2222_3333_4444_5555_6666_7777),
            ],
        };

        let mut buf = bytes::BytesMut::new();
        let result = pkt.encode(&mut buf);
        assert!(result.is_ok(), "encode failed: {:?}", result);
        let bytes = buf.freeze();

        // Layout: VarInt(2) + 16 + 16 = 33 bytes.
        assert_eq!(bytes[0], 2);
        assert_eq!(bytes.len(), 33);
    }

    #[test]
    fn move_player_pos_decode_roundtrip() {
        let mut raw = bytes::BytesMut::with_capacity(25);
        raw.put_f64(1.5);
        raw.put_f64(2.5);
        raw.put_f64(3.5);
        raw.put_u8(1);
        let mut cursor = std::io::Cursor::new(raw.freeze());
        let pkt = MovePlayerPos::decode(&mut cursor).unwrap();
        assert!(pkt.0.on_ground);
        assert!((pkt.0.x - 1.5).abs() < 1e-9);
        assert!((pkt.0.y - 2.5).abs() < 1e-9);
        assert!((pkt.0.z - 3.5).abs() < 1e-9);
    }

    #[test]
    fn move_player_pos_rot_decode_roundtrip() {
        let mut raw = bytes::BytesMut::with_capacity(33);
        raw.put_f64(10.0);
        raw.put_f64(20.0);
        raw.put_f64(30.0);
        raw.put_f32(45.0);
        raw.put_f32(-30.0);
        raw.put_u8(0);
        let mut cursor = std::io::Cursor::new(raw.freeze());
        let pkt = MovePlayerPosRot::decode(&mut cursor).unwrap();
        assert!(!pkt.0.on_ground);
        assert!((pkt.0.yaw - 45.0).abs() < 1e-5);
        assert!((pkt.0.pitch + 30.0).abs() < 1e-5);
    }

    #[test]
    fn move_player_status_only_decode() {
        let mut raw = bytes::BytesMut::with_capacity(1);
        raw.put_u8(1);
        let mut cursor = std::io::Cursor::new(raw.freeze());
        let pkt = MovePlayerStatusOnly::decode(&mut cursor).unwrap();
        assert!(pkt.0.on_ground);
    }
}
