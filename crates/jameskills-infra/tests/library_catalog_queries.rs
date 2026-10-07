use jameskills_core::{domain::SkillId, ports::{LibraryItemState, LibraryQuery}};
use jameskills_infra::sqlite::SqliteStore;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

struct TestRoot(std::path::PathBuf);

impl TestRoot {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "jameskills-catalog-query-{}-{}",
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

fn insert_skill(
    connection: &rusqlite::Connection,
    id: &str,
    display_name: &str,
) {
    connection
        .execute(
            "INSERT INTO skills(id, slug, display_name, created_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                id,
                display_name.to_lowercase(),
                display_name,
                "2026-10-07T00:00:00.000Z"
            ],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO library_catalog(skill_id, normalized_display_name) VALUES (?1, ?2)",
            rusqlite::params![id, display_name.to_lowercase()],
        )
        .unwrap();
}

fn temporary_store(root: &TestRoot) -> SqliteStore {
    let database = root.0.join("library.sqlite3");
    SqliteStore::open(&database).unwrap();
    let connection = rusqlite::Connection::open(&database).unwrap();
    for (id, name) in [
        ("11111111-1111-4111-8111-111111111111", "Alpha"),
        ("22222222-2222-4222-8222-222222222222", "Beta"),
        ("33333333-3333-4333-8333-333333333333", "Gamma"),
        ("44444444-4444-4444-8444-444444444444", "Alpha%Skill"),
        ("55555555-5555-4555-8555-555555555555", "AlphaXSkill"),
    ] {
        insert_skill(&connection, id, name);
    }
    drop(connection);
    SqliteStore::open(&database).unwrap()
}

#[test]
fn sqlite_catalog_pages_are_stable_bounded_and_metadata_only() {
    let root = TestRoot::new();
    let store = temporary_store(&root);
    let first = store
        .list_skills(
            &LibraryQuery::new(None, vec![], vec![], LibraryItemState::Any, None, 2).unwrap(),
        )
        .unwrap();
    assert_eq!(
        first
            .items()
            .iter()
            .map(|item| item.display_name())
            .collect::<Vec<_>>(),
        vec!["Alpha", "Alpha%Skill"]
    );
    assert!(first.items().iter().all(|item| item.heads().is_empty()));
    let cursor = first.next().cloned().unwrap();
    let second = store
        .list_skills(
            &LibraryQuery::new(
                None,
                vec![],
                vec![],
                LibraryItemState::Any,
                Some(cursor),
                2,
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(
        second
            .items()
            .iter()
            .map(|item| item.display_name())
            .collect::<Vec<_>>(),
        vec!["AlphaXSkill", "Beta"]
    );
    assert!(second.next().is_some());
}

#[test]
fn sqlite_catalog_search_escapes_like_metacharacters_literally() {
    let root = TestRoot::new();
    let store = temporary_store(&root);
    let page = store
        .list_skills(
            &LibraryQuery::new(Some("%"), vec![], vec![], LibraryItemState::Any, None, 10)
                .unwrap(),
        )
        .unwrap();
    assert_eq!(page.items().len(), 1);
    assert_eq!(page.items()[0].display_name(), "Alpha%Skill");
    assert!(page.next().is_none());
}

#[test]
fn catalog_skill_ids_are_validated_values() {
    assert!(SkillId::parse("not-a-uuid").is_err());
}
