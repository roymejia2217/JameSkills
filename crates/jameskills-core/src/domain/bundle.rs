use super::PortablePath;
use crate::Diagnostic;
use icu_casemap::{CaseMapper, CaseMapperBorrowed};
use std::collections::BTreeSet;
use unicode_normalization::UnicodeNormalization;

const MAX_BUNDLE_BYTES: u64 = 20 * 1024 * 1024;
const MAX_FILES: usize = 2_000;
const MAX_TEXT_FILE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_SKILL_FILE_BYTES: u64 = 256 * 1024;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    RegularFile,
    Directory,
    SymbolicLink,
    HardLink,
    ReparsePoint,
}

#[derive(Clone, PartialEq, Eq)]
pub struct BundleEntry {
    path: PortablePath,
    kind: EntryKind,
    compressed_bytes: u64,
    uncompressed_bytes: u64,
}

impl BundleEntry {
    pub fn new(
        path: PortablePath,
        kind: EntryKind,
        compressed_bytes: u64,
        uncompressed_bytes: u64,
    ) -> Self {
        Self {
            path,
            kind,
            compressed_bytes,
            uncompressed_bytes,
        }
    }

    pub fn path(&self) -> &PortablePath {
        &self.path
    }
    pub fn kind(&self) -> EntryKind {
        self.kind
    }
    pub fn compressed_bytes(&self) -> u64 {
        self.compressed_bytes
    }
    pub fn uncompressed_bytes(&self) -> u64 {
        self.uncompressed_bytes
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ValidatedFile {
    path: PortablePath,
}

impl ValidatedFile {
    pub fn path(&self) -> &PortablePath {
        &self.path
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ValidatedInventory {
    files: Vec<ValidatedFile>,
    total_compressed_bytes: u64,
    total_uncompressed_bytes: u64,
}

#[derive(Clone, Copy)]
enum PathNodeKind {
    Directory,
    File,
}

struct PathNode {
    spelling: String,
    kind: PathNodeKind,
}

impl ValidatedInventory {
    pub fn file_count(&self) -> usize {
        self.files.len()
    }
    pub fn total_compressed_bytes(&self) -> u64 {
        self.total_compressed_bytes
    }
    pub fn total_uncompressed_bytes(&self) -> u64 {
        self.total_uncompressed_bytes
    }
    pub fn files(&self) -> &[ValidatedFile] {
        &self.files
    }
}

pub fn validate_bundle_inventory(
    entries: &[BundleEntry],
) -> Result<ValidatedInventory, Vec<Diagnostic>> {
    if entries.len() > MAX_FILES {
        return Err(vec![Diagnostic::error(
            "bundle.file_count.limit",
            "Bundle contains too many files.",
        )]);
    }
    if entries.is_empty() {
        return Err(vec![Diagnostic::error(
            "bundle.skill.missing",
            "Bundle must contain SKILL.md.",
        )]);
    }

    let case_mapper = CaseMapper::new();
    let mut exact_paths = BTreeSet::new();
    let mut folded_nodes = std::collections::BTreeMap::new();
    let mut files = Vec::with_capacity(entries.len());
    let mut total_compressed_bytes = 0_u64;
    let mut total_uncompressed_bytes = 0_u64;
    let mut has_skill = false;

    for entry in entries {
        if entry.kind != EntryKind::RegularFile {
            return Err(vec![Diagnostic::error(
                "bundle.entry.kind.unsupported",
                "Bundle entries must be regular files.",
            )]);
        }
        let path = entry.path.as_str();
        if !exact_paths.insert(path) {
            return Err(vec![Diagnostic::error(
                "bundle.path.duplicate",
                "Bundle contains a duplicate path.",
            )]);
        }
        validate_path_tree(path, &case_mapper, &mut folded_nodes)?;
        if path == "SKILL.md" {
            has_skill = true;
            if entry.uncompressed_bytes > MAX_SKILL_FILE_BYTES {
                return Err(vec![Diagnostic::error(
                    "bundle.file.too_large",
                    "SKILL.md exceeds its size limit.",
                )]);
            }
        }
        if is_text_path(path) && entry.uncompressed_bytes > MAX_TEXT_FILE_BYTES {
            return Err(vec![Diagnostic::error(
                "bundle.file.too_large",
                "Text file exceeds its size limit.",
            )]);
        }
        total_compressed_bytes = total_compressed_bytes
            .checked_add(entry.compressed_bytes)
            .ok_or_else(|| {
                vec![Diagnostic::error(
                    "bundle.total_size.limit",
                    "Bundle exceeds its size limit.",
                )]
            })?;
        total_uncompressed_bytes = total_uncompressed_bytes
            .checked_add(entry.uncompressed_bytes)
            .ok_or_else(|| {
                vec![Diagnostic::error(
                    "bundle.total_size.limit",
                    "Bundle exceeds its size limit.",
                )]
            })?;
        if total_compressed_bytes > MAX_BUNDLE_BYTES || total_uncompressed_bytes > MAX_BUNDLE_BYTES
        {
            return Err(vec![Diagnostic::error(
                "bundle.total_size.limit",
                "Bundle exceeds its size limit.",
            )]);
        }
        files.push(ValidatedFile {
            path: entry.path.clone(),
        });
    }

    if !has_skill {
        return Err(vec![Diagnostic::error(
            "bundle.skill.missing",
            "Bundle must contain SKILL.md.",
        )]);
    }
    files.sort_by(|left, right| {
        left.path
            .as_str()
            .as_bytes()
            .cmp(right.path.as_str().as_bytes())
    });
    Ok(ValidatedInventory {
        files,
        total_compressed_bytes,
        total_uncompressed_bytes,
    })
}

fn validate_path_tree(
    path: &str,
    case_mapper: &CaseMapperBorrowed<'_>,
    nodes: &mut std::collections::BTreeMap<String, PathNode>,
) -> Result<(), Vec<Diagnostic>> {
    let components = path.split('/').collect::<Vec<_>>();
    let mut original_prefix = String::new();
    let mut folded_prefix = String::new();
    for (index, component) in components.iter().enumerate() {
        if index > 0 {
            original_prefix.push('/');
            folded_prefix.push('/');
        }
        original_prefix.push_str(component);
        let folded_component: String = case_mapper.fold_string(component).nfc().collect();
        folded_prefix.push_str(&folded_component);
        let final_component = index + 1 == components.len();
        let requested_kind = if final_component {
            PathNodeKind::File
        } else {
            PathNodeKind::Directory
        };
        if let Some(existing) = nodes.get(&folded_prefix) {
            if existing.spelling != original_prefix {
                return Err(vec![Diagnostic::error(
                    "bundle.path.case_collision",
                    "Bundle path components collide under portable Unicode case folding.",
                )]);
            }
            let is_file_conflict = matches!(
                (existing.kind, requested_kind),
                (PathNodeKind::File, PathNodeKind::Directory)
                    | (PathNodeKind::Directory, PathNodeKind::File)
            );
            if is_file_conflict {
                return Err(vec![Diagnostic::error(
                    "bundle.path.component_conflict",
                    "A bundle file conflicts with another file path component.",
                )]);
            }
        } else {
            nodes.insert(
                folded_prefix.clone(),
                PathNode {
                    spelling: original_prefix.clone(),
                    kind: requested_kind,
                },
            );
        }
    }
    Ok(())
}

fn is_text_path(path: &str) -> bool {
    let filename = path.rsplit('/').next().unwrap_or(path);
    let extension = filename.rsplit_once('.').map(|(_, extension)| extension);
    !matches!(
        extension.map(str::to_ascii_lowercase).as_deref(),
        Some(
            "bin"
                | "png"
                | "jpg"
                | "jpeg"
                | "gif"
                | "webp"
                | "ico"
                | "pdf"
                | "woff"
                | "woff2"
                | "ttf"
                | "otf"
        )
    )
}
