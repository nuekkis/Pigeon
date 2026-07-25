//! Biome data parsed from the embedded `biome_parameters/*.json` reports.
//!
//! Vanilla biome parameter lists map a biome resource location to a set of
//! multi-noise parameters (`temperature`, `humidity`, `continentalness`,
//! `erosion`, `weirdness`, `depth`, `offset`) used by the world generator.

use std::sync::OnceLock;

use serde::Deserialize;

use crate::raw;

#[derive(Debug, Clone, Deserialize)]
pub struct BiomeParameterList {
    pub biomes: Vec<BiomeEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BiomeEntry {
    /// Biome resource location, e.g. `minecraft:plains`.
    pub biome: String,
    /// Noise parameters.
    pub parameters: NoiseParameters,
}

/// A climate target; each axis is either a point (`f64`) or a `[lo, hi]` range.
///
/// `offset` is always a single float, the others are commonly a range.
#[derive(Debug, Clone, Deserialize)]
pub struct NoiseParameters {
    #[serde(default)]
    pub temperature: serde_json::Value,
    #[serde(default)]
    pub humidity: serde_json::Value,
    #[serde(default)]
    pub continentalness: serde_json::Value,
    #[serde(default)]
    pub erosion: serde_json::Value,
    #[serde(default)]
    pub weirdness: serde_json::Value,
    #[serde(default)]
    pub depth: serde_json::Value,
    #[serde(default)]
    pub offset: serde_json::Value,
}

static OVERWORLD: OnceLock<BiomeParameterList> = OnceLock::new();
static NETHER: OnceLock<BiomeParameterList> = OnceLock::new();

/// Returns the parsed overworld biome parameter list.
pub fn overworld() -> &'static BiomeParameterList {
    OVERWORLD.get_or_init(|| {
        serde_json::from_str(raw::BIOME_PARAMETERS_OVERWORLD_JSON)
            .expect("embedded overworld biome parameters must be valid")
    })
}

/// Returns the parsed nether biome parameter list.
pub fn nether() -> &'static BiomeParameterList {
    NETHER.get_or_init(|| {
        serde_json::from_str(raw::BIOME_PARAMETERS_NETHER_JSON)
            .expect("embedded nether biome parameters must be valid")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_without_panicking() {
        assert!(
            !overworld().biomes.is_empty(),
            "overworld biome list must be non-empty"
        );
        assert!(
            !nether().biomes.is_empty(),
            "nether biome list must be non-empty"
        );
    }

    #[test]
    fn overworld_contains_plains() {
        let has_plains = overworld()
            .biomes
            .iter()
            .any(|b| b.biome == "minecraft:plains");
        assert!(
            has_plains,
            "minecraft:plains must be in the overworld biome list"
        );
    }
}
