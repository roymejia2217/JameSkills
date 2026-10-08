use jameskills_core::{
    AppError,
    domain::{
        CANONICAL_HASH_VERSION, ContentHash, CreateSkill, DeviceId, RevisionKind, RevisionRecord,
        SNAPSHOT_SCHEMA_VERSION, SnapshotBundle, SnapshotHead, SnapshotId, SnapshotPayload,
        SnapshotRevision, VaultId, validate_bundle,
    },
    ports::write_bundle_archive,
};

fn content_snapshot() -> SnapshotPayload {
    let draft = CreateSkill::new("snapshot-demo".to_owned(), "Snapshot demo".to_owned())
        .unwrap()
        .initial_draft()
        .unwrap();
    let validated = validate_bundle(draft.files()).unwrap();
    let record = RevisionRecord::new(
        draft.skill_id(),
        Some(validated.content_hash().clone()),
        Vec::new(),
        RevisionKind::Content,
        "0.1.0".to_owned(),
    )
    .unwrap();
    SnapshotPayload::new(
        VaultId::new(),
        SnapshotId::new(),
        Vec::new(),
        DeviceId::new(),
        7,
        "2026-10-08T00:00:00Z".to_owned(),
        SNAPSHOT_SCHEMA_VERSION,
        CANONICAL_HASH_VERSION,
        vec![SnapshotRevision::from_record(&record)],
        vec![SnapshotHead::new(draft.skill_id(), vec![record.id().clone()]).unwrap()],
        vec![
            SnapshotBundle::new(
                validated.content_hash().clone(),
                write_bundle_archive(draft.files()).unwrap(),
            )
            .unwrap(),
        ],
    )
    .unwrap()
}

#[test]
fn snapshot_verification_checks_archive_manifest_hash_revision_and_heads() {
    let snapshot = content_snapshot();
    let verified = snapshot.verify().unwrap();
    assert_eq!(verified.payload().revisions().len(), 1);
    assert_eq!(verified.payload().heads().len(), 1);
    assert_eq!(verified.payload().bundles().len(), 1);
}

#[test]
fn snapshot_verification_rejects_missing_parent_and_incomplete_heads() {
    let valid = content_snapshot();
    let revision = valid.revisions()[0].clone();
    let skill_id = revision.skill_id();
    // Rebuild a correctly hashed revision that names an unavailable ancestor.
    let unavailable_parent = jameskills_core::domain::RevisionId::from_digest([91; 32]);
    let missing_parent_record = RevisionRecord::new(
        skill_id,
        revision.bundle_hash().cloned(),
        vec![unavailable_parent],
        RevisionKind::Content,
        revision.semantic_version().to_owned(),
    )
    .unwrap();
    let missing_parent = SnapshotPayload::new(
        valid.vault_id(),
        SnapshotId::new(),
        Vec::new(),
        DeviceId::new(),
        8,
        "2026-10-08T00:00:00Z".to_owned(),
        SNAPSHOT_SCHEMA_VERSION,
        CANONICAL_HASH_VERSION,
        vec![SnapshotRevision::from_record(&missing_parent_record)],
        vec![SnapshotHead::new(skill_id, vec![missing_parent_record.id().clone()]).unwrap()],
        valid
            .bundles()
            .iter()
            .map(|bundle| {
                SnapshotBundle::new(
                    bundle.content_hash().clone(),
                    bundle.archive_bytes().to_vec(),
                )
                .unwrap()
            })
            .collect(),
    )
    .unwrap();
    assert!(matches!(
        missing_parent.verify(),
        Err(AppError::Validation(_))
    ));

    let no_heads = SnapshotPayload::new(
        valid.vault_id(),
        SnapshotId::new(),
        Vec::new(),
        DeviceId::new(),
        8,
        "2026-10-08T00:00:00Z".to_owned(),
        SNAPSHOT_SCHEMA_VERSION,
        CANONICAL_HASH_VERSION,
        valid.revisions().to_vec(),
        Vec::new(),
        valid
            .bundles()
            .iter()
            .map(|bundle| {
                SnapshotBundle::new(
                    bundle.content_hash().clone(),
                    bundle.archive_bytes().to_vec(),
                )
                .unwrap()
            })
            .collect(),
    )
    .unwrap();
    assert!(matches!(no_heads.verify(), Err(AppError::Validation(_))));
}

#[test]
fn snapshot_verification_rejects_archive_hash_mismatch() {
    let valid = content_snapshot();
    let bundle = &valid.bundles()[0];
    let mismatched_bundle = SnapshotBundle::new(
        ContentHash::from_digest([99; 32]),
        bundle.archive_bytes().to_vec(),
    )
    .unwrap();
    let snapshot = SnapshotPayload::new(
        valid.vault_id(),
        SnapshotId::new(),
        Vec::new(),
        DeviceId::new(),
        8,
        "2026-10-08T00:00:00Z".to_owned(),
        SNAPSHOT_SCHEMA_VERSION,
        CANONICAL_HASH_VERSION,
        valid.revisions().to_vec(),
        valid.heads().to_vec(),
        vec![mismatched_bundle],
    )
    .unwrap();

    assert!(matches!(snapshot.verify(), Err(AppError::Validation(_))));
}
