use jameskills_core::{
    AppError, SkillId,
    application::PublishDraft,
    domain::{CreateSkill, PortablePath, SkillDraft, validate_bundle},
    ports::{LibraryItemState, LibraryQuery, SaveDraftRequest, StoragePort},
};
use jameskills_infra::{
    composition::build_services, platform::UserDirectories, sqlite::SqliteStore,
};
use rusqlite::Connection;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static CASE_COUNTER: AtomicU64 = AtomicU64::new(0);

struct Case {
    root: PathBuf,
    skill_id: SkillId,
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
        "jameskills-draft-storage-{}-{id}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let db = root.join("library.sqlite3");
    let store = SqliteStore::open(&db).unwrap();
    let skill_id = SkillId::parse("f9c0199f-c4ce-4b04-85dd-ae12a7db292b").unwrap();
    Connection::open(&db)
        .unwrap()
        .execute(
            "INSERT INTO skills(id, slug, display_name, created_at) VALUES (?1, 'draft', 'Draft', 'now')",
            [skill_id.as_uuid().to_string()],
        )
        .unwrap();
    Case {
        root,
        skill_id,
        store: Some(store),
    }
}

fn draft_files(markdown: &[u8]) -> BTreeMap<PortablePath, Vec<u8>> {
    [
        (
            PortablePath::new("SKILL.md".to_owned()).unwrap(),
            markdown.to_vec(),
        ),
        (
            PortablePath::new("assets/raw.bin".to_owned()).unwrap(),
            (0..=255).collect(),
        ),
    ]
    .into_iter()
    .collect()
}

#[test]
fn invalid_drafts_roundtrip_exactly_and_generation_cas_never_publishes_heads() {
    let mut case = setup();
    let store = case.store.as_ref().unwrap();
    let storage: &dyn StoragePort = store;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let initial = SkillDraft::new(
        case.skill_id,
        None,
        1,
        draft_files(b"not valid frontmatter and kept verbatim\r\n"),
    )
    .unwrap();
    runtime
        .block_on(storage.save_draft(SaveDraftRequest::new(initial.clone(), None, None).unwrap()))
        .unwrap();
    assert_eq!(
        runtime.block_on(storage.load_draft(case.skill_id)).unwrap(),
        Some(initial.clone())
    );

    let stale_base = jameskills_core::domain::RevisionId::from_digest([9; 32]);
    let rebased = SkillDraft::new(
        case.skill_id,
        Some(stale_base.clone()),
        2,
        draft_files(b"different base must conflict\n"),
    )
    .unwrap();
    assert!(matches!(
        runtime.block_on(
            storage.save_draft(SaveDraftRequest::new(rebased, Some(1), Some(stale_base)).unwrap(),)
        ),
        Err(AppError::Conflict { .. })
    ));

    let updated = initial
        .replace_files(draft_files(b"still invalid; preserve this exact edit\n"))
        .unwrap();
    runtime
        .block_on(
            storage.save_draft(SaveDraftRequest::new(updated.clone(), Some(1), None).unwrap()),
        )
        .unwrap();
    let stale_edit = initial
        .replace_files(draft_files(b"stale concurrent edit\n"))
        .unwrap();
    assert!(matches!(
        runtime.block_on(
            storage.save_draft(SaveDraftRequest::new(stale_edit, Some(1), None).unwrap(),)
        ),
        Err(AppError::Conflict { .. })
    ));
    assert_eq!(
        runtime.block_on(storage.load_draft(case.skill_id)).unwrap(),
        Some(updated.clone())
    );

    let heads: i64 = Connection::open(case.root.join("library.sqlite3"))
        .unwrap()
        .query_row("SELECT COUNT(*) FROM skill_heads", [], |row| row.get(0))
        .unwrap();
    assert_eq!(heads, 0);

    case.store.take();
    let reopened = SqliteStore::open(&case.root.join("library.sqlite3")).unwrap();
    let reopened_port: &dyn StoragePort = &reopened;
    assert_eq!(
        runtime
            .block_on(reopened_port.load_draft(case.skill_id))
            .unwrap(),
        Some(updated)
    );
}

#[test]
fn corrupt_draft_payload_is_rejected_instead_of_returned_as_content() {
    let case = setup();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let storage: &dyn StoragePort = case.store.as_ref().unwrap();
    let draft = SkillDraft::new(case.skill_id, None, 1, draft_files(b"bad")).unwrap();
    runtime
        .block_on(storage.save_draft(SaveDraftRequest::new(draft, None, None).unwrap()))
        .unwrap();
    Connection::open(case.root.join("library.sqlite3"))
        .unwrap()
        .execute(
            "UPDATE drafts SET draft_json = X'00010203' WHERE skill_id = ?1",
            [case.skill_id.as_uuid().to_string()],
        )
        .unwrap();

    assert!(matches!(
        runtime.block_on(storage.load_draft(case.skill_id)),
        Err(AppError::Storage { ref code }) if code == "storage.draft.corrupt"
    ));
}

#[test]
fn draft_write_requires_an_existing_skill_record() {
    let case = setup();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let storage: &dyn StoragePort = case.store.as_ref().unwrap();
    let missing = SkillId::new();
    let draft = SkillDraft::new(missing, None, 1, draft_files(b"invalid draft")).unwrap();
    assert!(matches!(
        runtime.block_on(storage.save_draft(SaveDraftRequest::new(draft, None, None).unwrap())),
        Err(AppError::NotFound)
    ));
}

#[test]
fn runtime_create_skill_persists_valid_initial_template_without_publishing() {
    let mut case = setup();
    let store = case.store.take().unwrap();
    drop(store);
    std::fs::remove_dir_all(&case.root).unwrap();
    let directories = UserDirectories {
        config: case.root.join("config"),
        data: case.root.join("data"),
        cache: case.root.join("cache"),
    };
    let runtime_services = build_services(directories.clone()).unwrap();
    let executor = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let request = CreateSkill::new(
        "repository-basics".to_owned(),
        "Repository basics".to_owned(),
    )
    .unwrap();
    let draft = executor
        .block_on(runtime_services.library().create_skill(request))
        .unwrap();
    assert!(validate_bundle(draft.files()).is_ok());
    assert_eq!(
        executor
            .block_on(runtime_services.library().load_draft(draft.skill_id()))
            .unwrap(),
        Some(draft.clone())
    );
    let page = executor
        .block_on(runtime_services.library().list_skills(
            LibraryQuery::new(None, vec![], vec![], LibraryItemState::Any, None, 10).unwrap(),
        ))
        .unwrap();
    assert_eq!(page.items().len(), 1);
    assert_eq!(page.items()[0].slug(), "repository-basics");
    assert!(page.items()[0].heads().is_empty());
    drop(runtime_services);
    std::fs::remove_dir_all(&case.root).unwrap();
}

#[test]
fn failed_initial_draft_insert_rolls_back_skill_and_catalog_rows() {
    let mut case = setup();
    drop(case.store.take());
    std::fs::remove_dir_all(&case.root).unwrap();
    let directories = UserDirectories {
        config: case.root.join("config"),
        data: case.root.join("data"),
        cache: case.root.join("cache"),
    };
    let runtime_services = build_services(directories.clone()).unwrap();
    Connection::open(directories.data.join("library.sqlite3"))
        .unwrap()
        .execute_batch(
            "CREATE TRIGGER fail_initial_draft BEFORE INSERT ON drafts BEGIN SELECT RAISE(ABORT, 'injected'); END;",
        )
        .unwrap();
    let executor = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let result = executor.block_on(runtime_services.library().create_skill(
        CreateSkill::new("transactional".to_owned(), "Transactional".to_owned()).unwrap(),
    ));
    assert!(matches!(
        result,
        Err(AppError::Storage { ref code }) if code == "storage.skill.create.failed"
    ));
    let connection = Connection::open(directories.data.join("library.sqlite3")).unwrap();
    for table in ["skills", "library_catalog", "drafts"] {
        let count: i64 = connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0, "partial create left rows in {table}");
    }
    drop(connection);
    drop(runtime_services);
    std::fs::remove_dir_all(&case.root).unwrap();
}

#[test]
fn publish_is_validated_idempotent_and_requires_semver_bump_for_changed_content() {
    let id = CASE_COUNTER.fetch_add(1, Ordering::SeqCst);
    let root = std::env::temp_dir().join(format!(
        "jameskills-publish-flow-{}-{id}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let directories = UserDirectories {
        config: root.join("config"),
        data: root.join("data"),
        cache: root.join("cache"),
    };
    let runtime_services = build_services(directories).unwrap();
    let executor = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let initial_draft = executor
        .block_on(runtime_services.library().create_skill(
            CreateSkill::new("publish-flow".to_owned(), "Publish flow".to_owned()).unwrap(),
        ))
        .unwrap();
    let skill_id = initial_draft.skill_id();
    let mut invalid_files = initial_draft.files().clone();
    invalid_files.insert(
        PortablePath::new("jameskills.toml".to_owned()).unwrap(),
        b"this is not valid TOML = [\n".to_vec(),
    );
    let invalid_draft = initial_draft.replace_files(invalid_files).unwrap();
    executor
        .block_on(
            runtime_services
                .library()
                .save_draft(SaveDraftRequest::new(invalid_draft.clone(), Some(1), None).unwrap()),
        )
        .unwrap();
    assert!(matches!(
        executor.block_on(
            runtime_services
                .library()
                .publish(PublishDraft::new(skill_id, 2, vec![]).unwrap(),)
        ),
        Err(AppError::Validation(_))
    ));
    assert_eq!(
        executor
            .block_on(runtime_services.library().load_draft(skill_id))
            .unwrap(),
        Some(invalid_draft.clone())
    );
    let restored = invalid_draft
        .replace_files(initial_draft.files().clone())
        .unwrap();
    executor
        .block_on(
            runtime_services
                .library()
                .save_draft(SaveDraftRequest::new(restored, Some(2), None).unwrap()),
        )
        .unwrap();
    let first = executor
        .block_on(
            runtime_services
                .library()
                .publish(PublishDraft::new(skill_id, 3, vec![]).unwrap()),
        )
        .unwrap();
    assert_eq!(first.revision().semantic_version(), "0.1.0");
    assert!(
        executor
            .block_on(runtime_services.library().load_draft(skill_id))
            .unwrap()
            .is_none()
    );

    let same_draft = SkillDraft::new(
        skill_id,
        Some(first.revision().id().clone()),
        1,
        initial_draft.files().clone(),
    )
    .unwrap();
    executor
        .block_on(
            runtime_services
                .library()
                .save_draft(SaveDraftRequest::new(same_draft, None, None).unwrap()),
        )
        .unwrap();
    let no_op =
        executor
            .block_on(runtime_services.library().publish(
                PublishDraft::new(skill_id, 1, vec![first.revision().id().clone()]).unwrap(),
            ))
            .unwrap();
    assert_eq!(no_op.revision().id(), first.revision().id());

    let mut changed_files = initial_draft.files().clone();
    let skill_path = PortablePath::new("SKILL.md".to_owned()).unwrap();
    changed_files
        .get_mut(&skill_path)
        .unwrap()
        .extend_from_slice(b"\nAdditional valid body text.\n");
    let changed_draft = SkillDraft::new(
        skill_id,
        Some(first.revision().id().clone()),
        1,
        changed_files,
    )
    .unwrap();
    executor
        .block_on(
            runtime_services
                .library()
                .save_draft(SaveDraftRequest::new(changed_draft, None, None).unwrap()),
        )
        .unwrap();
    let same_version = executor.block_on(
        runtime_services
            .library()
            .publish(PublishDraft::new(skill_id, 1, vec![first.revision().id().clone()]).unwrap()),
    );
    assert!(matches!(
        same_version,
        Err(AppError::Validation(ref diagnostics))
            if diagnostics.iter().any(|diagnostic| diagnostic.code() == "revision.version.bump_required")
    ));
    let current_draft = executor
        .block_on(runtime_services.library().load_draft(skill_id))
        .unwrap()
        .unwrap();
    let unchanged = executor
        .block_on(runtime_services.library().load_skill(skill_id))
        .unwrap()
        .unwrap();
    assert_eq!(unchanged.summary().heads().len(), 1);
    assert_eq!(
        unchanged.summary().heads()[0].revision_id(),
        first.revision().id()
    );

    let mut bumped_files = current_draft.files().clone();
    let manifest_path = PortablePath::new("jameskills.toml".to_owned()).unwrap();
    let manifest = String::from_utf8(bumped_files[&manifest_path].clone()).unwrap();
    bumped_files.insert(
        manifest_path,
        manifest
            .replace("\nversion = \"0.1.0\"\n", "\nversion = \"0.2.0\"\n")
            .into_bytes(),
    );
    let bumped = current_draft.replace_files(bumped_files).unwrap();
    executor
        .block_on(runtime_services.library().save_draft(
            SaveDraftRequest::new(bumped, Some(1), Some(first.revision().id().clone())).unwrap(),
        ))
        .unwrap();
    let published =
        executor
            .block_on(runtime_services.library().publish(
                PublishDraft::new(skill_id, 2, vec![first.revision().id().clone()]).unwrap(),
            ))
            .unwrap();
    assert_eq!(published.revision().semantic_version(), "0.2.0");
    assert_ne!(published.revision().id(), first.revision().id());
    assert!(
        executor
            .block_on(runtime_services.library().load_draft(skill_id))
            .unwrap()
            .is_none()
    );

    drop(runtime_services);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn asset_editing_changes_only_the_saved_draft_until_a_later_publish() {
    let id = CASE_COUNTER.fetch_add(1, Ordering::SeqCst);
    let root = std::env::temp_dir().join(format!(
        "jameskills-asset-draft-{}-{id}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let runtime_services = build_services(UserDirectories {
        config: root.join("config"),
        data: root.join("data"),
        cache: root.join("cache"),
    })
    .unwrap();
    let executor = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let initial = executor
        .block_on(runtime_services.library().create_skill(
            CreateSkill::new("asset-flow".to_owned(), "Asset flow".to_owned()).unwrap(),
        ))
        .unwrap();
    let published = executor
        .block_on(
            runtime_services
                .library()
                .publish(PublishDraft::new(initial.skill_id(), 1, vec![]).unwrap()),
        )
        .unwrap();
    let editable = SkillDraft::new(
        initial.skill_id(),
        Some(published.revision().id().clone()),
        1,
        initial.files().clone(),
    )
    .unwrap();
    executor
        .block_on(
            runtime_services
                .library()
                .save_draft(SaveDraftRequest::new(editable, None, None).unwrap()),
        )
        .unwrap();

    let with_asset = executor
        .block_on(runtime_services.library().add_asset(
            initial.skill_id(),
            1,
            "references/guide.md",
            b"A plain text reference.\n".to_vec(),
        ))
        .unwrap();
    let preview = executor
        .block_on(
            runtime_services
                .library()
                .preview_asset(initial.skill_id(), "references/guide.md"),
        )
        .unwrap();
    assert_eq!(preview.text(), Some("A plain text reference.\n"));
    assert_eq!(with_asset.generation(), 2);

    let renamed = executor
        .block_on(runtime_services.library().rename_asset(
            initial.skill_id(),
            2,
            "references/guide.md",
            "assets/guide.md",
            preview.content_hash(),
        ))
        .unwrap();
    assert_eq!(renamed.generation(), 3);
    let renamed_preview = executor
        .block_on(
            runtime_services
                .library()
                .preview_asset(initial.skill_id(), "assets/guide.md"),
        )
        .unwrap();
    let removed = executor
        .block_on(runtime_services.library().remove_asset(
            initial.skill_id(),
            3,
            "assets/guide.md",
            renamed_preview.content_hash(),
        ))
        .unwrap();
    assert_eq!(removed.generation(), 4);
    assert!(
        removed
            .files()
            .keys()
            .all(|path| path.as_str() != "assets/guide.md")
    );

    assert!(matches!(
        executor.block_on(runtime_services.library().add_asset(
            initial.skill_id(),
            3,
            "assets/stale.txt",
            b"stale edit".to_vec(),
        )),
        Err(AppError::Conflict { .. })
    ));
    let unchanged = executor
        .block_on(runtime_services.library().load_skill(initial.skill_id()))
        .unwrap()
        .unwrap();
    assert_eq!(unchanged.summary().heads().len(), 1);
    assert_eq!(
        unchanged.summary().heads()[0].revision_id(),
        published.revision().id()
    );

    drop(runtime_services);
    std::fs::remove_dir_all(root).unwrap();
}
