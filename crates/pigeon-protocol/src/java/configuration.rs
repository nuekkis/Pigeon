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
// Complex clientbound packets (deferred to M5+)
//
// The remaining S → C packets carry heavy state bodies whose typed Rust
// representation is part of the upcoming world-data milestone (tag maps,
// resource-pack metadata, dialog payloads, …). They are declared here
// for visibility and to reserve the ids; their `encode` body returns
// `todo!()` so attempts to use them surface clearly at runtime.
// ---------------------------------------------------------------------------

macro_rules! deferred_encode_packet {
    ($name:ident, $id:expr, $doc:expr) => {
        #[derive(Debug, Clone, Default)]
        #[doc = $doc]
        pub struct $name;

        impl PacketEncode for $name {
            const ID: i32 = $id;
            fn encode<B: BufMut>(&self, _buf: &mut B) -> Result<(), PacketSerError> {
                todo!(concat!(
                    "encode body for `",
                    stringify!($name),
                    "` lands in M5+"
                ));
            }
        }
    };
}

deferred_encode_packet!(
    ResourcePackPop,
    0x08,
    "S → C — pop a previously pushed resource pack."
);
deferred_encode_packet!(
    ResourcePackPush,
    0x09,
    "S → C — push a resource pack to the client."
);
deferred_encode_packet!(StoreCookie, 0x0A, "S → C — store a cookie on the client.");
deferred_encode_packet!(
    Transfer,
    0x0B,
    "S → C — transfer the client to another host."
);
deferred_encode_packet!(
    UpdateEnabledFeatures,
    0x0C,
    "S → C — update the set of enabled gameplay features."
);
deferred_encode_packet!(UpdateTags, 0x0D, "S → C — synchronize all tag registries.");
deferred_encode_packet!(
    SelectKnownPacks,
    0x0E,
    "S → C — ask the client which known-packs it has."
);
deferred_encode_packet!(
    CustomReportDetails,
    0x0F,
    "S → C — custom report metadata for telemetry."
);
deferred_encode_packet!(
    ServerLinks,
    0x10,
    "S → C — server link graph (support URLs, etc)."
);
deferred_encode_packet!(ClearDialog, 0x11, "S → C — close a currently-open dialog.");
deferred_encode_packet!(ShowDialog, 0x12, "S → C — open a client dialog.");
deferred_encode_packet!(
    CodeOfConduct,
    0x13,
    "S → C — server's code of conduct text."
);

// ===========================================================================
// Serverbound (C → S)
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
// Remaining serverbound packets (deferred to M5+)
// ---------------------------------------------------------------------------

macro_rules! deferred_decode_packet {
    ($name:ident, $id:expr, $doc:expr) => {
        #[derive(Debug, Clone, Default)]
        #[doc = $doc]
        pub struct $name;

        impl PacketDecode for $name {
            const ID: i32 = $id;
            fn decode<B: Buf>(_buf: &mut B) -> Result<Self, PacketSerError> {
                todo!(concat!(
                    "decode body for `",
                    stringify!($name),
                    "` lands in M5+"
                ));
            }
        }
    };
}

deferred_decode_packet!(
    ResourcePackStatus,
    0x06,
    "C → S — resource-pack status update."
);
deferred_decode_packet!(
    SelectKnownPacksAck,
    0x07,
    "C → S — reply to `SelectKnownPacks`."
);
deferred_decode_packet!(
    CustomClickAction,
    0x08,
    "C → S — custom click action (1.21.6+)."
);
deferred_decode_packet!(
    AcceptCodeOfConduct,
    0x09,
    "C → S — client accepts the code of conduct."
);

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
