use jameskills_core::{
    AppError, AppResult,
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
