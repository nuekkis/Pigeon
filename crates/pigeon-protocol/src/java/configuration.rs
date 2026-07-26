//! Configuration-state packets.
//!
//! The configuration phase sits between Login and Play and is used to push
//! all the registry / tag / resource-pack state the client needs before
//! gameplay may begin (1.20.2+). It carries 30 packets in 1.21.11.
//!
//! Packet IDs (Java 1.21.11 — frozen in `packets.json`):
//!
//! ### Clientbound (S → C)
//!
//! | ID   | Canonical name                  | Rust type                |
//! |------|----------------------------------|--------------------------|
//! | 0x00 | `minecraft:cookie_request`       | `CookieRequest`          |
//! | 0x01 | `minecraft:custom_payload`        | `CustomPayload`          |
//! | 0x02 | `minecraft:disconnect`           | `Disconnect`             |
//! | 0x03 | `minecraft:finish_configuration`  | `FinishConfiguration`    |
//! | 0x04 | `minecraft:keep_alive`           | `KeepAlive`             |
//! | 0x05 | `minecraft:ping`                 | `Ping`                   |
//! | 0x06 | `minecraft:reset_chat`           | `ResetChat`             |
//! | 0x07 | `minecraft:registry_data`        | `RegistryData`          |
//! | 0x08 | `minecraft:resource_pack_pop`    | `ResourcePackPop`        |
//! | 0x09 | `minecraft:resource_pack_push`    | `ResourcePackPush`      |
//! | 0x0A | `minecraft:store_cookie`          | `StoreCookie`           |
//! | 0x0B | `minecraft:transfer`              | `Transfer`              |
//! | 0x0C | `minecraft:update_enabled_features` | `UpdateEnabledFeatures` |
//! | 0x0D | `minecraft:update_tags`           | `UpdateTags`            |
//! | 0x0E | `minecraft:select_known_packs`    | `SelectKnownPacks`     |
//! | 0x0F | `minecraft:custom_report_details` | `CustomReportDetails`  |
//! | 0x10 | `minecraft:server_links`          | `ServerLinks`           |
//! | 0x11 | `minecraft:clear_dialog`          | `ClearDialog`           |
//! | 0x12 | `minecraft:show_dialog`           | `ShowDialog`            |
//! | 0x13 | `minecraft:code_of_conduct`       | `CodeOfConduct`         |
//!
//! ### Serverbound (C → S)
//!
//! | ID   | Canonical name                  | Rust type                |
//! |------|----------------------------------|--------------------------|
//! | 0x00 | `minecraft:client_information`   | `ClientInformation`     |
//! | 0x01 | `minecraft:cookie_response`       | `CookieResponse`        |
//! | 0x02 | `minecraft:custom_payload`        | `CustomPayloadResponse` |
//! | 0x03 | `minecraft:finish_configuration`  | `FinishConfigurationAck`|
//! | 0x04 | `minecraft:keep_alive`            | `KeepAliveAck`          |
//! | 0x05 | `minecraft:pong`                  | `Pong`                   |
//! | 0x06 | `minecraft:resource_pack`         | `ResourcePackStatus`    |
//! | 0x07 | `minecraft:select_known_packs`    | `SelectKnownPacksAck`   |
//! | 0x08 | `minecraft:custom_click_action`    | `CustomClickAction`    |
//! | 0x09 | `minecraft:accept_code_of_conduct`| `AcceptCodeOfConduct`   |
//!
//! Wire ids are kept in sync with `pigeon-data`'s embedded `packets.json`
//! report — see [`crate::java::ids::configuration`] and the regression
//! tests in [`crate::java::ids`].

use bytes::{Buf, BufMut};

use crate::ser::{PacketDecode, PacketEncode, PacketSerError};

// ===========================================================================
// Clientbound (S → C)
// ===========================================================================

// ---------------------------------------------------------------------------
// S → C : Cookie Request (id = 0x00)
// ---------------------------------------------------------------------------

/// Server asks the client to provide a previously-stored cookie.
///
/// Mirror of [`crate::java::login::CookieRequest`] but in the
/// configuration phase.
#[derive(Debug, Clone)]
pub struct CookieRequest {
    pub key: String,
}

impl PacketEncode for CookieRequest {
    const ID: i32 = 0x00;

    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<(), PacketSerError> {
        crate::ser::write_string(&self.key, buf, 32767)
    }
}

// ---------------------------------------------------------------------------
// S → C : Custom Payload (id = 0x01)
// ---------------------------------------------------------------------------

/// A server-to-client plugin message on a registered channel.
#[derive(Debug, Clone)]
pub struct CustomPayload {
    pub channel: String,
    pub data: Vec<u8>,
}

impl PacketEncode for CustomPayload {
    const ID: i32 = 0x01;

    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<(), PacketSerError> {
        crate::ser::write_string(&self.channel, buf, 32767)?;
        if buf.remaining_mut() < self.data.len() {
            return Err(PacketSerError::Overflow);
        }
        buf.put_slice(&self.data);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// S → C : Disconnect (id = 0x02)
// ---------------------------------------------------------------------------

/// Disconnect the client during configuration with a JSON reason component.
#[derive(Debug, Clone)]
pub struct Disconnect {
    pub reason_json: String,
}

impl PacketEncode for Disconnect {
    const ID: i32 = 0x02;

    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<(), PacketSerError> {
        crate::ser::write_string(&self.reason_json, buf, 262144)
    }
}

// ---------------------------------------------------------------------------
// S → C : Finish Configuration (id = 0x03)
// ---------------------------------------------------------------------------

/// Server tells the client there is no more configuration data, the client
/// should respond with [`FinishConfigurationAck`] and transition to Play.
#[derive(Debug, Clone, Default)]
pub struct FinishConfiguration;

impl PacketEncode for FinishConfiguration {
    const ID: i32 = 0x03;

    fn encode<B: BufMut>(&self, _buf: &mut B) -> Result<(), PacketSerError> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// S → C : Keep Alive (id = 0x04)
// ---------------------------------------------------------------------------

/// Periodic keep-alive ping. The client must reply with a
/// [`KeepAliveAck`] carrying the same payload.
#[derive(Debug, Clone, Copy)]
pub struct KeepAlive {
    pub payload: u64,
}

impl PacketEncode for KeepAlive {
    const ID: i32 = 0x04;

    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<(), PacketSerError> {
        if buf.remaining_mut() < 8 {
            return Err(PacketSerError::Overflow);
        }
        buf.put_u64(self.payload);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// S → C : Ping (id = 0x05)
// ---------------------------------------------------------------------------

/// Server-to-client ping probe. The client echoes the same payload in a
/// [`Pong`].
#[derive(Debug, Clone, Copy)]
pub struct Ping {
    pub payload: i32,
}

impl PacketEncode for Ping {
    const ID: i32 = 0x05;

    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<(), PacketSerError> {
        pigeon_codecs::write_var_int(self.payload, buf)?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// S → C : Reset Chat (id = 0x06)
// ---------------------------------------------------------------------------

/// Tell the client to clear its chat state (used when the player is moved
/// across server instances).
#[derive(Debug, Clone, Default)]
pub struct ResetChat;

impl PacketEncode for ResetChat {
    const ID: i32 = 0x06;

    fn encode<B: BufMut>(&self, _buf: &mut B) -> Result<(), PacketSerError> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// S → C : Registry Data (id = 0x07)
// ---------------------------------------------------------------------------

/// Synchronize a single registry's contents to the client.
///
/// The body is the raw Mojang wire payload produced by
/// [`pigeon_registry::RegistryCodec::encode`]; the packet just forwards
/// those bytes after the id. This keeps the heavy registry serialization
/// (VarStr identifiers + per-entry NBT roots) in the `pigeon-registry`
/// crate where the typed `Registry<T>` machinery lives.
///
/// The `registry_codec` field is owned rather than borrowed so that the
/// whole packet can be moved into the send queue without lifetime juggling.
/// For pre-encoded payloads (e.g. cached vanilla registries) prefer the
/// `from_bytes` constructor which skips re-serialization.
#[derive(Debug, Clone)]
pub struct RegistryData {
    /// Pre-serialized wire body (the bytes that follow the packet id).
    /// When constructed via [`Self::from_codec`] this is the output of
    /// [`pigeon_registry::RegistryCodec::encode`].
    pub body: Vec<u8>,
}

impl RegistryData {
    /// Build the packet from a [`pigeon_registry::RegistryCodec`],
    /// encoding it into a fresh byte vector.
    pub fn from_codec(codec: &pigeon_registry::RegistryCodec) -> Result<Self, PacketSerError> {
        let mut buf = bytes::BytesMut::with_capacity(256);
        codec
            .encode(&mut buf)
            .map_err(registry_codec_err_to_packet_ser)?;
        Ok(Self { body: buf.to_vec() })
    }

    /// Build the packet from an already-encoded registry body (for
    /// example, a cached vanilla registry payload).
    pub fn from_bytes(body: Vec<u8>) -> Self {
        Self { body }
    }
}

impl PacketEncode for RegistryData {
    const ID: i32 = 0x07;

    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<(), PacketSerError> {
        if buf.remaining_mut() < self.body.len() {
            return Err(PacketSerError::Overflow);
        }
        buf.put_slice(&self.body);
        Ok(())
    }
}

/// Map a [`pigeon_registry::RegistryCodecError`] onto [`PacketSerError`].
///
/// The codec's error variants correspond closely to ours; identifiers and
/// NBT-shaped failures become `InvalidValue`, while buffer sizing issues
/// become `Overflow`/`Underflow`.
fn registry_codec_err_to_packet_ser(err: pigeon_registry::RegistryCodecError) -> PacketSerError {
    use pigeon_registry::RegistryCodecError as E;
    match err {
        E::Underflow => PacketSerError::Underflow,
        E::Overflow => PacketSerError::Overflow,
        E::InvalidEntryCount(_) => PacketSerError::InvalidValue,
        E::UnexpectedRootName { .. } => PacketSerError::InvalidValue,
        E::NbtDecode(_) | E::NbtEncode(_) => PacketSerError::InvalidValue,
        E::VarInt(pigeon_codecs::VarIntReadError::Underflow)
        | E::VarInt(pigeon_codecs::VarIntReadError::TooLarge) => PacketSerError::InvalidValue,
        E::Identifier(_) => PacketSerError::InvalidValue,
    }
}

// ---------------------------------------------------------------------------
// S → C : Resource Pack Pop (id = 0x08)
// ---------------------------------------------------------------------------
//
// Removes a resource pack previously pushed with `ResourcePackPush`. If
// the optional `uuid` is `None`, the client clears ALL resource packs.

use uuid::Uuid;

/// Remove resource pack(s) from the client. `uuid = None` means "clear all".
#[derive(Debug, Clone)]
pub struct ResourcePackPop {
    pub uuid: Option<Uuid>,
}

impl PacketEncode for ResourcePackPop {
    const ID: i32 = 0x08;

    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<(), PacketSerError> {
        match self.uuid {
            Some(uuid) => {
                if buf.remaining_mut() < 1 {
                    return Err(PacketSerError::Overflow);
                }
                buf.put_u8(1);
                crate::ser::write_uuid(uuid, buf)
            }
            None => {
                if buf.remaining_mut() < 1 {
                    return Err(PacketSerError::Overflow);
                }
                buf.put_u8(0);
                Ok(())
            }
        }
    }
}

// ---------------------------------------------------------------------------
// S → C : Resource Pack Push (id = 0x09)
// ---------------------------------------------------------------------------

/// Push a resource pack to the client.
///
/// `forced` requests the client download without prompting; `prompt_message`
/// is an optional chat component used as the prompt text.
#[derive(Debug, Clone)]
pub struct ResourcePackPush {
    pub uuid: Uuid,
    pub url: String,
    pub hash: String,
    pub forced: bool,
    /// Optional chat-component prompt message (encoded as a present-flag byte
    /// followed by NBT compound when `Some`).
    pub prompt_message: Option<pigeon_nbt::NbtCompound>,
}

impl PacketEncode for ResourcePackPush {
    const ID: i32 = 0x09;

    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<(), PacketSerError> {
        crate::ser::write_uuid(self.uuid, buf)?;
        crate::ser::write_string(&self.url, buf, 32767)?;
        crate::ser::write_string(&self.hash, buf, 32767)?;
        if buf.remaining_mut() < 1 {
            return Err(PacketSerError::Overflow);
        }
        buf.put_u8(self.forced as u8);
        match &self.prompt_message {
            Some(compound) => {
                if buf.remaining_mut() < 1 {
                    return Err(PacketSerError::Overflow);
                }
                buf.put_u8(1);
                let nbt = pigeon_nbt::Nbt::new(String::new(), compound.clone());
                pigeon_nbt::NbtWriter::new(buf)
                    .write_root(&nbt)
                    .map_err(|_| PacketSerError::InvalidValue)?;
            }
            None => {
                if buf.remaining_mut() < 1 {
                    return Err(PacketSerError::Overflow);
                }
                buf.put_u8(0);
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// S → C : Store Cookie (id = 0x0A)
// ---------------------------------------------------------------------------

/// Ask the client to persist a cookie under `key` with the supplied `value`
/// (a VarInt-prefixed byte array).
///
/// Cookie responses are returned in the matching phase via the
/// `CookieResponse` serverbound packet.
#[derive(Debug, Clone)]
pub struct StoreCookie {
    pub key: String,
    pub value: Vec<u8>,
}

impl PacketEncode for StoreCookie {
    const ID: i32 = 0x0A;

    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<(), PacketSerError> {
        crate::ser::write_string(&self.key, buf, 32767)?;
        pigeon_codecs::write_var_int(self.value.len() as i32, buf)?;
        if buf.remaining_mut() < self.value.len() {
            return Err(PacketSerError::Overflow);
        }
        buf.put_slice(&self.value);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// S → C : Transfer (id = 0x0B)
// ---------------------------------------------------------------------------

/// Instruct the client to immediately reconnect at `host:port` after this
/// packet is processed.
#[derive(Debug, Clone)]
pub struct Transfer {
    pub host: String,
    pub port: u16,
}

impl PacketEncode for Transfer {
    const ID: i32 = 0x0B;

    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<(), PacketSerError> {
        crate::ser::write_string(&self.host, buf, 32767)?;
        Ok(pigeon_codecs::write_var_int(self.port as i32, buf)?)
    }
}

// ---------------------------------------------------------------------------
// S → C : Update Enabled Features (id = 0x0C)
// ---------------------------------------------------------------------------

/// Synchronize the set of enabled gameplay feature flags
/// (e.g. `minecraft:vanilla`).
#[derive(Debug, Clone, Default)]
pub struct UpdateEnabledFeatures {
    pub features: Vec<String>,
}

impl PacketEncode for UpdateEnabledFeatures {
    const ID: i32 = 0x0C;

    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<(), PacketSerError> {
        pigeon_codecs::write_var_int(self.features.len() as i32, buf)?;
        for feature in &self.features {
            crate::ser::write_string(feature, buf, 32767)?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// S → C : Update Tags (id = 0x0D)
// ---------------------------------------------------------------------------

/// A single tag (a named id-list) within a tag registry.
///
/// Wire layout: `tag_name: String`, `entries: Vec<VarInt>` — those VarInts
/// are protocol ids of the entries the tag contains.
#[derive(Debug, Clone)]
pub struct Tag {
    pub name: String,
    pub entries: Vec<i32>,
}

/// A tag registry (e.g. `minecraft:items`, `minecraft:blocks`) carrying a
/// set of tags.
#[derive(Debug, Clone)]
pub struct TagRegistry {
    pub registry: String,
    pub tags: Vec<Tag>,
}

/// Synchronize all tag registries to the client in one go.
///
/// This is a large packet — for vanilla 1.21.x it bundles ~80 tag
/// registries, each with multiple tags containing thousands of ids. It is
/// emitted at the very end of configuration before `FinishConfiguration`.
#[derive(Debug, Clone, Default)]
pub struct UpdateTags {
    pub registries: Vec<TagRegistry>,
}

impl PacketEncode for UpdateTags {
    const ID: i32 = 0x0D;

    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<(), PacketSerError> {
        pigeon_codecs::write_var_int(self.registries.len() as i32, buf)?;
        for registry in &self.registries {
            crate::ser::write_string(&registry.registry, buf, 32767)?;
            pigeon_codecs::write_var_int(registry.tags.len() as i32, buf)?;
            for tag in &registry.tags {
                crate::ser::write_string(&tag.name, buf, 32767)?;
                pigeon_codecs::write_var_int(tag.entries.len() as i32, buf)?;
                for entry in &tag.entries {
                    pigeon_codecs::write_var_int(*entry, buf)?;
                }
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// S → C : Select Known Packs (id = 0x0E)
// ---------------------------------------------------------------------------

/// A (namespace, id, version) tuple identifying a datapack known to the
/// server.
#[derive(Debug, Clone)]
pub struct KnownPack {
    pub namespace: String,
    pub id: String,
    pub version: String,
}

/// Ask the client which of the supplied known-packs it already has cached
/// locally so the server can avoid re-sending their data.
///
/// The client responds with the subset it actually has via the serverbound
/// `SelectKnownPacksAck` packet (id 0x07).
#[derive(Debug, Clone, Default)]
pub struct SelectKnownPacks {
    pub packs: Vec<KnownPack>,
}

impl PacketEncode for SelectKnownPacks {
    const ID: i32 = 0x0E;

    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<(), PacketSerError> {
        pigeon_codecs::write_var_int(self.packs.len() as i32, buf)?;
        for pack in &self.packs {
            crate::ser::write_string(&pack.namespace, buf, 32767)?;
            crate::ser::write_string(&pack.id, buf, 32767)?;
            crate::ser::write_string(&pack.version, buf, 32767)?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// S → C : Custom Report Details (id = 0x0F)
// ---------------------------------------------------------------------------

/// A single `key -> value` pair in the custom report details payload.
#[derive(Debug, Clone)]
pub struct CustomReportEntry {
    pub key: String,
    pub value: String,
}

/// Pairs of (key, value) shipped to the client so it can attach custom
/// context to outbound telemetry/crash reports.
#[derive(Debug, Clone, Default)]
pub struct CustomReportDetails {
    pub details: Vec<CustomReportEntry>,
}

impl PacketEncode for CustomReportDetails {
    const ID: i32 = 0x0F;

    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<(), PacketSerError> {
        pigeon_codecs::write_var_int(self.details.len() as i32, buf)?;
        for entry in &self.details {
            crate::ser::write_string(&entry.key, buf, 32767)?;
            crate::ser::write_string(&entry.value, buf, 32767)?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// S → C : Server Links (id = 0x10)
// ---------------------------------------------------------------------------

/// Built-in Mojang-known server link kinds.
///
/// Wire-encoded as the small enum id; the unmapped variants ship an
/// `unknownType` chat component instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ServerLinkKind {
    BugReport = 0,
    CommunityGuidelines = 1,
    Support = 2,
    Status = 3,
    Feedback = 4,
    Community = 5,
    Website = 6,
    Forums = 7,
    News = 8,
    Announcements = 9,
}

/// A single server link: either a built-in known kind or an arbitrary
/// chat-component label, plus the URL string.
#[derive(Debug, Clone)]
pub enum ServerLinkLabel {
    Known(ServerLinkKind),
    /// A chat component (encoded as NBT) when the label is custom.
    Unknown(pigeon_nbt::NbtCompound),
}

#[derive(Debug, Clone)]
pub struct ServerLink {
    pub label: ServerLinkLabel,
    pub url: String,
}

/// Send the client a list of clickable server links (e.g. bug report URL,
/// support URL, community guidelines …).
#[derive(Debug, Clone, Default)]
pub struct ServerLinks {
    pub links: Vec<ServerLink>,
}

impl PacketEncode for ServerLinks {
    const ID: i32 = 0x10;

    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<(), PacketSerError> {
        pigeon_codecs::write_var_int(self.links.len() as i32, buf)?;
        for link in &self.links {
            match &link.label {
                ServerLinkLabel::Known(kind) => {
                    if buf.remaining_mut() < 1 {
                        return Err(PacketSerError::Overflow);
                    }
                    buf.put_u8(1);
                    pigeon_codecs::write_var_int(*kind as i32, buf)?;
                }
                ServerLinkLabel::Unknown(compound) => {
                    if buf.remaining_mut() < 1 {
                        return Err(PacketSerError::Overflow);
                    }
                    buf.put_u8(0);
                    let nbt = pigeon_nbt::Nbt::new(String::new(), compound.clone());
                    pigeon_nbt::NbtWriter::new(buf)
                        .write_root(&nbt)
                        .map_err(|_| PacketSerError::InvalidValue)?;
                }
            }
            crate::ser::write_string(&link.url, buf, 32767)?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// S → C : Clear Dialog (id = 0x11)
// ---------------------------------------------------------------------------

/// Closes any currently-open client dialog.
#[derive(Debug, Clone, Default)]
pub struct ClearDialog;

impl PacketEncode for ClearDialog {
    const ID: i32 = 0x11;

    fn encode<B: BufMut>(&self, _buf: &mut B) -> Result<(), PacketSerError> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// S → C : Show Dialog (id = 0x12)
// ---------------------------------------------------------------------------

/// Open a client-side dialog identified by an NBT compound (root name `""`).
#[derive(Debug, Clone)]
pub struct ShowDialog {
    pub dialog: pigeon_nbt::NbtCompound,
}

impl PacketEncode for ShowDialog {
    const ID: i32 = 0x12;

    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<(), PacketSerError> {
        let nbt = pigeon_nbt::Nbt::new(String::new(), self.dialog.clone());
        pigeon_nbt::NbtWriter::new(buf)
            .write_root(&nbt)
            .map_err(|_| PacketSerError::InvalidValue)
    }
}

// ---------------------------------------------------------------------------
// S → C : Code of Conduct (id = 0x13)
// ---------------------------------------------------------------------------

/// Ship the server's code-of-conduct text to the client (chat component
/// encoded as a JSON string).
#[derive(Debug, Clone)]
pub struct CodeOfConduct {
    pub contents: String,
}

impl PacketEncode for CodeOfConduct {
    const ID: i32 = 0x13;

    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<(), PacketSerError> {
        crate::ser::write_string(&self.contents, buf, 32767)
    }
}

// ===========================================================================

// ---------------------------------------------------------------------------
// C → S : Client Information (id = 0x00)
// ---------------------------------------------------------------------------

/// Client's user-preference payload sent at the start of configuration.
#[derive(Debug, Clone)]
pub struct ClientInformation {
    pub locale: String,
    pub view_distance: u8,
    pub chat_mode: ChatMode,
    pub chat_colors: bool,
    pub displayed_skin_parts: u8,
    pub main_hand: MainHand,
    pub filter_text: bool,
    pub allow_listing: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ChatMode {
    Enabled = 0,
    CommandsOnly = 1,
    Hidden = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum MainHand {
    Left = 0,
    Right = 1,
}

impl PacketDecode for ClientInformation {
    const ID: i32 = 0x00;

    fn decode<B: Buf>(buf: &mut B) -> Result<Self, PacketSerError> {
        let locale = crate::ser::read_string(buf, 16)?;
        if buf.remaining() < 1 {
            return Err(PacketSerError::Underflow);
        }
        let view_distance = buf.get_u8();
        let chat_mode_raw = pigeon_codecs::read_var_int(buf)?;
        let chat_mode = match chat_mode_raw {
            0 => ChatMode::Enabled,
            1 => ChatMode::CommandsOnly,
            2 => ChatMode::Hidden,
            _ => return Err(PacketSerError::InvalidValue),
        };
        if buf.remaining() < 1 {
            return Err(PacketSerError::Underflow);
        }
        let chat_colors = buf.get_u8() != 0;
        if buf.remaining() < 1 {
            return Err(PacketSerError::Underflow);
        }
        let displayed_skin_parts = buf.get_u8();
        let main_hand_raw = pigeon_codecs::read_var_int(buf)?;
        let main_hand = match main_hand_raw {
            0 => MainHand::Left,
            1 => MainHand::Right,
            _ => return Err(PacketSerError::InvalidValue),
        };
        if buf.remaining() < 1 {
            return Err(PacketSerError::Underflow);
        }
        let filter_text = buf.get_u8() != 0;
        if buf.remaining() < 1 {
            return Err(PacketSerError::Underflow);
        }
        let allow_listing = buf.get_u8() != 0;
        Ok(Self {
            locale,
            view_distance,
            chat_mode,
            chat_colors,
            displayed_skin_parts,
            main_hand,
            filter_text,
            allow_listing,
        })
    }
}

// ---------------------------------------------------------------------------
// C → S : Cookie Response (id = 0x01)
// ---------------------------------------------------------------------------

/// Client reply to a [`CookieRequest`] in the configuration phase.
#[derive(Debug, Clone)]
pub struct CookieResponse {
    pub key: String,
    pub has_cookies: bool,
    pub payload: Vec<u8>,
}

impl PacketDecode for CookieResponse {
    const ID: i32 = 0x01;

    fn decode<B: Buf>(buf: &mut B) -> Result<Self, PacketSerError> {
        let key = crate::ser::read_string(buf, 32767)?;
        if buf.remaining() < 1 {
            return Err(PacketSerError::Underflow);
        }
        let has_cookies = buf.get_u8() != 0;
        let payload = if has_cookies {
            let len = pigeon_codecs::read_var_int(buf)?;
            if len < 0 || len as usize > buf.remaining() {
                return Err(PacketSerError::InvalidValue);
            }
            let len = len as usize;
            let mut payload = vec![0u8; len];
            for byte in payload.iter_mut() {
                *byte = buf.get_u8();
            }
            payload
        } else {
            Vec::new()
        };
        Ok(Self {
            key,
            has_cookies,
            payload,
        })
    }
}

// ---------------------------------------------------------------------------
// C → S : Custom Payload (id = 0x02)
// ---------------------------------------------------------------------------

/// Client-to-server plugin message on a registered channel.
#[derive(Debug, Clone)]
pub struct CustomPayloadResponse {
    pub channel: String,
    pub data: Vec<u8>,
}

impl PacketDecode for CustomPayloadResponse {
    const ID: i32 = 0x02;

    fn decode<B: Buf>(buf: &mut B) -> Result<Self, PacketSerError> {
        let channel = crate::ser::read_string(buf, 32767)?;
        let remaining = buf.remaining();
        let mut data = vec![0u8; remaining];
        for byte in data.iter_mut() {
            *byte = buf.get_u8();
        }
        Ok(Self { channel, data })
    }
}

// ---------------------------------------------------------------------------
// C → S : Finish Configuration (id = 0x03)
// ---------------------------------------------------------------------------

/// Client acknowledges the end of configuration; the server then switches
/// the connection state to Play.
#[derive(Debug, Clone, Default)]
pub struct FinishConfigurationAck;

impl PacketDecode for FinishConfigurationAck {
    const ID: i32 = 0x03;

    fn decode<B: Buf>(_buf: &mut B) -> Result<Self, PacketSerError> {
        Ok(Self)
    }
}

// ---------------------------------------------------------------------------
// C → S : Keep Alive (id = 0x04)
// ---------------------------------------------------------------------------

/// Client reply to [`KeepAlive`] echoing the same payload.
#[derive(Debug, Clone, Copy)]
pub struct KeepAliveAck {
    pub payload: u64,
}

impl PacketDecode for KeepAliveAck {
    const ID: i32 = 0x04;

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
// C → S : Pong (id = 0x05)
// ---------------------------------------------------------------------------

/// Client reply to [`Ping`] carrying the same `payload`.
#[derive(Debug, Clone, Copy)]
pub struct Pong {
    pub payload: i32,
}

impl PacketDecode for Pong {
    const ID: i32 = 0x05;

    fn decode<B: Buf>(buf: &mut B) -> Result<Self, PacketSerError> {
        Ok(Self {
            payload: pigeon_codecs::read_var_int(buf)?,
        })
    }
}

// ---------------------------------------------------------------------------
// C → S : Resource Pack Status (id = 0x06)
// ---------------------------------------------------------------------------

/// Outcome the client reports for a pushed resource pack. The numeric ids
/// follow Mojang's vanilla enum ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ResourcePackResult {
    /// Server successfully applied the pack.
    Accepted = 0,
    /// Server refused the pack.
    Declined = 1,
    /// Download failed.
    FailedDownload = 2,
    /// Pack downloaded successfully.
    SuccessfullyLoaded = 3,
}

impl ResourcePackResult {
    pub fn from_i32(v: i32) -> Result<Self, PacketSerError> {
        Ok(match v {
            0 => Self::Accepted,
            1 => Self::Declined,
            2 => Self::FailedDownload,
            3 => Self::SuccessfullyLoaded,
            _ => return Err(PacketSerError::InvalidValue),
        })
    }
}

#[derive(Debug, Clone)]
pub struct ResourcePackStatus {
    pub uuid: Uuid,
    pub result: ResourcePackResult,
}

impl PacketDecode for ResourcePackStatus {
    const ID: i32 = 0x06;

    fn decode<B: Buf>(buf: &mut B) -> Result<Self, PacketSerError> {
        let uuid = crate::ser::read_uuid(buf)?;
        let raw = pigeon_codecs::read_var_int(buf)?;
        Ok(Self {
            uuid,
            result: ResourcePackResult::from_i32(raw)?,
        })
    }
}

// ---------------------------------------------------------------------------
// C → S : Select Known Packs Ack (id = 0x07)
// ---------------------------------------------------------------------------

/// Reply to the server's `SelectKnownPacks`: the subset of packs the
/// client already has cached locally. The structure mirrors
/// `SelectKnownPacks` itself: an array of (namespace, id, version).
#[derive(Debug, Clone, Default)]
pub struct SelectKnownPacksAck {
    pub packs: Vec<KnownPack>,
}

impl PacketDecode for SelectKnownPacksAck {
    const ID: i32 = 0x07;

    fn decode<B: Buf>(buf: &mut B) -> Result<Self, PacketSerError> {
        let count = pigeon_codecs::read_var_int(buf)?;
        if !(0..=4096).contains(&count) {
            return Err(PacketSerError::InvalidValue);
        }
        let mut packs = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let namespace = crate::ser::read_string(buf, 32767)?;
            let id = crate::ser::read_string(buf, 32767)?;
            let version = crate::ser::read_string(buf, 32767)?;
            packs.push(KnownPack {
                namespace,
                id,
                version,
            });
        }
        Ok(Self { packs })
    }
}

// ---------------------------------------------------------------------------
// C → S : Custom Click Action (id = 0x08)
// ---------------------------------------------------------------------------

/// Reply from a custom-click-action dialog: the action id plus an optional
/// NBT compound of arbitrary client-side data.
#[derive(Debug, Clone)]
pub struct CustomClickAction {
    pub id: String,
    pub nbt: Option<pigeon_nbt::NbtCompound>,
}

impl PacketDecode for CustomClickAction {
    const ID: i32 = 0x08;

    fn decode<B: Buf>(buf: &mut B) -> Result<Self, PacketSerError> {
        let id = crate::ser::read_string(buf, 32767)?;
        let has_nbt = if buf.remaining() < 1 {
            return Err(PacketSerError::Underflow);
        } else {
            buf.get_u8() != 0
        };
        let nbt = if has_nbt {
            let nbt = pigeon_nbt::NbtReader::new(buf)
                .read_root()
                .map_err(|_| PacketSerError::InvalidValue)?;
            // Root name conventionally empty for these transmissions.
            Some(nbt.root)
        } else {
            None
        };
        Ok(Self { id, nbt })
    }
}

// ---------------------------------------------------------------------------
// C → S : Accept Code of Conduct (id = 0x09)
// ---------------------------------------------------------------------------

/// Acknowledgement from the client that it has accepted the server's
/// code of conduct. There is no body.
#[derive(Debug, Clone, Default)]
pub struct AcceptCodeOfConduct;

impl PacketDecode for AcceptCodeOfConduct {
    const ID: i32 = 0x09;

    fn decode<B: Buf>(_buf: &mut B) -> Result<Self, PacketSerError> {
        Ok(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    fn decode_from_bytes<T: PacketDecode>(bytes: &[u8]) -> Result<T, PacketSerError> {
        let mut buf = bytes;
        T::decode(&mut buf)
    }

    #[test]
    fn client_information_decodes_full_payload() {
        // VarStr("en_US") + u8(10) + VarInt(0=Enabled) + u8(1=colors on)
        // + u8(0x7F=skin parts) + VarInt(1=Right) + u8(1=filter on) + u8(0=no listing)
        let mut wire = Vec::new();
        let mut s = BytesMut::new();
        crate::ser::write_string("en_US", &mut s, 16).unwrap();
        wire.extend_from_slice(&s);
        wire.push(10);
        pigeon_codecs::write_var_int(0, &mut wire).unwrap();
        wire.push(1);
        wire.push(0x7F);
        pigeon_codecs::write_var_int(1, &mut wire).unwrap();
        wire.push(1);
        wire.push(0);
        let info = decode_from_bytes::<ClientInformation>(&wire).expect("decode must succeed");
        assert_eq!(info.locale, "en_US");
        assert_eq!(info.view_distance, 10);
        assert_eq!(info.chat_mode, ChatMode::Enabled);
        assert!(info.chat_colors);
        assert_eq!(info.displayed_skin_parts, 0x7F);
        assert_eq!(info.main_hand, MainHand::Right);
        assert!(info.filter_text);
        assert!(!info.allow_listing);
    }

    #[test]
    fn client_information_rejects_invalid_chat_mode() {
        let mut wire = Vec::new();
        let mut s = BytesMut::new();
        crate::ser::write_string("de", &mut s, 16).unwrap();
        wire.extend_from_slice(&s);
        wire.push(10);
        pigeon_codecs::write_var_int(42, &mut wire).unwrap();
        let err =
            decode_from_bytes::<ClientInformation>(&wire).expect_err("invalid chat mode must fail");
        assert!(matches!(err, PacketSerError::InvalidValue), "got {err:?}");
    }

    #[test]
    fn cookie_response_decodes_without_payload() {
        let mut wire = Vec::new();
        let mut s = BytesMut::new();
        crate::ser::write_string("minecraft:key", &mut s, 32767).unwrap();
        wire.extend_from_slice(&s);
        wire.push(0x00);
        let decoded = decode_from_bytes::<CookieResponse>(&wire).expect("decode must succeed");
        assert_eq!(decoded.key, "minecraft:key");
        assert!(!decoded.has_cookies);
        assert!(decoded.payload.is_empty());
    }

    #[test]
    fn keep_alive_ack_decodes_payload_be() {
        let wire = (0x12345678u64).to_be_bytes();
        let decoded = decode_from_bytes::<KeepAliveAck>(&wire).expect("decode must succeed");
        assert_eq!(decoded.payload, 0x12345678u64);
    }

    #[test]
    fn pong_decodes_payload_varint() {
        let mut wire = Vec::new();
        pigeon_codecs::write_var_int(1337, &mut wire).unwrap();
        let decoded = decode_from_bytes::<Pong>(&wire).expect("decode must succeed");
        assert_eq!(decoded.payload, 1337);
    }

    #[test]
    fn custom_payload_response_decodes_channel_and_bytes() {
        let mut wire = Vec::new();
        let mut s = BytesMut::new();
        crate::ser::write_string("minecraft:brand", &mut s, 32767).unwrap();
        wire.extend_from_slice(&s);
        wire.extend_from_slice(&[0x42, 0x55, 0x4E]);
        let decoded =
            decode_from_bytes::<CustomPayloadResponse>(&wire).expect("decode must succeed");
        assert_eq!(decoded.channel, "minecraft:brand");
        assert_eq!(decoded.data, vec![0x42, 0x55, 0x4E]);
    }

    #[test]
    fn ping_encodes_varint_payload() {
        let pkt = Ping { payload: 765 };
        let mut buf = BytesMut::new();
        PacketEncode::encode(&pkt, &mut buf).expect("encode must succeed");
        let bytes = buf.freeze();
        // VarInt(765) = 0xFD 0x05 (=765).
        assert_eq!(&*bytes, &[0xFD, 0x05]);
    }

    #[test]
    fn keep_alive_encodes_payload_be() {
        let pkt = KeepAlive {
            payload: 0xDEADBEEFCAFEBABE,
        };
        let mut buf = BytesMut::new();
        PacketEncode::encode(&pkt, &mut buf).expect("encode must succeed");
        let bytes = buf.freeze();
        assert_eq!(bytes.len(), 8);
        assert_eq!(&*bytes, &0xDEADBEEFCAFEBABEu64.to_be_bytes());
    }

    #[test]
    fn finish_configuration_encodes_empty_body() {
        let pkt = FinishConfiguration;
        let mut buf = BytesMut::new();
        PacketEncode::encode(&pkt, &mut buf).expect("encode must succeed");
        assert!(
            buf.is_empty(),
            "FinishConfiguration must have no body bytes"
        );
    }

    #[test]
    fn reset_chat_encodes_empty_body() {
        let pkt = ResetChat;
        let mut buf = BytesMut::new();
        PacketEncode::encode(&pkt, &mut buf).expect("encode must succeed");
        assert!(buf.is_empty(), "ResetChat must have no body bytes");
    }

    #[test]
    fn disconnect_encodes_reason_json() {
        let pkt = Disconnect {
            reason_json: "{\"text\":\"bye\"}".to_string(),
        };
        let mut buf = BytesMut::new();
        PacketEncode::encode(&pkt, &mut buf).expect("encode must succeed");
        let bytes = buf.freeze();
        let mut reader = bytes.as_ref();
        let s = crate::ser::read_string(&mut reader, 262144).expect("must read string back");
        assert_eq!(s, pkt.reason_json);
        assert!(reader.is_empty());
    }
}
