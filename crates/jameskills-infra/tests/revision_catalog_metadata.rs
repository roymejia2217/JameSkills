use jameskills_core::{
    domain::{PortablePath, RevisionKind, SaveRevisionRequest, validate_bundle},
    ports::{LibraryItemState, LibraryQuery},
};
use jameskills_infra::sqlite::SqliteStore;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

struct TestRoot(std::path::PathBuf);

impl TestRoot {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "jameskills-revision-catalog-{}-{}",
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

fn valid_files() -> BTreeMap<PortablePath, Vec<u8>> {
    [
        (
            "SKILL.md",
            include_bytes!("../../../docs/examples/repository-foundation/SKILL.md").as_slice(),
        ),
        (
            "jameskills.toml",
            include_bytes!("../../../docs/examples/repository-foundation/jameskills.toml")
                .as_slice(),
        ),
        (
            "policies/repository.toml",
            include_bytes!("../../../docs/examples/repository-foundation/policies/repository.toml")
                .as_slice(),
        ),
        (
            "guidance/repository.toml",
            include_bytes!("../../../docs/examples/repository-foundation/guidance/repository.toml")
                .as_slice(),
        ),
    ]
    .into_iter()
    .map(|(path, bytes)| (PortablePath::new(path.to_owned()).unwrap(), bytes.to_vec()))
    .collect()
}

#[test]
fn validated_revision_catalog_tags_are_committed_atomically_and_searchable() {
    let root = TestRoot::new();
    let database = root.0.join("library.sqlite3");
    let store = SqliteStore::open(&database).unwrap();
    let files = valid_files();
    let bundle = validate_bundle(&files).unwrap();
    store.store_validated_bundle(&bundle, &files).unwrap();
    let skill_id = bundle.manifest().id();
    let connection = rusqlite::Connection::open(&database).unwrap();
    connection
        .execute(
            "INSERT INTO skills(id, slug, display_name, created_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                skill_id.as_uuid().to_string(),
                bundle.manifest().slug(),
                bundle.manifest().display_name(),
                "2026-10-07T00:00:00.000Z"
            ],
        )
        .unwrap();
    drop(connection);

    let request = SaveRevisionRequest::new(
        skill_id,
        Some(bundle.content_hash().clone()),
        vec![],
        RevisionKind::Content,
        bundle.manifest().version().to_string(),
        bundle.manifest().schema_version(),
        vec![],
    )
    .with_validated_bundle(&bundle)
    .unwrap();
    let committed = store.commit_revision(&request).unwrap();
    let page = store
        .list_skills(
            &LibraryQuery::new(
                None,
                vec!["SECURITY".to_owned()],
                vec![],
                LibraryItemState::Active,
                None,
                10,
            )
            .unwrap(),
        )
        .unwrap();

    assert_eq!(page.items().len(), 1);
    assert_eq!(page.items()[0].skill_id(), skill_id);
    assert_eq!(page.items()[0].tags(), &["ci", "git", "security"]);
    assert_eq!(
        page.items()[0].heads()[0].revision_id(),
        committed.revision().id()
    );
}
