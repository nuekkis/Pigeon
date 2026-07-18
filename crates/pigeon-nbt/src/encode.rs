use crate::value::{Nbt, NbtTag, NbtValue};
use bytes::BufMut;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NbtEncodeError {
    #[error("buffer overflow while encoding nbt")]
    Overflow,
}

pub struct NbtWriter<'a, B> {
    buf: &'a mut B,
}

impl<'a, B: BufMut> NbtWriter<'a, B> {
    pub fn new(buf: &'a mut B) -> Self {
        Self { buf }
    }

    fn put_u8(&mut self, v: u8) -> Result<(), NbtEncodeError> {
        if self.buf.remaining_mut() < 1 {
            return Err(NbtEncodeError::Overflow);
        }
        self.buf.put_u8(v);
        Ok(())
    }

    fn put_u16(&mut self, v: u16) -> Result<(), NbtEncodeError> {
        if self.buf.remaining_mut() < 2 {
            return Err(NbtEncodeError::Overflow);
        }
        self.buf.put_u16(v);
        Ok(())
    }

    fn put_i16(&mut self, v: i16) -> Result<(), NbtEncodeError> {
        if self.buf.remaining_mut() < 2 {
            return Err(NbtEncodeError::Overflow);
        }
        self.buf.put_i16(v);
        Ok(())
    }

    fn put_i32(&mut self, v: i32) -> Result<(), NbtEncodeError> {
        if self.buf.remaining_mut() < 4 {
            return Err(NbtEncodeError::Overflow);
        }
        self.buf.put_i32(v);
        Ok(())
    }

    fn put_i64(&mut self, v: i64) -> Result<(), NbtEncodeError> {
        if self.buf.remaining_mut() < 8 {
            return Err(NbtEncodeError::Overflow);
        }
        self.buf.put_i64(v);
        Ok(())
    }

    fn put_f32(&mut self, v: f32) -> Result<(), NbtEncodeError> {
        if self.buf.remaining_mut() < 4 {
            return Err(NbtEncodeError::Overflow);
        }
        self.buf.put_f32(v);
        Ok(())
    }

    fn put_f64(&mut self, v: f64) -> Result<(), NbtEncodeError> {
        if self.buf.remaining_mut() < 8 {
            return Err(NbtEncodeError::Overflow);
        }
        self.buf.put_f64(v);
        Ok(())
    }

    fn put_string(&mut self, value: &str) -> Result<(), NbtEncodeError> {
        let bytes = cesu8::to_java_cesu8(value);
        let len = bytes.len();
        if len > u16::MAX as usize {
            return Err(NbtEncodeError::Overflow);
        }
        self.put_u16(len as u16)?;
        if self.buf.remaining_mut() < len {
            return Err(NbtEncodeError::Overflow);
        }
        self.buf.put_slice(&bytes);
        Ok(())
    }

    fn write_payload(&mut self, value: &NbtValue) -> Result<(), NbtEncodeError> {
        match value {
            NbtValue::Byte(v) => self.put_u8(*v as u8)?,
            NbtValue::Short(v) => self.put_i16(*v)?,
            NbtValue::Int(v) => self.put_i32(*v)?,
            NbtValue::Long(v) => self.put_i64(*v)?,
            NbtValue::Float(v) => self.put_f32(*v)?,
            NbtValue::Double(v) => self.put_f64(*v)?,
            NbtValue::ByteArray(arr) => {
                self.put_i32(arr.len() as i32)?;
                for &b in arr {
                    self.put_u8(b as u8)?;
                }
            }
            NbtValue::String(s) => self.put_string(s)?,
            NbtValue::List(list) => {
                self.put_u8(list.element_type as u8)?;
                self.put_i32(list.elements.len() as i32)?;
                for element in &list.elements {
                    self.write_payload(element)?;
                }
            }
            NbtValue::Compound(compound) => {
                for (name, value) in compound.iter() {
                    self.put_u8(value.tag() as u8)?;
                    self.put_string(name)?;
                    self.write_payload(value)?;
                }
                self.put_u8(NbtTag::End as u8)?;
            }
            NbtValue::IntArray(arr) => {
                self.put_i32(arr.len() as i32)?;
                for &v in arr {
                    self.put_i32(v)?;
                }
            }
            NbtValue::LongArray(arr) => {
                self.put_i32(arr.len() as i32)?;
                for &v in arr {
                    self.put_i64(v)?;
                }
            }
        }
        Ok(())
    }

    pub fn write_root(&mut self, nbt: &Nbt) -> Result<(), NbtEncodeError> {
        self.put_u8(NbtTag::Compound as u8)?;
        self.put_string(&nbt.name)?;
        let root_value = NbtValue::Compound(nbt.root.clone());
        self.write_payload(&root_value)
    }
}
