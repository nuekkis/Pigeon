use bytes::{Buf, BufMut};
use thiserror::Error;

const SEGMENT_BITS: u8 = 0x7F;
const CONTINUE_BIT: u8 = 0x80;

/// Maximum number of bytes a VarInt can occupy (7 bits × 5 = 35 bits).
pub const MAX_VAR_INT_BYTES: usize = 5;

#[derive(Debug, Error)]
pub enum VarIntReadError {
    #[error("var int is too large (more than {MAX_VAR_INT_BYTES} bytes)")]
    TooLarge,
    #[error("buffer underflow while reading var int")]
    Underflow,
}

#[derive(Debug, Error)]
pub enum VarIntWriteError {
    #[error("buffer overflow while writing var int")]
    Overflow,
}

/// Reads a signed VarInt (LEB128 zig-zag is NOT used by MC; raw two's complement bits).
pub fn read_var_int<B: Buf>(buf: &mut B) -> Result<i32, VarIntReadError> {
    let mut result: u32 = 0;
    for i in 0..MAX_VAR_INT_BYTES {
        if !buf.has_remaining() {
            return Err(VarIntReadError::Underflow);
        }
        let byte = buf.get_u8();
        let value = (byte & SEGMENT_BITS) as u32;
        result |= value << (7 * i);
        if (byte & CONTINUE_BIT) == 0 {
            return Ok(result as i32);
        }
    }
    Err(VarIntReadError::TooLarge)
}

/// Writes a signed VarInt.
pub fn write_var_int<B: BufMut>(value: i32, buf: &mut B) -> Result<(), VarIntWriteError> {
    let mut value = value as u32;
    loop {
        if (value & !u32::from(SEGMENT_BITS)) == 0 {
            if buf.remaining_mut() < 1 {
                return Err(VarIntWriteError::Overflow);
            }
            buf.put_u8(value as u8);
            return Ok(());
        }
        if buf.remaining_mut() < 1 {
            return Err(VarIntWriteError::Overflow);
        }
        buf.put_u8((value as u8 & SEGMENT_BITS) | CONTINUE_BIT);
        value >>= 7;
    }
}

/// Size in bytes a VarInt will occupy once serialized.
pub fn var_int_size(value: i32) -> usize {
    let mut value = value as u32;
    let mut size = 0;
    loop {
        size += 1;
        value >>= 7;
        if value == 0 {
            break;
        }
    }
    size
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn round_trip_zero() {
        let mut buf = BytesMut::new();
        write_var_int(0, &mut buf).unwrap();
        assert_eq!(buf, &[0]);
        let mut read_buf = buf.freeze();
        assert_eq!(read_var_int(&mut read_buf).unwrap(), 0);
    }

    #[test]
    fn round_trip_one() {
        let mut buf = BytesMut::new();
        write_var_int(1, &mut buf).unwrap();
        assert_eq!(buf, &[1]);
        let mut read_buf = buf.freeze();
        assert_eq!(read_var_int(&mut read_buf).unwrap(), 1);
    }

    #[test]
    fn round_trip_127() {
        let mut buf = BytesMut::new();
        write_var_int(127, &mut buf).unwrap();
        assert_eq!(buf, &[127]);
        let mut read_buf = buf.freeze();
        assert_eq!(read_var_int(&mut read_buf).unwrap(), 127);
    }

    #[test]
    fn round_trip_128() {
        let mut buf = BytesMut::new();
        write_var_int(128, &mut buf).unwrap();
        assert_eq!(buf, &[128, 1]);
        let mut read_buf = buf.freeze();
        assert_eq!(read_var_int(&mut read_buf).unwrap(), 128);
    }

    #[test]
    fn round_trip_minus_one() {
        let mut buf = BytesMut::new();
        write_var_int(-1, &mut buf).unwrap();
        assert_eq!(buf, &[255, 255, 255, 255, 15]);
        let mut read_buf = buf.freeze();
        assert_eq!(read_var_int(&mut read_buf).unwrap(), -1);
    }

    #[test]
    fn size_examples() {
        assert_eq!(var_int_size(0), 1);
        assert_eq!(var_int_size(1), 1);
        assert_eq!(var_int_size(127), 1);
        assert_eq!(var_int_size(128), 2);
        assert_eq!(var_int_size(-1), 5);
    }
}
