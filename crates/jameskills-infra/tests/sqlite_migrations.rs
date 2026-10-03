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
