//! Wire-format codec for the `Configuration`-phase `RegistryData` packet.
//!
//! The Mojang `RegistryData` packet (1.20.2+) carries a single registry's
//! complete contents: a `registry_id`, an entry count, and a list of
//! `(identifier, nbt)` pairs. Each entry's NBT is a root compound whose
//! name is conventionally empty (`""`) and which contains the entry's
//! serialized data — `dimension_type`, `chat_type`, `cat_variant`, etc.
//!
//! Example wire layout for `minecraft:cat_variant`:
//!
//! ```text
//! registry_id: VarStr("minecraft:cat_variant")
//! entry_count: VarInt(11)
//! repeat entry_count times:
//!     VarStr("minecraft:all_black")
//!     NBT root compound ""
//!         {"asset_id":"minecraft:all_black"}
//! ```
//!
//! This module is independent of the concrete `pigeon-nbt` reader/writer —
//! it pushes bytes through [`bytes::Buf`] / [`bytes::BufMut`] and the
//! codec primitives in [`pigeon_codecs`] and [`pigeon_nbt`].

use bytes::{Buf, BufMut};
use std::str::FromStr;
use thiserror::Error;

use pigeon_nbt::{Nbt, NbtCompound, NbtReader, NbtWriter};
use pigeon_util::Identifier;

#[derive(Debug, Error)]
pub enum RegistryCodecError {
    #[error("buffer underflow while decoding registry")]
    Underflow,
    #[error("buffer overflow while encoding registry")]
    Overflow,
    #[error("invalid registry entry count {0}")]
    InvalidEntryCount(i32),
    #[error("nbt decode error: {0}")]
    NbtDecode(#[from] pigeon_nbt::NbtDecodeError),
    #[error("nbt encode error: {0}")]
    NbtEncode(#[from] pigeon_nbt::NbtEncodeError),
    #[error("varint codec error: {0}")]
    VarInt(#[from] pigeon_codecs::VarIntReadError),
    #[error("invalid identifier {0}")]
    Identifier(#[from] pigeon_util::IdentifierParseError),
    /// An entry's NBT root is not named `""` as expected.
    #[error("nbt root for {entry} has unexpected name {name:?}")]
    UnexpectedRootName { entry: String, name: String },
}

impl From<pigeon_codecs::VarIntWriteError> for RegistryCodecError {
    fn from(value: pigeon_codecs::VarIntWriteError) -> Self {
        // VarIntWriteError has no variant containing useful info today; map
        // it to overflow since the only failure mode is insufficient buffer
        // space.
        match value {
            pigeon_codecs::VarIntWriteError::Overflow => Self::Overflow,
        }
    }
}

/// A wire-format entry: a registry id together with a list of NBT payloads.
#[derive(Debug, Clone)]
pub struct RegistryCodec {
    pub registry_id: Identifier,
    pub entries: Vec<(Identifier, NbtCompound)>,
}

impl RegistryCodec {
    pub fn new(registry_id: Identifier) -> Self {
        Self {
            registry_id,
            entries: Vec::new(),
        }
    }

    /// Push a new entry.
    pub fn push(&mut self, id: Identifier, body: NbtCompound) {
        self.entries.push((id, body));
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Encode this registry into the supplied buffer using the Mojang wire
    /// format expected by the `RegistryData` packet body.
    pub fn encode<B: BufMut>(&self, buf: &mut B) -> Result<(), RegistryCodecError> {
        // registry_id (VarStr).
        let id_str = self.registry_id.as_str();
        let id_bytes = id_str.as_bytes();
        pigeon_codecs::write_var_int(id_bytes.len() as i32, buf)?;
        if buf.remaining_mut() < id_bytes.len() {
            return Err(RegistryCodecError::Overflow);
        }
        buf.put_slice(id_bytes);

        // entry_count (VarInt).
        pigeon_codecs::write_var_int(self.entries.len() as i32, buf)?;

        // Each entry: VarStr(identifier) + NBT root compound (named "").
        for (name, body) in &self.entries {
            let name_str = name.as_str();
            let name_bytes = name_str.as_bytes();
            pigeon_codecs::write_var_int(name_bytes.len() as i32, buf)?;
            if buf.remaining_mut() < name_bytes.len() {
                return Err(RegistryCodecError::Overflow);
            }
            buf.put_slice(name_bytes);

            let nbt = Nbt::new(String::new(), body.clone());
            NbtWriter::new(buf).write_root(&nbt)?;
        }
        Ok(())
    }

    /// Decode a registry from the supplied buffer — the inverse of
    /// [`Self::encode`].
    ///
    /// `nbt_root_name` is the expected root compound name (vanilla uses
    /// `""`); an unexpected name returns [`Self::UnexpectedRootName`].
    pub fn decode<B: Buf>(buf: &mut B) -> Result<Self, RegistryCodecError> {
        let registry_id_str = read_var_int_prefixed_str(buf)?;
        let registry_id = Identifier::from_str(&registry_id_str)?;

        let count = pigeon_codecs::read_var_int(buf)?;
        if count < 0 {
            return Err(RegistryCodecError::InvalidEntryCount(count));
        }
        let count = count as usize;

        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            let name = read_var_int_prefixed_str(buf)?;
            let entry_id = Identifier::from_str(&name)?;
            let nbt = NbtReader::new(buf).read_root()?;
            if !nbt.name.is_empty() {
                return Err(RegistryCodecError::UnexpectedRootName {
                    entry: entry_id.as_str(),
                    name: nbt.name.clone(),
                });
            }
            // `read_root` guarantees the root tag is a compound (it returns
            // `InvalidTagId` otherwise) so we can move the compound out
            // unconditionally.
            entries.push((entry_id, nbt.root));
        }

        Ok(Self {
            registry_id,
            entries,
        })
    }
}

/// Pull a VarInt-prefixed UTF-8 string from the buffer.
fn read_var_int_prefixed_str<B: Buf>(buf: &mut B) -> Result<String, RegistryCodecError> {
    let len = pigeon_codecs::read_var_int(buf)?;
    if len < 0 || len as usize > buf.remaining() {
        return Err(RegistryCodecError::Underflow);
    }
    let len = len as usize;
    let mut bytes = vec![0u8; len];
    for byte in bytes.iter_mut() {
        if buf.remaining() < 1 {
            return Err(RegistryCodecError::Underflow);
        }
        *byte = buf.get_u8();
    }
    let s = String::from_utf8(bytes).map_err(|_| RegistryCodecError::Underflow)?;
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;
    use pigeon_nbt::{NbtCompound, NbtValue};

    /// Round-trips a tiny registry through the codec.
    fn make_sample_codec() -> RegistryCodec {
        let mut root = NbtCompound::new();
        root.insert("asset_id", NbtValue::String("minecraft:tabby".into()));
        let entry_id = Identifier::minecraft("tabby").unwrap();
        let registry_id = Identifier::minecraft("cat_variant").unwrap();
        let mut codec = RegistryCodec::new(registry_id);
        codec.push(entry_id, root);
        codec
    }

    #[test]
    fn round_trip_minimal_registry() {
        let original = make_sample_codec();
        let mut buf = BytesMut::new();
        original.encode(&mut buf).expect("encode must succeed");
        let bytes = buf.freeze();
        let mut reader = bytes.as_ref();
        let decoded = RegistryCodec::decode(&mut reader).expect("decode must succeed");
        assert!(reader.is_empty(), "no trailing bytes after decode");
        assert_eq!(decoded.registry_id, original.registry_id);
        assert_eq!(decoded.entries.len(), 1);
        assert_eq!(decoded.entries[0].0, original.entries[0].0);
    }

    #[test]
    fn round_trip_empty_registry() {
        let registry_id = Identifier::minecraft("dimension_type").unwrap();
        let codec = RegistryCodec::new(registry_id.clone());
        let mut buf = BytesMut::new();
        codec.encode(&mut buf).expect("encode must succeed");
        let decoded = RegistryCodec::decode(&mut buf.freeze()).expect("decode must succeed");
        assert_eq!(decoded.registry_id, registry_id);
        assert!(decoded.entries.is_empty());
    }

    #[test]
    fn round_trip_multiple_entries_preserve_order() {
        let registry_id = Identifier::minecraft("cat_variant").unwrap();
        let mut codec = RegistryCodec::new(registry_id);
        let names = vec![
            "all_black",
            "british_shorthair",
            "calico",
            "jellie",
            "persian",
            "ragdoll",
            "tabby",
        ];
        for n in &names {
            let mut c = NbtCompound::new();
            c.insert("asset_id", NbtValue::String(format!("minecraft:{n}")));
            codec.push(Identifier::minecraft(*n).unwrap(), c);
        }
        let mut buf = BytesMut::new();
        codec.encode(&mut buf).expect("encode must succeed");
        let decoded = RegistryCodec::decode(&mut buf.freeze()).expect("decode must succeed");
        let got: Vec<String> = decoded
            .entries
            .iter()
            .map(|(id, _)| id.path().to_string())
            .collect();
        assert_eq!(got, names);
    }
}
