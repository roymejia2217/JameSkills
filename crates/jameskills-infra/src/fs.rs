use jameskills_core::{
    Diagnostic,
    domain::{BundleEntry, EntryKind, PortablePath, ValidatedInventory, validate_bundle_inventory},
    ports::filesystem::bundle_entry_from_path,
};
use std::collections::BTreeMap;
use std::io::Read;
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

/// Reads the raw bytes of every validated file under the canonical root, in
/// inventory order. Reads are capped by the validated totals, so a file that
/// grows between inspection and hashing fails instead of exhausting memory;
/// kind is rechecked at read time, but staging flows must revalidate before
/// writing anywhere. Export and hashing share this single byte source.
pub fn read_bundle_bytes(
    root: &Path,
    inventory: &ValidatedInventory,
) -> Result<BTreeMap<PortablePath, Vec<u8>>, Vec<Diagnostic>> {
    let canonical = root
        .canonicalize()
        .map_err(|_| invalid_root("Bundle root is not accessible."))?;
    let mut budget = inventory.total_uncompressed_bytes();
    let mut files = BTreeMap::new();
    for file in inventory.files() {
        let mut path = canonical.clone();
        for component in file.path().as_str().split('/') {
            path.push(component);
        }
        let kind = std::fs::symlink_metadata(&path)
            .map_err(|_| unreadable())?
            .file_type();
        if !kind.is_file() {
            return Err(vec![Diagnostic::error(
                "bundle.entry.kind.unsupported",
                "Bundle entries must be regular files.",
            )]);
        }
        let handle = std::fs::File::open(&path).map_err(|_| unreadable())?;
        let mut content = Vec::new();
        handle
            .take(budget.saturating_add(1))
            .read_to_end(&mut content)
            .map_err(|_| unreadable())?;
        if content.len() as u64 > budget {
            return Err(vec![Diagnostic::error(
                "bundle.file.changed",
                "Bundle file changed during hashing.",
            )]);
        }
        budget -= content.len() as u64;
        files.insert(file.path().clone(), content);
    }
    Ok(files)
}

fn unreadable() -> Vec<Diagnostic> {
    vec![Diagnostic::error(
        "bundle.file.unreadable",
        "Bundle file is not readable.",
    )]
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

#[cfg(test)]
mod tests {
    use super::*;
    use jameskills_core::domain::hash_bundle;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static BUNDLE_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct TempBundle {
        root: PathBuf,
    }

    impl TempBundle {
        fn new(files: &[(&str, &[u8])]) -> Self {
            let id = BUNDLE_COUNTER.fetch_add(1, Ordering::SeqCst);
            let root =
                std::env::temp_dir().join(format!("jameskills-bundle-{}-{id}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            for (name, content) in files {
                let mut path = root.clone();
                for component in name.split('/') {
                    path.push(component);
                }
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(&path, content).unwrap();
            }
            Self { root }
        }

        fn remove(&self, name: &str) {
            let mut path = self.root.clone();
            for component in name.split('/') {
                path.push(component);
            }
            std::fs::remove_file(&path).unwrap();
        }
    }

    impl Drop for TempBundle {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn reads_validated_tree_and_hashes_through_domain() {
        let bundle =
            TempBundle::new(&[("SKILL.md", b"# Skill\n"), ("docs/guide.md", b"# Guide\n")]);
        let system = LocalFileSystem;
        let inventory = system.inspect_bundle(&bundle.root).unwrap();
        let bytes = read_bundle_bytes(&bundle.root, &inventory).unwrap();
        assert_eq!(bytes.len(), 2);
        assert_eq!(bytes.values().map(Vec::len).sum::<usize>(), 16);
        assert!(hash_bundle(&inventory, &bytes).is_ok());
    }

    #[test]
    fn rejects_file_grown_after_inspection() {
        let bundle = TempBundle::new(&[("SKILL.md", b"# Skill\n")]);
        let entries = vec![BundleEntry::new(
            PortablePath::new("SKILL.md".to_owned()).unwrap(),
            EntryKind::RegularFile,
            4,
            4,
        )];
        let inventory = validate_bundle_inventory(&entries).unwrap();
        let result = read_bundle_bytes(&bundle.root, &inventory);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err()[0].code(), "bundle.file.changed");
    }

    #[test]
    fn rejects_file_removed_before_reading() {
        let bundle = TempBundle::new(&[("SKILL.md", b"# Skill\n")]);
        let system = LocalFileSystem;
        let inventory = system.inspect_bundle(&bundle.root).unwrap();
        bundle.remove("SKILL.md");
        let result = read_bundle_bytes(&bundle.root, &inventory);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err()[0].code(), "bundle.file.unreadable");
    }
}
