//! Vanilla data tables extracted from the Minecraft 1.21.11 server.jar.
//!
//! Generated from Mojang's `client.txt` / server reports. The full set
//! of blocks, items, biomes, entities, etc. lands in M4.1.

pub const MINECRAFT_VERSION: &str = "1.21.11";
pub const DATA_VERSION: i32 = 4440;

pub mod biomes;
pub mod blocks;
pub mod entities;
pub mod items;
