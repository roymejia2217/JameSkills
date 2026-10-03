use chrono::{SecondsFormat, Utc};
use jameskills_core::{
    AppError, AppResult,
    domain::{
        RevisionId, RevisionKind, RevisionRecord, SaveRevisionRequest, SaveRevisionResult, SkillId,
    },
    ports::{CURRENT_SCHEMA_VERSION, StoragePort},
};
use rusqlite::{Connection, OpenFlags, Transaction};
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
        Ok(Self {
            connection: Mutex::new(connection),
            path: path.to_path_buf(),
            read_only: false,
        })
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
