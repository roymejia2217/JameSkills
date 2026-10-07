use super::{
    BundleEntry, ContentHash, EntryKind, PathValidationError, PortablePath, ValidatedInventory,
    validate_bundle_inventory,
};
use crate::{AppError, AppResult, Diagnostic};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub type AssetFiles = BTreeMap<PortablePath, Vec<u8>>;

const EDITABLE_ROOTS: [&str; 3] = ["assets", "references", "templates"];
const MAX_TEXT_PREVIEW_BYTES: usize = 64 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetPreviewKind {
    Text,
    Unsupported,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetPreview {
    path: PortablePath,
    kind: AssetPreviewKind,
    byte_len: usize,
    content_hash: ContentHash,
    text: Option<String>,
    truncated: bool,
}

impl AssetPreview {
    pub fn path(&self) -> &PortablePath {
        &self.path
    }
    pub fn kind(&self) -> AssetPreviewKind {
        self.kind
    }
    pub fn byte_len(&self) -> usize {
        self.byte_len
    }
    pub fn content_hash(&self) -> &ContentHash {
        &self.content_hash
    }
    pub fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }
    pub fn truncated(&self) -> bool {
        self.truncated
    }
}

pub fn add_asset(files: &AssetFiles, path: &str, bytes: Vec<u8>) -> AppResult<AssetFiles> {
    let path = editable_path(path)?;
    if files.contains_key(&path) {
        return Err(asset_error("library.asset.exists"));
    }
    let mut updated = files.clone();
    updated.insert(path, bytes);
    validate_inventory(&updated)?;
    Ok(updated)
}

pub fn replace_asset(
    files: &AssetFiles,
    path: &str,
    expected_hash: &ContentHash,
    bytes: Vec<u8>,
) -> AppResult<AssetFiles> {
    let path = editable_path(path)?;
    let current = files
        .get(&path)
        .ok_or_else(|| asset_error("library.asset.not_found"))?;
    ensure_hash(current, expected_hash)?;
    let mut updated = files.clone();
    updated.insert(path, bytes);
    validate_inventory(&updated)?;
    Ok(updated)
}

pub fn remove_asset(
    files: &AssetFiles,
    path: &str,
    expected_hash: &ContentHash,
) -> AppResult<AssetFiles> {
    let path = editable_path(path)?;
    let current = files
        .get(&path)
        .ok_or_else(|| asset_error("library.asset.not_found"))?;
    ensure_hash(current, expected_hash)?;
    if is_referenced(files, &path) {
        return Err(asset_error("library.asset.reference.update_required"));
    }
    let mut updated = files.clone();
    updated.remove(&path);
    validate_inventory(&updated)?;
    Ok(updated)
}

pub fn rename_bundle_path(
    files: &AssetFiles,
    from: &str,
    to: &str,
    expected_hash: &ContentHash,
) -> AppResult<AssetFiles> {
    let from = editable_path(from)?;
    let to = editable_path(to)?;
    if from == to {
        return Ok(files.clone());
    }
    let current = files
        .get(&from)
        .ok_or_else(|| asset_error("library.asset.not_found"))?;
    ensure_hash(current, expected_hash)?;
    if files.contains_key(&to) {
        return Err(asset_error("library.asset.destination.exists"));
    }
    if is_referenced(files, &from) {
        return Err(asset_error("library.asset.reference.update_required"));
    }
    let mut updated = files.clone();
    let bytes = updated
        .remove(&from)
        .ok_or_else(|| asset_error("library.asset.not_found"))?;
    updated.insert(to, bytes);
    validate_inventory(&updated)?;
    Ok(updated)
}

pub fn preview_asset(path: &PortablePath, bytes: &[u8]) -> AssetPreview {
    let content_hash = ContentHash::from_digest(Sha256::digest(bytes).into());
    let supported_text = matches!(
        path.as_str()
            .rsplit_once('.')
            .map(|(_, extension)| extension.to_ascii_lowercase())
            .as_deref(),
        Some("md" | "txt" | "toml" | "json" | "yaml" | "yml" | "svg")
    );
    let parsed = supported_text
        .then(|| std::str::from_utf8(bytes).ok())
        .flatten();
    let (kind, text, truncated) = if let Some(text) = parsed {
        let mut end = bytes.len().min(MAX_TEXT_PREVIEW_BYTES);
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        (
            AssetPreviewKind::Text,
            Some(text[..end].to_owned()),
            end < bytes.len(),
        )
    } else {
        (AssetPreviewKind::Unsupported, None, false)
    };
    AssetPreview {
        path: path.clone(),
        kind,
        byte_len: bytes.len(),
        content_hash,
        text,
        truncated,
    }
}

fn editable_path(value: &str) -> AppResult<PortablePath> {
    let path = PortablePath::new(value.to_owned()).map_err(|error| {
        let code = match error {
            PathValidationError::Absolute | PathValidationError::InvalidCharacter => {
                "library.asset.path.invalid"
            }
            PathValidationError::ReservedDeviceName => "library.asset.path.device_name",
            _ => "library.asset.path.nonportable",
        };
        asset_error(code)
    })?;
    let mut components = path.as_str().split('/');
    let root = components.next().unwrap_or_default();
    if components.next().is_none() || !EDITABLE_ROOTS.contains(&root) {
        return Err(asset_error("library.asset.path.unsupported_root"));
    }
    Ok(path)
}

fn is_referenced(files: &AssetFiles, target: &PortablePath) -> bool {
    let raw = target.as_str().to_owned();
    let windows = raw.replace('/', "\\");
    let encoded = encode_uri_path(&raw);
    let candidates = [raw, windows, encoded]
        .into_iter()
        .map(|value| value.to_lowercase())
        .collect::<Vec<_>>();
    files.iter().any(|(path, bytes)| {
        path != target
            && std::str::from_utf8(bytes).is_ok_and(|text| {
                let text = text.to_lowercase();
                candidates.iter().any(|candidate| text.contains(candidate))
            })
    })
}

fn encode_uri_path(path: &str) -> String {
    let mut encoded = String::with_capacity(path.len());
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'/') {
            encoded.push(char::from(byte));
        } else {
            use std::fmt::Write as _;
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    encoded
}

fn ensure_hash(bytes: &[u8], expected: &ContentHash) -> AppResult<()> {
    let actual = ContentHash::from_digest(Sha256::digest(bytes).into());
    if &actual == expected {
        Ok(())
    } else {
        Err(asset_error("library.asset.changed"))
    }
}

fn validate_inventory(files: &AssetFiles) -> AppResult<ValidatedInventory> {
    let entries = files
        .iter()
        .map(|(path, bytes)| {
            let size = bytes.len() as u64;
            BundleEntry::new(path.clone(), EntryKind::RegularFile, size, size)
        })
        .collect::<Vec<_>>();
    validate_bundle_inventory(&entries).map_err(AppError::Validation)
}

fn asset_error(code: &'static str) -> AppError {
    AppError::Validation(vec![Diagnostic::error(
        code,
        "Asset operation is invalid, stale, referenced or outside editable roots.",
    )])
}
