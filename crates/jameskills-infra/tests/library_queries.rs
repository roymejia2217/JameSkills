use jameskills_core::{
    SkillId,
    domain::{
        ContentHash, PortablePath, RevisionKind, SaveRevisionRequest, hash_bundle, validate_bundle,
    },
    ports::{
        BundleFiles, LibraryCursor, LibraryHistoryQuery, LibraryItemState, LibraryQuery,
        StoragePort, write_bundle_archive,
    },
};
use jameskills_infra::{
    fs::{read_bundle, store_blob_bytes},
    sqlite::SqliteStore,
};
use rusqlite::{Connection, params};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static CASE_COUNTER: AtomicU64 = AtomicU64::new(0);

struct Case {
    root: PathBuf,
    store: Option<SqliteStore>,
}

impl Drop for Case {
    fn drop(&mut self) {
        self.store.take();
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn setup() -> Case {
    let id = CASE_COUNTER.fetch_add(1, Ordering::SeqCst);
    let root = std::env::temp_dir().join(format!(
        "jameskills-library-query-{}-{id}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let db = root.join("library.sqlite3");
    let store = SqliteStore::open(&db).unwrap();
    let insert_skill = |connection: &Connection, id: &str, slug: &str, name: &str| {
        connection
            .execute(
                "INSERT INTO skills(id, slug, display_name, created_at) VALUES (?1, ?2, ?3, 'now')",
                params![id, slug, name],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO library_catalog(skill_id, normalized_display_name) VALUES (?1, ?2)",
                params![id, name.to_lowercase()],
            )
            .unwrap();
    };
    let connection = Connection::open(&db).unwrap();
    insert_skill(
        &connection,
        "00000000-0000-4000-8000-000000000001",
        "alpha",
        "Alpha",
    );
    insert_skill(
        &connection,
        "00000000-0000-4000-8000-000000000002",
        "beta",
        "Beta",
    );
    insert_skill(
        &connection,
        "00000000-0000-4000-8000-000000000003",
        "deleted",
        "Deleted",
    );
    insert_skill(
        &connection,
        "00000000-0000-4000-8000-000000000004",
        "percent",
        "Percent % literal",
    );
    insert_revision(&connection, 1, 1, "content");
    insert_revision(&connection, 2, 2, "content");
    insert_revision(&connection, 3, 2, "content");
    insert_revision(&connection, 4, 3, "tombstone");
    insert_revision(&connection, 5, 4, "content");
    for (skill, revision) in [(1, 1), (2, 2), (2, 3), (3, 4), (4, 5)] {
        connection
            .execute(
                "INSERT INTO skill_heads(skill_id, revision_id) VALUES (?1, ?2)",
                params![skill_id(skill), revision_id(revision)],
            )
            .unwrap();
    }
    connection
        .execute(
            "INSERT INTO revision_tags(revision_id, tag) VALUES (?1, 'shared')",
            [revision_id(2)],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO revision_capabilities(revision_id, capability) VALUES (?1, 'repository-read')",
            [revision_id(3)],
        )
        .unwrap();
    drop(connection);
    Case {
        root,
        store: Some(store),
    }
}

fn skill_id(number: u8) -> String {
    format!("00000000-0000-4000-8000-{:012}", number)
}

fn revision_id(number: u8) -> String {
    format!("{:064x}", number)
}

fn insert_revision(connection: &Connection, revision: u8, skill: u8, state: &str) {
    connection
        .execute(
            "INSERT INTO revisions(id, skill_id, bundle_hash, semantic_version, schema_version, state, created_at) VALUES (?1, ?2, ?3, '1.0.0', 1, ?4, 'now')",
            params![revision_id(revision), skill_id(skill), "0".repeat(64), state],
        )
        .unwrap();
}

fn insert_catalog_skill(connection: &Connection, id: &str, name: &str) {
    connection
        .execute(
            "INSERT INTO skills(id, slug, display_name, created_at) VALUES (?1, ?2, ?3, 'now')",
            params![id, id, name],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO library_catalog(skill_id, normalized_display_name) VALUES (?1, ?2)",
            params![
                id,
                name.chars()
                    .flat_map(char::to_lowercase)
                    .collect::<String>()
            ],
        )
        .unwrap();
}

#[test]
fn catalog_pages_are_stable_bounded_and_include_conflicting_heads() {
    let case = setup();
    let first = case
        .store
        .as_ref()
        .unwrap()
        .list_skills(
            &LibraryQuery::new(None, vec![], vec![], LibraryItemState::Any, None, 2).unwrap(),
        )
        .unwrap();
    assert_eq!(
        first
            .items()
            .iter()
            .map(|item| item.slug())
            .collect::<Vec<_>>(),
        vec!["alpha", "beta"]
    );
    assert_eq!(first.items()[1].heads().len(), 2);
    assert!(first.items()[1].conflicted());
    let cursor = first.next().unwrap();
    assert_eq!(cursor.normalized_display_name(), "beta");

    let second = case
        .store
        .as_ref()
        .unwrap()
        .list_skills(
            &LibraryQuery::new(
                None,
                vec![],
                vec![],
                LibraryItemState::Any,
                Some(cursor.clone()),
                2,
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(
        second
            .items()
            .iter()
            .map(|item| item.slug())
            .collect::<Vec<_>>(),
        vec!["deleted", "percent"]
    );
    assert!(second.next().is_none());
}

#[test]
fn storage_port_lists_catalog_on_the_blocking_pool_through_its_object_safe_api() {
    let case = setup();
    let storage: &dyn StoragePort = case.store.as_ref().unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let page = runtime
        .block_on(storage.list_skills(
            LibraryQuery::new(None, vec![], vec![], LibraryItemState::Any, None, 1).unwrap(),
        ))
        .unwrap();

    assert_eq!(page.items().len(), 1);
    assert!(page.next().is_some());
}

#[test]
fn catalog_search_escapes_like_metacharacters_and_applies_metadata_state_filters() {
    let case = setup();
    let percent = case
        .store
        .as_ref()
        .unwrap()
        .list_skills(
            &LibraryQuery::new(Some("%"), vec![], vec![], LibraryItemState::Any, None, 50).unwrap(),
        )
        .unwrap();
    assert_eq!(
        percent
            .items()
            .iter()
            .map(|item| item.slug())
            .collect::<Vec<_>>(),
        vec!["percent"]
    );

    let metadata = case
        .store
        .as_ref()
        .unwrap()
        .list_skills(
            &LibraryQuery::new(
                None,
                vec!["shared".to_owned()],
                vec!["repository-read".to_owned()],
                LibraryItemState::Conflicted,
                None,
                50,
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(metadata.items().len(), 1);
    assert_eq!(metadata.items()[0].slug(), "beta");
    assert_eq!(metadata.items()[0].tags(), &["shared"]);
    assert_eq!(metadata.items()[0].capabilities(), &["repository-read"]);

    let deleted = case
        .store
        .as_ref()
        .unwrap()
        .list_skills(
            &LibraryQuery::new(None, vec![], vec![], LibraryItemState::Deleted, None, 50).unwrap(),
        )
        .unwrap();
    assert_eq!(
        deleted
            .items()
            .iter()
            .map(|item| item.slug())
            .collect::<Vec<_>>(),
        vec!["deleted"]
    );

    let cursor = LibraryCursor::new("Alpha", SkillId::parse(&skill_id(1)).unwrap()).unwrap();
    assert_eq!(cursor.normalized_display_name(), "alpha");

    let unicode = case
        .store
        .as_ref()
        .unwrap()
        .list_skills(
            &LibraryQuery::new(Some("ALP"), vec![], vec![], LibraryItemState::Any, None, 50)
                .unwrap(),
        )
        .unwrap();
    assert_eq!(
        unicode
            .items()
            .iter()
            .map(|item| item.slug())
            .collect::<Vec<_>>(),
        vec!["alpha"]
    );
}

#[test]
fn large_catalog_pagination_does_not_repeat_or_omit_skills() {
    let case = setup();
    let connection = Connection::open(case.root.join("library.sqlite3")).unwrap();
    for number in 10..130 {
        let id = format!("00000000-0000-4000-8000-{number:012}");
        let name = format!("Skill {number:03}");
        insert_catalog_skill(&connection, &id, &name);
        let revision = format!("{:064x}", number);
        connection
            .execute(
                "INSERT INTO revisions(id, skill_id, bundle_hash, semantic_version, schema_version, state, created_at) VALUES (?1, ?2, ?3, '1.0.0', 1, 'content', 'now')",
                params![revision, id, "0".repeat(64)],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO skill_heads(skill_id, revision_id) VALUES (?1, ?2)",
                params![id, revision],
            )
            .unwrap();
    }
    let duplicate_id = "00000000-0000-4000-8000-000000000200";
    let duplicate_revision = revision_id(200);
    insert_catalog_skill(&connection, duplicate_id, "Skill 010");
    connection
        .execute(
            "INSERT INTO revisions(id, skill_id, bundle_hash, semantic_version, schema_version, state, created_at) VALUES (?1, ?2, ?3, '1.0.0', 1, 'content', 'now')",
            params![duplicate_revision, duplicate_id, "0".repeat(64)],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO skill_heads(skill_id, revision_id) VALUES (?1, ?2)",
            params![duplicate_id, duplicate_revision],
        )
        .unwrap();
    drop(connection);

    let mut cursor = None;
    let mut listed = Vec::new();
    let mut order_keys = Vec::new();
    loop {
        let page = case
            .store
            .as_ref()
            .unwrap()
            .list_skills(
                &LibraryQuery::new(None, vec![], vec![], LibraryItemState::Any, cursor, 17)
                    .unwrap(),
            )
            .unwrap();
        assert!(page.items().len() <= 17);
        for item in page.items() {
            let skill_id = item.skill_id().as_uuid().to_string();
            order_keys.push((
                item.display_name()
                    .chars()
                    .flat_map(char::to_lowercase)
                    .collect::<String>(),
                skill_id.clone(),
            ));
            listed.push(skill_id);
        }
        cursor = page.next().cloned();
        if cursor.is_none() {
            break;
        }
    }
    assert!(order_keys.windows(2).all(|pair| pair[0] <= pair[1]));
    listed.sort();
    listed.dedup();
    assert_eq!(listed.len(), 125);
}

#[test]
fn reopening_backfills_unicode_normalized_names_for_legacy_catalog_rows() {
    let id = CASE_COUNTER.fetch_add(1, Ordering::SeqCst);
    let root = std::env::temp_dir().join(format!(
        "jameskills-unicode-catalog-{}-{id}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let db = root.join("library.sqlite3");
    let first_open = SqliteStore::open(&db).unwrap();
    let connection = Connection::open(&db).unwrap();
    connection
        .execute(
            "INSERT INTO skills(id, slug, display_name, created_at) VALUES (?1, 'alpha', 'Älpha', 'now')",
            [skill_id(1)],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO library_catalog(skill_id, normalized_display_name) VALUES (?1, 'Älpha')",
            [skill_id(1)],
        )
        .unwrap();
    connection.execute_batch("PRAGMA user_version = 3").unwrap();
    drop(connection);
    drop(first_open);
    let case = Case {
        root,
        store: Some(SqliteStore::open(&db).unwrap()),
    };

    let page = case
        .store
        .as_ref()
        .unwrap()
        .list_skills(
            &LibraryQuery::new(Some("ÄLP"), vec![], vec![], LibraryItemState::Any, None, 50)
                .unwrap(),
        )
        .unwrap();
    assert_eq!(
        page.items()
            .iter()
            .map(|item| item.slug())
            .collect::<Vec<_>>(),
        vec!["alpha"]
    );
}

#[test]
fn selected_skill_loads_verified_bytes_and_history_keeps_causal_tombstone_data() {
    let id = CASE_COUNTER.fetch_add(1, Ordering::SeqCst);
    let root = std::env::temp_dir().join(format!(
        "jameskills-skill-detail-{}-{id}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let db = root.join("library.sqlite3");
    let store = SqliteStore::open(&db).unwrap();
    let files: BundleFiles = [(
        PortablePath::new("SKILL.md".to_owned()).unwrap(),
        b"# Selected skill\n".to_vec(),
    )]
    .into_iter()
    .collect();
    let archive = write_bundle_archive(&files).unwrap();
    let (inventory, recovered) = read_bundle(&archive).unwrap();
    assert_eq!(recovered, files);
    let bundle_hash = hash_bundle(&inventory, &files).unwrap();
    store_blob_bytes(&root.join("blobs"), &bundle_hash, &archive).unwrap();

    let skill = skill_id(1);
    let content_revision = revision_id(1);
    let tombstone_revision = revision_id(2);
    let connection = Connection::open(&db).unwrap();
    connection
        .execute(
            "INSERT INTO skills(id, slug, display_name, created_at) VALUES (?1, 'selected', 'Selected skill', 'now')",
            [&skill],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO library_catalog(skill_id, normalized_display_name) VALUES (?1, 'selected skill')",
            [&skill],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO revisions(id, skill_id, bundle_hash, semantic_version, schema_version, state, created_at) VALUES (?1, ?2, ?3, '1.0.0', 1, 'content', 'now')",
            params![content_revision, skill, bundle_hash.as_str()],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO revisions(id, skill_id, bundle_hash, semantic_version, schema_version, state, created_at) VALUES (?1, ?2, ?3, '1.0.0', 1, 'tombstone', 'now')",
            params![tombstone_revision, skill, ContentHash::from_digest([0; 32]).as_str()],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO revision_parents(revision_id, parent_revision_id) VALUES (?1, ?2)",
            params![tombstone_revision, content_revision],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO deletions(skill_id, deletion_revision_id, observed_heads_json) VALUES (?1, ?2, ?3)",
            params![skill, tombstone_revision, format!("[\"{content_revision}\"]")],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO skill_heads(skill_id, revision_id) VALUES (?1, ?2)",
            params![skill, content_revision],
        )
        .unwrap();
    drop(connection);

    let skill_id = SkillId::parse(&skill).unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let storage: &dyn StoragePort = &store;
    let detail = runtime
        .block_on(storage.load_skill(skill_id))
        .unwrap()
        .unwrap();
    assert_eq!(detail.summary().slug(), "selected");
    assert_eq!(detail.heads().len(), 1);
    assert_eq!(detail.heads()[0].files(), Some(&files));

    let history = runtime
        .block_on(storage.load_history(LibraryHistoryQuery::new(skill_id, None, 1).unwrap()))
        .unwrap();
    assert_eq!(history.entries().len(), 1);
    assert!(history.next().is_some());
    let tombstones =
        runtime
            .block_on(storage.load_history(
                LibraryHistoryQuery::new(skill_id, history.next().cloned(), 1).unwrap(),
            ))
            .unwrap();
    assert_eq!(tombstones.entries().len(), 1);
    let tombstone = &tombstones.entries()[0];
    assert!(tombstone.deleted());
    assert_eq!(tombstone.bundle_hash(), None);
    assert_eq!(
        tombstone.parents(),
        &[jameskills_core::domain::RevisionId::parse_hex(&content_revision).unwrap()]
    );
    assert_eq!(tombstone.observed_heads(), tombstone.parents());
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn validated_revision_commit_populates_searchable_catalog_metadata_atomically() {
    let id = CASE_COUNTER.fetch_add(1, Ordering::SeqCst);
    let root = std::env::temp_dir().join(format!(
        "jameskills-catalog-index-{}-{id}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let db = root.join("library.sqlite3");
    let store = SqliteStore::open(&db).unwrap();
    let files: BundleFiles = [
        (
            PortablePath::new("SKILL.md".to_owned()).unwrap(),
            include_bytes!("../../../docs/examples/repository-foundation/SKILL.md").to_vec(),
        ),
        (
            PortablePath::new("jameskills.toml".to_owned()).unwrap(),
            include_bytes!("../../../docs/examples/repository-foundation/jameskills.toml").to_vec(),
        ),
        (
            PortablePath::new("policies/repository.toml".to_owned()).unwrap(),
            include_bytes!("../../../docs/examples/repository-foundation/policies/repository.toml")
                .to_vec(),
        ),
        (
            PortablePath::new("guidance/repository.toml".to_owned()).unwrap(),
            include_bytes!("../../../docs/examples/repository-foundation/guidance/repository.toml")
                .to_vec(),
        ),
    ]
    .into_iter()
    .collect();
    let validated = validate_bundle(&files).unwrap();
    let archive = write_bundle_archive(&files).unwrap();
    let (inventory, recovered) = read_bundle(&archive).unwrap();
    let content_hash = hash_bundle(&inventory, &recovered).unwrap();
    assert_eq!(content_hash, *validated.content_hash());
    store_blob_bytes(&root.join("blobs"), &content_hash, &archive).unwrap();
    let manifest = validated.manifest();
    let skill_id = manifest.id();
    let connection = Connection::open(&db).unwrap();
    connection
        .execute(
            "INSERT INTO skills(id, slug, display_name, created_at) VALUES (?1, ?2, ?3, 'now')",
            params![
                skill_id.as_uuid().to_string(),
                manifest.slug(),
                manifest.display_name()
            ],
        )
        .unwrap();
    drop(connection);

    let mismatched = SaveRevisionRequest::new(
        SkillId::new(),
        Some(content_hash.clone()),
        vec![],
        RevisionKind::Content,
        manifest.version().to_string(),
        manifest.schema_version(),
        vec![],
    )
    .with_validated_bundle(&validated);
    assert!(mismatched.is_err());

    let request = SaveRevisionRequest::new(
        skill_id,
        Some(content_hash),
        vec![],
        RevisionKind::Content,
        manifest.version().to_string(),
        manifest.schema_version(),
        vec![],
    )
    .with_validated_bundle(&validated)
    .unwrap();
    let committed = store.commit_revision(&request).unwrap();
    let page = store
        .list_skills(
            &LibraryQuery::new(
                None,
                vec!["CI".to_owned()],
                vec![],
                LibraryItemState::Active,
                None,
                50,
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(page.items().len(), 1);
    assert_eq!(page.items()[0].skill_id(), skill_id);
    assert_eq!(
        page.items()[0].heads()[0].revision_id(),
        committed.revision().id()
    );
    assert_eq!(page.items()[0].tags(), &["ci", "git", "security"]);
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
