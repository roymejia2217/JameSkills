use jameskills_core::{
    application::{ExportRequest, PublishDraft},
    domain::{
        ContentHash, CreateSkill, RestoreRevisionRequest, RevisionId, SkillId,
        import::ImportSourceKind,
    },
    ports::{LibraryCursor, LibraryPage, LibrarySkillSummary},
};
use jameskills_desktop::views::library::{
    LibraryCatalogState, LibraryLoadState, LibraryOperationPhase, LibraryOperationState,
    LibraryPageDisposition,
};
use jameskills_desktop::{file_dialog::NativeFileDialog, services::DesktopServices};
use jameskills_infra::{composition::build_services, platform::UserDirectories};
use std::{path::PathBuf, sync::Arc};

struct TempRoot(PathBuf);

impl TempRoot {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "jameskills-library-flow-{}-{id}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn item(skill_id: SkillId, slug: &str, name: &str) -> LibrarySkillSummary {
    LibrarySkillSummary::new(
        skill_id,
        slug.to_owned(),
        name.to_owned(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
}

#[test]
fn stale_search_page_and_failure_do_not_replace_newer_catalog_or_selection() {
    let mut catalog = LibraryCatalogState::new();
    let first_id = SkillId::new();
    let older = catalog.begin_search("first query".to_owned()).unwrap();
    assert_eq!(catalog.load_state(), LibraryLoadState::Loading);
    assert_eq!(
        catalog.apply_page(
            older.generation(),
            LibraryPage::new(vec![item(first_id, "first", "First skill")], None,),
        ),
        LibraryPageDisposition::Applied
    );
    assert_eq!(catalog.load_state(), LibraryLoadState::Ready);
    assert!(catalog.select_skill(first_id));

    let newer = catalog.begin_search("second query".to_owned()).unwrap();
    let stale_page = LibraryPage::new(vec![item(SkillId::new(), "stale", "Stale result")], None);
    assert_eq!(
        catalog.apply_page(older.generation(), stale_page),
        LibraryPageDisposition::IgnoredStale
    );
    assert_eq!(catalog.generation(), newer.generation());
    assert_eq!(catalog.items()[0].skill_id(), first_id);
    assert_eq!(catalog.selected_skill(), Some(first_id));
    assert_eq!(catalog.load_state(), LibraryLoadState::Loading);

    assert_eq!(
        catalog.apply_failure(older.generation()),
        LibraryPageDisposition::IgnoredStale
    );
    assert_eq!(catalog.load_state(), LibraryLoadState::Loading);
    assert_eq!(catalog.selected_skill(), Some(first_id));
}

#[test]
fn current_page_updates_empty_state_and_invalid_query_does_not_advance_generation() {
    let mut catalog = LibraryCatalogState::new();
    let query = catalog.begin_search("   ".to_owned()).unwrap();
    assert!(query.query().search().is_none());
    assert_eq!(catalog.search_query(), "");
    assert_eq!(
        catalog.apply_page(query.generation(), LibraryPage::new(Vec::new(), None)),
        LibraryPageDisposition::Applied
    );
    assert_eq!(catalog.load_state(), LibraryLoadState::Empty);
    let generation = catalog.generation();

    assert!(catalog.begin_search("x".repeat(257)).is_err());
    assert_eq!(catalog.generation(), generation);
    assert_eq!(catalog.load_state(), LibraryLoadState::Empty);
}

#[test]
fn current_search_reconciles_selection_by_skill_id() {
    let mut catalog = LibraryCatalogState::new();
    let selected = SkillId::new();
    let initial = catalog.begin_search(String::new()).unwrap();
    catalog.apply_page(
        initial.generation(),
        LibraryPage::new(vec![item(selected, "selected", "Selected")], None),
    );
    assert!(catalog.select_skill(selected));

    let refresh = catalog.begin_search("different query".to_owned()).unwrap();
    catalog.apply_page(
        refresh.generation(),
        LibraryPage::new(vec![item(SkillId::new(), "other", "Other")], None),
    );

    assert_eq!(catalog.selected_skill(), None);
}

#[test]
fn cursor_pagination_retains_search_and_supports_previous_next_pages() {
    let mut catalog = LibraryCatalogState::new();
    let first_id = SkillId::new();
    let first = catalog.begin_search("helper".to_owned()).unwrap();
    let cursor = LibraryCursor::new("First helper", first_id).unwrap();
    catalog.apply_page(
        first.generation(),
        LibraryPage::new(
            vec![item(first_id, "first-helper", "First helper")],
            Some(cursor.clone()),
        ),
    );
    assert!(catalog.select_skill(first_id));
    assert!(catalog.can_next());

    let second = catalog.begin_next_page().unwrap().unwrap();
    assert_eq!(second.query().search(), Some("helper"));
    assert_eq!(second.query().after(), Some(&cursor));
    assert_eq!(catalog.page_number(), 2);
    assert!(!catalog.can_previous());
    catalog.apply_page(
        second.generation(),
        LibraryPage::new(
            vec![item(SkillId::new(), "second-helper", "Second helper")],
            None,
        ),
    );
    assert_eq!(catalog.selected_skill(), Some(first_id));
    assert!(catalog.can_previous());
    assert!(!catalog.can_next());

    let back = catalog.begin_previous_page().unwrap().unwrap();
    assert_eq!(back.query().search(), Some("helper"));
    assert_eq!(back.query().after(), None);
    assert_eq!(catalog.page_number(), 1);
}

#[test]
fn page_history_is_bounded_without_changing_the_absolute_page_number() {
    let mut catalog = LibraryCatalogState::new();
    let first_id = SkillId::new();
    let first = catalog.begin_search(String::new()).unwrap();
    catalog.apply_page(
        first.generation(),
        LibraryPage::new(
            vec![item(first_id, "skill-1", "Skill 1")],
            Some(LibraryCursor::new("Skill 1", first_id).unwrap()),
        ),
    );

    for page_number in 2..=24 {
        let request = catalog.begin_next_page().unwrap().unwrap();
        let skill_id = SkillId::new();
        let next = (page_number < 24)
            .then(|| LibraryCursor::new(&format!("Skill {page_number}"), skill_id).unwrap());
        catalog.apply_page(
            request.generation(),
            LibraryPage::new(
                vec![item(
                    skill_id,
                    &format!("skill-{page_number}"),
                    &format!("Skill {page_number}"),
                )],
                next,
            ),
        );
    }

    assert_eq!(catalog.page_number(), 24);
    let previous = catalog.begin_previous_page().unwrap().unwrap();
    assert_eq!(catalog.page_number(), 23);
    assert!(previous.query().after().is_some());
}

#[test]
fn import_workflow_binds_preview_resolution_and_apply_to_one_generation() {
    let draft = CreateSkill::new("import-demo".to_owned(), "Import demo".to_owned())
        .unwrap()
        .initial_draft()
        .unwrap();
    let preview = jameskills_core::domain::ImportPreview::new(
        draft.files().clone(),
        ImportSourceKind::Directory,
        false,
        Vec::new(),
        None,
    )
    .unwrap();
    let mut operation = LibraryOperationState::new();
    let generation = operation.begin_import_selection().unwrap();
    assert!(operation.begin_import_preview(generation));
    assert!(operation.set_import_preview(generation, preview.clone()));
    assert_eq!(operation.phase(), LibraryOperationPhase::ImportPreview);
    assert!(operation.begin_import_apply(generation).is_none());
    assert!(operation.import_preview().is_some());

    let resolution = jameskills_core::domain::ImportResolution::AddConcurrentRoot;
    let digest = operation
        .select_import_resolution(generation, resolution)
        .unwrap();
    let (applied_preview, applied_resolution, applied_digest) = operation
        .begin_import_apply(generation)
        .expect("explicit preview resolution permits apply");
    assert_eq!(applied_preview.content_hash(), preview.content_hash());
    assert_eq!(applied_resolution, resolution);
    assert_eq!(applied_digest, digest);
    assert_eq!(operation.phase(), LibraryOperationPhase::ApplyingImport);
    assert!(operation.cancel().is_err());
    assert!(operation.begin_create().is_err());
    assert!(operation.complete(generation));
    assert_eq!(operation.phase(), LibraryOperationPhase::Complete);
    assert!(!operation.fail(generation));

    let stale = operation.begin_import_selection().unwrap();
    assert!(operation.begin_import_preview(stale));
    operation.cancel().unwrap();
    assert!(!operation.set_import_preview(stale, preview));
    assert_eq!(operation.phase(), LibraryOperationPhase::Idle);
    assert!(!operation.fail(stale));
}

#[test]
fn export_workflow_requires_the_previewed_overwrite_digest() {
    let root = TempRoot::new();
    let runtime = build_services(UserDirectories {
        config: root.0.join("config"),
        data: root.0.join("data"),
        cache: root.0.join("cache"),
    })
    .unwrap();
    let services =
        DesktopServices::from_runtime_services(runtime, Arc::new(NativeFileDialog)).unwrap();
    let library = services.runtime_services().library();
    let draft = services
        .run_io(library.create_skill(
            CreateSkill::new("export-demo".to_owned(), "Export demo".to_owned()).unwrap(),
        ))
        .unwrap();
    let published =
        services
            .run_io(library.publish(
                PublishDraft::new(draft.skill_id(), draft.generation(), Vec::new()).unwrap(),
            ))
            .unwrap();
    let output_dir = root.0.join("output");
    std::fs::create_dir_all(&output_dir).unwrap();
    let destination = output_dir.join("export-demo.jskill");
    std::fs::write(&destination, b"user data before preview").unwrap();
    let preview = services
        .run_io(library.preview_export(
            ExportRequest::new(draft.skill_id(), Some(published.revision().id().clone())),
            destination,
        ))
        .unwrap();

    let mut operation = LibraryOperationState::new();
    let generation = operation.begin_export_selection().unwrap();
    assert!(operation.begin_export_preview(generation));
    assert!(operation.set_export_preview(generation, preview));
    assert!(operation.begin_export_apply(generation).is_none());
    assert!(operation.export_preview().is_some());
    assert!(
        operation
            .select_export_overwrite(generation, false)
            .is_err()
    );
    assert_eq!(operation.phase(), LibraryOperationPhase::ExportPreview);
    let digest = operation.select_export_overwrite(generation, true).unwrap();
    let (preview, overwrite, confirmation) = operation
        .begin_export_apply(generation)
        .expect("confirmed export is ready to apply");
    assert!(overwrite);
    assert_eq!(confirmation, digest);
    assert!(preview.confirmation_digest(overwrite).is_ok());
    assert!(operation.complete(generation));
}

#[test]
fn history_page_generation_and_content_selection_reject_stale_or_deleted_revisions() {
    use jameskills_core::ports::{LibraryHistoryEntry, LibraryHistoryPage};

    let mut operation = LibraryOperationState::new();
    let skill_id = SkillId::new();
    let (generation, query) = operation.begin_history(skill_id).unwrap();
    assert_eq!(query.skill_id(), skill_id);
    let content_revision = RevisionId::from_digest([21; 32]);
    let tombstone = RevisionId::from_digest([22; 32]);
    let next_cursor = RevisionId::from_digest([24; 32]);
    let page = LibraryHistoryPage::new(
        vec![
            LibraryHistoryEntry::new(
                tombstone.clone(),
                vec![content_revision.clone()],
                None,
                "1.0.0".to_owned(),
                true,
                vec![content_revision.clone()],
            ),
            LibraryHistoryEntry::new(
                content_revision.clone(),
                Vec::new(),
                Some(ContentHash::from_digest([23; 32])),
                "1.0.0".to_owned(),
                false,
                Vec::new(),
            ),
        ],
        Some(next_cursor.clone()),
    );
    assert!(operation.set_history_page(generation, page));
    assert_eq!(operation.phase(), LibraryOperationPhase::HistoryReady);
    assert!(!operation.select_history_revision(generation, &tombstone));
    assert!(operation.select_history_revision(generation, &content_revision));
    assert_eq!(
        operation.history_selected_revision(),
        Some(&content_revision)
    );

    let (next_generation, next_query) = operation.begin_next_history_page().unwrap().unwrap();
    assert_eq!(next_query.after(), Some(&next_cursor));
    assert!(!operation.set_history_page(generation, LibraryHistoryPage::new(Vec::new(), None)));
    assert!(operation.set_history_page(next_generation, LibraryHistoryPage::new(Vec::new(), None)));

    let (fresh_generation, _) = operation.begin_history(skill_id).unwrap();
    assert!(
        !operation.set_history_page(next_generation, LibraryHistoryPage::new(Vec::new(), None))
    );
    assert!(
        operation.set_history_page(fresh_generation, LibraryHistoryPage::new(Vec::new(), None))
    );
}

#[test]
fn delete_and_restore_workflows_retain_causal_expected_heads_until_apply() {
    let mut operation = LibraryOperationState::new();
    let skill_id = SkillId::new();
    let content_revision = RevisionId::from_digest([31; 32]);
    let expected_heads = vec![content_revision.clone()];
    let delete_generation = operation
        .begin_delete_confirmation(skill_id, expected_heads.clone())
        .unwrap();
    assert_eq!(
        operation.delete_request().unwrap().expected_heads(),
        expected_heads
    );
    let delete_request = operation.begin_delete_apply(delete_generation).unwrap();
    assert_eq!(delete_request.skill_id(), skill_id);
    assert_eq!(delete_request.expected_heads(), expected_heads);
    assert!(operation.cancel().is_err());
    assert!(operation.complete(delete_generation));

    let tombstone = RevisionId::from_digest([32; 32]);
    let expected_tombstone = vec![tombstone.clone()];
    let (history_generation, _) = operation.begin_history(skill_id).unwrap();
    operation.set_history_page(
        history_generation,
        jameskills_core::ports::LibraryHistoryPage::new(
            vec![jameskills_core::ports::LibraryHistoryEntry::new(
                content_revision.clone(),
                Vec::new(),
                Some(ContentHash::from_digest([33; 32])),
                "1.0.0".to_owned(),
                false,
                Vec::new(),
            )],
            None,
        ),
    );
    assert!(operation.select_history_revision(history_generation, &content_revision));
    let restore = RestoreRevisionRequest::new(
        skill_id,
        content_revision.clone(),
        expected_tombstone.clone(),
        "2.0.0".to_owned(),
    )
    .unwrap();
    let restore_generation = operation.begin_restore_confirmation(restore, true).unwrap();
    assert_eq!(
        operation.restore_request().unwrap().source_revision_id(),
        &content_revision
    );
    let (restore_request, deleted_only) =
        operation.begin_restore_apply(restore_generation).unwrap();
    assert!(deleted_only);
    assert_eq!(restore_request.expected_heads(), expected_tombstone);
}
