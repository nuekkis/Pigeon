//! Anvil region (.mca) reader/writer placeholder.
//!
//! Each region file covers 32×32 chunks. Contains a 4096-byte header
//! of (offset, sector_count) u32 packed entries, plus a timestamp table.

use std::path::Path;

pub struct RegionFile;

impl RegionFile {
    pub fn open(_path: &Path) -> std::io::Result<Self> {
        Ok(Self)
    }
}
