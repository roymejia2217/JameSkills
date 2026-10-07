use jameskills_infra::sqlite::SqliteStore;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

struct TestRoot(std::path::PathBuf);

impl TestRoot {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "jameskills-catalog-migration-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed),
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        Self(root)
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn library_catalog_migration_installs_metadata_indexes() {
    let root = TestRoot::new();
    let database = root.0.join("library.sqlite3");
    let store = SqliteStore::open(&database).unwrap();

    assert!(
        store.schema_version().unwrap() >= 4,
        "the library catalog migration must be applied"
    );
    let connection = rusqlite::Connection::open(&database).unwrap();
    for table in ["library_catalog", "revision_tags", "revision_capabilities"] {
        let exists: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
                [table],
                |row| row.get(0),
            )
            .unwrap();
        assert!(exists, "expected {table} after catalog migration");
    }
    for index in ["skills_catalog_order", "revisions_by_skill_and_id"] {
        let exists: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='index' AND name=?1)",
                [index],
                |row| row.get(0),
            )
            .unwrap();
        assert!(exists, "expected {index} after catalog migration");
    }
    drop(connection);
    drop(store);
}
