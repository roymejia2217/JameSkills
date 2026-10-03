use jameskills_core::domain::{
    ContentHash, RevisionId, RevisionKind, RevisionRecord, SkillId, compute_revision,
};
use sha2::{Digest, Sha256};

const SKILL: &str = "f9c0199f-c4ce-4b04-85dd-ae12a7db292b";
const BUNDLE: &str = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";
const PARENT_A: &str = "3a6eb07960d8d770613c1e5e6d3d9d3f4a3f3e5e5e5e5e5e5e5e5e5e5e5e5e5e";
const PARENT_B: &str = "7d8653fd7c4d3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b3b";

fn skill() -> SkillId {
    SkillId::parse(SKILL).unwrap()
}

fn hash(value: &str) -> ContentHash {
    ContentHash::parse_hex(value).unwrap()
}

fn revision(value: &str) -> RevisionId {
    RevisionId::parse_hex(value).unwrap()
}

/// Independent hex decoder so the golden below does not reuse the unit's parser.
fn decode_hex(value: &str) -> [u8; 32] {
    assert_eq!(value.len(), 64);
    let mut out = [0u8; 32];
    for (index, chunk) in value.as_bytes().chunks(2).enumerate() {
        out[index] = u8::from_str_radix(std::str::from_utf8(chunk).unwrap(), 16).unwrap();
    }
    out
}

#[test]
fn content_revision_matches_hand_built_spec_digest() {
    let computed = compute_revision(
        skill(),
        &RevisionKind::Content,
        Some(&hash(BUNDLE)),
        "0.1.0",
        &[],
    )
    .unwrap();
    let mut expected = Sha256::new();
    expected.update(b"JAMESKILLS-REVISION-V1\0");
    expected.update(skill().as_uuid().into_bytes());
    expected.update([0x01]);
    expected.update(decode_hex(BUNDLE));
    expected.update(5u32.to_be_bytes());
    expected.update(b"0.1.0");
    expected.update(0u32.to_be_bytes());
    let digest: [u8; 32] = expected.finalize().into();
    assert_eq!(computed, RevisionId::from_digest(digest));
}

#[test]
fn parent_order_does_not_change_revision() {
    let parents = [revision(PARENT_A), revision(PARENT_B)];
    let forward = compute_revision(
        skill(),
        &RevisionKind::Content,
        Some(&hash(BUNDLE)),
        "0.1.0",
        &parents,
    )
    .unwrap();
    let reversed = compute_revision(
        skill(),
        &RevisionKind::Content,
        Some(&hash(BUNDLE)),
        "0.1.0",
        &[revision(PARENT_B), revision(PARENT_A)],
    )
    .unwrap();
    assert_eq!(forward, reversed);
}

#[test]
fn duplicate_parents_are_deduplicated_before_hashing() {
    let single = compute_revision(
        skill(),
        &RevisionKind::Content,
        Some(&hash(BUNDLE)),
        "0.1.0",
        &[revision(PARENT_A)],
    )
    .unwrap();
    let duplicated = compute_revision(
        skill(),
        &RevisionKind::Content,
        Some(&hash(BUNDLE)),
        "0.1.0",
        &[revision(PARENT_A), revision(PARENT_A)],
    )
    .unwrap();
    assert_eq!(single, duplicated);
}

#[test]
fn tombstone_ignores_bundle_hash_and_differs_from_content() {
    let tombstone_some = compute_revision(
        skill(),
        &RevisionKind::Tombstone {
            observed_heads: vec![revision(PARENT_A)],
        },
        Some(&hash(BUNDLE)),
        "0.1.0",
        &[],
    )
    .unwrap();
    let tombstone_none = compute_revision(
        skill(),
        &RevisionKind::Tombstone {
            observed_heads: vec![revision(PARENT_A)],
        },
        None,
        "0.1.0",
        &[],
    )
    .unwrap();
    assert_eq!(tombstone_some, tombstone_none);
    let content = compute_revision(
        skill(),
        &RevisionKind::Content,
        Some(&hash(BUNDLE)),
        "0.1.0",
        &[],
    )
    .unwrap();
    assert_ne!(tombstone_none, content);
}

#[test]
fn tombstone_observed_heads_are_ordered_and_significant() {
    let heads = || RevisionKind::Tombstone {
        observed_heads: vec![revision(PARENT_A), revision(PARENT_B)],
    };
    let forward = compute_revision(skill(), &heads(), None, "0.1.0", &[]).unwrap();
    let swapped = compute_revision(
        skill(),
        &RevisionKind::Tombstone {
            observed_heads: vec![revision(PARENT_B), revision(PARENT_A)],
        },
        None,
        "0.1.0",
        &[],
    )
    .unwrap();
    assert_eq!(forward, swapped);
    let fewer = compute_revision(
        skill(),
        &RevisionKind::Tombstone {
            observed_heads: vec![revision(PARENT_A)],
        },
        None,
        "0.1.0",
        &[],
    )
    .unwrap();
    assert_ne!(forward, fewer);
}

#[test]
fn created_at_never_reaches_the_digest() {
    let first = compute_revision(
        skill(),
        &RevisionKind::Content,
        Some(&hash(BUNDLE)),
        "0.1.0",
        &[],
    )
    .unwrap();
    let second = compute_revision(
        skill(),
        &RevisionKind::Content,
        Some(&hash(BUNDLE)),
        "0.1.0",
        &[],
    )
    .unwrap();
    assert_eq!(first, second);
    assert_eq!(first.as_str().len(), 64);
}

#[test]
fn rejects_invalid_semantic_version() {
    let result = compute_revision(
        skill(),
        &RevisionKind::Content,
        Some(&hash(BUNDLE)),
        "not-a-version",
        &[],
    );
    assert!(matches!(
        result,
        Err(jameskills_core::AppError::Validation(_))
    ));
}

#[test]
fn revision_record_carries_sorted_unique_parents() {
    let record = RevisionRecord::new(
        skill(),
        Some(hash(BUNDLE)),
        vec![revision(PARENT_B), revision(PARENT_A), revision(PARENT_A)],
        RevisionKind::Content,
        "0.1.0".to_owned(),
    )
    .unwrap();
    assert_eq!(record.parents(), &[revision(PARENT_A), revision(PARENT_B)]);
    assert_eq!(record.skill_id(), skill());
    assert_eq!(record.bundle_hash().as_str(), BUNDLE);
    assert_eq!(record.semantic_version(), "0.1.0");
}
