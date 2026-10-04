use jameskills_core::{
    Diagnostic,
    domain::{
        BundleEntry, ContentHash, EntryKind, PortablePath, ValidatedInventory, hash_bundle,
        validate_bundle_inventory,
    },
    ports::filesystem::{
        BundleFiles, FileSystemPort, bundle_entry_from_path, extract_archive_files,
        validate_archive_entries,
    },
};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Local filesystem adapter: validation, explicit staging, and content-addressed
/// blob IO live here so core never touches the disk. Writes follow validation.
pub struct LocalFileSystem;

impl LocalFileSystem {
    pub fn inspect_bundle(&self, root: &Path) -> Result<ValidatedInventory, Vec<Diagnostic>> {
        validate_bundle_inventory(&inspect_bundle_tree(root)?)
    }
}

impl FileSystemPort for LocalFileSystem {
    fn read_bundle_directory(&self, root: &Path) -> Result<BundleFiles, Vec<Diagnostic>> {
        let inventory = self.inspect_bundle(root)?;
        read_bundle_bytes(root, &inventory)
    }
}

/// Reads a `.jskill` archive fully before trusting it: central validation,
/// then stored-entry extraction with checksums and validated totals. Shares
/// validation with directory import so both accept exact canonical bytes.
pub fn read_bundle(bytes: &[u8]) -> Result<(ValidatedInventory, BundleFiles), Vec<Diagnostic>> {
    let inventory = validate_archive_entries(bytes)?;
    let files = extract_archive_files(bytes, &inventory)?;
    Ok((inventory, files))
}

/// Addresses a content blob by canonical hash: a two-hex prefix directory
/// keeps single directories small, mirroring the documented blob layout.
pub fn blob_path(blobs_root: &Path, hash: &ContentHash) -> PathBuf {
    let hex = hash.as_str();
    blobs_root.join(&hex[..2]).join(format!("{hex}.bundle"))
}

/// Stores archive bytes under their canonical hash. Identical bytes are
/// idempotent; different bytes under the same hash fail instead of
/// overwriting published content.
pub fn store_blob_bytes(
    blobs_root: &Path,
    hash: &ContentHash,
    archive: &[u8],
) -> Result<PathBuf, Vec<Diagnostic>> {
    read_verified_blob_archive(hash, archive)?;
    let path = blob_path(blobs_root, hash);
    ensure_private_directory(blobs_root)?;
    ensure_private_directory(&blobs_root.join(&hash.as_str()[..2]))?;
    match std::fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_file() => {}
        Ok(_) => return Err(blob_io("Bundle blob path is not a regular file.")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err(blob_io("Bundle blob path is not readable.")),
    }
    match std::fs::read(&path) {
        Ok(existing) if existing == archive => {
            restrict_file(&path).map_err(|_| blob_io("Bundle blob is not private."))?;
            return Ok(path);
        }
        Ok(_) => {
            return Err(vec![Diagnostic::error(
                "bundle.blob.conflict",
                "Different bundle bytes share one hash.",
            )]);
        }
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            return Err(blob_io("Bundle blob is not readable."));
        }
        Err(_) => {}
    }
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = match options.open(&path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let metadata = std::fs::symlink_metadata(&path)
                .map_err(|_| blob_io("Bundle blob path is not readable."))?;
            if !metadata.file_type().is_file() {
                return Err(blob_io("Bundle blob path is not a regular file."));
            }
            let existing =
                std::fs::read(&path).map_err(|_| blob_io("Bundle blob is not readable."))?;
            if existing == archive {
                restrict_file(&path).map_err(|_| blob_io("Bundle blob is not private."))?;
                return Ok(path);
            }
            return Err(vec![Diagnostic::error(
                "bundle.blob.conflict",
                "Different bundle bytes share one hash.",
            )]);
        }
        Err(_) => return Err(blob_io("Bundle blob is not writable.")),
    };
    let result = file
        .write_all(archive)
        .and_then(|()| file.sync_all())
        .and_then(|()| restrict_file(&path));
    drop(file);
    if result.is_err() {
        let _ = std::fs::remove_file(&path);
        return Err(blob_io("Bundle blob could not be durably stored."));
    }
    Ok(path)
}

fn ensure_private_directory(path: &Path) -> Result<(), Vec<Diagnostic>> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_dir() => {}
        Ok(_) => return Err(blob_io("Bundle blob path is not a regular directory.")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            std::fs::create_dir(path)
                .map_err(|_| blob_io("Bundle blob directory is not writable."))?;
        }
        Err(_) => return Err(blob_io("Bundle blob directory is not readable.")),
    }
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|_| blob_io("Bundle blob directory is not readable."))?;
    if !metadata.file_type().is_dir() {
        return Err(blob_io("Bundle blob path is not a regular directory."));
    }
    restrict_directory(path).map_err(|_| blob_io("Bundle blob directory is not private."))
}

fn read_verified_blob_archive(
    expected: &ContentHash,
    archive: &[u8],
) -> Result<BundleFiles, Vec<Diagnostic>> {
    let (inventory, files) = read_bundle(archive)?;
    let actual = hash_bundle(&inventory, &files).map_err(|_| {
        vec![Diagnostic::error(
            "bundle.blob.unverifiable",
            "Bundle blob cannot be hashed.",
        )]
    })?;
    if &actual != expected {
        return Err(vec![Diagnostic::error(
            "bundle.blob.checksum_mismatch",
            "Bundle blob bytes do not match their hash.",
        )]);
    }
    Ok(files)
}

/// Verifies blob bytes end to end: the archive must parse, its canonical
/// hash must match, and the recovered files return for callers that keep
/// going. Anything else fails closed.
pub fn verify_blob_bytes(
    blobs_root: &Path,
    expected: &ContentHash,
) -> Result<BundleFiles, Vec<Diagnostic>> {
    let path = blob_path(blobs_root, expected);
    let prefix = path
        .parent()
        .ok_or_else(|| blob_io("Bundle blob path is invalid."))?;
    for directory in [blobs_root, prefix] {
        let metadata = std::fs::symlink_metadata(directory)
            .map_err(|_| blob_io("Bundle blob directory is not readable."))?;
        if !metadata.file_type().is_dir() {
            return Err(blob_io("Bundle blob path is not a regular directory."));
        }
    }
    let metadata =
        std::fs::symlink_metadata(&path).map_err(|_| blob_io("Bundle blob is not readable."))?;
    if !metadata.file_type().is_file() {
        return Err(blob_io("Bundle blob path is not a regular file."));
    }
    let bytes = std::fs::read(path).map_err(|_| blob_io("Bundle blob is not readable."))?;
    read_verified_blob_archive(expected, &bytes)
}

/// Lists only canonical content-addressed blob files. Unknown filesystem
/// entries are ignored; symlinks/reparse-like entries at the enumerated
/// levels fail closed instead of being followed.
pub fn list_blob_hashes(blobs_root: &Path) -> Result<Vec<ContentHash>, Vec<Diagnostic>> {
    let root_meta = match std::fs::symlink_metadata(blobs_root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(_) => return Err(blob_io("Bundle blob directory is not readable.")),
    };
    if !root_meta.file_type().is_dir() || root_meta.file_type().is_symlink() {
        return Err(blob_io("Bundle blob directory is not a regular directory."));
    }
    let mut hashes = Vec::new();
    let prefixes = std::fs::read_dir(blobs_root)
        .map_err(|_| blob_io("Bundle blob directory is not readable."))?;
    for prefix in prefixes {
        let prefix = prefix.map_err(|_| blob_io("Bundle blob directory is not readable."))?;
        let prefix_name = prefix.file_name();
        let Some(prefix_name) = prefix_name.to_str() else {
            continue;
        };
        if prefix_name.len() != 2
            || !prefix_name
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            continue;
        }
        let prefix_meta = std::fs::symlink_metadata(prefix.path())
            .map_err(|_| blob_io("Bundle blob directory is not readable."))?;
        if prefix_meta.file_type().is_symlink() {
            return Err(blob_io("Bundle blob path contains a symlink."));
        }
        if !prefix_meta.file_type().is_dir() {
            continue;
        }
        let entries = std::fs::read_dir(prefix.path())
            .map_err(|_| blob_io("Bundle blob directory is not readable."))?;
        for entry in entries {
            let entry = entry.map_err(|_| blob_io("Bundle blob directory is not readable."))?;
            let metadata = std::fs::symlink_metadata(entry.path())
                .map_err(|_| blob_io("Bundle blob is not readable."))?;
            if metadata.file_type().is_symlink() {
                return Err(blob_io("Bundle blob path contains a symlink."));
            }
            if !metadata.file_type().is_file() {
                continue;
            }
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            let Some(hex) = name.strip_suffix(".bundle") else {
                continue;
            };
            let Ok(hash) = ContentHash::parse_hex(hex) else {
                continue;
            };
            if &hex[..2] == prefix_name {
                hashes.push(hash);
            }
        }
    }
    hashes.sort();
    hashes.dedup();
    Ok(hashes)
}

fn blob_io(message: &'static str) -> Vec<Diagnostic> {
    vec![Diagnostic::error("bundle.blob.io", message)]
}

/// Unpacks a validated archive into private staging: nothing is written
/// before the full archive validates, the staging directory must not exist
/// (a pre-existing destination is never touched), and any failure removes
/// what this call created. On Unix, staging dirs are 0700 and files 0600;
/// on Windows the directory inherits the user's ACL. Staging is never the
/// library: install flows revalidate from staging before touching targets.
pub fn unpack_bundle_to_staging(
    bytes: &[u8],
    staging: &Path,
) -> Result<ValidatedInventory, Vec<Diagnostic>> {
    let (inventory, files) = read_bundle(bytes)?;
    if std::fs::symlink_metadata(staging).is_ok() {
        return Err(vec![Diagnostic::error(
            "bundle.staging.exists",
            "Staging directory already exists.",
        )]);
    }
    let failed = |message: &'static str| {
        let _ = std::fs::remove_dir_all(staging);
        vec![Diagnostic::error("bundle.staging.unwritable", message)]
    };
    if std::fs::create_dir_all(staging).is_err() {
        return Err(failed("Staging directory is not writable."));
    }
    restrict_directory(staging).map_err(|_| failed("Staging directory is not private."))?;
    for file in inventory.files() {
        let content = files.get(file.path()).ok_or_else(|| {
            let _ = std::fs::remove_dir_all(staging);
            vec![Diagnostic::error(
                "bundle.archive.mismatch",
                "Archive extraction disagrees with its inventory.",
            )]
        })?;
        let mut path = staging.to_path_buf();
        for component in file.path().as_str().split('/') {
            path.push(component);
        }
        if let Some(parent) = path.parent()
            && std::fs::create_dir_all(parent).is_err()
        {
            return Err(failed("Staging directory is not writable."));
        }
        if std::fs::write(&path, content).is_err() {
            return Err(failed("Staging file is not writable."));
        }
        if restrict_file(&path).is_err() {
            return Err(failed("Staging file is not private."));
        }
    }
    Ok(inventory)
}

#[cfg(unix)]
fn restrict_directory(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
}

#[cfg(not(unix))]
fn restrict_directory(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

#[cfg(unix)]
fn restrict_file(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn restrict_file(_path: &Path) -> std::io::Result<()> {
    Ok(())
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
