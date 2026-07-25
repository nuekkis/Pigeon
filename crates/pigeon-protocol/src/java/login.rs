//! Login-state packets.
//!
//! Packet IDs (Java 1.21.11 — frozen in `packets.json`):
//!
//! | Direction | ID   | Name (canonical)              | Rust type           |
//! |-----------|------|-------------------------------|---------------------|
//! | C → S     | 0x00 | `minecraft:hello`             | `LoginStart`        |
//! | S → C     | 0x00 | `minecraft:login_disconnect`  | `DisconnectLogin`   |
//! | S → C     | 0x01 | `minecraft:hello`            | `EncryptionRequest` |
//! | C → S     | 0x01 | `minecraft:key`               | `EncryptionResponse`|
//! | S → C     | 0x02 | `minecraft:login_finished`    | `LoginSuccess`      |
//! | S → C     | 0x03 | `minecraft:login_compression` | `SetCompression`    |
//! | C → S     | 0x02 | `minecraft:custom_query_answer`| (LoginPluginResponse) |
//! | S → C     | 0x04 | `minecraft:custom_query`     | (LoginPluginRequest) |
//! | C → S     | 0x03 | `minecraft:login_acknowledged`| `LoginAcknowledged` |
//! | S → C     | 0x05 | `minecraft:cookie_request`   | (CookieRequest)     |
//! | C → S     | 0x04 | `minecraft:cookie_response`  | (CookieResponse)    |
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
