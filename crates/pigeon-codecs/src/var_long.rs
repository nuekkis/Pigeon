use bytes::{Buf, BufMut};
use thiserror::Error;

const SEGMENT_BITS: u8 = 0x7F;
const CONTINUE_BIT: u8 = 0x80;

/// Maximum number of bytes a VarLong can occupy (7 bits × 10 = 70 bits).
pub const MAX_VAR_LONG_BYTES: usize = 10;

#[derive(Debug, Error)]
pub enum VarLongReadError {
    #[error("var long is too large (more than {MAX_VAR_LONG_BYTES} bytes)")]
    TooLarge,
    #[error("buffer underflow while reading var long")]
    Underflow,
}

#[derive(Debug, Error)]
pub enum VarLongWriteError {
    #[error("buffer overflow while writing var long")]
    Overflow,
}

pub fn read_var_long<B: Buf>(buf: &mut B) -> Result<i64, VarLongReadError> {
    let mut result: u64 = 0;
    for i in 0..MAX_VAR_LONG_BYTES {
        if !buf.has_remaining() {
            return Err(VarLongReadError::Underflow);
        }
        let byte = buf.get_u8();
        let value = (byte & SEGMENT_BITS) as u64;
        result |= value << (7 * i);
        if (byte & CONTINUE_BIT) == 0 {
            return Ok(result as i64);
        }
    }
    Err(VarLongReadError::TooLarge)
}

pub fn write_var_long<B: BufMut>(value: i64, buf: &mut B) -> Result<(), VarLongWriteError> {
    let mut value = value as u64;
    loop {
        if (value & !u64::from(SEGMENT_BITS)) == 0 {
            if buf.remaining_mut() < 1 {
                return Err(VarLongWriteError::Overflow);
            }
            buf.put_u8(value as u8);
            return Ok(());
        }
        if buf.remaining_mut() < 1 {
            return Err(VarLongWriteError::Overflow);
        }
        buf.put_u8((value as u8 & SEGMENT_BITS) | CONTINUE_BIT);
        value >>= 7;
    }
}

#[allow(dead_code)]
pub fn var_long_size(value: i64) -> usize {
    let mut value = value as u64;
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
    fn round_trip_simple() {
        let mut buf = BytesMut::new();
        write_var_long(0, &mut buf).unwrap();
        assert_eq!(buf.as_ref(), &[0][..]);
        let mut read_buf = buf.freeze();
        assert_eq!(read_var_long(&mut read_buf).unwrap(), 0);
    }

    #[test]
    fn round_trip_large() {
        let mut buf = BytesMut::new();
        write_var_long(9223372036854775807, &mut buf).unwrap();
        let mut read_buf = buf.freeze();
        assert_eq!(read_var_long(&mut read_buf).unwrap(), 9223372036854775807);
    }

    #[test]
    fn round_trip_negative() {
        let mut buf = BytesMut::new();
        write_var_long(-1, &mut buf).unwrap();
        let mut read_buf = buf.freeze();
        assert_eq!(read_var_long(&mut read_buf).unwrap(), -1);
    }
}
