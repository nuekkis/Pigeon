//! Trait surface for packets: (de)serialization helpers built on top of
//! the codec layer. Concrete packet implementations live in the `java`
//! module per protocol state.
//!
//! `PacketEncode`/`PacketDecode` are intentionally generic over buffer
//! types so they can be implemented by zero-copy `Bytes` readers as well
//! as owned `Vec<u8>` writers.

use bytes::{Buf, BufMut};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PacketSerError {
    #[error("buffer underflow")]
    Underflow,
    #[error("buffer overflow")]
    Overflow,
    #[error("invalid value")]
    InvalidValue,
    #[error("decode error: {0}")]
    Decode(#[from] pigeon_codecs::VarIntReadError),
    #[error("encode error: {0}")]
    Encode(#[from] pigeon_codecs::VarIntWriteError),
}

/// Trait implemented by every outbound packet enum variant.
pub trait PacketEncode {
    /// Numeric packet id used on the wire.
    const ID: i32;

    /// Write the packet body (without var int length/id prefix) into the buffer.
    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<(), PacketSerError>;
}

/// Trait implemented by every inbound packet enum variant.
pub trait PacketDecode: Sized {
    /// Numeric packet id used on the wire.
    const ID: i32;

    /// Read the packet body (without var int length/id prefix).
    fn decode<B: Buf>(buf: &mut B) -> Result<Self, PacketSerError>;
}

/// Read a VarInt as `i32`.
pub fn read_var_int_buf<B: Buf>(buf: &mut B) -> Result<i32, PacketSerError> {
    Ok(pigeon_codecs::read_var_int(buf)?)
}

/// Write a VarInt.
pub fn write_var_int_buf<B: BufMut>(v: i32, buf: &mut B) -> Result<(), PacketSerError> {
    Ok(pigeon_codecs::write_var_int(v, buf)?)
}

/// VarInt-encoded length is also used for string lengths.
pub fn read_string<B: Buf>(buf: &mut B, max_len: usize) -> Result<String, PacketSerError> {
    let len = pigeon_codecs::read_var_int(buf)?;
    if len < 0 || len as usize > max_len {
        return Err(PacketSerError::InvalidValue);
    }
    let len = len as usize;
    if buf.remaining() < len {
        return Err(PacketSerError::Underflow);
    }
    let mut bytes = vec![0u8; len];
    for byte in bytes.iter_mut() {
        *byte = buf.get_u8();
    }
    String::from_utf8(bytes).map_err(|_| PacketSerError::InvalidValue)
}

pub fn write_string<B: BufMut>(value: &str, buf: &mut B, _max_len: usize) -> Result<(), PacketSerError> {
    let bytes = cesu8::encode_java(value);
    pigeon_codecs::write_var_int(bytes.len() as i32, buf)?;
    if buf.remaining_mut() < bytes.len() {
        return Err(PacketSerError::Overflow);
    }
    buf.put_slice(&bytes);
    Ok(())
}

pub fn read_uuid<B: Buf>(buf: &mut B) -> Result<uuid::Uuid, PacketSerError> {
    if buf.remaining() < 16 {
        return Err(PacketSerError::Underflow);
    }
    let mut bytes = [0u8; 16];
    for byte in bytes.iter_mut() {
        *byte = buf.get_u8();
    }
    Ok(uuid::Uuid::from_bytes(bytes))
}

pub fn write_uuid<B: BufMut>(value: uuid::Uuid, buf: &mut B) -> Result<(), PacketSerError> {
    let bytes = value.as_bytes();
    if buf.remaining_mut() < 16 {
        return Err(PacketSerError::Overflow);
    }
    buf.put_slice(bytes);
    Ok(())
}
