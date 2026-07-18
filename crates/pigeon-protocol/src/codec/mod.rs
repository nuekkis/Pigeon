//! Packet-level codec: framing, compression, encryption.
//!
//! The codec owns the optional compression threshold and an optional
//! AES-CFB8 cipher. It produces/consumes `Bytes` frames containing a
//! fully decoded `(packet_id, payload)` pair, prefixed by the VarInt length.

mod compression;
mod encryption;
mod framed;

pub use compression::{compress_payload, decompress_payload, CompressionError};
pub use encryption::{CipherRead, CipherWrite, EncryptionError, SharedSecret};
pub use framed::{DecodedPacket, EncodedPacket, PacketCodec, PacketDecodeError, PacketEncodeError};
