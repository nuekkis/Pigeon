//! Embedded vanilla reports JSON.
//!
//! Each report produced by the Minecraft data generator is embedded into
//! the binary via `include_str!`. The raw strings are exposed alongside
//! lazy parses into typed [`serde`] forms defined in the sibling modules.
//!
//! Sizes at the time of writing (1.21.11):
//!
//! | report      | size  |
//! |-------------|-------|
//! | blocks.json | ~6 MB |
//! | items.json  | ~1 MB |
//! | others      | <0.5 MB each |

/// Raw `blocks.json` — block definitions, properties and state ids.
pub const BLOCKS_JSON: &str = include_str!("../reports/blocks.json");

/// Raw `items.json` — item definitions and their default data components.
pub const ITEMS_JSON: &str = include_str!("../reports/items.json");

/// Raw `registries.json` — full registry dump (95 registries, protocol ids).
pub const REGISTRIES_JSON: &str = include_str!("../reports/registries.json");

/// Raw `commands.json` — brigadier command tree (parser schema).
pub const COMMANDS_JSON: &str = include_str!("../reports/commands.json");

/// Raw `packets.json` — protocol ids by phase and direction.
pub const PACKETS_JSON: &str = include_str!("../reports/packets.json");

/// Raw `datapack.json` — datapack element + tag schema for each registry.
pub const DATAPACK_JSON: &str = include_str!("../reports/datapack.json");

/// Raw `json-rpc-api-schema.json` — JSON RPC API used by the dedicated server.
pub const JSON_RPC_API_SCHEMA_JSON: &str = include_str!("../reports/json-rpc-api-schema.json");

/// Raw `biome_parameters/minecraft/overworld.json`.
pub const BIOME_PARAMETERS_OVERWORLD_JSON: &str =
    include_str!("../reports/biome_parameters/minecraft/overworld.json");

/// Raw `biome_parameters/minecraft/nether.json`.
pub const BIOME_PARAMETERS_NETHER_JSON: &str =
    include_str!("../reports/biome_parameters/minecraft/nether.json");
