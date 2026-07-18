//! Chunk section storage: paletted block + biome data.
//!
//! Full implementation arrives in M4.

use serde::{Deserialize, Serialize};

/// A 16×16×16 chunk section (16 blocks tall in the overworld).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChunkSection {
    pub y: i32,
    pub block_palette: Vec<u32>,
    pub block_data: Vec<u64>,
    pub biome_palette: Vec<u32>,
    pub biome_data: Vec<u64>,
    pub block_light: Option<Vec<u8>>,
    pub sky_light: Option<Vec<u8>>,
}
