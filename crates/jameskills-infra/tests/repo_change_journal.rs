use async_trait::async_trait;
use jameskills_core::{
    AppError, AppResult,
    domain::{RepoChangePlan, RepoTemplateId, policy::RepositoryHead},
    ports::{
        ClockPort, OperationJournalPort, RepoChangeJournal, RepoChangeJournalState,
        process::{ApprovedRoot, ProcessOutput, ProcessPort, ProcessSpec},
    },
};
use jameskills_infra::{
    fs::{LocalRepoChangePort, RepoChangeRecoveryStatus, repository_root_fingerprint},
    sqlite::SqliteStore,
};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

static NEXT_JOURNAL: AtomicU64 = AtomicU64::new(0);

struct NeverProcess;

#[async_trait]
impl ProcessPort for NeverProcess {
    async fn run(&self, _spec: ProcessSpec) -> AppResult<ProcessOutput> {
        Err(AppError::PermissionDenied {
            operation: "test.recovery.must_not_spawn".to_owned(),
        })
    }
}

struct TestClock;

impl ClockPort for TestClock {
    fn now_utc(&self) -> String {
        "2026-10-05T12:00:00Z".to_owned()
    }

    fn monotonic_ms(&self) -> u64 {
        0
    }
}

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
        let root_fingerprint = repository_root_fingerprint(&root).unwrap();
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

fn recovery_service(store: Arc<SqliteStore>) -> LocalRepoChangePort {
    LocalRepoChangePort::new(Arc::new(NeverProcess), store, Arc::new(TestClock))
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
    assert_eq!(store.schema_version().unwrap(), 6);
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

#[test]
fn recovery_completes_a_hard_linked_commit_pending_operation_without_spawning() {
    let paths = TempPaths::new();
    let journal = paths.journal();
    let operation_id = journal.operation_id();
    let target = paths.root.join(journal.target().as_str());
    let stage = paths.root.join(journal.staging_target().as_str());
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    let content = b"registered template contents are not journaled";
    let store = Arc::new(SqliteStore::open(&paths.db).unwrap());
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
    store
        .transition_operation(
            operation_id,
            RepoChangeJournalState::Staged,
            RepoChangeJournalState::CommitPending,
            "2026-10-05T12:03:00Z",
        )
        .unwrap();
    std::fs::write(&stage, content).unwrap();
    std::fs::hard_link(&stage, &target).unwrap();

    let recovered = recovery_service(store.clone()).recover_pending().unwrap();

    assert_eq!(recovered.len(), 1);
    assert_eq!(recovered[0].operation_id(), operation_id);
    assert_eq!(recovered[0].status(), RepoChangeRecoveryStatus::Committed);
    assert_eq!(std::fs::read(&target).unwrap(), content);
    assert!(!stage.exists());
    assert!(store.pending_operations().unwrap().is_empty());
}

#[test]
fn recovery_preserves_an_edited_or_unowned_target_and_keeps_conflict_pending() {
    let paths = TempPaths::new();
    let journal = paths.journal();
    let operation_id = journal.operation_id();
    let target = paths.root.join(journal.target().as_str());
    let stage = paths.root.join(journal.staging_target().as_str());
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    let staged_bytes = b"registered template contents are not journaled";
    let user_bytes = b"user-owned workflow changed after commit preview\n";
    let store = Arc::new(SqliteStore::open(&paths.db).unwrap());
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
    store
        .transition_operation(
            operation_id,
            RepoChangeJournalState::Staged,
            RepoChangeJournalState::CommitPending,
            "2026-10-05T12:03:00Z",
        )
        .unwrap();
    std::fs::write(&stage, staged_bytes).unwrap();
    std::fs::write(&target, user_bytes).unwrap();

    let recovered = recovery_service(store.clone()).recover_pending().unwrap();

    assert_eq!(recovered.len(), 1);
    assert_eq!(recovered[0].status(), RepoChangeRecoveryStatus::Conflict);
    assert_eq!(std::fs::read(&target).unwrap(), user_bytes);
    assert_eq!(std::fs::read(&stage).unwrap(), staged_bytes);
    assert_eq!(
        store.load_operation(operation_id).unwrap().unwrap().state(),
        RepoChangeJournalState::CommitPending
    );
    assert_eq!(store.pending_operations().unwrap().len(), 1);
}

#[test]
fn recovery_reconciles_simulated_crash_states_idempotently() {
    let content = b"registered template contents are not journaled";
    let cases = [
        (
            RepoChangeJournalState::Planned,
            RepoChangeRecoveryStatus::Recovered,
            RepoChangeJournalState::Failed,
        ),
        (
            RepoChangeJournalState::Approved,
            RepoChangeRecoveryStatus::Recovered,
            RepoChangeJournalState::Failed,
        ),
        (
            RepoChangeJournalState::Staged,
            RepoChangeRecoveryStatus::Recovered,
            RepoChangeJournalState::Recovered,
        ),
        (
            RepoChangeJournalState::CommitPending,
            RepoChangeRecoveryStatus::Recovered,
            RepoChangeJournalState::Recovered,
        ),
        (
            RepoChangeJournalState::NewMoved,
            RepoChangeRecoveryStatus::Committed,
            RepoChangeJournalState::Committed,
        ),
        (
            RepoChangeJournalState::Verified,
            RepoChangeRecoveryStatus::Committed,
            RepoChangeJournalState::Committed,
        ),
        (
            RepoChangeJournalState::RollbackPending,
            RepoChangeRecoveryStatus::Recovered,
            RepoChangeJournalState::Recovered,
        ),
    ];

    for (interrupted_state, expected_status, expected_final_state) in cases {
        let paths = TempPaths::new();
        let journal = paths.journal();
        let operation_id = journal.operation_id();
        let target = paths.root.join(journal.target().as_str());
        let stage = paths.root.join(journal.staging_target().as_str());
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        let store = Arc::new(SqliteStore::open(&paths.db).unwrap());
        store.record_operation(&journal).unwrap();

        let mut current = RepoChangeJournalState::Planned;
        for next in [
            RepoChangeJournalState::Approved,
            RepoChangeJournalState::Staged,
            RepoChangeJournalState::CommitPending,
            RepoChangeJournalState::NewMoved,
            RepoChangeJournalState::Verified,
            RepoChangeJournalState::RollbackPending,
        ] {
            if next == current || current == interrupted_state {
                break;
            }
            store
                .transition_operation(operation_id, current, next, "2026-10-05T12:04:00Z")
                .unwrap();
            current = next;
            if current == interrupted_state {
                break;
            }
        }

        match interrupted_state {
            RepoChangeJournalState::Staged
            | RepoChangeJournalState::CommitPending
            | RepoChangeJournalState::RollbackPending => std::fs::write(&stage, content).unwrap(),
            RepoChangeJournalState::NewMoved | RepoChangeJournalState::Verified => {
                std::fs::write(&target, content).unwrap();
            }
            RepoChangeJournalState::Planned | RepoChangeJournalState::Approved => {}
            _ => unreachable!(),
        }

        let service = recovery_service(store.clone());
        let recovered = service.recover_pending().unwrap();
        assert_eq!(recovered.len(), 1, "state: {interrupted_state:?}");
        assert_eq!(
            recovered[0].status(),
            expected_status,
            "state: {interrupted_state:?}"
        );
        assert_eq!(
            store.load_operation(operation_id).unwrap().unwrap().state(),
            expected_final_state,
            "state: {interrupted_state:?}"
        );
        assert!(service.recover_pending().unwrap().is_empty());
        assert!(store.pending_operations().unwrap().is_empty());
        if matches!(
            interrupted_state,
            RepoChangeJournalState::NewMoved | RepoChangeJournalState::Verified
        ) {
            assert_eq!(std::fs::read(&target).unwrap(), content);
        } else {
            assert!(!target.exists());
        }
        assert!(!stage.exists());
    }
}
