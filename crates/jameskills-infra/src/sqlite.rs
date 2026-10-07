use crate::fs::{list_blob_hashes, verify_blob_bytes};
use chrono::{SecondsFormat, Utc};
use jameskills_core::{
    AppError, AppResult, OperationId, PortablePath,
    domain::{
        ContentHash, RevisionId, RevisionKind, RevisionRecord, SaveRevisionRequest,
        SaveRevisionResult, SkillId, policy::RepositoryHead,
    },
    ports::{
        CURRENT_SCHEMA_VERSION, OperationJournalPort, RepoChangeJournal, RepoChangeJournalState,
        StoragePort, process::ApprovedRoot,
    },
};
use rusqlite::{Connection, ErrorCode, OpenFlags, OptionalExtension, Transaction};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

const MIGRATIONS: [(u32, &str); 3] = [
    (1, include_str!("../migrations/001_library.sql")),
    (2, include_str!("../migrations/002_operations.sql")),
    (3, include_str!("../migrations/003_sync.sql")),
];

fn storage_error(code: &'static str) -> AppError {
    AppError::Storage {
        code: code.to_owned(),
    }
}

/// Single-writer SQLite actor: one mutex guards the only connection, so all
/// writes serialize without an external lock order and never block UI
/// rendering on a mutex. Migrations apply from embedded SQL with a file
/// backup before any destructive upgrade; a future schema opens read-only
/// and locked instead of downgrading.
pub struct SqliteStore {
    connection: Mutex<Connection>,
    path: PathBuf,
    blob_root: PathBuf,
    read_only: bool,
}

impl SqliteStore {
    /// Opens or creates the library database, applies pending migrations and
    /// enforces WAL, foreign keys and a 5s busy timeout. Corrupt files fail
    /// here, before any caller observes a store.
    pub fn open(path: &Path) -> AppResult<Self> {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).map_err(|_| storage_error("storage.open.failed"))?;
        }
        let connection =
            Connection::open(path).map_err(|_| storage_error("storage.open.failed"))?;
        connection
            .execute_batch("PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 5000;")
            .map_err(|_| storage_error("storage.open.failed"))?;
        let version = read_user_version(&connection)?;
        if version > CURRENT_SCHEMA_VERSION {
            return Self::open_locked(path);
        }
        let mode: String = connection
            .query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))
            .map_err(|_| storage_error("storage.open.failed"))?;
        if !mode.eq_ignore_ascii_case("wal") {
            return Err(storage_error("storage.open.failed"));
        }
        if version > 0 && version < CURRENT_SCHEMA_VERSION {
            backup_before_upgrade(path, version)?;
        }
        for (number, sql) in MIGRATIONS
            .iter()
            .skip_while(|(number, _)| *number <= version)
        {
            connection
                .execute_batch(sql)
                .map_err(|_| storage_error("storage.migrate.failed"))?;
            set_user_version(&connection, *number)?;
        }
        let blob_root = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
            .join("blobs");
        let store = Self {
            connection: Mutex::new(connection),
            path: path.to_path_buf(),
            blob_root,
            read_only: false,
        };
        store.verify_referenced_blobs()?;
        Ok(store)
    }

    /// Opens a future schema without writing: migrations are skipped, every
    /// write fails, reads and integrity checks keep working.
    fn open_locked(path: &Path) -> AppResult<Self> {
        let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|_| storage_error("storage.open.failed"))?;
        connection
            .execute_batch("PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 5000;")
            .map_err(|_| storage_error("storage.open.failed"))?;
        Ok(Self {
            connection: Mutex::new(connection),
            path: path.to_path_buf(),
            blob_root: path
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."))
                .join("blobs"),
            read_only: true,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    pub fn schema_version(&self) -> AppResult<u32> {
        let guard = self
            .connection
            .lock()
            .map_err(|_| storage_error("storage.lock.poisoned"))?;
        read_user_version(&guard)
    }

    pub fn check_integrity(&self) -> AppResult<()> {
        let guard = self
            .connection
            .lock()
            .map_err(|_| storage_error("storage.lock.poisoned"))?;
        let report: String = guard
            .query_row("PRAGMA quick_check", [], |row| row.get(0))
            .map_err(|_| storage_error("storage.integrity.failed"))?;
        if report.eq_ignore_ascii_case("ok") {
            Ok(())
        } else {
            Err(storage_error("storage.integrity.failed"))
        }
    }

    /// Lists valid content-addressed blobs with no committed content revision.
    /// Results are informational and never removed automatically: a writer
    /// may have staged a blob before a transaction failed or the process
    /// stopped.
    pub fn orphan_blob_hashes(&self) -> AppResult<Vec<ContentHash>> {
        let referenced = self.referenced_blob_hashes()?;
        let present = list_blob_hashes(&self.blob_root)
            .map_err(|_| storage_error("storage.blob.inventory.failed"))?;
        Ok(present
            .into_iter()
            .filter(|hash| !referenced.contains(hash))
            .collect())
    }

    fn referenced_blob_hashes(&self) -> AppResult<BTreeSet<ContentHash>> {
        let guard = self
            .connection
            .lock()
            .map_err(|_| storage_error("storage.lock.poisoned"))?;
        let mut statement = guard
            .prepare(
                "SELECT DISTINCT bundle_hash, state FROM revisions ORDER BY bundle_hash, state",
            )
            .map_err(|_| storage_error("storage.data.corrupt"))?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|_| storage_error("storage.data.corrupt"))?;
        let mut referenced = BTreeSet::new();
        for row in rows {
            let (value, state) = row.map_err(|_| storage_error("storage.data.corrupt"))?;
            let hash = ContentHash::parse_hex(&value)
                .map_err(|_| storage_error("storage.data.corrupt"))?;
            match state.as_str() {
                "content" => {
                    referenced.insert(hash);
                }
                "tombstone" if hash == ContentHash::from_digest([0; 32]) => {}
                "tombstone" => return Err(storage_error("storage.data.corrupt")),
                _ => return Err(storage_error("storage.data.corrupt")),
            }
        }
        Ok(referenced)
    }

    fn verify_referenced_blobs(&self) -> AppResult<()> {
        for hash in self.referenced_blob_hashes()? {
            verify_blob_bytes(&self.blob_root, &hash)
                .map_err(|_| storage_error("storage.blob.invalid"))?;
        }
        Ok(())
    }

    /// Runs work inside one transaction on the single writer. A returned
    /// error rolls back on drop; only a returned `Ok` commits.
    pub fn with_transaction<T>(
        &self,
        work: impl FnOnce(&Transaction<'_>) -> AppResult<T>,
    ) -> AppResult<T> {
        if self.read_only {
            return Err(storage_error("storage.store.read_only"));
        }
        let mut guard = self
            .connection
            .lock()
            .map_err(|_| storage_error("storage.lock.poisoned"))?;
        let transaction = guard
            .transaction()
            .map_err(|_| storage_error("storage.transaction.failed"))?;
        let output = work(&transaction)?;
        transaction
            .commit()
            .map_err(|_| storage_error("storage.transaction.failed"))?;
        Ok(output)
    }

    /// Commits one revision: the expected heads must match the stored heads
    /// exactly, otherwise the commit fails with the current heads and nothing
    /// is written. Revision, parents and the new head land in ONE
    /// transaction; tombstones additionally record their observed heads.
    /// Blobs are staged before this call, so a failed commit leaves an
    /// unreferenced blob but never an invalid head.
    pub fn commit_revision(&self, request: &SaveRevisionRequest) -> AppResult<SaveRevisionResult> {
        if self.read_only {
            return Err(storage_error("storage.store.read_only"));
        }
        let record = RevisionRecord::new(
            request.skill_id(),
            request.bundle_hash().cloned(),
            request.parents().to_vec(),
            request.kind().clone(),
            request.semantic_version().to_owned(),
        )?;
        if matches!(request.kind(), RevisionKind::Content) {
            verify_blob_bytes(&self.blob_root, record.bundle_hash())
                .map_err(|_| storage_error("storage.blob.invalid"))?;
        }
        let mut guard = self
            .connection
            .lock()
            .map_err(|_| storage_error("storage.lock.poisoned"))?;
        let transaction = guard
            .transaction()
            .map_err(|_| storage_error("storage.transaction.failed"))?;
        let current = read_heads(&transaction, request.skill_id())?;
        let mut expected: Vec<String> = request
            .expected_heads()
            .iter()
            .map(|head| head.as_str().to_owned())
            .collect();
        expected.sort();
        expected.dedup();
        if current != expected {
            let conflicting = current
                .iter()
                .map(|hex| RevisionId::parse_hex(hex))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| storage_error("storage.data.corrupt"))?;
            return Err(AppError::Conflict {
                current: conflicting,
            });
        }
        let parents: Vec<String> = record
            .parents()
            .iter()
            .map(|parent| parent.as_str().to_owned())
            .collect();
        if parents != current {
            let conflicting = current
                .iter()
                .map(|hex| RevisionId::parse_hex(hex))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| storage_error("storage.data.corrupt"))?;
            return Err(AppError::Conflict {
                current: conflicting,
            });
        }
        if let RevisionKind::Tombstone { observed_heads } = request.kind() {
            let mut observed: Vec<String> = observed_heads
                .iter()
                .map(|head| head.as_str().to_owned())
                .collect();
            observed.sort();
            observed.dedup();
            if observed != current {
                let conflicting = current
                    .iter()
                    .map(|hex| RevisionId::parse_hex(hex))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| storage_error("storage.data.corrupt"))?;
                return Err(AppError::Conflict {
                    current: conflicting,
                });
            }
        }
        let state = match request.kind() {
            RevisionKind::Content => "content",
            RevisionKind::Tombstone { .. } => "tombstone",
        };
        let created_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        transaction
            .execute(
                "INSERT INTO revisions(id, skill_id, bundle_hash, semantic_version, schema_version, state, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                (
                    record.id().as_str(),
                    request.skill_id().as_uuid().to_string(),
                    record.bundle_hash().as_str(),
                    request.semantic_version(),
                    request.schema_version(),
                    state,
                    created_at,
                ),
            )
            .map_err(|_| storage_error("storage.revision.rejected"))?;
        for parent in record.parents() {
            transaction
                .execute(
                    "INSERT INTO revision_parents(revision_id, parent_revision_id) VALUES (?1, ?2)",
                    (record.id().as_str(), parent.as_str()),
                )
                .map_err(|_| storage_error("storage.revision.rejected"))?;
        }
        if let RevisionKind::Tombstone { observed_heads } = request.kind() {
            transaction
                .execute(
                    "INSERT INTO deletions(skill_id, deletion_revision_id, observed_heads_json) VALUES (?1, ?2, ?3)",
                    (
                        request.skill_id().as_uuid().to_string(),
                        record.id().as_str(),
                        observed_heads_json(observed_heads),
                    ),
                )
                .map_err(|_| storage_error("storage.revision.rejected"))?;
        }
        transaction
            .execute(
                "DELETE FROM skill_heads WHERE skill_id = ?1",
                [request.skill_id().as_uuid().to_string()],
            )
            .map_err(|_| storage_error("storage.revision.rejected"))?;
        transaction
            .execute(
                "INSERT INTO skill_heads(skill_id, revision_id) VALUES (?1, ?2)",
                (
                    request.skill_id().as_uuid().to_string(),
                    record.id().as_str(),
                ),
            )
            .map_err(|_| storage_error("storage.revision.rejected"))?;
        transaction
            .commit()
            .map_err(|_| storage_error("storage.transaction.failed"))?;
        let new_heads = vec![record.id().clone()];
        Ok(SaveRevisionResult::new(record, new_heads))
    }
}

fn read_heads(transaction: &Transaction<'_>, skill_id: SkillId) -> AppResult<Vec<String>> {
    let mut statement = transaction
        .prepare("SELECT revision_id FROM skill_heads WHERE skill_id = ?1 ORDER BY revision_id")
        .map_err(|_| storage_error("storage.revision.rejected"))?;
    statement
        .query_map([skill_id.as_uuid().to_string()], |row| row.get(0))
        .map_err(|_| storage_error("storage.revision.rejected"))?
        .map(|id| id.map_err(|_| storage_error("storage.data.corrupt")))
        .collect()
}

/// Serializes observed heads as a JSON array by hand. Revision ids are
/// validated 64-char lowercase hex at construction, so no quoting or escape
/// sequence can appear; anything else fails the debug assertion in tests.
fn observed_heads_json(heads: &[RevisionId]) -> String {
    let mut out = String::from("[");
    for (index, head) in heads.iter().enumerate() {
        debug_assert!(head.as_str().bytes().all(|byte| byte.is_ascii_hexdigit()));
        if index > 0 {
            out.push(',');
        }
        out.push('"');
        out.push_str(head.as_str());
        out.push('"');
    }
    out.push(']');
    out
}

impl StoragePort for SqliteStore {
    fn schema_version(&self) -> AppResult<u32> {
        SqliteStore::schema_version(self)
    }

    fn check_integrity(&self) -> AppResult<()> {
        SqliteStore::check_integrity(self)
    }
}

const MAX_PENDING_REPO_CHANGE_JOURNALS: usize = 256;
const MAX_REPO_CHANGE_JOURNAL_BYTES: usize = 16 * 1024;

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredRepoChangeJournal {
    schema_version: u8,
    operation_id: String,
    root_path: String,
    root_fingerprint: String,
    expected_head: String,
    target: String,
    staging_target: String,
    previous_hash: Option<String>,
    proposed_hash: String,
}

fn encode_repo_change_journal(journal: &RepoChangeJournal) -> AppResult<Vec<u8>> {
    let root_path = journal
        .root()
        .path()
        .to_str()
        .ok_or_else(|| storage_error("storage.operation_journal.root.invalid"))?;
    let stored = StoredRepoChangeJournal {
        schema_version: 1,
        operation_id: journal.operation_id().as_uuid().to_string(),
        root_path: root_path.to_owned(),
        root_fingerprint: journal.root_fingerprint().as_str().to_owned(),
        expected_head: journal.expected_head().as_str().to_owned(),
        target: journal.target().as_str().to_owned(),
        staging_target: journal.staging_target().as_str().to_owned(),
        previous_hash: journal.previous_hash().map(|hash| hash.as_str().to_owned()),
        proposed_hash: journal.proposed_hash().as_str().to_owned(),
    };
    let payload = serde_json::to_vec(&stored)
        .map_err(|_| storage_error("storage.operation_journal.encode.failed"))?;
    if payload.len() > MAX_REPO_CHANGE_JOURNAL_BYTES {
        return Err(storage_error("storage.operation_journal.limit"));
    }
    Ok(payload)
}

impl OperationJournalPort for SqliteStore {
    fn record_operation(&self, journal: &RepoChangeJournal) -> AppResult<()> {
        if journal.state() != RepoChangeJournalState::Planned {
            return Err(AppError::Validation(vec![
                jameskills_core::Diagnostic::error(
                    "repo.change.journal.state.invalid",
                    "A new repository change journal must begin in Planned state.",
                ),
            ]));
        }
        let payload = encode_repo_change_journal(journal)?;
        let operation_id = journal.operation_id().as_uuid().to_string();
        let state = journal.state().as_str();
        let updated_at = journal.updated_at();
        self.with_transaction(|transaction| {
            transaction
                .execute(
                    "INSERT INTO operations(id,kind,state,journal_json,updated_at) VALUES(?1,'repo-change',?2,?3,?4)",
                    rusqlite::params![operation_id, state, payload, updated_at],
                )
                .map_err(map_journal_insert_error)?;
            Ok(())
        })
    }

    fn load_operation(&self, operation_id: OperationId) -> AppResult<Option<RepoChangeJournal>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| storage_error("storage.lock.poisoned"))?;
        let row = connection
            .query_row(
                "SELECT state,updated_at,journal_json FROM operations WHERE id=?1 AND kind='repo-change'",
                [operation_id.as_uuid().to_string()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Vec<u8>>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(|_| storage_error("storage.operation_journal.read.failed"))?;
        row.map(|(state, updated_at, payload)| {
            decode_repo_change_journal(operation_id, &state, &updated_at, &payload)
        })
        .transpose()
    }

    fn transition_operation(
        &self,
        operation_id: OperationId,
        expected: RepoChangeJournalState,
        next: RepoChangeJournalState,
        updated_at: &str,
    ) -> AppResult<()> {
        if !expected.allows_transition(next)
            || updated_at.is_empty()
            || updated_at.len() > 64
            || updated_at.chars().any(char::is_control)
        {
            return Err(AppError::Validation(vec![
                jameskills_core::Diagnostic::error(
                    "repo.change.journal.transition.invalid",
                    "Repository change journal transition is invalid.",
                ),
            ]));
        }
        self.with_transaction(|transaction| {
            let changed = transaction
                .execute(
                    "UPDATE operations SET state=?1,updated_at=?2 WHERE id=?3 AND kind='repo-change' AND state=?4",
                    rusqlite::params![
                        next.as_str(),
                        updated_at,
                        operation_id.as_uuid().to_string(),
                        expected.as_str(),
                    ],
                )
                .map_err(|_| storage_error("storage.operation_journal.write.failed"))?;
            if changed != 1 {
                return Err(AppError::Conflict { current: vec![] });
            }
            Ok(())
        })
    }

    fn pending_operations(&self) -> AppResult<Vec<RepoChangeJournal>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| storage_error("storage.lock.poisoned"))?;
        let mut statement = connection
            .prepare(
                "SELECT id,state,updated_at,journal_json FROM operations WHERE kind='repo-change' AND state NOT IN ('committed','failed','recovered') ORDER BY updated_at,id LIMIT ?1",
            )
            .map_err(|_| storage_error("storage.operation_journal.read.failed"))?;
        let rows = statement
            .query_map([MAX_PENDING_REPO_CHANGE_JOURNALS as i64 + 1], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                ))
            })
            .map_err(|_| storage_error("storage.operation_journal.read.failed"))?;
        let rows = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| storage_error("storage.operation_journal.read.failed"))?;
        if rows.len() > MAX_PENDING_REPO_CHANGE_JOURNALS {
            return Err(storage_error("storage.operation_journal.limit"));
        }
        rows.into_iter()
            .map(|(id, state, updated_at, payload)| {
                let operation_id =
                    OperationId::parse(&id).map_err(|_| storage_error("storage.data.corrupt"))?;
                decode_repo_change_journal(operation_id, &state, &updated_at, &payload)
            })
            .collect()
    }
}

fn decode_repo_change_journal(
    operation_id: OperationId,
    state: &str,
    updated_at: &str,
    payload: &[u8],
) -> AppResult<RepoChangeJournal> {
    if payload.len() > MAX_REPO_CHANGE_JOURNAL_BYTES {
        return Err(storage_error("storage.data.corrupt"));
    }
    let state = RepoChangeJournalState::parse(state)
        .ok_or_else(|| storage_error("storage.data.corrupt"))?;
    let stored = serde_json::from_slice::<StoredRepoChangeJournal>(payload)
        .map_err(|_| storage_error("storage.data.corrupt"))?;
    if stored.schema_version != 1
        || OperationId::parse(&stored.operation_id).ok() != Some(operation_id)
    {
        return Err(storage_error("storage.data.corrupt"));
    }
    let root = ApprovedRoot::from_absolute_path(PathBuf::from(stored.root_path))
        .map_err(|_| storage_error("storage.data.corrupt"))?;
    let root_fingerprint = ContentHash::parse_hex(&stored.root_fingerprint)
        .map_err(|_| storage_error("storage.data.corrupt"))?;
    let expected_head = RepositoryHead::parse(&stored.expected_head)
        .map_err(|_| storage_error("storage.data.corrupt"))?;
    let target =
        PortablePath::new(stored.target).map_err(|_| storage_error("storage.data.corrupt"))?;
    let staging_target = PortablePath::new(stored.staging_target)
        .map_err(|_| storage_error("storage.data.corrupt"))?;
    let previous_hash = stored
        .previous_hash
        .map(|value| ContentHash::parse_hex(&value))
        .transpose()
        .map_err(|_| storage_error("storage.data.corrupt"))?;
    let proposed_hash = ContentHash::parse_hex(&stored.proposed_hash)
        .map_err(|_| storage_error("storage.data.corrupt"))?;
    RepoChangeJournal::from_storage_parts(
        operation_id,
        state,
        root,
        root_fingerprint,
        expected_head,
        target,
        staging_target,
        previous_hash,
        proposed_hash,
        updated_at,
    )
    .map_err(|_| storage_error("storage.data.corrupt"))
}

fn map_journal_insert_error(error: rusqlite::Error) -> AppError {
    match error {
        rusqlite::Error::SqliteFailure(code, _) if code.code == ErrorCode::ConstraintViolation => {
            AppError::Conflict { current: vec![] }
        }
        _ => storage_error("storage.operation_journal.write.failed"),
    }
}

fn read_user_version(connection: &Connection) -> AppResult<u32> {
    connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|_| storage_error("storage.open.failed"))
}

fn set_user_version(connection: &Connection, version: u32) -> AppResult<()> {
    connection
        .execute_batch(&format!("PRAGMA user_version = {version}"))
        .map_err(|_| storage_error("storage.migrate.failed"))
}

/// Copies the database file next to itself before a destructive upgrade.
/// Same directory means same filesystem; the source is never renamed away.
fn backup_before_upgrade(path: &Path, from: u32) -> AppResult<PathBuf> {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .map_err(|_| storage_error("storage.backup.failed"))?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| storage_error("storage.backup.failed"))?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let backup = parent.join(format!("{file_name}.backup-v{from}-{seconds}.sqlite3"));
    std::fs::copy(path, &backup).map_err(|_| storage_error("storage.backup.failed"))?;
    Ok(backup)
}
