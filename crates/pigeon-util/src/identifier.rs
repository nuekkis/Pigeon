use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// A Minecraft resource location, e.g. `minecraft:stone`.
///
/// Format: `namespace:path`
/// - `namespace`: lowercase `[a-z0-9_.-]+`, defaults to `minecraft`
/// - `path`: lowercase `[a-z0-9_./-]+`
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Identifier {
    namespace: String,
    path: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum IdentifierParseError {
    #[error("identifier is empty")]
    Empty,
    #[error("invalid namespace character '{0}' in identifier")]
    InvalidNamespaceChar(char),
    #[error("invalid path character '{0}' in identifier")]
    InvalidPathChar(char),
}

impl Identifier {
    pub fn new(
        namespace: impl Into<String>,
        path: impl Into<String>,
    ) -> Result<Self, IdentifierParseError> {
        let namespace = namespace.into();
        let path = path.into();
        validate_namespace(&namespace)?;
        validate_path(&path)?;
        Ok(Self { namespace, path })
    }

    /// Construct an `Identifier` from the `minecraft` namespace.
    pub fn minecraft(path: impl Into<String>) -> Result<Self, IdentifierParseError> {
        Self::new("minecraft", path)
    }

    /// Bypass validation — only use when the input is statically known to be valid.
    pub fn new_unchecked(namespace: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            path: path.into(),
        }
    }

    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn into_parts(self) -> (String, String) {
        (self.namespace, self.path)
    }

    pub fn as_str(&self) -> String {
        format!("{}:{}", self.namespace, self.path)
    }
}

fn validate_namespace(s: &str) -> Result<(), IdentifierParseError> {
    if s.is_empty() {
        return Err(IdentifierParseError::Empty);
    }
    for c in s.chars() {
        if !is_valid_namespace_char(c) {
            return Err(IdentifierParseError::InvalidNamespaceChar(c));
        }
    }
    Ok(())
}

fn validate_path(s: &str) -> Result<(), IdentifierParseError> {
    if s.is_empty() {
        return Err(IdentifierParseError::Empty);
    }
    for c in s.chars() {
        if !is_valid_path_char(c) {
            return Err(IdentifierParseError::InvalidPathChar(c));
        }
    }
    Ok(())
}

fn is_valid_namespace_char(c: char) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-' || c == '.'
}

fn is_valid_path_char(c: char) -> bool {
    is_valid_namespace_char(c) || c == '/'
}

impl fmt::Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.namespace, self.path)
    }
}

impl std::str::FromStr for Identifier {
    type Err = IdentifierParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(IdentifierParseError::Empty);
        }
        let (namespace, path) = match s.find(':') {
            Some(idx) => {
                let (ns, rest) = s.split_at(idx);
                (ns.to_string(), rest[1..].to_string())
            }
            None => ("minecraft".to_string(), s.to_string()),
        };
        Self::new(namespace, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn parse_default_namespace() {
        let id = Identifier::from_str("stone").unwrap();
        assert_eq!(id.namespace(), "minecraft");
        assert_eq!(id.path(), "stone");
    }

    #[test]
    fn parse_explicit_namespace() {
        let id = Identifier::from_str("minecraft:dirt").unwrap();
        assert_eq!(id.namespace(), "minecraft");
        assert_eq!(id.path(), "dirt");
    }

    #[test]
    fn parse_custom_namespace() {
        let id = Identifier::from_str("pigeon:custom_block").unwrap();
        assert_eq!(id.namespace(), "pigeon");
        assert_eq!(id.path(), "custom_block");
    }

    #[test]
    fn reject_uppercase() {
        assert!(Identifier::from_str("Block").is_err());
    }

    #[test]
    fn display_roundtrip() {
        let id = Identifier::minecraft("stone").unwrap();
        assert_eq!(id.to_string(), "minecraft:stone");
    }
}
