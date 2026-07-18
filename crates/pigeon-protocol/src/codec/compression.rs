//! Compression (zlib) for the Java protocol packet stream.

use bytes::{BufMut, BytesMut};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::{Read, Write};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CompressionError {
    #[error("compression buffer overflow")]
    Overflow,
    #[error("invalid compression: data length mismatch")]
    LengthMismatch,
    #[error("zlib io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Compress `packet_id_and_payload` with zlib into `out`, prefixing the
/// frame with the uncompressed length as a VarInt (Java wire format).
pub fn compress_payload(
    packet_id_and_payload: &[u8],
    out: &mut BytesMut,
) -> Result<(), CompressionError> {
    let uncompressed_len = packet_id_and_payload.len();
    pigeon_codecs::write_var_int(uncompressed_len as i32, out)
        .map_err(|_| CompressionError::Overflow)?;
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(packet_id_and_payload)?;
    let compressed = encoder.finish()?;
    if out.remaining_mut() < compressed.len() {
        return Err(CompressionError::Overflow);
    }
    out.put_slice(&compressed);
    Ok(())
}

/// Decompress a single zlib frame from `input`. `input` should start with
/// a VarInt data length, followed by the compressed bytes. Returns the
/// uncompressed bytes. If `data_len == 0`, the rest of `input` is treated
/// as already-uncompressed (matching the Minecraft "encoded without
/// compression" sentinel).
pub fn decompress_payload(input: &[u8]) -> Result<Vec<u8>, CompressionError> {
    let mut buf = std::io::Cursor::new(input);
    let data_len =
        pigeon_codecs::read_var_int(&mut buf).map_err(|_| CompressionError::LengthMismatch)?;
    if data_len < 0 {
        return Err(CompressionError::LengthMismatch);
    }
    let consumed = buf.position() as usize;
    let rest = &input[consumed..];
    if data_len == 0 {
        return Ok(rest.to_vec());
    }
    let mut decoder = ZlibDecoder::new(rest);
    let mut out = Vec::with_capacity(data_len as usize);
    decoder
        .read_to_end(&mut out)
        .map_err(CompressionError::Io)?;
    Ok(out)
}
