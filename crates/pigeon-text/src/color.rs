use serde::{Deserialize, Serialize};
use std::fmt;

/// A Minecraft text color. Encoded as either a named lowercase string
/// (e.g. `"dark_purple"`) or a hex string `"#FFAA00"` in JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextColor {
    Named(NamedTextColor),
    Custom(u32),
}

/// Vanilla named colors — same id values as legacy chat codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NamedTextColor {
    Black,
    DarkBlue,
    DarkGreen,
    DarkAqua,
    DarkRed,
    DarkPurple,
    Gold,
    Gray,
    DarkGray,
    Blue,
    Green,
    Aqua,
    Red,
    LightPurple,
    Yellow,
    White,
}

impl TextColor {
    pub const BLACK: Self = Self::Named(NamedTextColor::Black);
    pub const DARK_BLUE: Self = Self::Named(NamedTextColor::DarkBlue);
    pub const DARK_GREEN: Self = Self::Named(NamedTextColor::DarkGreen);
    pub const DARK_AQUA: Self = Self::Named(NamedTextColor::DarkAqua);
    pub const DARK_RED: Self = Self::Named(NamedTextColor::DarkRed);
    pub const DARK_PURPLE: Self = Self::Named(NamedTextColor::DarkPurple);
    pub const GOLD: Self = Self::Named(NamedTextColor::Gold);
    pub const GRAY: Self = Self::Named(NamedTextColor::Gray);
    pub const DARK_GRAY: Self = Self::Named(NamedTextColor::DarkGray);
    pub const BLUE: Self = Self::Named(NamedTextColor::Blue);
    pub const GREEN: Self = Self::Named(NamedTextColor::Green);
    pub const AQUA: Self = Self::Named(NamedTextColor::Aqua);
    pub const RED: Self = Self::Named(NamedTextColor::Red);
    pub const LIGHT_PURPLE: Self = Self::Named(NamedTextColor::LightPurple);
    pub const YELLOW: Self = Self::Named(NamedTextColor::Yellow);
    pub const WHITE: Self = Self::Named(NamedTextColor::White);

    pub const fn rgb(self) -> u32 {
        match self {
            Self::Named(c) => c.rgb(),
            Self::Custom(c) => c,
        }
    }

    pub fn to_hex(self) -> String {
        format!("#{:06X}", self.rgb())
    }
}

impl NamedTextColor {
    pub const fn rgb(self) -> u32 {
        match self {
            Self::Black => 0x000000,
            Self::DarkBlue => 0x0000AA,
            Self::DarkGreen => 0x00AA00,
            Self::DarkAqua => 0x00AAAA,
            Self::DarkRed => 0xAA0000,
            Self::DarkPurple => 0xAA00AA,
            Self::Gold => 0xFFAA00,
            Self::Gray => 0xAAAAAA,
            Self::DarkGray => 0x555555,
            Self::Blue => 0x5555FF,
            Self::Green => 0x55FF55,
            Self::Aqua => 0x55FFFF,
            Self::Red => 0xFF5555,
            Self::LightPurple => 0xFF55FF,
            Self::Yellow => 0xFFFF55,
            Self::White => 0xFFFFFF,
        }
    }
}

impl fmt::Display for TextColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Named(c) => write!(f, "{}", serde_json::to_string(c).unwrap_or_default()),
            Self::Custom(_) => write!(f, "\"{}\"", self.to_hex()),
        }
    }
}

impl Serialize for TextColor {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Named(c) => c.serialize(s),
            Self::Custom(_) => s.serialize_str(&self.to_hex()),
        }
    }
}

impl<'de> Deserialize<'de> for TextColor {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        if let Some(stripped) = s.strip_prefix('#') {
            let raw = u32::from_str_radix(stripped, 16).map_err(serde::de::Error::custom)?;
            Ok(Self::Custom(raw))
        } else {
            let named = serde_json::from_value::<NamedTextColor>(serde_json::Value::String(s))
                .map_err(serde::de::Error::custom)?;
            Ok(Self::Named(named))
        }
    }
}
