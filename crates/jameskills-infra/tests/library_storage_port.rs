use jameskills_core::ports::{LibraryItemState, LibraryQuery, StoragePort};
use jameskills_infra::sqlite::SqliteStore;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

struct TestRoot(std::path::PathBuf);

impl TestRoot {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "jameskills-library-port-{}-{}",
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
fn storage_port_lists_catalog_metadata_through_the_blocking_pool() {
    let root = TestRoot::new();
    let database = root.0.join("library.sqlite3");
    let store = SqliteStore::open(&database).unwrap();
    drop(store);
    let connection = rusqlite::Connection::open(&database).unwrap();
    connection
        .execute(
            "INSERT INTO skills(id, slug, display_name, created_at) VALUES (?1, ?2, ?3, ?4)",
            (
                "11111111-1111-4111-8111-111111111111",
                "port-skill",
                "Port Skill",
                "2026-10-07T00:00:00.000Z",
            ),
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO library_catalog(skill_id, normalized_display_name) VALUES (?1, ?2)",
            (
                "11111111-1111-4111-8111-111111111111",
                "port skill",
            ),
        )
        .unwrap();
    drop(connection);

    let store = SqliteStore::open(&database).unwrap();
    let storage: &dyn StoragePort = &store;
    let query = LibraryQuery::new(None, vec![], vec![], LibraryItemState::Any, None, 10).unwrap();
    let page = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(storage.list_skills(query))
        .unwrap();
    assert_eq!(page.items().len(), 1);
    assert_eq!(page.items()[0].display_name(), "Port Skill");
    drop(store);
}
