//! Registry dump parsed from the embedded `registries.json` report.
//!
//! The Mojang data generator emits one record per vanilla registry. Each
//! registry entry carries a `protocol_id` suitable for wire-encoding
//! (e.g. entity type ids, fluid ids, block ids).

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::raw;

/// Top-level `registries.json` shape: `registry_name -> Registry`.
pub type RegistryReport = BTreeMap<String, Registry>;

#[derive(Debug, Clone, Deserialize)]
pub struct Registry {
    /// Default entry for the registry, e.g. `minecraft:stone` for `minecraft:block`.
    #[serde(default)]
    pub default: Option<String>,
    /// All registry entries keyed by resource location.
    pub entries: BTreeMap<String, RegistryEntry>,
    /// The registry's own protocol id (used to identify the registry itself
    /// in some protocol flows).
    #[serde(default)]
    pub protocol_id: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RegistryEntry {
    /// Wire id of the entry.
    #[serde(default)]
    pub protocol_id: i32,
}

static REGISTRIES: OnceLock<RegistryReport> = OnceLock::new();

/// Returns the parsed `registries.json` report.
pub fn registries() -> &'static RegistryReport {
    REGISTRIES.get_or_init(|| {
        serde_json::from_str(raw::REGISTRIES_JSON).expect("embedded registries.json must be valid")
    })
}

/// Returns the named registry if present (e.g. `minecraft:entity_type`).
pub fn get(name: &str) -> Option<&'static Registry> {
    registries().get(name)
}

/// Looks up a `protocol_id` for an entry in a registry.
///
/// Returns `None` if the registry or the entry is not present.
pub fn protocol_id(registry: &str, entry: &str) -> Option<i32> {
    get(registry)
        .and_then(|r| r.entries.get(entry))
        .map(|e| e.protocol_id)
}

/// Returns the resource location for a `protocol_id` within `registry`,
/// if any. When several entries map to the same id the first one in the
/// natural iteration order of the `BTreeMap` is returned.
pub fn resource_location(registry: &str, id: i32) -> Option<&'static str> {
    get(registry).and_then(|r| {
        r.entries
            .iter()
            .find_map(|(name, entry)| (entry.protocol_id == id).then(|| name.as_str()))
    })
}

/// Returns the number of registries in the report.
pub fn count() -> usize {
    registries().len()
}
