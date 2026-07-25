//! Login-state packets.
//!
//! Packet IDs (Java 1.21.11 — frozen in `packets.json`):
//!
//! | Direction | ID   | Name (canonical)                | Rust type                |
//! |-----------|------|----------------------------------|--------------------------|
//! | C → S     | 0x00 | `minecraft:hello`               | `LoginStart`             |
//! | S → C     | 0x00 | `minecraft:login_disconnect`     | `DisconnectLogin`        |
//! | S → C     | 0x01 | `minecraft:hello`               | `EncryptionRequest`      |
//! | C → S     | 0x01 | `minecraft:key`                  | `EncryptionResponse`     |
//! | S → C     | 0x02 | `minecraft:login_finished`       | `LoginSuccess`           |
//! | S → C     | 0x03 | `minecraft:login_compression`    | `SetCompression`         |
//! | C → S     | 0x02 | `minecraft:custom_query_answer`  | `LoginPluginResponse`    |
//! | S → C     | 0x04 | `minecraft:custom_query`         | `LoginPluginRequest`     |
//! | C → S     | 0x03 | `minecraft:login_acknowledged`   | `LoginAcknowledged`      |
//! | S → C     | 0x05 | `minecraft:cookie_request`        | `CookieRequest`          |
//! | C → S     | 0x04 | `minecraft:cookie_response`       | `CookieResponse`         |
//!
//! Wire ids are kept in sync with `pigeon-data`'s embedded `packets.json`
//! report — see [`crate::java::ids::login`] and the regression tests in
//! [`crate::java::ids`].

use bytes::{Buf, BufMut};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::ser::{PacketDecode, PacketEncode, PacketSerError};

// ---------------------------------------------------------------------------
// C → S : Login Start (id = 0x00)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct LoginStart {
    pub name: String,
    pub uuid: Uuid,
}

impl PacketDecode for LoginStart {
    const ID: i32 = 0x00;

    fn decode<B: Buf>(buf: &mut B) -> Result<Self, PacketSerError> {
        let name = crate::ser::read_string(buf, 16)?;
        let uuid = crate::ser::read_uuid(buf)?;
        Ok(Self { name, uuid })
    }
}

// ---------------------------------------------------------------------------
// S → C : Encryption Request (id = 0x01)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct EncryptionRequest {
    pub server_id: String,
    pub public_key: Vec<u8>,
    pub verify_token: Vec<u8>,
    pub should_authenticate: bool,
}

impl PacketEncode for EncryptionRequest {
    const ID: i32 = 0x01;

    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<(), PacketSerError> {
        crate::ser::write_string(&self.server_id, buf, 20)?;
        pigeon_codecs::write_var_int(self.public_key.len() as i32, buf)?;
        if buf.remaining_mut() < self.public_key.len() {
            return Err(PacketSerError::Overflow);
        }
        buf.put_slice(&self.public_key);
        pigeon_codecs::write_var_int(self.verify_token.len() as i32, buf)?;
        if buf.remaining_mut() < self.verify_token.len() {
            return Err(PacketSerError::Overflow);
        }
        buf.put_slice(&self.verify_token);
        if buf.remaining_mut() < 1 {
            return Err(PacketSerError::Overflow);
        }
        buf.put_u8(self.should_authenticate as u8);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// C → S : Encryption Response (id = 0x01)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct EncryptionResponse {
    pub shared_secret: Vec<u8>,
    pub verify_token: Vec<u8>,
}

impl PacketDecode for EncryptionResponse {
    const ID: i32 = 0x01;

    fn decode<B: Buf>(buf: &mut B) -> Result<Self, PacketSerError> {
        let secret_len = pigeon_codecs::read_var_int(buf)?;
        if secret_len < 0 || secret_len as usize > buf.remaining() {
            return Err(PacketSerError::InvalidValue);
        }
        let secret_len = secret_len as usize;
        let mut shared_secret = vec![0u8; secret_len];
        for byte in shared_secret.iter_mut() {
            *byte = buf.get_u8();
        }
        let token_len = pigeon_codecs::read_var_int(buf)?;
        if token_len < 0 || token_len as usize > buf.remaining() {
            return Err(PacketSerError::InvalidValue);
        }
        let token_len = token_len as usize;
        let mut verify_token = vec![0u8; token_len];
        for byte in verify_token.iter_mut() {
            *byte = buf.get_u8();
        }
        Ok(Self {
            shared_secret,
            verify_token,
        })
    }
}

// ---------------------------------------------------------------------------
// S → C : Login Success (id = 0x02)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct LoginSuccess {
    pub uuid: Uuid,
    pub username: String,
    pub properties: Vec<Property>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Property {
    pub name: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

impl PacketEncode for LoginSuccess {
    const ID: i32 = 0x02;

    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<(), PacketSerError> {
        crate::ser::write_uuid(self.uuid, buf)?;
        crate::ser::write_string(&self.username, buf, 16)?;
        pigeon_codecs::write_var_int(self.properties.len() as i32, buf)?;
        for prop in &self.properties {
            crate::ser::write_string(&prop.name, buf, 32767)?;
            crate::ser::write_string(&prop.value, buf, 32767)?;
            let has_sig = prop.signature.is_some() as u8;
            if buf.remaining_mut() < 1 {
                return Err(PacketSerError::Overflow);
            }
            buf.put_u8(has_sig);
            if let Some(sig) = &prop.signature {
                crate::ser::write_string(sig, buf, 32767)?;
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// S → C : Set Compression (id = 0x03)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct SetCompression {
    pub threshold: i32,
}

impl PacketEncode for SetCompression {
    const ID: i32 = 0x03;

    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<(), PacketSerError> {
        pigeon_codecs::write_var_int(self.threshold, buf)?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// S → C : Disconnect (Login) (id = 0x00 — `minecraft:login_disconnect`)
// ---------------------------------------------------------------------------

/// Login-phase disconnect. Wire id is **0** (not 4) in 1.21.11 — kept in
/// sync with `packets.json` via [`crate::java::ids`].
#[derive(Debug, Clone)]
pub struct DisconnectLogin {
    pub reason_json: String,
}

impl PacketEncode for DisconnectLogin {
    const ID: i32 = 0x00;

    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<(), PacketSerError> {
        crate::ser::write_string(&self.reason_json, buf, 262144)
    }
}

// ---------------------------------------------------------------------------
// C → S : Login Acknowledged (id = 0x03)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct LoginAcknowledged;

impl PacketDecode for LoginAcknowledged {
    const ID: i32 = 0x03;

    fn decode<B: Buf>(_buf: &mut B) -> Result<Self, PacketSerError> {
        Ok(Self)
    }
}

// ---------------------------------------------------------------------------
// S → C : Login Plugin Request (id = 0x04 — `minecraft:custom_query`)
// ---------------------------------------------------------------------------

/// Server-initiated plugin messaging during login. The client must answer
/// with [`LoginPluginResponse`] carrying the same `message_id`, even if
/// it does not understand the channel (in which case it sends
/// `successful = false` and an empty payload).
#[derive(Debug, Clone)]
pub struct LoginPluginRequest {
    /// Server-chosen id used to match the eventual response.
    pub message_id: i32,
    /// Channel identifier (resource location), e.g. `minecraft:brand`.
    pub channel: String,
    /// Optional raw payload. `None` corresponds to a `false` `has_data`
    /// marker byte and a body-less packet.
    pub data: Option<Vec<u8>>,
}

impl PacketEncode for LoginPluginRequest {
    const ID: i32 = 0x04;

    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<(), PacketSerError> {
        pigeon_codecs::write_var_int(self.message_id, buf)?;
        crate::ser::write_string(&self.channel, buf, 32767)?;
        if let Some(data) = &self.data {
            if buf.remaining_mut() < 1 {
                return Err(PacketSerError::Overflow);
            }
            buf.put_u8(1);
            if buf.remaining_mut() < data.len() {
                return Err(PacketSerError::Overflow);
            }
            buf.put_slice(data);
        } else if buf.remaining_mut() < 1 {
            return Err(PacketSerError::Overflow);
        } else {
            buf.put_u8(0);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// C → S : Login Plugin Response (id = 0x02 — `minecraft:custom_query_answer`)
// ---------------------------------------------------------------------------

/// Client reply to a [`LoginPluginRequest`]. When `successful` is `false`
/// the payload must be empty.
#[derive(Debug, Clone)]
pub struct LoginPluginResponse {
    /// Matches the request's `message_id`.
    pub message_id: i32,
    /// Whether the channel was understood and a payload returned.
    pub successful: bool,
    /// Raw payload data; only meaningful when `successful` is `true`.
    pub data: Vec<u8>,
}

impl PacketDecode for LoginPluginResponse {
    const ID: i32 = 0x02;

    fn decode<B: Buf>(buf: &mut B) -> Result<Self, PacketSerError> {
        let message_id = pigeon_codecs::read_var_int(buf)?;
        if buf.remaining() < 1 {
            return Err(PacketSerError::Underflow);
        }
        let successful = buf.get_u8() != 0;
        let remaining = buf.remaining();
        let mut data = vec![0u8; remaining];
        for byte in data.iter_mut() {
            *byte = buf.get_u8();
        }
        Ok(Self {
            message_id,
            successful,
            data,
        })
    }
}

// ---------------------------------------------------------------------------
// S → C : Cookie Request (id = 0x05 — `minecraft:cookie_request`)
// ---------------------------------------------------------------------------

/// Server asks the client to provide a previously stored cookie. The
/// client must respond with [`CookieResponse`] using the same key.
#[derive(Debug, Clone)]
pub struct CookieRequest {
    /// Cookie key as a resource location.
    pub key: String,
}

impl PacketEncode for CookieRequest {
    const ID: i32 = 0x05;

    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<(), PacketSerError> {
        crate::ser::write_string(&self.key, buf, 32767)
    }
}

// ---------------------------------------------------------------------------
// C → S : Cookie Response (id = 0x04 — `minecraft:cookie_response`)
// ---------------------------------------------------------------------------

/// Client reply to a [`CookieRequest`]. When `has_cookies` is `false` the
/// payload must be empty.
#[derive(Debug, Clone)]
pub struct CookieResponse {
    /// Matches the request's `key`.
    pub key: String,
    /// Whether the client has the cookie stored.
    pub has_cookies: bool,
    /// Raw payload bytes; only present when `has_cookies` is `true`.
    pub payload: Vec<u8>,
}

impl PacketDecode for CookieResponse {
    const ID: i32 = 0x04;

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

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    /// Helper: drain the given bytes through `T::decode` and return the value.
    fn decode_from_bytes<T: PacketDecode>(bytes: &[u8]) -> Result<T, PacketSerError> {
        let mut buf = bytes;
        T::decode(&mut buf)
    }

    #[test]
    fn login_plugin_response_decodes_successful_payload() {
        // VarInt(42) | 0x01 (successful=true) | 0x01 0x02 0x03 0x04 (payload)
        let body: &[u8] = &[0x2A, 0x01, 0x01, 0x02, 0x03, 0x04];
        let decoded = decode_from_bytes::<LoginPluginResponse>(body).expect("decode must succeed");
        assert_eq!(decoded.message_id, 42);
        assert!(decoded.successful);
        assert_eq!(decoded.data, vec![1, 2, 3, 4]);
    }

    #[test]
    fn login_plugin_response_decodes_unsuccessful_payload() {
        // VarInt(5) | 0x00 (successful=false) | (nothing)
        let body: &[u8] = &[0x05, 0x00];
        let decoded = decode_from_bytes::<LoginPluginResponse>(body).expect("decode must succeed");
        assert_eq!(decoded.message_id, 5);
        assert!(!decoded.successful);
        assert!(decoded.data.is_empty());
    }

    #[test]
    fn cookie_response_decodes_without_payload() {
        // VarStr("minecraft:key") = VarInt(13) + 'minecraft:key' | 0x00 (has_cookies=false)
        let mut wire = Vec::new();
        pigeon_codecs::write_var_int(13, &mut wire).unwrap();
        wire.extend_from_slice(b"minecraft:key");
        wire.push(0x00);
        let decoded = decode_from_bytes::<CookieResponse>(&wire).expect("decode must succeed");
        assert_eq!(decoded.key, "minecraft:key");
        assert!(!decoded.has_cookies);
        assert!(decoded.payload.is_empty());
    }

    #[test]
    fn cookie_response_decodes_with_payload() {
        // VarStr("k") + 0x01 + VarInt(3) + [0xAA, 0xBB, 0xCC]
        let mut wire = vec![];
        let mut w = BytesMut::new();
        crate::ser::write_string("k", &mut w, 32767).unwrap();
        wire.extend_from_slice(&w);
        wire.push(0x01);
        pigeon_codecs::write_var_int(3, &mut wire).unwrap();
        wire.extend_from_slice(&[0xAA, 0xBB, 0xCC]);
        let decoded = decode_from_bytes::<CookieResponse>(&wire).expect("decode must succeed");
        assert_eq!(decoded.key, "k");
        assert!(decoded.has_cookies);
        assert_eq!(decoded.payload, vec![0xAA, 0xBB, 0xCC]);
    }

    #[test]
    fn login_plugin_request_encodes_empty_payload() {
        let req = LoginPluginRequest {
            message_id: 7,
            channel: "minecraft:brand".to_string(),
            data: None,
        };
        let mut buf = BytesMut::new();
        PacketEncode::encode(&req, &mut buf).expect("encode must succeed");
        let bytes = buf.freeze();
        // Last byte must be the trailing 0 marker for `data: None`.
        assert_eq!(
            *bytes.last().unwrap(),
            0u8,
            "trailing data marker must be 0"
        );
    }

    #[test]
    fn login_plugin_request_encodes_with_payload() {
        let req = LoginPluginRequest {
            message_id: 1,
            channel: "minecraft:brand".to_string(),
            data: Some(vec![0xDE, 0xAD]),
        };
        let mut buf = BytesMut::new();
        PacketEncode::encode(&req, &mut buf).expect("encode must succeed");
        let bytes = buf.freeze();
        // The marker byte must be 1, followed immediately by the payload.
        let last_idx = bytes.len() - 1;
        assert_eq!(bytes[last_idx], 0xAD, "payload tail must be encoded");
        assert_eq!(bytes[last_idx - 1], 0xDE, "payload head must be encoded");
        assert_eq!(bytes[last_idx - 2], 1u8, "data marker must be 1");
    }
}
