//! Encryption layer for the Java protocol.
//!
//! Java Edition uses AES/CFB8 with a 16-byte shared secret derived
//! from the RSA key exchange during login. The same secret serves as the
//! AES key and the initialization vector (a quirk of vanilla Java).

use aes::Aes128;
use cfb8::cipher::{KeyIvInit, StreamCipher};
use thiserror::Error;

type Encryptor = cfb8::Encryptor<Aes128>;
type Decryptor = cfb8::Decryptor<Aes128>;

#[derive(Debug, Error)]
pub enum EncryptionError {
    #[error("invalid shared secret length (expected 16 bytes, got {0})")]
    InvalidKeyLength(usize),
}

/// 16-byte shared secret produced during login encryption.
#[derive(Debug, Clone)]
pub struct SharedSecret(pub [u8; 16]);

impl SharedSecret {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, EncryptionError> {
        if bytes.len() != 16 {
            return Err(EncryptionError::InvalidKeyLength(bytes.len()));
        }
        let mut key = [0u8; 16];
        key.copy_from_slice(bytes);
        Ok(Self(key))
    }

    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    pub fn iv(&self) -> [u8; 16] {
        self.0
    }
}

/// Stateful encryptor (write side).
pub struct CipherWrite {
    cipher: Encryptor,
}

impl CipherWrite {
    pub fn new(secret: &SharedSecret) -> Self {
        Self {
            cipher: Encryptor::new(secret.as_bytes().into(), (&secret.iv()).into()),
        }
    }

    pub fn encrypt(&mut self, buf: &mut [u8]) {
        self.cipher.encrypt(buf);
    }
}

/// Stateful decryptor (read side).
pub struct CipherRead {
    cipher: Decryptor,
}

impl CipherRead {
    pub fn new(secret: &SharedSecret) -> Self {
        Self {
            cipher: Decryptor::new(secret.as_bytes().into(), (&secret.iv()).into()),
        }
    }

    pub fn decrypt(&mut self, buf: &mut [u8]) {
        self.cipher.decrypt(buf);
    }
}
