use crate::TextColor;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A JSON text component following Minecraft's component tree spec.
///
/// We keep the model serializable as JSON for direct use in chat and
/// server list responses, with one enum variant per component kind.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Component {
    Text(TextComponent),
    Translatable(TranslatableComponent),
    Score(ScoreComponent),
    Selector(SelectorComponent),
    Keybind(KeybindComponent),
}

impl Component {
    pub fn text<S: Into<String>>(value: S) -> Self {
        Self::Text(TextComponent::new(value))
    }

    pub fn empty() -> Self {
        Self::text("")
    }

    pub fn translate<S: Into<String>>(key: S) -> Self {
        Self::Translatable(TranslatableComponent::new(key))
    }

    pub fn style_mut(&mut self) -> &mut ComponentStyle {
        match self {
            Self::Text(c) => &mut c.style,
            Self::Translatable(c) => &mut c.style,
            Self::Score(c) => &mut c.style,
            Self::Selector(c) => &mut c.style,
            Self::Keybind(c) => &mut c.style,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComponentStyle {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<TextColor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub underlined: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strikethrough: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub obfuscated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insertion: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub click_event: Option<ClickEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hover_event: Option<HoverEvent>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub extra: Vec<Component>,
}

impl ComponentStyle {
    pub fn color(mut self, color: TextColor) -> Self {
        self.color = Some(color);
        self
    }

    pub fn bold(mut self, value: bool) -> Self {
        self.bold = Some(value);
        self
    }

    pub fn italic(mut self, value: bool) -> Self {
        self.italic = Some(value);
        self
    }

    pub fn extra(mut self, components: Vec<Component>) -> Self {
        self.extra = components;
        self
    }

    pub fn push_extra(&mut self, component: Component) {
        self.extra.push(component);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextComponent {
    pub text: String,
    #[serde(flatten)]
    pub style: ComponentStyle,
}

impl TextComponent {
    pub fn new<S: Into<String>>(text: S) -> Self {
        Self {
            text: text.into(),
            style: ComponentStyle::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslatableComponent {
    pub translate: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[serde(rename = "with")]
    pub with: Vec<Component>,
    #[serde(flatten)]
    pub style: ComponentStyle,
}

impl TranslatableComponent {
    pub fn new<S: Into<String>>(key: S) -> Self {
        Self {
            translate: key.into(),
            with: Vec::new(),
            style: ComponentStyle::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreComponent {
    pub score: ScorePayload,
    #[serde(flatten)]
    pub style: ComponentStyle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScorePayload {
    pub name: String,
    pub objective: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectorComponent {
    pub selector: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub separator: Option<Value>,
    #[serde(flatten)]
    pub style: ComponentStyle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindComponent {
    pub keybind: String,
    #[serde(flatten)]
    pub style: ComponentStyle,
}

impl KeybindComponent {
    pub fn new<S: Into<String>>(keybind: S) -> Self {
        Self {
            keybind: keybind.into(),
            style: ComponentStyle::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClickEvent {
    pub action: ClickType,
    pub value: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClickType {
    OpenUrl,
    RunCommand,
    SuggestCommand,
    ChangePage,
    CopyToClipboard,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoverEvent {
    pub action: HoverType,
    pub contents: Value,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HoverType {
    ShowText,
    ShowItem,
    ShowEntity,
}
