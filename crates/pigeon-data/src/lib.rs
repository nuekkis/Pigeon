//! Vanilla data tables extracted from the Minecraft 1.21.11 server.jar.
//!
//! The raw JSON reports produced by `net.minecraft.data.Main --reports --server --all`
//! are embedded into the crate at compile time via `include_str!` and parsed once
//! on first access through `OnceLock` cells.
//!
//! See `tools/` at the workspace root for the regeneration script.

pub const MINECRAFT_VERSION: &str = "1.21.11";
/// Minecraft `SharedConstants.getWorldVersion()` data-version for 1.21.11.
pub const DATA_VERSION: i32 = 4440;

pub mod biomes;
pub mod blocks;
pub mod entities;
pub mod items;
pub mod packets;
pub mod raw;
pub mod registries;
