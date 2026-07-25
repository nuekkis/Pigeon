//! Item registry data parsed from the embedded `items.json` report.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::raw;

/// Top-level `items.json` shape: `resource_location -> Item`.
pub type ItemReport = BTreeMap<String, Item>;

#[derive(Debug, Clone, Deserialize)]
pub struct Item {
    /// Default data components for the item. The shapes vary by component
    /// type, so we keep them as raw values in the initial pass.
    #[serde(default)]
    pub components: BTreeMap<String, serde_json::Value>,
}

static ITEMS: OnceLock<ItemReport> = OnceLock::new();

/// Returns the parsed `items.json` report.
pub fn items() -> &'static ItemReport {
    ITEMS.get_or_init(|| {
        serde_json::from_str(raw::ITEMS_JSON).expect("embedded items.json must be valid")
    })
}

/// Returns the item with the given resource location (e.g. `minecraft:stone`).
pub fn get(resource_location: &str) -> Option<&'static Item> {
    items().get(resource_location)
}

/// Returns the total number of distinct item ids defined in the report.
pub fn count() -> usize {
    items().len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_without_panicking() {
        assert!(count() > 1000, "expected 1.21.11 to define >1000 items");
    }

    #[test]
    fn stone_item_has_components() {
        let stone = get("minecraft:stone").expect("minecraft:stone item must exist");
        assert!(stone.components.contains_key("minecraft:max_stack_size"));
        assert_eq!(
            stone.components["minecraft:max_stack_size"],
            serde_json::Value::Number(64.into()),
            "stone stack size must be 64",
        );
    }
}
