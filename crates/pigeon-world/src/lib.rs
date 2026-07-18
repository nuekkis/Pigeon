//! World layer: chunk storage, Anvil region file IO, biome palettes,
//! lighting. Multi-thread simulation happens in M5.

pub mod anvil;
pub mod chunk;

pub use chunk::ChunkSection;
