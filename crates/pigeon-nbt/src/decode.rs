use crate::value::{Nbt, NbtCompound, NbtList, NbtTag, NbtValue, MAX_DEPTH};
use bytes::Buf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NbtDecodeError {
    #[error("unexpected end of buffer")]
    Eof,
    #[error("invalid tag id: {0}")]
    InvalidTagId(u8),
    #[error("invalid utf-8 in nbt string")]
    InvalidUtf8,
    #[error("nbt nesting depth exceeded ({MAX_DEPTH})")]
    DepthExceeded,
    #[error("invalid list element type: {0}")]
    InvalidListType(u8),
}

/// A streaming NBT reader backed by `bytes::Buf`.
pub struct NbtReader<'a, B> {
    buf: &'a mut B,
    depth: usize,
}

impl<'a, B: Buf> NbtReader<'a, B> {
    pub fn new(buf: &'a mut B) -> Self {
        Self { buf, depth: 0 }
    }

    fn need(&self, n: usize) -> Result<(), NbtDecodeError> {
        if self.buf.remaining() < n {
            return Err(NbtDecodeError::Eof);
        }
        Ok(())
    }

    fn read_u8(&mut self) -> Result<u8, NbtDecodeError> {
        self.need(1)?;
        Ok(self.buf.get_u8())
    }

    fn read_u16(&mut self) -> Result<u16, NbtDecodeError> {
        self.need(2)?;
        Ok(self.buf.get_u16())
    }

    fn read_i16(&mut self) -> Result<i16, NbtDecodeError> {
        self.need(2)?;
        Ok(self.buf.get_i16())
    }

    fn read_i32(&mut self) -> Result<i32, NbtDecodeError> {
        self.need(4)?;
        Ok(self.buf.get_i32())
    }

    fn read_i64(&mut self) -> Result<i64, NbtDecodeError> {
        self.need(8)?;
        Ok(self.buf.get_i64())
    }

    fn read_f32(&mut self) -> Result<f32, NbtDecodeError> {
        self.need(4)?;
        Ok(self.buf.get_f32())
    }

    fn read_f64(&mut self) -> Result<f64, NbtDecodeError> {
        self.need(8)?;
        Ok(self.buf.get_f64())
    }

    fn read_string(&mut self) -> Result<String, NbtDecodeError> {
        let len = self.read_u16()? as usize;
        self.need(len)?;
        let mut out = vec![0u8; len];
        for byte in out.iter_mut() {
            *byte = self.buf.get_u8();
        }
        match cesu8::decode_java(&out) {
            Ok(s) => Ok(s.into_owned()),
            Err(_) => Err(NbtDecodeError::InvalidUtf8),
        }
    }

    fn read_payload(&mut self, tag: NbtTag) -> Result<NbtValue, NbtDecodeError> {
        match tag {
            NbtTag::End => unreachable!("End tag should never be decoded directly"),
            NbtTag::Byte => Ok(NbtValue::Byte(self.read_u8()? as i8)),
            NbtTag::Short => Ok(NbtValue::Short(self.read_i16()?)),
            NbtTag::Int => Ok(NbtValue::Int(self.read_i32()?)),
            NbtTag::Long => Ok(NbtValue::Long(self.read_i64()?)),
            NbtTag::Float => Ok(NbtValue::Float(self.read_f32()?)),
            NbtTag::Double => Ok(NbtValue::Double(self.read_f64()?)),
            NbtTag::ByteArray => {
                let len = self.read_i32()?;
                if len < 0 {
                    return Err(NbtDecodeError::Eof);
                }
                let len = len as usize;
                self.need(len)?;
                let mut out = Vec::with_capacity(len);
                for _ in 0..len {
                    out.push(self.read_u8()? as i8);
                }
                Ok(NbtValue::ByteArray(out))
            }
            NbtTag::String => Ok(NbtValue::String(self.read_string()?)),
            NbtTag::List => {
                self.depth += 1;
                if self.depth > MAX_DEPTH {
                    return Err(NbtDecodeError::DepthExceeded);
                }
                let element_type_id = self.read_u8()?;
                let element_type =
                    NbtTag::from_u8(element_type_id).ok_or(NbtDecodeError::InvalidListType(
                        element_type_id,
                    ))?;
                let len = self.read_i32()?;
                if len < 0 {
                    return Err(NbtDecodeError::Eof);
                }
                let len = len as usize;
                let mut elements = Vec::with_capacity(len);
                for _ in 0..len {
                    let value = self.read_payload(element_type)?;
                    elements.push(value);
                }
                self.depth -= 1;
                Ok(NbtValue::List(NbtList {
                    element_type,
                    elements,
                }))
            }
            NbtTag::Compound => {
                self.depth += 1;
                if self.depth > MAX_DEPTH {
                    return Err(NbtDecodeError::DepthExceeded);
                }
                let mut compound = NbtCompound::new();
                loop {
                    let tag_id = self.read_u8()?;
                    if tag_id == 0 {
                        break;
                    }
                    let tag = NbtTag::from_u8(tag_id).ok_or(NbtDecodeError::InvalidTagId(
                        tag_id,
                    ))?;
                    let name = self.read_string()?;
                    let value = self.read_payload(tag)?;
                    compound.insert(name, value);
                }
                self.depth -= 1;
                Ok(NbtValue::Compound(compound))
            }
            NbtTag::IntArray => {
                let len = self.read_i32()?;
                if len < 0 {
                    return Err(NbtDecodeError::Eof);
                }
                let len = len as usize;
                self.need(len * 4)?;
                let mut out = Vec::with_capacity(len);
                for _ in 0..len {
                    out.push(self.read_i32()?);
                }
                Ok(NbtValue::IntArray(out))
            }
            NbtTag::LongArray => {
                let len = self.read_i32()?;
                if len < 0 {
                    return Err(NbtDecodeError::Eof);
                }
                let len = len as usize;
                self.need(len * 8)?;
                let mut out = Vec::with_capacity(len);
                for _ in 0..len {
                    out.push(self.read_i64()?);
                }
                Ok(NbtValue::LongArray(out))
            }
        }
    }

    pub fn read_root(&mut self) -> Result<Nbt, NbtDecodeError> {
        let tag_id = self.read_u8()?;
        if tag_id == 0 {
            return Err(NbtDecodeError::InvalidTagId(0));
        }
        let tag =
            NbtTag::from_u8(tag_id).ok_or(NbtDecodeError::InvalidTagId(tag_id))?;
        let name = self.read_string()?;
        let value = self.read_payload(tag)?;
        match value {
            NbtValue::Compound(root) => Ok(Nbt { name, root }),
            other => Err(NbtDecodeError::InvalidTagId(tag_id)),
        }
    }
    /// Read a single named NBT value of the given tag without prefix header.
    pub fn read_named_payload(
        &mut self,
        tag: NbtTag,
    ) -> Result<NbtValue, NbtDecodeError> {
        self.read_payload(tag)
    }
}
