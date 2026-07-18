mod component;
mod color;

pub use color::TextColor;
pub use component::{Component, ComponentStyle, KeybindComponent, ScoreComponent, SelectorComponent, TextComponent, TranslatableComponent, ClickType, HoverType};

use serde::Serialize;

/// Serialize a component to Minecraft's JSON text format.
pub fn to_json<T: Serialize>(component: &T) -> serde_json::Result<String> {
    serde_json::to_string(component)
}

pub fn to_json_value<T: Serialize>(component: &T) -> serde_json::Result<serde_json::Value> {
    serde_json::to_value(component)
}
