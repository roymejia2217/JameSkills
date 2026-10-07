use jameskills_core::{
    domain::{PortablePath, SkillDraft, SkillId},
    ports::{LibraryHistoryQuery, LibraryItemState, LibraryQuery, SaveDraftRequest},
};
use jameskills_infra::{composition::build_services, platform::UserDirectories};
use rusqlite::Connection;
use std::{collections::BTreeMap, path::PathBuf};

#[test]
fn composed_library_service_validates_a_real_bundle_and_opens_its_local_catalog() {
    let root = std::env::temp_dir().join(format!("jameskills-validation-{}", std::process::id()));
    let directories = UserDirectories {
        config: root.join("config"),
        data: root.join("data"),
        cache: root.join("cache"),
    };
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/repository-foundation");

    let runtime = build_services(directories.clone()).unwrap();
    let draft_skill = SkillId::parse("f9c0199f-c4ce-4b04-85dd-ae12a7db292b").unwrap();
    let connection = Connection::open(directories.data.join("library.sqlite3")).unwrap();
    connection
        .execute(
            "INSERT INTO skills(id, slug, display_name, created_at) VALUES (?1, 'draft', 'Draft', 'now')",
            [draft_skill.as_uuid().to_string()],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO library_catalog(skill_id, normalized_display_name) VALUES (?1, 'draft')",
            [draft_skill.as_uuid().to_string()],
        )
        .unwrap();
    drop(connection);
    let report = runtime.library().validate_import(&fixture).unwrap();
    let runtime_executor = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let files = [(
        PortablePath::new("SKILL.md".to_owned()).unwrap(),
        b"invalid draft bytes are still user work\r\n".to_vec(),
    )]
    .into_iter()
    .collect::<BTreeMap<_, _>>();
    let draft = SkillDraft::new(draft_skill, None, 1, files).unwrap();
    runtime_executor
        .block_on(
            runtime
                .library()
                .save_draft(SaveDraftRequest::new(draft.clone(), None, None).unwrap()),
        )
        .unwrap();
    assert_eq!(
        runtime_executor
            .block_on(runtime.library().load_draft(draft_skill))
            .unwrap(),
        Some(draft)
    );
    let page = runtime_executor
        .block_on(runtime.library().list_skills(
            LibraryQuery::new(None, vec![], vec![], LibraryItemState::Any, None, 50).unwrap(),
        ))
        .unwrap();
    let missing_skill = SkillId::parse("f9c0199f-c4ce-4b04-85dd-ae12a7db292c").unwrap();
    assert!(
        runtime_executor
            .block_on(runtime.library().load_skill(missing_skill))
            .unwrap()
            .is_none()
    );
    let empty_history = runtime_executor
        .block_on(
            runtime
                .library()
                .load_history(LibraryHistoryQuery::new(missing_skill, None, 10).unwrap()),
        )
        .unwrap();
    assert!(empty_history.entries().is_empty());

    assert_eq!(report.manifest().slug(), "repository-foundation");
    assert_eq!(report.file_count(), 10);
    assert_eq!(page.items().len(), 1);
    assert_eq!(page.items()[0].slug(), "draft");
    assert!(page.items()[0].heads().is_empty());
    assert!(!directories.config.exists());
    assert!(directories.data.join("library.sqlite3").is_file());
    assert!(!directories.cache.exists());
    drop(runtime);
    std::fs::remove_dir_all(root).unwrap();
}
