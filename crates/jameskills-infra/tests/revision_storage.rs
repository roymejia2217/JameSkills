use jameskills_core::domain::{
    ContentHash, RevisionKind, SaveRevisionRequest, SkillId, hash_bundle,
};
use jameskills_core::ports::write_bundle_archive;
use jameskills_infra::fs::{blob_path, read_bundle, store_blob_bytes, verify_blob_bytes};
use jameskills_infra::sqlite::SqliteStore;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static CASE_COUNTER: AtomicU64 = AtomicU64::new(0);

const SKILL_ID: &str = "f9c0199f-c4ce-4b04-85dd-ae12a7db292b";

struct Case {
    _root: PathBuf,
    db: PathBuf,
    blobs: PathBuf,
    store: SqliteStore,
}

fn portable(name: &str) -> jameskills_core::domain::PortablePath {
    jameskills_core::domain::PortablePath::new(name.to_owned()).unwrap()
}

fn setup_case() -> Case {
    let id = CASE_COUNTER.fetch_add(1, Ordering::SeqCst);
    let root =
        std::env::temp_dir().join(format!("jameskills-revision-{}-{id}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let db = root.join("library.sqlite3");
    let blobs = root.join("blobs");
    let store = SqliteStore::open(&db).unwrap();
    store
        .with_transaction(|transaction| {
            transaction
                .execute(
                    "INSERT INTO skills(id, slug, display_name, created_at) VALUES (?1, ?2, ?3, ?4)",
                    (
                        SKILL_ID,
                        "demo",
                        "Demo",
                        "2026-01-01T00:00:00Z",
                    ),
                )
                .map_err(|_| storage_marker("test.skill.failed"))?;
            Ok(())
        })
        .unwrap();
    Case {
        _root: root,
        db,
        blobs,
        store,
    }
}

fn storage_marker(code: &str) -> jameskills_core::AppError {
    jameskills_core::AppError::Storage {
        code: code.to_owned(),
    }
}

fn files_v1() -> BTreeMap<jameskills_core::domain::PortablePath, Vec<u8>> {
    let mut files = BTreeMap::new();
    files.insert(portable("SKILL.md"), b"# Skill\n".to_vec());
    files.insert(portable("docs/guide.md"), b"# Guide\n".to_vec());
    files
}

fn files_v2() -> BTreeMap<jameskills_core::domain::PortablePath, Vec<u8>> {
    let mut files = files_v1();
    files.insert(portable("docs/guide.md"), b"# Guide v2\n".to_vec());
    files
}

fn bundle_hash_of(files: &BTreeMap<jameskills_core::domain::PortablePath, Vec<u8>>) -> ContentHash {
    let archive = write_bundle_archive(files).unwrap();
    let (inventory, recovered) = read_bundle(&archive).unwrap();
    assert_eq!(&recovered, files);
    hash_bundle(&inventory, &recovered).unwrap()
}

fn stage(
    case: &Case,
    files: &BTreeMap<jameskills_core::domain::PortablePath, Vec<u8>>,
) -> (Vec<u8>, ContentHash) {
    let archive = write_bundle_archive(files).unwrap();
    let hash = bundle_hash_of(files);
    store_blob_bytes(&case.blobs, &hash, &archive).unwrap();
    (archive, hash)
}

fn request(
    bundle_hash: Option<ContentHash>,
    parents: Vec<jameskills_core::domain::RevisionId>,
    kind: RevisionKind,
    version: &str,
    expected: Vec<jameskills_core::domain::RevisionId>,
) -> SaveRevisionRequest {
    SaveRevisionRequest::new(
        SkillId::parse(SKILL_ID).unwrap(),
        bundle_hash,
        parents,
        kind,
        version.to_owned(),
        1,
        expected,
    )
}

fn heads(case: &Case) -> Vec<String> {
    let connection = rusqlite::Connection::open(&case.db).unwrap();
    let mut statement = connection
        .prepare("SELECT revision_id FROM skill_heads WHERE skill_id = ?1 ORDER BY revision_id")
        .unwrap();
    statement
        .query_map([SKILL_ID], |row| row.get(0))
        .unwrap()
        .map(|id| id.unwrap())
        .collect()
}

#[test]
fn commit_content_revision_sets_head_and_addressable_blob() {
    let case = setup_case();
    let files = files_v1();
    let (_archive, hash) = stage(&case, &files);
    let result = case
        .store
        .commit_revision(&request(
            Some(hash.clone()),
            vec![],
            RevisionKind::Content,
            "0.1.0",
            vec![],
        ))
        .unwrap();
    assert_eq!(result.new_heads(), &[result.revision().id().clone()]);
    assert_eq!(
        heads(&case),
        vec![result.revision().id().as_str().to_owned()]
    );
    assert_eq!(result.revision().bundle_hash(), &hash);
    let expected_path =
        case.blobs
            .join(format!("{}/{}.bundle", &hash.as_str()[..2], hash.as_str()));
    assert_eq!(blob_path(&case.blobs, &hash), expected_path);
    assert!(expected_path.is_file());
    assert_eq!(verify_blob_bytes(&case.blobs, &hash).unwrap(), files);
}

#[test]
fn second_commit_advances_head_with_expected_parents() {
    let case = setup_case();
    let (_archive, first_hash) = stage(&case, &files_v1());
    let first = case
        .store
        .commit_revision(&request(
            Some(first_hash),
            vec![],
            RevisionKind::Content,
            "0.1.0",
            vec![],
        ))
        .unwrap();
    let (_archive, second_hash) = stage(&case, &files_v2());
    let second = case
        .store
        .commit_revision(&request(
            Some(second_hash),
            vec![first.revision().id().clone()],
            RevisionKind::Content,
            "0.2.0",
            vec![first.revision().id().clone()],
        ))
        .unwrap();
    assert_ne!(first.revision().id(), second.revision().id());
    assert_eq!(
        heads(&case),
        vec![second.revision().id().as_str().to_owned()]
    );
    let parent: String = rusqlite::Connection::open(&case.db)
        .unwrap()
        .query_row(
            "SELECT parent_revision_id FROM revision_parents WHERE revision_id = ?1",
            [second.revision().id().as_str()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(parent, first.revision().id().as_str().to_owned());
}

#[test]
fn stale_expected_heads_conflict_and_keep_previous_head() {
    let case = setup_case();
    let (_archive, first_hash) = stage(&case, &files_v1());
    let first = case
        .store
        .commit_revision(&request(
            Some(first_hash),
            vec![],
            RevisionKind::Content,
            "0.1.0",
            vec![],
        ))
        .unwrap();
    let (_archive, second_hash) = stage(&case, &files_v2());
    let conflict = case.store.commit_revision(&request(
        Some(second_hash),
        vec![],
        RevisionKind::Content,
        "0.2.0",
        vec![],
    ));
    assert!(matches!(
        conflict,
        Err(jameskills_core::AppError::Conflict { .. })
    ));
    assert_eq!(
        heads(&case),
        vec![first.revision().id().as_str().to_owned()]
    );
}

#[test]
fn duplicate_content_commit_fails_without_moving_head() {
    let case = setup_case();
    let (_archive, hash) = stage(&case, &files_v1());
    let first = case
        .store
        .commit_revision(&request(
            Some(hash.clone()),
            vec![],
            RevisionKind::Content,
            "0.1.0",
            vec![],
        ))
        .unwrap();
    let again = case.store.commit_revision(&request(
        Some(hash),
        vec![],
        RevisionKind::Content,
        "0.1.0",
        vec![first.revision().id().clone()],
    ));
    assert!(again.is_err());
    assert_eq!(
        heads(&case),
        vec![first.revision().id().as_str().to_owned()]
    );
}

#[test]
fn corrupt_blob_fails_verify_while_heads_stay_intact() {
    let case = setup_case();
    let files = files_v1();
    let (_archive, hash) = stage(&case, &files);
    let result = case
        .store
        .commit_revision(&request(
            Some(hash.clone()),
            vec![],
            RevisionKind::Content,
            "0.1.0",
            vec![],
        ))
        .unwrap();
    let path = blob_path(&case.blobs, &hash);
    let mut bytes = std::fs::read(&path).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0xFF;
    std::fs::write(&path, &bytes).unwrap();
    assert!(verify_blob_bytes(&case.blobs, &hash).is_err());
    assert_eq!(
        heads(&case),
        vec![result.revision().id().as_str().to_owned()]
    );
}

#[test]
fn staged_blob_without_commit_leaves_no_head() {
    let case = setup_case();
    let (_archive, hash) = stage(&case, &files_v1());
    assert!(heads(&case).is_empty());
    assert!(blob_path(&case.blobs, &hash).is_file());
}

#[test]
fn store_rejects_bytes_that_do_not_match_the_addressed_hash() {
    let case = setup_case();
    let archive = write_bundle_archive(&files_v2()).unwrap();
    let expected = bundle_hash_of(&files_v1());

    assert!(store_blob_bytes(&case.blobs, &expected, &archive).is_err());
    assert!(!blob_path(&case.blobs, &expected).exists());
}

#[test]
fn orphaned_blobs_are_listed_and_preserved_across_reopen() {
    let case = setup_case();
    let (_archive, hash) = stage(&case, &files_v1());

    assert_eq!(case.store.orphan_blob_hashes().unwrap(), vec![hash.clone()]);
    drop(case.store);
    let reopened = SqliteStore::open(&case.db).unwrap();
    assert_eq!(reopened.orphan_blob_hashes().unwrap(), vec![hash.clone()]);
    assert!(blob_path(&case.blobs, &hash).is_file());
    assert!(reopened.check_integrity().is_ok());
}

#[test]
fn reopening_rejects_a_missing_blob_referenced_by_a_revision() {
    let case = setup_case();
    let (_archive, hash) = stage(&case, &files_v1());
    case.store
        .commit_revision(&request(
            Some(hash.clone()),
            vec![],
            RevisionKind::Content,
            "0.1.0",
            vec![],
        ))
        .unwrap();
    std::fs::remove_file(blob_path(&case.blobs, &hash)).unwrap();
    let db = case.db.clone();
    drop(case.store);

    assert!(SqliteStore::open(&db).is_err());
}

#[test]
fn reopening_rejects_a_corrupt_blob_referenced_by_a_revision() {
    let case = setup_case();
    let (_archive, hash) = stage(&case, &files_v1());
    case.store
        .commit_revision(&request(
            Some(hash.clone()),
            vec![],
            RevisionKind::Content,
            "0.1.0",
            vec![],
        ))
        .unwrap();
    let path = blob_path(&case.blobs, &hash);
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[0] ^= 0x01;
    std::fs::write(path, bytes).unwrap();
    let db = case.db.clone();
    drop(case.store);

    assert!(SqliteStore::open(&db).is_err());
}

#[test]
fn commit_requires_parents_to_cover_every_current_head() {
    let case = setup_case();
    let (_archive, first_hash) = stage(&case, &files_v1());
    let first = case
        .store
        .commit_revision(&request(
            Some(first_hash),
            vec![],
            RevisionKind::Content,
            "0.1.0",
            vec![],
        ))
        .unwrap();
    let (_archive, second_hash) = stage(&case, &files_v2());

    let result = case.store.commit_revision(&request(
        Some(second_hash),
        vec![],
        RevisionKind::Content,
        "0.2.0",
        vec![first.revision().id().clone()],
    ));

    assert!(result.is_err());
    assert_eq!(
        heads(&case),
        vec![first.revision().id().as_str().to_owned()]
    );
}

#[test]
fn failed_revision_transaction_preserves_previous_head_and_orphan_blob() {
    let case = setup_case();
    let (_archive, first_hash) = stage(&case, &files_v1());
    let first = case
        .store
        .commit_revision(&request(
            Some(first_hash),
            vec![],
            RevisionKind::Content,
            "0.1.0",
            vec![],
        ))
        .unwrap();
    let (_archive, second_hash) = stage(&case, &files_v2());
    case.store
        .with_transaction(|transaction| {
            transaction
                .execute_batch(
                    "CREATE TRIGGER fail_head_insert BEFORE INSERT ON skill_heads BEGIN SELECT RAISE(ABORT, 'injected failure'); END;",
                )
                .map_err(|_| storage_marker("test.trigger.failed"))?;
            Ok(())
        })
        .unwrap();

    let retry = request(
        Some(second_hash.clone()),
        vec![first.revision().id().clone()],
        RevisionKind::Content,
        "0.2.0",
        vec![first.revision().id().clone()],
    );
    let result = case.store.commit_revision(&retry);

    assert!(result.is_err());
    assert_eq!(
        heads(&case),
        vec![first.revision().id().as_str().to_owned()]
    );
    let count: u32 = rusqlite::Connection::open(&case.db)
        .unwrap()
        .query_row("SELECT COUNT(*) FROM revisions", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1);
    assert!(blob_path(&case.blobs, &second_hash).is_file());
    assert_eq!(
        case.store.orphan_blob_hashes().unwrap(),
        vec![second_hash.clone()]
    );
    case.store
        .with_transaction(|transaction| {
            transaction
                .execute_batch("DROP TRIGGER fail_head_insert")
                .map_err(|_| storage_marker("test.trigger.drop_failed"))?;
            Ok(())
        })
        .unwrap();
    let retried = case.store.commit_revision(&retry).unwrap();
    assert_eq!(
        heads(&case),
        vec![retried.revision().id().as_str().to_owned()]
    );
    assert!(case.store.orphan_blob_hashes().unwrap().is_empty());
}

#[test]
fn commit_without_a_verified_content_blob_does_not_write_a_revision() {
    let case = setup_case();
    let hash = bundle_hash_of(&files_v1());

    let result = case.store.commit_revision(&request(
        Some(hash),
        vec![],
        RevisionKind::Content,
        "0.1.0",
        vec![],
    ));

    assert!(result.is_err());
    assert!(heads(&case).is_empty());
    let count: u32 = rusqlite::Connection::open(&case.db)
        .unwrap()
        .query_row("SELECT COUNT(*) FROM revisions", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn tombstone_replaces_heads_and_records_observed_heads() {
    let case = setup_case();
    let (_archive, first_hash) = stage(&case, &files_v1());
    let first = case
        .store
        .commit_revision(&request(
            Some(first_hash),
            vec![],
            RevisionKind::Content,
            "0.1.0",
            vec![],
        ))
        .unwrap();
    let tombstone = case
        .store
        .commit_revision(&request(
            None,
            vec![first.revision().id().clone()],
            RevisionKind::Tombstone {
                observed_heads: vec![first.revision().id().clone()],
            },
            "0.1.0",
            vec![first.revision().id().clone()],
        ))
        .unwrap();
    assert_eq!(
        heads(&case),
        vec![tombstone.revision().id().as_str().to_owned()]
    );
    let stored: String = rusqlite::Connection::open(&case.db)
        .unwrap()
        .query_row(
            "SELECT observed_heads_json FROM deletions WHERE deletion_revision_id = ?1",
            [tombstone.revision().id().as_str()],
            |row| row.get(0),
        )
        .unwrap();
    assert!(stored.contains(first.revision().id().as_str()));
    let state: String = rusqlite::Connection::open(&case.db)
        .unwrap()
        .query_row(
            "SELECT state FROM revisions WHERE id = ?1",
            [tombstone.revision().id().as_str()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(state, "tombstone");
}
