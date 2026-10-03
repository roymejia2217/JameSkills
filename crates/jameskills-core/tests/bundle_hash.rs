mod support;

use jameskills_core::domain::{canonical_inventory, hash_bundle};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

const SKILL_MD: &[u8] = b"# Skill\n";
const GUIDE_MD: &[u8] = b"# Guide\n";

fn standard_files() -> Vec<(&'static str, &'static [u8])> {
    vec![("SKILL.md", SKILL_MD), ("docs/guide.md", GUIDE_MD)]
}

fn hand_built_digest(files: &[(&str, &[u8])]) -> String {
    let mut ordered: Vec<(&str, &[u8])> = files.to_vec();
    ordered.sort_by(|left, right| left.0.as_bytes().cmp(right.0.as_bytes()));
    let mut digest = Sha256::new();
    digest.update(b"JAMESKILLS-BUNDLE-V1\0");
    for (path, content) in ordered {
        digest.update((path.len() as u32).to_be_bytes());
        digest.update(path.as_bytes());
        digest.update((content.len() as u64).to_be_bytes());
        digest.update(content);
    }
    let bytes: [u8; 32] = digest.finalize().into();
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn same_content_in_different_entry_order_hashes_equal() {
    let (first_inventory, first_bytes) = support::fixture_bundle(&standard_files());
    let (second_inventory, second_bytes) =
        support::fixture_bundle(&[("docs/guide.md", GUIDE_MD), ("SKILL.md", SKILL_MD)]);
    let first = hash_bundle(&first_inventory, &first_bytes).unwrap();
    let second = hash_bundle(&second_inventory, &second_bytes).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.as_str(), hand_built_digest(&standard_files()));
}

#[test]
fn canonical_inventory_follows_utf8_byte_order() {
    let (inventory, _) = support::fixture_bundle(&[
        ("SKILL.md", SKILL_MD),
        ("cherry.md", b"x"),
        ("Banana.md", b"x"),
        ("apple.md", b"x"),
    ]);
    let order: Vec<String> = canonical_inventory(&inventory)
        .iter()
        .map(|path| path.as_str().to_owned())
        .collect();
    assert_eq!(
        order,
        vec!["Banana.md", "SKILL.md", "apple.md", "cherry.md"]
    );
}

#[test]
fn changed_bytes_change_the_hash() {
    let (inventory, _) = support::fixture_bundle(&standard_files());
    let mut altered = BTreeMap::new();
    altered.insert(
        jameskills_core::domain::PortablePath::new("SKILL.md".to_owned()).unwrap(),
        b"# Changed\n".to_vec(),
    );
    altered.insert(
        jameskills_core::domain::PortablePath::new("docs/guide.md".to_owned()).unwrap(),
        GUIDE_MD.to_vec(),
    );
    let original = hash_bundle(&inventory, &support::fixture_bundle(&standard_files()).1).unwrap();
    let changed = hash_bundle(&inventory, &altered).unwrap();
    assert_ne!(original, changed);
}

#[test]
fn crlf_bytes_hash_different_from_lf() {
    let (lf_inventory, lf_bytes) =
        support::fixture_bundle(&[("SKILL.md", b"line\n"), ("docs/guide.md", GUIDE_MD)]);
    let (crlf_inventory, crlf_bytes) =
        support::fixture_bundle(&[("SKILL.md", b"line\r\n"), ("docs/guide.md", GUIDE_MD)]);
    assert_ne!(
        hash_bundle(&lf_inventory, &lf_bytes).unwrap(),
        hash_bundle(&crlf_inventory, &crlf_bytes).unwrap()
    );
}

#[test]
fn renamed_path_changes_the_hash() {
    let (inventory, _) = support::fixture_bundle(&standard_files());
    let (renamed_inventory, renamed_bytes) =
        support::fixture_bundle(&[("SKILL.md", SKILL_MD), ("docs/manual.md", GUIDE_MD)]);
    assert_ne!(
        hash_bundle(&inventory, &support::fixture_bundle(&standard_files()).1).unwrap(),
        hash_bundle(&renamed_inventory, &renamed_bytes).unwrap()
    );
}

#[test]
fn entry_size_metadata_does_not_reach_the_digest() {
    let (inventory, bytes) = support::fixture_bundle(&standard_files());
    let inflated = vec![
        jameskills_core::domain::BundleEntry::new(
            jameskills_core::domain::PortablePath::new("SKILL.md".to_owned()).unwrap(),
            jameskills_core::domain::EntryKind::RegularFile,
            9999,
            9999,
        ),
        jameskills_core::domain::BundleEntry::new(
            jameskills_core::domain::PortablePath::new("docs/guide.md".to_owned()).unwrap(),
            jameskills_core::domain::EntryKind::RegularFile,
            9999,
            9999,
        ),
    ];
    let inflated_inventory = jameskills_core::domain::validate_bundle_inventory(&inflated).unwrap();
    assert_eq!(
        hash_bundle(&inventory, &bytes).unwrap(),
        hash_bundle(&inflated_inventory, &bytes).unwrap()
    );
}

#[test]
fn missing_bytes_are_rejected() {
    let (inventory, mut bytes) = support::fixture_bundle(&standard_files());
    let guide = jameskills_core::domain::PortablePath::new("docs/guide.md".to_owned()).unwrap();
    bytes.remove(&guide);
    let result = hash_bundle(&inventory, &bytes);
    assert!(matches!(
        result,
        Err(jameskills_core::AppError::Validation(_))
    ));
}
