use jameskills_core::ports::{LibraryItemState, LibraryQuery};
use jameskills_infra::{composition::build_services, platform::UserDirectories};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

struct RuntimeCase {
    root: std::path::PathBuf,
    services: Option<jameskills_infra::composition::RuntimeServices>,
}

impl RuntimeCase {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "jameskills-library-service-query-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed),
        ));
        let _ = std::fs::remove_dir_all(&root);
        let services = build_services(UserDirectories {
            config: root.join("config"),
            data: root.join("data"),
            cache: root.join("cache"),
        })
        .unwrap();
        Self {
            root,
            services: Some(services),
        }
    }
}

impl Drop for RuntimeCase {
    fn drop(&mut self) {
        self.services.take();
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn runtime_library_service_queries_the_persistent_catalog() {
    let case = RuntimeCase::new();
    let database = case.root.join("data/library.sqlite3");
    let connection = rusqlite::Connection::open(&database).unwrap();
    connection
        .execute(
            "INSERT INTO skills(id, slug, display_name, created_at) VALUES (?1, ?2, ?3, ?4)",
            (
                "11111111-1111-4111-8111-111111111111",
                "runtime-skill",
                "Runtime Skill",
                "2026-10-07T00:00:00.000Z",
            ),
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO library_catalog(skill_id, normalized_display_name) VALUES (?1, ?2)",
            ("11111111-1111-4111-8111-111111111111", "runtime skill"),
        )
        .unwrap();
    drop(connection);

    let query = LibraryQuery::new(None, vec![], vec![], LibraryItemState::Any, None, 10).unwrap();
    let page = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(case.services.as_ref().unwrap().library().list_skills(query))
        .unwrap();
    assert_eq!(page.items().len(), 1);
    assert_eq!(page.items()[0].display_name(), "Runtime Skill");
    assert!(page.items()[0].heads().is_empty());
}
