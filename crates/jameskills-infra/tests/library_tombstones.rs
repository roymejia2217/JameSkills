use jameskills_core::{
    AppError,
    application::library::ExportRequest,
    domain::{
        DeleteRequest, ForkRequest, ImportResolution, ImportSourceKind, PortablePath,
        RestoreRevisionRequest, RevisionKind, RevisionTrust, SaveRevisionRequest, SkillId,
    },
    ports::{
        BundleFiles, LibraryHistoryQuery, LibraryItemState, LibraryQuery, StoragePort,
        filesystem::FileSystemPort,
    },
};
use jameskills_infra::{
    composition::build_services, fs::LocalFileSystem, platform::UserDirectories,
    sqlite::SqliteStore,
};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_CASE: AtomicU64 = AtomicU64::new(0);

struct TestRoot(PathBuf);

impl TestRoot {
    fn new() -> Self {
        let id = NEXT_CASE.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("jameskills-tombstone-{}-{id}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn services(root: &Path) -> jameskills_infra::composition::RuntimeServices {
    build_services(UserDirectories {
        config: root.join("config"),
        data: root.join("data"),
        cache: root.join("cache"),
    })
    .unwrap()
}

fn write_fixture(root: &Path, alternate_content: bool) -> PathBuf {
    let source = root.join("source");
    for (path, original) in [
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
    ] {
        let destination = source.join(path);
        std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
        let mut bytes = original.to_vec();
        if alternate_content && path == "SKILL.md" {
            bytes.extend_from_slice(b"\nConcurrent local revision fixture.\n");
        }
        std::fs::write(destination, bytes).unwrap();
    }
    source
}

async fn import_as_new(
    services: &jameskills_infra::composition::RuntimeServices,
    source: &Path,
) -> SkillId {
    let preview = services
        .library()
        .preview_import(source, ImportSourceKind::Directory)
        .await
        .unwrap();
    let skill_id = preview.skill_id();
    services
        .library()
        .apply_import(preview, ImportResolution::AddConcurrentRoot)
        .await
        .unwrap();
    skill_id
}

#[test]
fn delete_commits_causal_tombstone_and_retains_exportable_history() {
    let root = TestRoot::new();
    let services = services(&root.0);
    let executor = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    executor.block_on(async {
        let skill_id = import_as_new(&services, &write_fixture(&root.0, false)).await;
        let detail = services
            .library()
            .load_skill(skill_id)
            .await
            .unwrap()
            .unwrap();
        let content_head = detail.heads()[0].summary().revision_id().clone();
        assert_eq!(
            services
                .library()
                .load_revision_trust(skill_id, &content_head)
                .await
                .unwrap(),
            RevisionTrust::quarantined(ImportSourceKind::Directory)
        );
        let content_hash = detail.heads()[0]
            .files()
            .map(|files| {
                jameskills_core::domain::validate_bundle(files)
                    .unwrap()
                    .content_hash()
                    .clone()
            })
            .unwrap();

        let deleted = services
            .library()
            .delete_skill(DeleteRequest::new(skill_id, vec![content_head.clone()]).unwrap())
            .await
            .unwrap();
        assert!(matches!(
            deleted.revision().kind(),
            RevisionKind::Tombstone { observed_heads }
                if observed_heads == std::slice::from_ref(&content_head)
        ));
        assert_eq!(
            deleted.revision().parents(),
            std::slice::from_ref(&content_head)
        );
        assert_eq!(
            services
                .library()
                .load_revision_trust(skill_id, deleted.revision().id())
                .await
                .unwrap(),
            RevisionTrust::reviewed()
        );
        let current = services
            .library()
            .load_skill(skill_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(current.heads().len(), 1);
        assert!(current.heads()[0].summary().deleted());
        let history = services
            .library()
            .load_history(LibraryHistoryQuery::new(skill_id, None, 50).unwrap())
            .await
            .unwrap();
        let tombstone_history = history
            .entries()
            .iter()
            .find(|entry| entry.revision_id() == deleted.revision().id())
            .expect("new tombstone must appear in history");
        assert!(tombstone_history.deleted());
        assert_eq!(
            tombstone_history.observed_heads(),
            std::slice::from_ref(&content_head)
        );

        let deleted_page = services
            .library()
            .list_skills(
                LibraryQuery::new(None, vec![], vec![], LibraryItemState::Deleted, None, 50)
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(deleted_page.items().len(), 1);
        let old_revision = services
            .library()
            .export_bundle(ExportRequest::new(skill_id, Some(content_head)))
            .await
            .unwrap();
        assert_eq!(old_revision.content_hash(), &content_hash);
    });
}

#[test]
fn delete_rejects_heads_changed_after_the_user_reviewed_them() {
    let root = TestRoot::new();
    let services = services(&root.0);
    let executor = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    executor.block_on(async {
        let original_id = import_as_new(&services, &write_fixture(&root.0, false)).await;
        let before_edit = services
            .library()
            .load_skill(original_id)
            .await
            .unwrap()
            .unwrap();
        let stale_head = before_edit.heads()[0].summary().revision_id().clone();

        let concurrent = services
            .library()
            .preview_import(&write_fixture(&root.0, true), ImportSourceKind::Directory)
            .await
            .unwrap();
        services
            .library()
            .apply_import(concurrent, ImportResolution::AddConcurrentRoot)
            .await
            .unwrap();

        let error = services
            .library()
            .delete_skill(DeleteRequest::new(original_id, vec![stale_head]).unwrap())
            .await
            .unwrap_err();
        assert!(matches!(error, AppError::Conflict { .. }));

        let current = services
            .library()
            .load_skill(original_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(current.heads().len(), 2);
        assert!(current.heads().iter().all(|head| !head.summary().deleted()));
    });
}

#[test]
fn sqlite_commit_keeps_explicit_quarantine_and_source_kind_on_new_revisions() {
    let root = TestRoot::new();
    let services = services(&root.0);
    let executor = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    executor.block_on(async {
        let source = write_fixture(&root.0, false);
        let skill_id = import_as_new(&services, &source).await;
        let detail = services
            .library()
            .load_skill(skill_id)
            .await
            .unwrap()
            .unwrap();
        let parent = detail.heads()[0].summary().revision_id().clone();
        let mut files: BundleFiles = LocalFileSystem.read_bundle_directory(&source).unwrap();
        let manifest_path = PortablePath::new("jameskills.toml".to_owned()).unwrap();
        let manifest = String::from_utf8(files[&manifest_path].clone()).unwrap();
        let bumped = manifest.replace("version = \"1.0.0\"", "version = \"1.1.0\"");
        assert_ne!(bumped, manifest);
        files.insert(manifest_path, bumped.into_bytes());
        let skill_path = PortablePath::new("SKILL.md".to_owned()).unwrap();
        let skill = String::from_utf8(files[&skill_path].clone()).unwrap();
        let bumped_skill = skill.replace("jameskills-version: 1.0.0", "jameskills-version: 1.1.0");
        assert_ne!(bumped_skill, skill);
        files.insert(skill_path, bumped_skill.into_bytes());
        let bundle = jameskills_core::domain::validate_bundle(&files).unwrap();
        let store = SqliteStore::open(&root.0.join("data").join("library.sqlite3")).unwrap();
        store.store_validated_bundle(&bundle, &files).unwrap();
        let request = SaveRevisionRequest::new(
            skill_id,
            Some(bundle.content_hash().clone()),
            vec![parent.clone()],
            RevisionKind::Content,
            "1.1.0".to_owned(),
            bundle.manifest().schema_version(),
            vec![parent],
        )
        .with_validated_bundle(&bundle)
        .unwrap()
        .with_trust(RevisionTrust::quarantined(ImportSourceKind::Directory))
        .unwrap();
        let committed = store.commit_revision(&request).unwrap();
        assert_eq!(
            store
                .load_revision_trust(skill_id, committed.revision().id())
                .await
                .unwrap(),
            RevisionTrust::quarantined(ImportSourceKind::Directory)
        );
    });
}

#[test]
fn restore_historical_content_creates_new_causal_revision_and_preserves_quarantine() {
    let root = TestRoot::new();
    let services = services(&root.0);
    let executor = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    executor.block_on(async {
        let original_source = write_fixture(&root.0, false);
        let skill_id = import_as_new(&services, &original_source).await;
        let original = services
            .library()
            .load_skill(skill_id)
            .await
            .unwrap()
            .unwrap();
        let historical_revision = original.heads()[0].summary().revision_id().clone();
        let historical_files = original.heads()[0].files().unwrap().clone();

        let concurrent = services
            .library()
            .preview_import(&write_fixture(&root.0, true), ImportSourceKind::Directory)
            .await
            .unwrap();
        services
            .library()
            .apply_import(concurrent, ImportResolution::AddConcurrentRoot)
            .await
            .unwrap();
        let current = services
            .library()
            .load_skill(skill_id)
            .await
            .unwrap()
            .unwrap();
        let expected_heads = current
            .heads()
            .iter()
            .map(|head| head.summary().revision_id().clone())
            .collect::<Vec<_>>();
        assert_eq!(expected_heads.len(), 2);

        let same_version = services
            .library()
            .restore_revision_as_new(
                RestoreRevisionRequest::new(
                    skill_id,
                    historical_revision.clone(),
                    expected_heads.clone(),
                    "1.0.0".to_owned(),
                )
                .unwrap(),
            )
            .await;
        assert!(matches!(same_version, Err(AppError::Validation(_))));
        assert_eq!(
            services
                .library()
                .load_skill(skill_id)
                .await
                .unwrap()
                .unwrap()
                .heads()
                .len(),
            2
        );

        let restored = services
            .library()
            .restore_revision_as_new(
                RestoreRevisionRequest::new(
                    skill_id,
                    historical_revision,
                    expected_heads.clone(),
                    "2.0.0".to_owned(),
                )
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(restored.revision().parents(), expected_heads);
        assert_eq!(restored.revision().semantic_version(), "2.0.0");
        assert_eq!(
            services
                .library()
                .load_revision_trust(skill_id, restored.revision().id())
                .await
                .unwrap(),
            RevisionTrust::quarantined(ImportSourceKind::Directory)
        );

        let restored_detail = services
            .library()
            .load_skill(skill_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(restored_detail.heads().len(), 1);
        assert!(!restored_detail.heads()[0].summary().deleted());
        let restored_files = restored_detail.heads()[0].files().unwrap();
        let restored_bundle = jameskills_core::domain::validate_bundle(restored_files).unwrap();
        assert_eq!(restored_bundle.manifest().version().to_string(), "2.0.0");
        let original_skill = String::from_utf8(
            historical_files[&PortablePath::new("SKILL.md".to_owned()).unwrap()].clone(),
        )
        .unwrap();
        let restored_skill = String::from_utf8(
            restored_files[&PortablePath::new("SKILL.md".to_owned()).unwrap()].clone(),
        )
        .unwrap();
        let original_frontmatter =
            jameskills_core::domain::skill::parse_frontmatter(original_skill.as_bytes()).unwrap();
        let restored_frontmatter =
            jameskills_core::domain::skill::parse_frontmatter(restored_skill.as_bytes()).unwrap();
        assert_eq!(restored_frontmatter.body(), original_frontmatter.body());
        assert_eq!(
            restored_frontmatter.metadata()["jameskills-version"],
            "2.0.0"
        );
    });
}

#[test]
fn fork_creates_new_quarantined_draft_with_rewritten_identity() {
    let root = TestRoot::new();
    let services = services(&root.0);
    let executor = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    executor.block_on(async {
        let skill_id = import_as_new(&services, &write_fixture(&root.0, false)).await;
        let source = services
            .library()
            .load_skill(skill_id)
            .await
            .unwrap()
            .unwrap();
        let source_revision = source.heads()[0].summary().revision_id().clone();
        let fork_result = services
            .library()
            .fork_skill(
                ForkRequest::new(
                    skill_id,
                    source_revision,
                    "repository-fork".to_owned(),
                    "Repository Fork".to_owned(),
                )
                .unwrap(),
            )
            .await;
        let fork = match fork_result {
            Ok(draft) => draft,
            Err(AppError::Validation(diagnostics)) => panic!(
                "fork validation failed: {:?}",
                diagnostics
                    .iter()
                    .map(|diagnostic| diagnostic.code())
                    .collect::<Vec<_>>()
            ),
            Err(AppError::Storage { code }) => panic!("fork storage failure: {code}"),
            Err(AppError::Conflict { .. }) => panic!("fork unexpectedly conflicted"),
            Err(_) => panic!("fork failed with a redacted non-validation error"),
        };
        assert_ne!(fork.skill_id(), skill_id);
        assert_eq!(fork.generation(), 1);
        assert_eq!(fork.base_head(), None);
        assert_eq!(
            fork.trust_state(),
            jameskills_core::domain::TrustState::Quarantined
        );
        let validated = jameskills_core::domain::validate_bundle(fork.files()).unwrap();
        assert_eq!(validated.manifest().id(), fork.skill_id());
        assert_eq!(validated.manifest().slug(), "repository-fork");
        assert_eq!(validated.manifest().display_name(), "Repository Fork");
        assert!(
            services
                .library()
                .load_skill(fork.skill_id())
                .await
                .unwrap()
                .unwrap()
                .heads()
                .is_empty()
        );
        assert_eq!(
            services
                .library()
                .load_skill(skill_id)
                .await
                .unwrap()
                .unwrap()
                .heads()[0]
                .summary()
                .revision_id(),
            source.heads()[0].summary().revision_id()
        );
        let path = PortablePath::new("SKILL.md".to_owned()).unwrap();
        let frontmatter =
            jameskills_core::domain::skill::parse_frontmatter(&fork.files()[&path]).unwrap();
        assert_eq!(frontmatter.name(), "repository-fork");
        assert_eq!(
            frontmatter.metadata()["jameskills-id"],
            fork.skill_id().as_uuid().to_string()
        );
        let foreign_revision = services
            .library()
            .load_revision_trust(fork.skill_id(), source.heads()[0].summary().revision_id())
            .await
            .unwrap_err();
        assert!(matches!(foreign_revision, AppError::NotFound));
    });
}

#[test]
fn deleted_skill_restore_requires_tombstones_and_creates_a_new_descendant() {
    let root = TestRoot::new();
    let services = services(&root.0);
    let executor = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    executor.block_on(async {
        let skill_id = import_as_new(&services, &write_fixture(&root.0, false)).await;
        let original = services
            .library()
            .load_skill(skill_id)
            .await
            .unwrap()
            .unwrap();
        let content_revision = original.heads()[0].summary().revision_id().clone();
        let deleted = services
            .library()
            .delete_skill(DeleteRequest::new(skill_id, vec![content_revision.clone()]).unwrap())
            .await
            .unwrap();
        let tombstone_id = deleted.revision().id().clone();

        let restored = services
            .library()
            .restore_deleted_skill(
                RestoreRevisionRequest::new(
                    skill_id,
                    content_revision.clone(),
                    vec![tombstone_id.clone()],
                    "2.0.0".to_owned(),
                )
                .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(restored.revision().parents(), &[tombstone_id]);
        assert!(matches!(restored.revision().kind(), RevisionKind::Content));
        assert_eq!(
            services
                .library()
                .load_revision_trust(skill_id, restored.revision().id())
                .await
                .unwrap(),
            RevisionTrust::quarantined(ImportSourceKind::Directory)
        );

        let current = services
            .library()
            .load_skill(skill_id)
            .await
            .unwrap()
            .unwrap();
        let active_head = current.heads()[0].summary().revision_id().clone();
        let wrong_mode = services
            .library()
            .restore_deleted_skill(
                RestoreRevisionRequest::new(
                    skill_id,
                    content_revision,
                    vec![active_head],
                    "3.0.0".to_owned(),
                )
                .unwrap(),
            )
            .await;
        assert!(matches!(wrong_mode, Err(AppError::Conflict { .. })));
    });
}

#[test]
fn tombstone_and_trust_survive_a_fresh_sqlite_runtime() {
    let root = TestRoot::new();
    let initial = services(&root.0);
    let executor = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let (skill_id, original_revision, tombstone_revision) = executor.block_on(async {
        let skill_id = import_as_new(&initial, &write_fixture(&root.0, false)).await;
        let detail = initial
            .library()
            .load_skill(skill_id)
            .await
            .unwrap()
            .unwrap();
        let original_revision = detail.heads()[0].summary().revision_id().clone();
        let deleted = initial
            .library()
            .delete_skill(DeleteRequest::new(skill_id, vec![original_revision.clone()]).unwrap())
            .await
            .unwrap();
        (skill_id, original_revision, deleted.revision().id().clone())
    });
    drop(initial);

    let reopened = services(&root.0);
    executor.block_on(async {
        let detail = reopened
            .library()
            .load_skill(skill_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(detail.heads().len(), 1);
        assert_eq!(
            detail.heads()[0].summary().revision_id(),
            &tombstone_revision
        );
        assert!(detail.heads()[0].summary().deleted());
        assert_eq!(
            reopened
                .library()
                .load_revision_trust(skill_id, &original_revision)
                .await
                .unwrap(),
            RevisionTrust::quarantined(ImportSourceKind::Directory)
        );
        let history = reopened
            .library()
            .load_history(LibraryHistoryQuery::new(skill_id, None, 50).unwrap())
            .await
            .unwrap();
        assert!(
            history
                .entries()
                .iter()
                .any(|entry| entry.revision_id() == &tombstone_revision && entry.deleted())
        );
    });
}
