use jameskills_core::{
    Diagnostic,
    domain::{BundleEntry, EntryKind, ValidatedInventory, validate_bundle_inventory},
    ports::filesystem::bundle_entry_from_path,
};
use std::path::Path;

/// Local filesystem adapter: read-only walks and validation live here so
/// core never touches the disk. Nothing here writes: staging happens only
/// after validation, in a later slice.
pub struct LocalFileSystem;

impl LocalFileSystem {
    pub fn inspect_bundle(&self, root: &Path) -> Result<ValidatedInventory, Vec<Diagnostic>> {
        validate_bundle_inventory(&inspect_bundle_tree(root)?)
    }
}

fn invalid_root(message: &'static str) -> Vec<Diagnostic> {
    vec![Diagnostic::error("bundle.root.invalid", message)]
}

/// Walks a bundle tree without following symlinks. Every entry is classified
/// with `symlink_metadata`: links, sockets and other non-regular files are
/// rejected, directories are descended but never listed, and each listed
/// path must stay under the canonical root. Hard links with more than one
/// name are rejected on Unix via link count; Windows reparse points that do
/// not present as links need OS APIs and arrive with the Windows port.
pub fn inspect_bundle_tree(root: &Path) -> Result<Vec<BundleEntry>, Vec<Diagnostic>> {
    let canonical = root
        .canonicalize()
        .map_err(|_| invalid_root("Bundle root is not accessible."))?;
    if !canonical.is_dir() {
        return Err(invalid_root("Bundle root is not a directory."));
    }
    let mut entries = Vec::new();
    let mut stack = vec![canonical.clone()];
    while let Some(dir) = stack.pop() {
        let read = std::fs::read_dir(&dir)
            .map_err(|_| invalid_root("Bundle directory is not readable."))?;
        for child in read {
            let child =
                child.map_err(|_| invalid_root("Bundle directory entry is not readable."))?;
            let path = child.path();
            if !path.starts_with(&canonical) {
                return Err(vec![Diagnostic::error(
                    "bundle.path.escape",
                    "Bundle entry escapes its root.",
                )]);
            }
            let metadata = std::fs::symlink_metadata(&path)
                .map_err(|_| invalid_root("Bundle entry metadata is not readable."))?;
            let file_type = metadata.file_type();
            if file_type.is_symlink() || (!file_type.is_dir() && !file_type.is_file()) {
                return Err(vec![Diagnostic::error(
                    "bundle.entry.kind.unsupported",
                    "Bundle entries must be regular files.",
                )]);
            }
            if file_type.is_dir() {
                stack.push(path);
                continue;
            }
            reject_hard_link(&metadata)?;
            let relative = path
                .strip_prefix(&canonical)
                .map_err(|_| {
                    vec![Diagnostic::error(
                        "bundle.path.escape",
                        "Bundle entry escapes its root.",
                    )]
                })?
                .to_path_buf();
            entries.push(tree_entry(&relative, metadata.len())?);
        }
    }
    Ok(entries)
}

fn tree_entry(relative: &Path, size: u64) -> Result<BundleEntry, Vec<Diagnostic>> {
    let mut name = String::new();
    for component in relative.components() {
        let text = component.as_os_str().to_str().ok_or_else(|| {
            vec![Diagnostic::error(
                "bundle.path.invalid_utf8",
                "Bundle path is not valid UTF-8.",
            )]
        })?;
        if !name.is_empty() {
            name.push('/');
        }
        name.push_str(text);
    }
    bundle_entry_from_path(&name, EntryKind::RegularFile, size, size)
}

#[cfg(unix)]
fn reject_hard_link(metadata: &std::fs::Metadata) -> Result<(), Vec<Diagnostic>> {
    use std::os::unix::fs::MetadataExt;
    if metadata.nlink() > 1 {
        return Err(vec![Diagnostic::error(
            "bundle.entry.kind.unsupported",
            "Bundle entries must be regular files.",
        )]);
    }
    Ok(())
}

#[cfg(not(unix))]
fn reject_hard_link(_metadata: &std::fs::Metadata) -> Result<(), Vec<Diagnostic>> {
    Ok(())
}
