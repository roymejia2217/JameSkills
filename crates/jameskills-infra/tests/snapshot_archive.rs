use jameskills_core::{
    domain::{
        CANONICAL_HASH_VERSION, CreateSkill, DeviceId, RevisionKind, RevisionRecord,
        SNAPSHOT_SCHEMA_VERSION, SnapshotBundle, SnapshotHead, SnapshotId, SnapshotPayload,
        SnapshotRevision, VaultId, validate_bundle,
    },
    ports::write_bundle_archive,
};
use jameskills_infra::snapshot_archive::{decode, encode};
use std::io::Cursor;
use zip::{CompressionMethod, DateTime, ZipWriter, write::SimpleFileOptions};

fn payload() -> SnapshotPayload {
    let draft = CreateSkill::new(
        "portable-snapshot".to_owned(),
        "Portable snapshot".to_owned(),
    )
    .unwrap()
    .initial_draft()
    .unwrap();
    let validated = validate_bundle(draft.files()).unwrap();
    let archive = write_bundle_archive(draft.files()).unwrap();
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
        4,
        "2026-10-08T00:00:00Z".to_owned(),
        SNAPSHOT_SCHEMA_VERSION,
        CANONICAL_HASH_VERSION,
        vec![SnapshotRevision::from_record(&record)],
        vec![SnapshotHead::new(draft.skill_id(), vec![record.id().clone()]).unwrap()],
        vec![SnapshotBundle::new(validated.content_hash().clone(), archive).unwrap()],
    )
    .unwrap()
}

#[test]
fn stored_snapshot_zip_roundtrips_metadata_and_nested_portable_bundle() {
    let payload = payload();
    let expected_snapshot = payload.snapshot_id();
    let expected_skill = payload.revisions()[0].skill_id();
    let bytes = encode(&payload).unwrap();
    let decoded = decode(&bytes).unwrap();
    let verified = decoded.verify().unwrap();

    assert_eq!(verified.payload().snapshot_id(), expected_snapshot);
    assert_eq!(verified.payload().revisions()[0].skill_id(), expected_skill);
    assert_eq!(
        verified.payload().bundles()[0].content_hash(),
        payload.bundles()[0].content_hash()
    );
}

#[test]
fn snapshot_zip_rejects_truncated_or_crc_corrupted_archives() {
    let bytes = encode(&payload()).unwrap();
    assert!(decode(&bytes[..bytes.len() - 1]).is_err());
    let mut corrupt = bytes;
    corrupt[43] ^= 1;
    assert!(decode(&corrupt).is_err());
}

#[test]
fn snapshot_zip_rejects_traversal_paths() {
    let make_archive = |entries: &[(&str, &[u8])]| {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .last_modified_time(DateTime::DEFAULT);
        for (name, contents) in entries {
            writer.start_file(*name, options).unwrap();
            std::io::Write::write_all(&mut writer, contents).unwrap();
        }
        writer.finish().unwrap().into_inner()
    };

    assert!(decode(&make_archive(&[("../escape", b"x")])).is_err());
}
