use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};
use std::{fmt, str::FromStr};
use unicode_normalization::UnicodeNormalization;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, thiserror::Error)]
pub enum IdValidationError {
    #[error("invalid UUID")]
    InvalidUuid,
    #[error("invalid SHA-256 identifier")]
    InvalidSha256,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SkillId(Uuid);

impl SkillId {
    /// Generates a fresh identity explicitly; the type intentionally has no `Default`.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn parse(value: &str) -> Result<Self, IdValidationError> {
        Uuid::parse_str(value)
            .map(Self)
            .map_err(|_| IdValidationError::InvalidUuid)
    }

    pub const fn as_uuid(&self) -> Uuid {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OperationId(Uuid);

impl OperationId {
    /// Generates a fresh operation identity explicitly; the type intentionally has no `Default`.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn parse(value: &str) -> Result<Self, IdValidationError> {
        Uuid::parse_str(value)
            .map(Self)
            .map_err(|_| IdValidationError::InvalidUuid)
    }

    pub const fn as_uuid(&self) -> Uuid {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RevisionId(String);

impl RevisionId {
    pub fn from_digest(digest: [u8; 32]) -> Self {
        Self(encode_digest(digest))
    }

    pub fn parse_hex(value: &str) -> Result<Self, IdValidationError> {
        validate_digest(value)
            .then(|| Self(value.to_owned()))
            .ok_or(IdValidationError::InvalidSha256)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Serialize for RevisionId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for RevisionId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse_hex(&value).map_err(D::Error::custom)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentHash(String);

impl ContentHash {
    pub fn from_digest(digest: [u8; 32]) -> Self {
        Self(encode_digest(digest))
    }

    pub fn parse_hex(value: &str) -> Result<Self, IdValidationError> {
        validate_digest(value)
            .then(|| Self(value.to_owned()))
            .ok_or(IdValidationError::InvalidSha256)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Serialize for ContentHash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ContentHash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse_hex(&value).map_err(D::Error::custom)
    }
}

fn encode_digest(digest: [u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn validate_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PortablePath(String);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, thiserror::Error)]
pub enum PathValidationError {
    #[error("path is empty or too long")]
    Length,
    #[error("path must use Unicode NFC")]
    NonCanonicalUnicode,
    #[error("path must be relative")]
    Absolute,
    #[error("path contains a non-portable separator or character")]
    InvalidCharacter,
    #[error("path contains an invalid component")]
    InvalidComponent,
    #[error("path contains a reserved Windows device name")]
    ReservedDeviceName,
    #[error("path component ends in a dot or space")]
    TrailingDotOrSpace,
}

impl PortablePath {
    pub fn new(value: String) -> Result<Self, PathValidationError> {
        if value.is_empty() || value.len() > 240 {
            return Err(PathValidationError::Length);
        }
        if !value.nfc().eq(value.chars()) {
            return Err(PathValidationError::NonCanonicalUnicode);
        }
        if value.starts_with('/') {
            return Err(PathValidationError::Absolute);
        }
        if value.contains('\\')
            || value.contains(':')
            || value.chars().any(char::is_control)
            || value
                .chars()
                .any(|ch| matches!(ch, '<' | '>' | '"' | '|' | '?' | '*'))
        {
            return Err(PathValidationError::InvalidCharacter);
        }

        for component in value.split('/') {
            if component.is_empty() || matches!(component, "." | "..") {
                return Err(PathValidationError::InvalidComponent);
            }
            if component.ends_with('.') || component.ends_with(' ') {
                return Err(PathValidationError::TrailingDotOrSpace);
            }
            if is_windows_device_name(component) {
                return Err(PathValidationError::ReservedDeviceName);
            }
        }

        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn is_windows_device_name(component: &str) -> bool {
    let stem = component
        .split('.')
        .next()
        .unwrap_or_default()
        .trim_end_matches([' ', '.']);
    let uppercase = stem.to_ascii_uppercase();
    matches!(uppercase.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (uppercase.len() == 4
            && (uppercase.starts_with("COM") || uppercase.starts_with("LPT"))
            && matches!(uppercase.as_bytes()[3], b'1'..=b'9'))
}

impl Serialize for PortablePath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for PortablePath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(D::Error::custom)
    }
}

impl fmt::Display for PortablePath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl FromStr for SkillId {
    type Err = IdValidationError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}
