//! Runtime registries for block states, biomes, dimensions, etc.
//!
//! Populated from `pigeon-data`'s embedded vanilla reports and
//! together with the wire-format codec used by the Configuration phase
//! `RegistryData` packet.

pub mod codec;
pub mod vanilla;

pub use codec::{RegistryCodec, RegistryCodecError};
pub use vanilla::VanillaRegistries;

use indexmap::IndexMap;
use pigeon_util::Identifier;

/// An ordered registry keyed by entry identifier, with optional
/// per-entry payload.
///
/// Insertion order is preserved so that wire indexes match the
/// canonical protocol ids assigned by the data generator.
#[derive(Debug, Clone)]
pub struct Registry<T> {
    pub entries: Vec<(Identifier, T)>,
    pub by_id: IndexMap<Identifier, usize>,
}

impl<T> Registry<T> {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            by_id: IndexMap::new(),
        }
    }

    /// Append an entry. Returns the wire index assigned to it.
    pub fn push(&mut self, id: Identifier, value: T) -> usize {
        let idx = self.entries.len();
        self.by_id.insert(id.clone(), idx);
        self.entries.push((id, value));
        idx
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Identifier, &T)> {
        self.entries.iter().map(|(id, value)| (id, value))
    }

    /// Look up an entry by its identifier.
    pub fn get(&self, id: &Identifier) -> Option<&T> {
        self.by_id.get(id).map(|&idx| &self.entries[idx].1)
    }

    /// Look up the wire index for the given identifier.
    pub fn index_of(&self, id: &Identifier) -> Option<usize> {
        self.by_id.get(id).copied()
    }

    /// Look up an entry by its wire id (insertion index).
    pub fn get_by_index(&self, wire: usize) -> Option<(&Identifier, &T)> {
        self.entries.get(wire).map(|(id, value)| (id, value))
    }
}

impl<T> Default for Registry<T> {
    fn default() -> Self {
        Self::new()
    }
}
