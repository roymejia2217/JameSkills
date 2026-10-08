use jameskills_core::domain::{
    CANONICAL_HASH_VERSION, ContentHash, DeviceId, MAX_SNAPSHOT_BUNDLE_BYTES, RevisionId,
    RevisionKind, RevisionRecord, SNAPSHOT_SCHEMA_VERSION, SkillId, SnapshotBundle, SnapshotHead,
    SnapshotId, SnapshotPayload, SnapshotRevision, VaultId,
};

fn revision(seed: u8, skill_id: SkillId) -> RevisionRecord {
    RevisionRecord::new(
        skill_id,
        Some(ContentHash::from_digest([seed; 32])),
        Vec::new(),
        RevisionKind::Content,
        format!("1.0.{seed}"),
    )
    .unwrap()
}

#[test]
fn snapshot_payload_bounds_and_canonically_orders_metadata() {
    let skill_id = SkillId::new();
    let first_record = revision(1, skill_id);
    let second_record = revision(2, skill_id);
    let first_id = first_record.id().clone();
    let second_id = second_record.id().clone();
    let first_archive_hash = ContentHash::from_digest([31; 32]);
    let second_archive_hash = ContentHash::from_digest([32; 32]);

    let payload = SnapshotPayload::new(
        VaultId::new(),
        SnapshotId::new(),
        vec![SnapshotId::new(), SnapshotId::new()],
        DeviceId::new(),
        9,
        "2026-10-08T00:00:00Z".to_owned(),
        SNAPSHOT_SCHEMA_VERSION,
        CANONICAL_HASH_VERSION,
        vec![
            SnapshotRevision::from_record(&second_record),
            SnapshotRevision::from_record(&first_record),
        ],
        vec![SnapshotHead::new(skill_id, vec![second_id.clone(), first_id.clone()]).unwrap()],
        vec![
            SnapshotBundle::new(second_archive_hash.clone(), vec![2, 3]).unwrap(),
            SnapshotBundle::new(first_archive_hash.clone(), vec![1, 2]).unwrap(),
        ],
    )
    .unwrap();

    assert!(payload.revisions()[0].revision_id() < payload.revisions()[1].revision_id());
    assert!(payload.heads()[0].revision_ids()[0] < payload.heads()[0].revision_ids()[1]);
    assert!(payload.bundles()[0].content_hash() < payload.bundles()[1].content_hash());
    assert_eq!(payload.library_generation(), 9);
    assert_eq!(payload.bundles()[0].archive_bytes(), &[1, 2]);
}

#[test]
fn snapshot_bundle_rejects_empty_or_oversized_archives() {
    let hash = ContentHash::from_digest([41; 32]);
    assert!(SnapshotBundle::new(hash.clone(), Vec::new()).is_err());
    assert!(SnapshotBundle::new(hash, vec![0; MAX_SNAPSHOT_BUNDLE_BYTES + 1]).is_err());
}

#[test]
fn snapshot_payload_rejects_unknown_schema_duplicate_ids_and_metadata_overflow() {
    let skill_id = SkillId::new();
    let record = revision(3, skill_id);
    let content = SnapshotRevision::from_record(&record);
    let build = |schema_version, revisions| {
        SnapshotPayload::new(
            VaultId::new(),
            SnapshotId::new(),
            Vec::new(),
            DeviceId::new(),
            0,
            "display timestamp".to_owned(),
            schema_version,
            CANONICAL_HASH_VERSION,
            revisions,
            vec![SnapshotHead::new(skill_id, vec![record.id().clone()]).unwrap()],
            Vec::new(),
        )
    };

    assert!(build(SNAPSHOT_SCHEMA_VERSION + 1, vec![content.clone()]).is_err());
    assert!(
        build(
            SNAPSHOT_SCHEMA_VERSION,
            vec![content.clone(), content.clone()]
        )
        .is_err()
    );
    let too_many_revisions = (0..=jameskills_core::domain::MAX_SNAPSHOT_METADATA_ENTRIES)
        .map(|_| SnapshotRevision::from_record(&record))
        .collect::<Vec<_>>();
    assert!(build(SNAPSHOT_SCHEMA_VERSION, too_many_revisions).is_err());

    let head_one = SnapshotHead::new(skill_id, vec![record.id().clone()]).unwrap();
    let head_two = SnapshotHead::new(skill_id, vec![RevisionId::from_digest([42; 32])]).unwrap();
    assert!(
        SnapshotPayload::new(
            VaultId::new(),
            SnapshotId::new(),
            Vec::new(),
            DeviceId::new(),
            0,
            "timestamp".to_owned(),
            SNAPSHOT_SCHEMA_VERSION,
            CANONICAL_HASH_VERSION,
            vec![SnapshotRevision::from_record(&record)],
            vec![head_one, head_two],
            Vec::new(),
        )
        .is_err()
    );

    let repeated_parent = SnapshotId::new();
    assert!(
        SnapshotPayload::new(
            VaultId::new(),
            SnapshotId::new(),
            vec![repeated_parent, repeated_parent],
            DeviceId::new(),
            0,
            "timestamp".to_owned(),
            SNAPSHOT_SCHEMA_VERSION,
            CANONICAL_HASH_VERSION,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .is_err()
    );
}
