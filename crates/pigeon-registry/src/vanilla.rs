//! Vanilla registries sourced from `pigeon-data::registries`.
//!
//! Currently a thin accessor so transport code (the `RegistryData` packet)
//! can build a [`Registry`] listing the canonical wire ids without
//! re-parsing the JSON at runtime. The full per-entry payloads (NBT
//! dimension type / chat type / cat variant …) are wired in the world-
//! data milestone (M5+).

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::OnceLock;

use pigeon_data::registries as data_registries;
use pigeon_util::Identifier;

use crate::Registry;

/// Lightweight view over every vanilla registry in 1.21.11.
///
/// Entries are keyed by registry identifier (e.g. `minecraft:chat_type`)
/// and carry a typed [`Registry<i32>`] where the per-entry payload is
/// the entry's `protocol_id` from `registries.json`.
pub struct VanillaRegistries {
    pub registries: HashMap<Identifier, Registry<i32>>,
}

static VANILLA: OnceLock<VanillaRegistries> = OnceLock::new();

/// Returns the singleton over the embedded vanilla registries.
pub fn vanilla() -> &'static VanillaRegistries {
    VANILLA.get_or_init(|| {
        let report = data_registries::registries();
        let mut out = HashMap::new();
        for (name, registry) in report {
            // Skip worldgen/* registries — they have no protocol_id mappings
            // (entries are pure data, not separately numbered). Only wire
            // registries with non-empty entries land here.
            if registry.entries.is_empty() {
                continue;
            }
            if let Ok(reg_id) = Identifier::from_str(name) {
                let mut wire = Registry::new();
                for (entry_name, entry) in &registry.entries {
                    if let Ok(entry_id) = Identifier::from_str(entry_name) {
                        wire.push(entry_id, entry.protocol_id);
                    }
                }
                out.insert(reg_id, wire);
            }
        }
        VanillaRegistries { registries: out }
    })
}

impl VanillaRegistries {
    /// Returns the named registry, if present.
    pub fn get(&self, id: &Identifier) -> Option<&Registry<i32>> {
        self.registries.get(id)
    }

    /// Returns the number of registries exposed.
    pub fn len(&self) -> usize {
        self.registries.len()
    }

    /// Returns `true` if there are no registries wired (should not happen
    /// for the embedded 1.21.11 data, but cheap to provide).
    pub fn is_empty(&self) -> bool {
        self.registries.is_empty()
    }

    /// Iterate over all registries.
    pub fn iter(&self) -> impl Iterator<Item = (&Identifier, &Registry<i32>)> {
        self.registries.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vanilla_registries_load_all_non_empty() {
        let v = vanilla();
        assert!(v.len() > 50, "expected >50 vanilla wired registries");
    }

    #[test]
    fn entity_types_have_protocol_ids() {
        let v = vanilla();
        let entities = v
            .get(&Identifier::minecraft("entity_type").unwrap())
            .expect("minecraft:entity_type must exist in vanilla registries");
        assert!(entities.len() > 100, "expected >100 entity types");
        // The first entry should have a numeric protocol_id payload.
        for (_, id) in entities.iter() {
            assert!(*id >= 0, "protocol_id must be non-negative");
        }
    }

    #[test]
    fn round_trip_protocol_id_lookup() {
        let v = vanilla();
        let blocks = v
            .get(&Identifier::minecraft("block").unwrap())
            .expect("minecraft:block registry must exist");
        let stone = Identifier::minecraft("stone").unwrap();
        let idx = blocks
            .index_of(&stone)
            .expect("minecraft:stone must be in the registry");
        assert_eq!(blocks.get_by_index(idx).unwrap().0, &stone);
    }
}
