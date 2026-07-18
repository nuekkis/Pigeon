//! Packet framing codec compatible with `tokio_util::codec`.
//!
//! The codec switches between three modes during the lifetime of a
//! connection:
//! 1. **Uncompressed** — initial state, frames are `[VarInt length][VarInt packet_id][payload]`.
//! 2. **Compressed** (after `Set Compression` is sent/received with threshold N): frames
//!    become `[VarInt compressed_len][VarInt data_len?][compressed bytes]`.
//!    When the (packet_id + payload) is shorter than N, the data_len prefix is `0` and
//!    the bytes are sent uncompressed.
//! 3. **Encrypted** — independent of 1/2; when active, the underlying
//!    byte stream is AES-CFB8 transformed before being framed.

use crate::codec::encryption::{CipherRead, CipherWrite, SharedSecret};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use thiserror::Error;
use tokio_util::codec::{Decoder, Encoder};

#[derive(Debug, Error)]
pub enum PacketDecodeError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("buffer underflow while reading frame")]
    Underflow,
    #[error("var int error: {0}")]
    VarInt(#[from] pigeon_codecs::VarIntReadError),
    #[error("compression error: {0}")]
    Compression(#[from] crate::codec::CompressionError),
    #[error("frame too large: {0} bytes (max {1})")]
    TooLarge(usize, usize),
}

#[derive(Debug, Error)]
pub enum PacketEncodeError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("var int error: {0}")]
    VarInt(#[from] pigeon_codecs::VarIntWriteError),
    #[error("compression error: {0}")]
    Compression(#[from] crate::codec::CompressionError),
    #[error("encryption error: {0}")]
    Encryption(#[from] crate::codec::encryption::EncryptionError),
}

/// A decoded packet: `(packet_id, payload_bytes)`.
/// The payload is a `Bytes` slice (zero-copy) referencing the codec's
/// internal buffer once decoded.
#[derive(Debug, Clone)]
pub struct DecodedPacket {
    pub id: i32,
    pub payload: Bytes,
}

/// Hard cap on a single framed packet length to prevent malicious clients
/// from exhausting memory. Vanilla allows up to 2 MiB; we use 8 MiB for
/// headroom on chunk packets.
pub const MAX_PACKET_BYTES: usize = 8 * 1024 * 1024;

pub struct PacketCodec {
    /// Compression threshold. `Some(N)` means enabled: payloads of length
    /// >= N (counting packet_id + payload) get zlib-compressed.
    compression_threshold: Option<i32>,
    cipher_read: Option<CipherRead>,
    cipher_write: Option<CipherWrite>,
    /// Number of bytes at the front of the next decode call's buffer that
    /// have already been decrypted. The bytes beyond this offset in the
    /// incoming `BytesMut` are still ciphertext.
    decrypt_offset: usize,
}

impl Default for PacketCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl PacketCodec {
    pub fn new() -> Self {
        Self {
            compression_threshold: None,
            cipher_read: None,
            cipher_write: None,
            decrypt_offset: 0,
        }
    }

    pub fn set_compression(&mut self, threshold: i32) {
        self.compression_threshold = Some(threshold);
    }

    pub fn disable_compression(&mut self) {
        self.compression_threshold = None;
    }

    pub fn enable_encryption(&mut self, secret: SharedSecret) {
        self.cipher_read = Some(CipherRead::new(&secret));
        self.cipher_write = Some(CipherWrite::new(&secret));
        self.decrypt_offset = 0;
    }
}

impl Decoder for PacketCodec {
    type Item = DecodedPacket;
    type Error = PacketDecodeError;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<DecodedPacket>, PacketDecodeError> {
        // If encryption is enabled, decrypt any new ciphertext bytes that
        // arrived since the previous decode call. We track `decrypt_offset`
        // to know which bytes have already been transformed.
        if let Some(cipher) = self.cipher_read.as_mut() {
            if buf.len() <= self.decrypt_offset {
                return Ok(None);
            }
            let mut tail = buf.split_off(self.decrypt_offset);
            let mut chunk = tail.to_vec();
            cipher.decrypt(&mut chunk);
            buf.truncate(self.decrypt_offset);
            buf.extend_from_slice(&chunk);
            self.decrypt_offset = buf.len();
        }
        // Peek at the VarInt length using a cursor over a reference to `buf`.
        // We must not consume the bytes until we know the full frame is
        // available, so we use a non-mutating cursor here.
        let mut peek = std::io::Cursor::new(&buf[..]);
        let total_len = match pigeon_codecs::read_var_int(&mut peek) {
            Ok(v) => v,
            Err(pigeon_codecs::VarIntReadError::Underflow) => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        if total_len < 0 {
            return Err(PacketDecodeError::Underflow);
        }
        let total_len = total_len as usize;
        if total_len > MAX_PACKET_BYTES {
            return Err(PacketDecodeError::TooLarge(total_len, MAX_PACKET_BYTES));
        }
        let header_len = peek.position() as usize;
        let needed = header_len + total_len;
        if buf.remaining() < needed {
            buf.reserve(needed - buf.remaining());
            return Ok(None);
        }

        // Advance past the length var int and take the body bytes out.
        buf.advance(header_len);
        let body = buf.split_to(total_len).freeze();

        // When encryption was active the decrypted bytes have been merged
        // back into `buf` above; in both cases we must keep `decrypt_offset`
        // consistent so future ciphertext isn't double-processed.
        if self.cipher_read.is_some() {
            self.decrypt_offset = buf.len();
        }

        // Parse the body: optional compression + VarInt packet id + payload.
        let (packet_id, payload) = match self.compression_threshold {
            Some(_threshold) => {
                let mut cursor = std::io::Cursor::new(&body[..]);
                let data_len = pigeon_codecs::read_var_int(&mut cursor)?;
                let header_consumed = cursor.position() as usize;
                if data_len == 0 {
                    // Uncompressed sentinel: rest of body is `[VarInt id][payload]`.
                    let rest = body.slice(header_consumed..);
                    let mut rest_cursor = std::io::Cursor::new(&rest[..]);
                    let id = pigeon_codecs::read_var_int(&mut rest_cursor)?;
                    let id_consumed = rest_cursor.position() as usize;
                    (id, rest.slice(id_consumed..))
                } else {
                    let data_len = data_len as usize;
                    let compressed = &body[header_consumed..];
                    let decompressed =
                        crate::codec::compression::decompress_payload(compressed)?;
                    if decompressed.len() != data_len {
                        return Err(PacketDecodeError::Compression(
                            crate::codec::CompressionError::LengthMismatch,
                        ));
                    }
                    let mut pcursor = std::io::Cursor::new(&decompressed[..]);
                    let id = pigeon_codecs::read_var_int(&mut pcursor)?;
                    let id_consumed = pcursor.position() as usize;
                    (id, Bytes::copy_from_slice(&decompressed[id_consumed..]))
                }
            }
            None => {
                let mut cursor = std::io::Cursor::new(&body[..]);
                let id = pigeon_codecs::read_var_int(&mut cursor)?;
                let consumed = cursor.position() as usize;
                (id, body.slice(consumed..))
            }
        };

        Ok(Some(DecodedPacket { id, payload }))
    }
}

impl Encoder<EncodedPacket> for PacketCodec {
    type Error = PacketEncodeError;

    fn encode(
        &mut self,
        item: EncodedPacket,
        buf: &mut BytesMut,
    ) -> Result<(), PacketEncodeError> {
        // Step 1: pack [VarInt packet_id][payload] into a temp.
        let mut inner = BytesMut::with_capacity(8 + item.payload.len());
        pigeon_codecs::write_var_int(item.id, &mut inner)?;
        inner.put_slice(item.payload.as_ref());

        // Step 2: optionally compress.
        let frame_body = match self.compression_threshold {
            Some(threshold) if inner.len() >= threshold as usize => {
                let mut compressed = BytesMut::new();
                crate::codec::compression::compress_payload(&inner, &mut compressed)?;
                compressed
            }
            Some(_) => {
                // Send uncompressed path: prepend a `0` VarInt data_length.
                let mut framed = BytesMut::with_capacity(1 + inner.len());
                pigeon_codecs::write_var_int(0, &mut framed)?;
                framed.put_slice(&inner);
                framed
            }
            None => inner,
        };

        // Step 3: build the full frame (outer var int length + body) into a
        // temporary `Vec<u8>` so that AES/CFB8 can mutate the bytes in place.
        let mut frame = Vec::with_capacity(5 + frame_body.len());
        let mut frame_buf = BytesMut::new();
        pigeon_codecs::write_var_int(frame_body.len() as i32, &mut frame_buf)?;
        frame.extend_from_slice(&frame_buf);
        frame.extend_from_slice(&frame_body);

        // Step 4: optionally encrypt in place.
        if let Some(cipher) = self.cipher_write.as_mut() {
            cipher.encrypt(&mut frame);
        }

        // Step 5: copy the (now possibly encrypted) frame into the output buf.
        if buf.remaining_mut() < frame.len() {
            return Err(PacketEncodeError::Compression(
                crate::codec::CompressionError::Overflow,
            ));
        }
        buf.put_slice(&frame);

        Ok(())
    }
}

/// Packet to encode: `(packet_id, payload)`.
#[derive(Debug, Clone)]
pub struct EncodedPacket {
    pub id: i32,
    pub payload: Bytes,
}

impl EncodedPacket {
    pub fn new(id: i32, payload: Bytes) -> Self {
        Self { id, payload }
    }
}
