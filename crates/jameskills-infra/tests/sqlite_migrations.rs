use jameskills_core::ports::StoragePort;
use jameskills_infra::sqlite::SqliteStore;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static DB_COUNTER: AtomicU64 = AtomicU64::new(0);

const MIGRATIONS: [&str; 3] = ["001_library.sql", "002_operations.sql", "003_sync.sql"];

fn migrations_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("migrations")
}

fn migration_sql(name: &str) -> String {
    std::fs::read_to_string(migrations_dir().join(name)).expect("migration file is readable")
}

fn migrated_memory_db() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    for name in MIGRATIONS {
        connection.execute_batch(&migration_sql(name)).unwrap();
    }
    connection
}

fn tables(connection: &Connection) -> Vec<String> {
    let mut statement = connection
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .unwrap();
    statement
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(|name| name.unwrap())
        .collect()
}

fn columns(connection: &Connection, table: &str) -> Vec<String> {
    let mut statement = connection
        .prepare(&format!("PRAGMA table_info({table})"))
        .unwrap();
    statement
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .map(|name| name.unwrap())
        .collect()
}

fn fresh_db_file() -> PathBuf {
    let id = DB_COUNTER.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!(
        "jameskills-migrations-{}-{id}.sqlite3",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    path
}

fn remove_backups(path: &std::path::Path) {
    let stem = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_owned();
    if let Some(parent) = path.parent() {
        let Ok(entries) = std::fs::read_dir(parent) else {
            return;
        };
        for entry in entries.flatten() {
            let candidate = entry.path();
            if candidate
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(&stem) && name.contains(".backup-v"))
            {
                let _ = std::fs::remove_file(&candidate);
            }
        }
    }
}

#[test]
fn fresh_db_applies_all_migrations_with_exact_tables() {
    let connection = migrated_memory_db();
    assert_eq!(
        tables(&connection),
        vec![
            "deletions",
            "drafts",
            "guidance_sessions",
            "installations",
            "operations",
            "remote_files",
            "revision_parents",
            "revisions",
            "skill_heads",
            "skills",
            "snapshots",
        ]
    );
}

#[test]
fn migrations_apply_twice_without_error() {
    let connection = Connection::open_in_memory().unwrap();
    for _ in 0..2 {
        for name in MIGRATIONS {
            connection.execute_batch(&migration_sql(name)).unwrap();
        }
    }
    assert_eq!(tables(&connection).len(), 11);
}

#[test]
fn foreign_keys_reject_dangling_revision_but_allow_unknown_parent() {
    let connection = migrated_memory_db();
    connection
        .execute_batch("PRAGMA foreign_keys = ON")
        .unwrap();
    connection
        .execute(
            "INSERT INTO skills(id, slug, display_name, created_at) VALUES (?1, ?2, ?3, ?4)",
            ("skill-1", "demo", "Demo", "2026-01-01T00:00:00Z"),
        )
        .unwrap();
    let revision = "INSERT INTO revisions(id, skill_id, bundle_hash, semantic_version, schema_version, state, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)";
    assert!(
        connection
            .execute(
                revision,
                (
                    "rev-1",
                    "missing-skill",
                    "hash",
                    "0.1.0",
                    1,
                    "content",
                    "now"
                )
            )
            .is_err()
    );
    connection
        .execute(
            revision,
            ("rev-1", "skill-1", "hash", "0.1.0", 1, "content", "now"),
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO skill_heads(skill_id, revision_id) VALUES (?1, ?2)",
            ("skill-1", "rev-1"),
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO revision_parents(revision_id, parent_revision_id) VALUES (?1, ?2)",
            ("rev-1", "not-yet-downloaded"),
        )
        .unwrap();
}

#[test]
fn migrations_do_not_own_user_version() {
    for name in MIGRATIONS {
        assert!(
            !migration_sql(name).contains("PRAGMA user_version"),
            "{name} must not manage user_version"
        );
    }
    let connection = migrated_memory_db();
    let version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(version, 0);
}

#[test]
fn corrupt_file_is_not_a_database() {
    let path = fresh_db_file();
    std::fs::write(&path, b"not a sqlite database").unwrap();
    let connection = Connection::open(&path).unwrap();
    assert!(
        connection
            .query_row("SELECT name FROM sqlite_master", [], |_| Ok(()))
            .is_err()
    );
    drop(connection);
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn revisions_schema_matches_architecture() {
    let connection = migrated_memory_db();
    assert_eq!(
        columns(&connection, "skills"),
        vec!["id", "slug", "display_name", "created_at"]
    );
    assert_eq!(
        columns(&connection, "revisions"),
        vec![
            "id",
            "skill_id",
            "bundle_hash",
            "semantic_version",
            "schema_version",
            "state",
            "created_at",
        ]
    );
    assert_eq!(
        columns(&connection, "revision_parents"),
        vec!["revision_id", "parent_revision_id"]
    );
    assert_eq!(
        columns(&connection, "deletions"),
        vec!["skill_id", "deletion_revision_id", "observed_heads_json"]
    );
}

fn pragma(connection: &Connection, statement: &str) -> String {
    connection
        .query_row(statement, [], |row| row.get::<_, String>(0))
        .unwrap()
}

fn pragma_int(connection: &Connection, statement: &str) -> i64 {
    connection
        .query_row(statement, [], |row| row.get(0))
        .unwrap()
}

#[test]
fn fresh_open_applies_schema_and_storage_pragmas() {
    let path = fresh_db_file();
    let store = SqliteStore::open(&path).unwrap();
    let port: &dyn StoragePort = &store;
    assert_eq!(port.schema_version().unwrap(), 3);
    assert!(!store.is_read_only());
    assert!(port.check_integrity().is_ok());
    let connection = Connection::open(&path).unwrap();
    assert_eq!(pragma(&connection, "PRAGMA journal_mode"), "wal");
    assert_eq!(pragma_int(&connection, "PRAGMA foreign_keys"), 1);
    assert_eq!(pragma_int(&connection, "PRAGMA busy_timeout"), 5000);
    assert!(store.check_integrity().is_ok());
    drop(store);
    drop(connection);
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn upgrade_from_v1_backs_up_then_migrates_with_data_kept() {
    let path = fresh_db_file();
    remove_backups(&path);
    {
        let setup = Connection::open(&path).unwrap();
        setup
            .execute_batch(&migration_sql("001_library.sql"))
            .unwrap();
        setup
            .execute(
                "INSERT INTO skills(id, slug, display_name, created_at) VALUES (?1, ?2, ?3, ?4)",
                ("skill-1", "demo", "Demo", "2026-01-01T00:00:00Z"),
            )
            .unwrap();
        setup.execute_batch("PRAGMA user_version = 1").unwrap();
    }
    let store = SqliteStore::open(&path).unwrap();
    assert_eq!(store.schema_version().unwrap(), 3);
    assert_eq!(tables(&Connection::open(&path).unwrap()).len(), 11);
    let kept: String = Connection::open(&path)
        .unwrap()
        .query_row("SELECT slug FROM skills WHERE id = 'skill-1'", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(kept, "demo");
    let parent = path.parent().unwrap();
    let stem = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_owned();
    let backups: Vec<_> = std::fs::read_dir(parent)
        .unwrap()
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|candidate| {
            candidate
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(&stem) && name.contains(".backup-v1-"))
        })
        .collect();
    assert_eq!(backups.len(), 1);
    let backup_tables = tables(&Connection::open(&backups[0]).unwrap());
    assert_eq!(
        backup_tables,
        vec!["revision_parents", "revisions", "skill_heads", "skills"]
    );
    drop(store);
    std::fs::remove_file(&path).unwrap();
    std::fs::remove_file(&backups[0]).unwrap();
}

#[test]
fn future_version_opens_locked_read_only_without_writes() {
    let path = fresh_db_file();
    {
        let setup = Connection::open(&path).unwrap();
        setup.execute_batch("PRAGMA user_version = 999").unwrap();
    }
    let store = SqliteStore::open(&path).unwrap();
    assert!(store.is_read_only());
    assert_eq!(store.schema_version().unwrap(), 999);
    assert!(store.check_integrity().is_ok());
    let write = store.with_transaction(|transaction| {
        transaction
            .execute("CREATE TABLE probe(x TEXT)", [])
            .map_err(|_| jameskills_core::AppError::Storage {
                code: "test.write.failed".to_owned(),
            })?;
        Ok(())
    });
    assert!(write.is_err());
    assert!(tables(&Connection::open(&path).unwrap()).is_empty());
    drop(store);
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn corrupt_file_open_fails_without_panicking() {
    let path = fresh_db_file();
    std::fs::write(&path, b"not a sqlite database").unwrap();
    assert!(SqliteStore::open(&path).is_err());
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn transaction_rolls_back_failed_work() {
    let path = fresh_db_file();
    let store = SqliteStore::open(&path).unwrap();
    let failed = store.with_transaction(|transaction| {
        transaction
            .execute(
                "INSERT INTO skills(id, slug, display_name, created_at) VALUES (?1, ?2, ?3, ?4)",
                ("skill-1", "demo", "Demo", "now"),
            )
            .map_err(|_| jameskills_core::AppError::Storage {
                code: "test.insert.failed".to_owned(),
            })?;
        Err::<(), _>(jameskills_core::AppError::Storage {
            code: "test.rollback.marker".to_owned(),
        })
    });
    assert!(failed.is_err());
    let count: i64 = Connection::open(&path)
        .unwrap()
        .query_row("SELECT COUNT(*) FROM skills", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
    drop(store);
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn committed_transaction_persists_across_connections() {
    let path = fresh_db_file();
    let store = SqliteStore::open(&path).unwrap();
    store
        .with_transaction(|transaction| {
            transaction
                .execute(
                    "INSERT INTO skills(id, slug, display_name, created_at) VALUES (?1, ?2, ?3, ?4)",
                    ("skill-1", "demo", "Demo", "now"),
                )
                .map_err(|_| {
                    jameskills_core::AppError::Storage {
                        code: "test.insert.failed".to_owned(),
                    }
                })?;
            Ok(())
        })
        .unwrap();
    let count: i64 = Connection::open(&path)
        .unwrap()
        .query_row("SELECT COUNT(*) FROM skills", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1);
    drop(store);
    std::fs::remove_file(&path).unwrap();
}
