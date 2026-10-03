use jameskills_core::domain::{PortablePath, hash_bundle};
use jameskills_core::ports::write_bundle_archive;
use jameskills_infra::fs::{read_bundle, unpack_bundle_to_staging};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static STAGING_COUNTER: AtomicU64 = AtomicU64::new(0);

fn portable(name: &str) -> PortablePath {
    PortablePath::new(name.to_owned()).unwrap()
}

fn tree() -> BTreeMap<PortablePath, Vec<u8>> {
    let mut files = BTreeMap::new();
    files.insert(portable("SKILL.md"), b"# Skill\n".to_vec());
    files.insert(portable("docs/guide.md"), b"# Guide\r\nline\n".to_vec());
    files.insert(
        portable("assets/logo.bin"),
        vec![0x00, 0xFF, 0x50, 0x4B, 0x03, 0x04],
    );
    files.insert(portable("assets/nested.zip"), b"PK-fake-nested".to_vec());
    files.insert(portable("guia.md"), "Guía\n".as_bytes().to_vec());
    files
}

/// Minimal stored-only ZIP builder with exact caller-controlled names, so the
/// adversarial cases below do not depend on the unit under test.
fn raw_zip(names: &[&[u8]], contents: &[&[u8]]) -> Vec<u8> {
    assert_eq!(names.len(), contents.len());
    let mut out = Vec::new();
    let mut offsets = Vec::new();
    for (name, content) in names.iter().zip(contents.iter()) {
        offsets.push(out.len() as u32);
        push_u16(&mut out, 0x0403);
        push_u16(&mut out, 0x4B50);
        push_u16(&mut out, 20);
        push_u16(&mut out, 1 << 11);
        push_u16(&mut out, 0);
        push_u16(&mut out, 0);
        push_u16(&mut out, 0x0021);
        push_u32(&mut out, crc32(content));
        push_u32(&mut out, content.len() as u32);
        push_u32(&mut out, content.len() as u32);
        push_u16(&mut out, name.len() as u16);
        push_u16(&mut out, 0);
        out.extend_from_slice(name);
        out.extend_from_slice(content);
    }
    let central_start = out.len() as u32;
    for ((name, content), offset) in names.iter().zip(contents.iter()).zip(offsets.iter()) {
        push_u16(&mut out, 0x0201);
        push_u16(&mut out, 0x4B50);
        push_u16(&mut out, 20);
        push_u16(&mut out, 20);
        push_u16(&mut out, 1 << 11);
        push_u16(&mut out, 0);
        push_u16(&mut out, 0);
        push_u16(&mut out, 0x0021);
        push_u32(&mut out, crc32(content));
        push_u32(&mut out, content.len() as u32);
        push_u32(&mut out, content.len() as u32);
        push_u16(&mut out, name.len() as u16);
        push_u16(&mut out, 0);
        push_u16(&mut out, 0);
        push_u16(&mut out, 0);
        push_u16(&mut out, 0);
        push_u32(&mut out, 0o100644 << 16);
        push_u32(&mut out, *offset);
        out.extend_from_slice(name);
    }
    let central_end = out.len() as u32;
    push_u16(&mut out, 0x0605);
    push_u16(&mut out, 0x4B50);
    push_u16(&mut out, 0);
    push_u16(&mut out, 0);
    push_u16(&mut out, names.len() as u16);
    push_u16(&mut out, names.len() as u16);
    push_u32(&mut out, central_end - central_start);
    push_u32(&mut out, central_start);
    push_u16(&mut out, 0);
    out
}

fn push_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn fresh_staging() -> PathBuf {
    let id = STAGING_COUNTER.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!("jameskills-staging-{}-{id}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    path
}

#[test]
fn roundtrip_preserves_bytes_and_bundle_hash() {
    let files = tree();
    let archive = write_bundle_archive(&files).unwrap();
    let (inventory, recovered) = read_bundle(&archive).unwrap();
    assert_eq!(recovered, files);
    assert_eq!(
        hash_bundle(&inventory, &recovered).unwrap().as_str(),
        hash_bundle(
            &jameskills_core::domain::validate_bundle_inventory(
                &files
                    .iter()
                    .map(|(path, content)| {
                        jameskills_core::domain::BundleEntry::new(
                            path.clone(),
                            jameskills_core::domain::EntryKind::RegularFile,
                            content.len() as u64,
                            content.len() as u64,
                        )
                    })
                    .collect::<Vec<_>>(),
            )
            .unwrap(),
            &files,
        )
        .unwrap()
        .as_str()
    );
}

#[test]
fn exports_are_byte_identical() {
    let files = tree();
    assert_eq!(
        write_bundle_archive(&files).unwrap(),
        write_bundle_archive(&files).unwrap()
    );
}

#[test]
fn rejects_traversal_entry() {
    let archive = raw_zip(&[b"SKILL.md", b"../evil.md"], &[b"# Skill\n", b"x"]);
    assert!(read_bundle(&archive).is_err());
}

#[test]
fn rejects_absolute_entry() {
    let archive = raw_zip(&[b"SKILL.md", b"/abs.md"], &[b"# Skill\n", b"x"]);
    assert!(read_bundle(&archive).is_err());
}

#[test]
fn rejects_duplicate_entry() {
    let archive = raw_zip(&[b"SKILL.md", b"SKILL.md"], &[b"# Skill\n", b"other"]);
    assert!(read_bundle(&archive).is_err());
}

#[test]
fn rejects_windows_device_name() {
    let archive = raw_zip(&[b"SKILL.md", b"AUX.md"], &[b"# Skill\n", b"x"]);
    assert!(read_bundle(&archive).is_err());
}

#[test]
fn rejects_checksum_mismatch() {
    let files = tree();
    let mut archive = write_bundle_archive(&files).unwrap();
    let position = archive
        .windows(b"# Skill\n".len())
        .position(|window| window == b"# Skill\n")
        .unwrap();
    archive[position] ^= 0xFF;
    let result = read_bundle(&archive);
    assert!(result.is_err());
}

#[test]
fn rejects_truncation_without_panicking() {
    let files = tree();
    let archive = write_bundle_archive(&files).unwrap();
    let cut = archive.len() * 3 / 4;
    assert!(read_bundle(&archive[..cut]).is_err());
}

#[test]
fn unpack_writes_staging_files() {
    let files = tree();
    let archive = write_bundle_archive(&files).unwrap();
    let staging = fresh_staging();
    let inventory = unpack_bundle_to_staging(&archive, &staging).unwrap();
    assert_eq!(inventory.file_count(), 5);
    for (path, content) in &files {
        let mut disk = staging.clone();
        for component in path.as_str().split('/') {
            disk.push(component);
        }
        assert_eq!(std::fs::read(&disk).unwrap(), *content);
    }
    std::fs::remove_dir_all(&staging).unwrap();
}

#[test]
fn unpack_cleans_staging_on_corrupt_archive() {
    let files = tree();
    let mut archive = write_bundle_archive(&files).unwrap();
    archive.truncate(archive.len() / 2);
    let staging = fresh_staging();
    assert!(unpack_bundle_to_staging(&archive, &staging).is_err());
    assert!(!staging.exists());
}

#[test]
fn unpack_refuses_existing_staging() {
    let files = tree();
    let archive = write_bundle_archive(&files).unwrap();
    let staging = fresh_staging();
    std::fs::create_dir_all(&staging).unwrap();
    std::fs::write(staging.join("sentinel.txt"), b"keep").unwrap();
    assert!(unpack_bundle_to_staging(&archive, &staging).is_err());
    assert_eq!(
        std::fs::read(staging.join("sentinel.txt")).unwrap(),
        b"keep"
    );
    std::fs::remove_dir_all(&staging).unwrap();
}
