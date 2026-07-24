//! Entity type registry data.
//!
//! Entity types come from the `minecraft:entity_type` entry in
//! [`crate::registries`].

use crate::registries;

/// Registry name for entity types in `registries.json`.
pub const REGISTRY_NAME: &str = "minecraft:entity_type";

/// Returns the entity-type registry.
pub fn registry() -> Option<&'static registries::Registry> {
    registries::get(REGISTRY_NAME)
}

/// Returns the wire [`protocol_id`][registries::RegistryEntry::protocol_id]
/// for the given entity resource location (e.g. `minecraft:cow`).
pub fn protocol_id(resource_location: &str) -> Option<i32> {
    registries::protocol_id(REGISTRY_NAME, resource_location)
}

/// Returns the resource location for a given entity `protocol_id`.
pub fn resource_location(protocol_id: i32) -> Option<&'static str> {
    registries::resource_location(REGISTRY_NAME, protocol_id)
}

/// Returns how many entity types the report defines.
pub fn count() -> Option<usize> {
    registry().map(|r| r.entries.len())
}
