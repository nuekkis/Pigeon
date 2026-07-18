//! Runtime registries for block states, biomes, dimensions, etc.
//!
//! Populated by `pigeon-data` extraction in M4.

use pigeon_util::Identifier;

pub struct Registry<T> {
    pub entries: Vec<(Identifier, T)>,
}

impl<T> Registry<T> {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl<T> Default for Registry<T> {
    fn default() -> Self {
        Self::new()
    }
}
