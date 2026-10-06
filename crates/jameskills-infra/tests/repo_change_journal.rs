use jameskills_core::{
    AppError, ContentHash,
    domain::{RepoChangePlan, RepoTemplateId, policy::RepositoryHead},
    ports::{
        OperationJournalPort, RepoChangeJournal, RepoChangeJournalState, process::ApprovedRoot,
    },
};
use jameskills_infra::sqlite::SqliteStore;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_JOURNAL: AtomicU64 = AtomicU64::new(0);

struct TempPaths {
    db: PathBuf,
    root: PathBuf,
}

impl TempPaths {
    fn new() -> Self {
        let id = NEXT_JOURNAL.fetch_add(1, Ordering::Relaxed);
        let base = std::env::temp_dir().join(format!(
            "jameskills-repo-journal-{}-{id}",
            std::process::id()
        ));
        std::fs::create_dir_all(&base).unwrap();
        Self {
            db: base.join("library.sqlite3"),
            root: base.join("selected-repository"),
        }
    }

    fn journal(&self) -> RepoChangeJournal {
        std::fs::create_dir_all(&self.root).unwrap();
        let root =
            ApprovedRoot::from_absolute_path(std::fs::canonicalize(&self.root).unwrap()).unwrap();
        let root_fingerprint = ContentHash::parse_hex(&"a".repeat(64)).unwrap();
        let plan = RepoChangePlan::new(
            RepoTemplateId::RustCi,
            root_fingerprint,
            RepositoryHead::parse(&"b".repeat(40)).unwrap(),
            None,
            b"registered template contents are not journaled",
            "+ generated workflow\n".to_owned(),
        )
        .unwrap();
        RepoChangeJournal::planned(root, &plan, "2026-10-05T12:00:00Z").unwrap()
    }
}

impl Drop for TempPaths {
    fn drop(&mut self) {
        if let Some(parent) = self.db.parent() {
            let _ = std::fs::remove_dir_all(parent);
        }
    }
}

#[test]
fn duplicate_operation_id_is_conflict_and_does_not_replace_the_original_journal() {
    let paths = TempPaths::new();
    let store = SqliteStore::open(&paths.db).unwrap();
    let journal = paths.journal();
    let operation_id = journal.operation_id();
    store.record_operation(&journal).unwrap();

    assert!(matches!(
        store.record_operation(&journal),
        Err(AppError::Conflict { .. })
    ));
    let loaded = store.load_operation(operation_id).unwrap().unwrap();
    assert_eq!(loaded.state(), RepoChangeJournalState::Planned);
    assert_eq!(loaded.updated_at(), "2026-10-05T12:00:00Z");
    assert_eq!(store.schema_version().unwrap(), 3);
}

#[test]
fn invalid_transition_is_rejected_and_valid_journal_survives_reopen() {
    let paths = TempPaths::new();
    let journal = paths.journal();
    let operation_id = journal.operation_id();
    let store = SqliteStore::open(&paths.db).unwrap();
    store.record_operation(&journal).unwrap();

    assert!(matches!(
        store.transition_operation(
            operation_id,
            RepoChangeJournalState::Planned,
            RepoChangeJournalState::Committed,
            "2026-10-05T12:01:00Z",
        ),
        Err(AppError::Validation(_))
    ));
    store
        .transition_operation(
            operation_id,
            RepoChangeJournalState::Planned,
            RepoChangeJournalState::Approved,
            "2026-10-05T12:02:00Z",
        )
        .unwrap();
    assert_eq!(store.pending_operations().unwrap().len(), 1);
    drop(store);

    let reopened = SqliteStore::open(&paths.db).unwrap();
    let loaded = reopened.load_operation(operation_id).unwrap().unwrap();
    assert_eq!(loaded.state(), RepoChangeJournalState::Approved);
    assert_eq!(
        loaded.target().as_str(),
        ".github/workflows/jameskills-ci.yml"
    );
    assert!(matches!(
        reopened.transition_operation(
            operation_id,
            RepoChangeJournalState::Planned,
            RepoChangeJournalState::Failed,
            "2026-10-05T12:02:30Z",
        ),
        Err(AppError::Conflict { .. })
    ));
    reopened
        .transition_operation(
            operation_id,
            RepoChangeJournalState::Approved,
            RepoChangeJournalState::Staged,
            "2026-10-05T12:03:00Z",
        )
        .unwrap();
    reopened
        .transition_operation(
            operation_id,
            RepoChangeJournalState::Staged,
            RepoChangeJournalState::CommitPending,
            "2026-10-05T12:04:00Z",
        )
        .unwrap();
    reopened
        .transition_operation(
            operation_id,
            RepoChangeJournalState::CommitPending,
            RepoChangeJournalState::NewMoved,
            "2026-10-05T12:04:30Z",
        )
        .unwrap();
    reopened
        .transition_operation(
            operation_id,
            RepoChangeJournalState::NewMoved,
            RepoChangeJournalState::Verified,
            "2026-10-05T12:05:00Z",
        )
        .unwrap();
    reopened
        .transition_operation(
            operation_id,
            RepoChangeJournalState::Verified,
            RepoChangeJournalState::Committed,
            "2026-10-05T12:06:00Z",
        )
        .unwrap();
    assert!(reopened.pending_operations().unwrap().is_empty());
}

#[test]
fn journal_json_contains_hash_metadata_but_not_template_content() {
    let paths = TempPaths::new();
    let store = SqliteStore::open(&paths.db).unwrap();
    let journal = paths.journal();
    let operation_id = journal.operation_id();
    store.record_operation(&journal).unwrap();
    drop(store);

    let connection = rusqlite::Connection::open(&paths.db).unwrap();
    let payload: Vec<u8> = connection
        .query_row(
            "SELECT journal_json FROM operations WHERE id=?1",
            [operation_id.as_uuid().to_string()],
            |row| row.get(0),
        )
        .unwrap();
    let serialized = String::from_utf8(payload).unwrap();
    assert!(serialized.contains("root_fingerprint"));
    assert!(serialized.contains("proposed_hash"));
    assert!(!serialized.contains("registered template contents are not journaled"));
}

#[test]
fn staged_failure_remains_pending_until_recovery_finishes() {
    let paths = TempPaths::new();
    let journal = paths.journal();
    let operation_id = journal.operation_id();
    let store = SqliteStore::open(&paths.db).unwrap();
    store.record_operation(&journal).unwrap();
    store
        .transition_operation(
            operation_id,
            RepoChangeJournalState::Planned,
            RepoChangeJournalState::Approved,
            "2026-10-05T12:01:00Z",
        )
        .unwrap();
    store
        .transition_operation(
            operation_id,
            RepoChangeJournalState::Approved,
            RepoChangeJournalState::Staged,
            "2026-10-05T12:02:00Z",
        )
        .unwrap();

    assert!(matches!(
        store.transition_operation(
            operation_id,
            RepoChangeJournalState::Staged,
            RepoChangeJournalState::Failed,
            "2026-10-05T12:03:00Z",
        ),
        Err(AppError::Validation(_))
    ));
    store
        .transition_operation(
            operation_id,
            RepoChangeJournalState::Staged,
            RepoChangeJournalState::RollbackPending,
            "2026-10-05T12:04:00Z",
        )
        .unwrap();
    assert_eq!(store.pending_operations().unwrap().len(), 1);
}

#[test]
fn unknown_journal_fields_are_classified_as_corrupt_storage() {
    let paths = TempPaths::new();
    let journal = paths.journal();
    let operation_id = journal.operation_id();
    let store = SqliteStore::open(&paths.db).unwrap();
    store.record_operation(&journal).unwrap();
    drop(store);

    let connection = rusqlite::Connection::open(&paths.db).unwrap();
    connection
        .execute(
            "UPDATE operations SET journal_json=?1 WHERE id=?2",
            rusqlite::params![
                br#"{"schema_version":1,"unknown":"value"}"#.as_slice(),
                operation_id.as_uuid().to_string(),
            ],
        )
        .unwrap();
    drop(connection);
    let reopened = SqliteStore::open(&paths.db).unwrap();

    assert!(matches!(
        reopened.load_operation(operation_id),
        Err(AppError::Storage { .. })
    ));
}
