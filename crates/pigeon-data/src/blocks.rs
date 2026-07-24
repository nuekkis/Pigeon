//! Block state registry data parsed from the embedded `blocks.json` report.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::raw;

/// Top-level `blocks.json` shape: `resource_location -> Block`.
pub type BlockReport = BTreeMap<String, Block>;

#[derive(Debug, Clone, Deserialize)]
pub struct Block {
    /// Definition used by the data generator; opaque to us today.
    #[serde(default)]
    pub definition: serde_json::Value,
    /// All possible values for each state property.
    #[serde(default)]
    pub properties: BTreeMap<String, Vec<String>>,
    /// Concrete block states with stable ids.
    pub states: Vec<BlockState>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BlockState {
    /// Numeric block-state id sent on the wire.
    pub id: i32,
    /// Whether this is the block's default state.
    #[serde(default)]
    pub default: bool,
    /// The specific value chosen for each property in this state.
    #[serde(default)]
    pub properties: BTreeMap<String, String>,
}

static BLOCKS: OnceLock<BlockReport> = OnceLock::new();

/// Returns the parsed `blocks.json` report.
///
/// The map is parsed once on first access via [`OnceLock::get_or_init`].
pub fn blocks() -> &'static BlockReport {
    BLOCKS.get_or_init(|| {
        serde_json::from_str(raw::BLOCKS_JSON).expect("embedded blocks.json must be valid")
    })
}

/// Returns the block with the given resource location (e.g. `minecraft:stone`).
pub fn get(resource_location: &str) -> Option<&'static Block> {
    blocks().get(resource_location)
}

/// Returns the total number of distinct block ids defined in the report.
pub fn count() -> usize {
    blocks().len()
}
