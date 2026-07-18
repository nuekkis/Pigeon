//! World layer: chunk storage, Anvil region file IO, biome palettes,
//! lighting. Multi-thread simulation happens in M5.

pub mod chunk;
pub mod anvil;

pub use chunk::ChunkSection;
