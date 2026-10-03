use jameskills_core::domain::{BundleEntry, EntryKind, PortablePath, validate_bundle_inventory};
use jameskills_core::ports::filesystem::validate_archive_entries;
use jameskills_infra::fs::{LocalFileSystem, inspect_bundle_tree};
use std::path::{Path, PathBuf};

fn sandbox(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "jameskills-safepaths-{}-{}-{}",
        std::process::id(),
        name,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("sandbox");
    root
}

fn write_file(path: &Path, contents: &[u8]) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parents");
    }
    std::fs::write(path, contents).expect("write");
}

fn portable(name: &str) -> PortablePath {
    PortablePath::new(name.to_string()).expect("portable test path")
}

fn codes(errors: &[jameskills_core::Diagnostic]) -> Vec<&'static str> {
    errors.iter().map(|error| error.code()).collect()
}

fn expect_errors<T>(
    result: Result<T, Vec<jameskills_core::Diagnostic>>,
    what: &str,
) -> Vec<jameskills_core::Diagnostic> {
    match result {
        Ok(_) => panic!("expected {what} to fail validation"),
        Err(errors) => errors,
    }
}

struct ZipEntry<'a> {
    name: &'a [u8],
    unix_mode: u32,
    compressed: u32,
    uncompressed: u32,
    method: u16,
    flags: u16,
    data: &'a [u8],
}

/// Minimal stored-entry ZIP builder: local headers, central directory and
/// EOCD with caller-controlled sizes, modes and raw names.
fn zip_bytes(entries: &[ZipEntry<'_>]) -> Vec<u8> {
    fn u16(value: u16, out: &mut Vec<u8>) {
        out.extend_from_slice(&value.to_le_bytes());
    }
    fn u32(value: u32, out: &mut Vec<u8>) {
        out.extend_from_slice(&value.to_le_bytes());
    }

    let mut bytes = Vec::new();
    let mut central = Vec::new();
    for entry in entries {
        let offset = bytes.len() as u32;
        bytes.extend_from_slice(&0x0403_4b50_u32.to_le_bytes());
        u16(20, &mut bytes);
        u16(entry.flags, &mut bytes);
        u16(entry.method, &mut bytes);
        u16(0, &mut bytes);
        u16(0, &mut bytes);
        u32(0, &mut bytes);
        u32(entry.compressed, &mut bytes);
        u32(entry.uncompressed, &mut bytes);
        u16(entry.name.len() as u16, &mut bytes);
        u16(0, &mut bytes);
        bytes.extend_from_slice(entry.name);
        bytes.extend_from_slice(entry.data);

        central.extend_from_slice(&0x0201_4b50_u32.to_le_bytes());
        u16(20, &mut central);
        u16(20, &mut central);
        u16(entry.flags, &mut central);
        u16(entry.method, &mut central);
        u16(0, &mut central);
        u16(0, &mut central);
        u32(0, &mut central);
        u32(entry.compressed, &mut central);
        u32(entry.uncompressed, &mut central);
        u16(entry.name.len() as u16, &mut central);
        u16(0, &mut central);
        u16(0, &mut central);
        u16(0, &mut central);
        u16(0, &mut central);
        u32(entry.unix_mode << 16, &mut central);
        u32(offset, &mut central);
        central.extend_from_slice(entry.name);
    }
    let central_offset = bytes.len() as u32;
    let central_size = central.len() as u32;
    bytes.extend_from_slice(&central);
    bytes.extend_from_slice(&0x0605_4b50_u32.to_le_bytes());
    u16(0, &mut bytes);
    u16(0, &mut bytes);
    u16(entries.len() as u16, &mut bytes);
    u16(entries.len() as u16, &mut bytes);
    u32(central_size, &mut bytes);
    u32(central_offset, &mut bytes);
    u16(0, &mut bytes);
    bytes
}

fn stored<'a>(name: &'a [u8], data: &[u8]) -> ZipEntry<'a> {
    ZipEntry {
        name,
        unix_mode: 0o100644,
        compressed: data.len() as u32,
        uncompressed: data.len() as u32,
        method: 0,
        flags: 0,
        data: Box::leak(data.to_vec().into_boxed_slice()),
    }
}

#[test]
fn local_filesystem_inspects_a_valid_bundle_end_to_end() {
    let root = sandbox("local-fs");
    write_file(&root.join("SKILL.md"), b"# skill");
    write_file(&root.join("instructions").join("start.md"), b"go");
    let filesystem = LocalFileSystem;
    let inventory = filesystem.inspect_bundle(&root).expect("inspect");
    assert_eq!(inventory.file_count(), 2);
    assert!(inventory.total_uncompressed_bytes() > 0);
}

#[test]
fn tree_walk_lists_nested_files_and_keeps_everything_inside_staging() {
    let base = sandbox("inside-base");
    let root = base.join("staging");
    std::fs::create_dir_all(&root).expect("staging");
    write_file(&root.join("SKILL.md"), b"# skill");
    write_file(&root.join("sub").join("notes.md"), b"notes");
    let outside = base.join("outside.txt");
    write_file(&outside, b"untouched");

    let entries = inspect_bundle_tree(&root).expect("walk");
    let names: Vec<String> = entries
        .iter()
        .map(|entry| entry.path().as_str().to_string())
        .collect();

    assert!(names.contains(&"SKILL.md".to_string()));
    assert!(names.contains(&"sub/notes.md".to_string()));
    assert!(!names.iter().any(|name| name.contains("outside")));
    assert_eq!(std::fs::read(&outside).expect("sentinel"), b"untouched");
    let inventory = validate_bundle_inventory(&entries).expect("inventory");
    assert_eq!(inventory.file_count(), 2);
}

#[test]
fn tree_walk_rejects_symlink_escape() {
    let base = sandbox("symlink-base");
    let root = base.join("staging");
    std::fs::create_dir_all(&root).expect("staging");
    write_file(&root.join("SKILL.md"), b"# skill");
    let outside = base.join("secret.txt");
    write_file(&outside, b"classified");
    let link = root.join("sub").join("link.md");
    if let Some(parent) = link.parent() {
        std::fs::create_dir_all(parent).expect("parents");
    }
    match symlink_impl(&outside, &link) {
        Ok(()) => {}
        Err(error)
            if error.kind() == std::io::ErrorKind::PermissionDenied
                || error.raw_os_error() == Some(1314) =>
        {
            eprintln!("UNSUPPORTED: symlink creation needs privileges here");
            return;
        }
        Err(error) => panic!("symlink setup failed: {error}"),
    }
    let errors = expect_errors(inspect_bundle_tree(&root), "symlink must be rejected");
    assert!(codes(&errors).contains(&"bundle.entry.kind.unsupported"));
    assert_eq!(std::fs::read(&outside).expect("sentinel"), b"classified");
}

#[cfg(unix)]
use std::os::unix::fs::symlink as symlink_impl;
#[cfg(windows)]
use std::os::windows::fs::symlink_file as symlink_impl;

#[test]
fn tree_walk_detects_case_collision_portably() {
    let entries = vec![
        BundleEntry::new(portable("Docs/guide.md"), EntryKind::RegularFile, 10, 10),
        BundleEntry::new(portable("docs/guide.md"), EntryKind::RegularFile, 10, 10),
        BundleEntry::new(portable("SKILL.md"), EntryKind::RegularFile, 10, 10),
    ];
    let errors = expect_errors(validate_bundle_inventory(&entries), "collision");
    assert!(codes(&errors).contains(&"bundle.path.case_collision"));
}

#[test]
fn portable_paths_reject_absolute_and_dotdot_names() {
    assert!(PortablePath::new("/abs.md".to_string()).is_err());
    assert!(PortablePath::new("../evil.md".to_string()).is_err());
    assert!(PortablePath::new("sub/../../evil.md".to_string()).is_err());
}

#[test]
#[cfg(unix)]
fn tree_walk_rejects_non_utf8_names() {
    use std::os::unix::ffi::OsStringExt;
    let root = sandbox("nonutf8");
    write_file(&root.join("SKILL.md"), b"# skill");
    let raw = std::ffi::OsString::from_vec(vec![0xff, 0xfe, 0x2e, 0x6d, 0x64]);
    std::fs::write(root.join(raw), b"bytes").expect("raw name");
    let errors = expect_errors(inspect_bundle_tree(&root), "non-utf8");
    assert!(codes(&errors).contains(&"bundle.path.invalid_utf8"));
}

#[test]
fn zip_central_validation_accepts_minimal_archive() {
    let bytes = zip_bytes(&[
        stored(b"SKILL.md", b"# skill"),
        stored(b"instructions.md", b"do things"),
    ]);
    let inventory = validate_archive_entries(&bytes).expect("archive");
    assert_eq!(inventory.file_count(), 2);
}

#[test]
fn zip_headers_cannot_hide_a_bomb() {
    let data = vec![0u8; 64];
    let payload = ZipEntry {
        name: b"data.bin",
        unix_mode: 0o100644,
        compressed: 64,
        uncompressed: 40 * 1024 * 1024,
        method: 8,
        flags: 0,
        data: Box::leak(data.into_boxed_slice()),
    };
    let bytes = zip_bytes(&[stored(b"SKILL.md", b"# skill"), payload]);
    assert!(bytes.len() < 4096, "bomb fixture must stay tiny");
    let errors = expect_errors(validate_archive_entries(&bytes), "bomb");
    assert!(codes(&errors).contains(&"bundle.total_size.limit"));
}

#[test]
fn zip_rejects_traversal_absolute_and_symlink_entries() {
    let traversal = zip_bytes(&[stored(b"SKILL.md", b"# skill"), stored(b"../evil.md", b"x")]);
    let errors = expect_errors(validate_archive_entries(&traversal), "traversal");
    assert!(codes(&errors).contains(&"bundle.path.invalid_component"));

    let absolute = zip_bytes(&[stored(b"SKILL.md", b"# skill"), stored(b"/abs.md", b"x")]);
    let errors = expect_errors(validate_archive_entries(&absolute), "absolute");
    assert!(codes(&errors).contains(&"bundle.path.absolute"));

    let link = ZipEntry {
        name: b"link.md",
        unix_mode: 0o120777,
        compressed: 1,
        uncompressed: 1,
        method: 0,
        flags: 0,
        data: Box::leak(vec![b'x'].into_boxed_slice()),
    };
    let bytes = zip_bytes(&[stored(b"SKILL.md", b"# skill"), link]);
    let errors = expect_errors(validate_archive_entries(&bytes), "symlink bit");
    assert!(codes(&errors).contains(&"bundle.entry.kind.unsupported"));
}

#[test]
fn zip_rejects_duplicate_oversized_and_missing_skill() {
    let duplicate = zip_bytes(&[
        stored(b"SKILL.md", b"# skill"),
        stored(b"SKILL.md", b"# skill again"),
    ]);
    let errors = expect_errors(validate_archive_entries(&duplicate), "duplicate");
    assert!(codes(&errors).contains(&"bundle.path.duplicate"));

    let big = vec![b'a'; 512];
    let oversized = ZipEntry {
        name: b"SKILL.md",
        unix_mode: 0o100644,
        compressed: 512,
        uncompressed: 512 * 1024,
        method: 0,
        flags: 0,
        data: Box::leak(big.into_boxed_slice()),
    };
    let bytes = zip_bytes(&[oversized]);
    let errors = expect_errors(validate_archive_entries(&bytes), "oversized skill");
    assert!(codes(&errors).contains(&"bundle.file.too_large"));

    let missing = zip_bytes(&[stored(b"notes.md", b"no skill here")]);
    let errors = expect_errors(validate_archive_entries(&missing), "missing skill");
    assert!(codes(&errors).contains(&"bundle.skill.missing"));
}

#[test]
fn validation_writes_nothing_inside_or_outside_staging() {
    let root = sandbox("nowrite");
    write_file(&root.join("SKILL.md"), b"# skill");
    let before: Vec<PathBuf> = walk_raw(&root);
    let bad = zip_bytes(&[stored(b"SKILL.md", b"# skill"), stored(b"../x.md", b"x")]);
    let _ = expect_errors(validate_archive_entries(&bad), "invalid");
    let _ = inspect_bundle_tree(&root).expect("walk");
    let after: Vec<PathBuf> = walk_raw(&root);
    assert_eq!(before, after, "validation must not write");
}

fn walk_raw(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(&dir)
            .expect("read")
            .map(|entry| entry.expect("entry").path())
            .collect();
        entries.sort();
        for entry in entries {
            if entry.is_dir() {
                stack.push(entry);
            } else {
                out.push(entry);
            }
        }
    }
    out.sort();
    out
}
