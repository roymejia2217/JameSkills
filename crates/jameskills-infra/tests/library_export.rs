use jameskills_core::{
    AppError,
    application::ExportRequest,
    domain::{
        ImportResolution, ImportSourceKind, PortablePath, RevisionKind, SaveRevisionRequest,
        validate_bundle,
    },
    ports::filesystem::{BundleFiles, FileSystemPort, write_bundle_archive},
};
use jameskills_infra::{
    composition::build_services, fs::LocalFileSystem, platform::UserDirectories,
    sqlite::SqliteStore,
};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

struct TestRoot(PathBuf);

impl TestRoot {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "jameskills-library-export-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
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

fn fixture_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("examples")
        .join("repository-foundation")
}

fn write_source(root: &Path, files: &BundleFiles) -> PathBuf {
    let source = root.to_path_buf();
    for (portable, bytes) in files {
        let path = source.join(portable.as_str());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }
    source
}

#[test]
fn export_roundtrips_exact_content_and_requires_revision_selection_for_conflicting_heads() {
    let root = TestRoot::new();
    let app_dirs = UserDirectories {
        config: root.0.join("config"),
        data: root.0.join("data"),
        cache: root.0.join("cache"),
    };
    let runtime = build_services(app_dirs).unwrap();
    let library = runtime.library();
    let filesystem = LocalFileSystem;
    let files = filesystem
        .read_bundle_directory(&fixture_directory())
        .unwrap();
    let validated = validate_bundle(&files).unwrap();
    let skill_id = validated.manifest().id();
    let source = write_source(&root.0.join("first"), &files);
    let preview = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(library.preview_import(&source, ImportSourceKind::Directory))
        .unwrap();
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(library.apply_import(preview, ImportResolution::AddConcurrentRoot))
        .unwrap();

    let first_export = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(library.export_bundle(ExportRequest::new(skill_id, None)))
        .unwrap();
    assert_eq!(first_export.content_hash(), validated.content_hash());
    let archive = root.0.join("roundtrip.jskill");
    std::fs::write(&archive, first_export.archive_bytes()).unwrap();
    let roundtrip = filesystem.read_bundle_source(&archive).unwrap();
    assert_eq!(
        validate_bundle(&roundtrip).unwrap().content_hash(),
        validated.content_hash()
    );
    let first_revision = first_export.revision_id().clone();

    let mut modified_files = files.clone();
    let skill_document = modified_files
        .iter_mut()
        .find(|(path, _)| path.as_str() == "SKILL.md")
        .unwrap();
    skill_document
        .1
        .extend_from_slice(b"\nAdditional portable guidance.\n");
    let second_source = write_source(&root.0.join("second"), &modified_files);
    let second_preview = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(library.preview_import(&second_source, ImportSourceKind::Directory))
        .unwrap();
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(library.apply_import(second_preview, ImportResolution::AddConcurrentRoot))
        .unwrap();

    let no_selection = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(library.export_bundle(ExportRequest::new(skill_id, None)));
    assert!(matches!(no_selection, Err(AppError::Conflict { .. })));

    let detail = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(library.load_skill(skill_id))
        .unwrap()
        .unwrap();
    let observed_heads = detail
        .heads()
        .iter()
        .map(|head| head.summary().revision_id().clone())
        .collect::<Vec<_>>();
    let manifest = modified_files
        .iter_mut()
        .find(|(path, _)| path.as_str() == "jameskills.toml")
        .unwrap();
    let manifest_text = std::str::from_utf8(manifest.1.as_slice()).unwrap();
    *manifest.1 = manifest_text
        .replace("version = \"1.0.0\"", "version = \"1.1.0\"")
        .into_bytes();
    let skill_document = modified_files
        .iter_mut()
        .find(|(path, _)| path.as_str() == "SKILL.md")
        .unwrap();
    let skill_text = std::str::from_utf8(skill_document.1.as_slice()).unwrap();
    *skill_document.1 = skill_text
        .replace("jameskills-version: 1.0.0", "jameskills-version: 1.1.0")
        .into_bytes();
    let next_bundle = validate_bundle(&modified_files).unwrap();
    let store = SqliteStore::open(&root.0.join("data").join("library.sqlite3")).unwrap();
    let request = SaveRevisionRequest::new(
        skill_id,
        Some(next_bundle.content_hash().clone()),
        observed_heads.clone(),
        RevisionKind::Content,
        next_bundle.manifest().version().to_string(),
        next_bundle.manifest().schema_version(),
        observed_heads,
    )
    .with_validated_bundle(&next_bundle)
    .unwrap();
    store
        .store_validated_bundle(&next_bundle, &modified_files)
        .unwrap();
    store.commit_revision(&request).unwrap();

    let selected = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(library.export_bundle(ExportRequest::new(skill_id, Some(first_revision.clone()))))
        .unwrap();
    assert_eq!(selected.revision_id(), &first_revision);
    assert_eq!(selected.content_hash(), validated.content_hash());
    assert_eq!(
        selected.archive_bytes(),
        write_bundle_archive(&files).unwrap()
    );

    let current = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(library.export_bundle(ExportRequest::new(skill_id, None)))
        .unwrap();
    let destination = root.0.join("output.jskill");
    let missing = library.inspect_export_destination(&destination).unwrap();
    assert!(matches!(
        &missing,
        jameskills_core::ports::filesystem::ExportDestinationState::Missing
    ));
    library
        .write_export_archive(&destination, selected.archive_bytes(), &missing, false)
        .unwrap();
    let existing = library.inspect_export_destination(&destination).unwrap();
    assert!(matches!(
        &existing,
        jameskills_core::ports::filesystem::ExportDestinationState::Existing(_)
    ));

    let stale = library.write_export_archive(&destination, current.archive_bytes(), &missing, true);
    assert!(matches!(stale, Err(AppError::Conflict { .. })));
    let overwrite_required =
        library.write_export_archive(&destination, current.archive_bytes(), &existing, false);
    assert!(matches!(
        overwrite_required,
        Err(AppError::PermissionDenied { .. })
    ));
    assert_eq!(
        validate_bundle(&filesystem.read_bundle_source(&destination).unwrap())
            .unwrap()
            .content_hash(),
        selected.content_hash()
    );

    std::fs::write(&destination, current.archive_bytes()).unwrap();
    let concurrent_edit =
        library.write_export_archive(&destination, selected.archive_bytes(), &existing, true);
    assert!(matches!(concurrent_edit, Err(AppError::Conflict { .. })));
    assert_eq!(
        std::fs::read(&destination).unwrap(),
        current.archive_bytes()
    );
    let current_state = library.inspect_export_destination(&destination).unwrap();
    let malformed =
        library.write_export_archive(&destination, b"not a jskill", &current_state, true);
    assert!(matches!(malformed, Err(AppError::Validation(_))));
    assert_eq!(
        std::fs::read(&destination).unwrap(),
        current.archive_bytes()
    );

    library
        .write_export_archive(&destination, selected.archive_bytes(), &current_state, true)
        .unwrap();
    assert_eq!(
        validate_bundle(&filesystem.read_bundle_source(&destination).unwrap())
            .unwrap()
            .content_hash(),
        selected.content_hash()
    );

    let stale_preview = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(library.preview_export(
            ExportRequest::new(skill_id, Some(first_revision.clone())),
            destination.clone(),
        ))
        .unwrap();
    let stale_digest = stale_preview.confirmation_digest(true).unwrap().clone();
    std::fs::write(&destination, current.archive_bytes()).unwrap();
    let stale_apply = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(library.apply_export(stale_preview, true, &stale_digest));
    assert!(matches!(stale_apply, Err(AppError::Conflict { .. })));
    assert_eq!(
        validate_bundle(&filesystem.read_bundle_source(&destination).unwrap())
            .unwrap()
            .content_hash(),
        current.content_hash()
    );

    let preview = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(library.preview_export(
            ExportRequest::new(skill_id, Some(first_revision)),
            destination.clone(),
        ))
        .unwrap();
    let digest = preview.confirmation_digest(true).unwrap().clone();
    let applied = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(library.apply_export(preview, true, &digest))
        .unwrap();
    assert_eq!(applied.content_hash(), selected.content_hash());
}

#[test]
fn maximum_inventory_export_archive_fits_the_portable_archive_container_limit() {
    const MAX_FILES: usize = 2_000;
    const ENTRY_BYTES: usize = 10_485;
    let root = TestRoot::new();
    let filesystem = LocalFileSystem;
    let mut files = filesystem
        .read_bundle_directory(&fixture_directory())
        .unwrap();
    for index in 0..(MAX_FILES - files.len()) {
        files.insert(
            PortablePath::new(format!("export-padding-{index:04}.txt")).unwrap(),
            vec![b'x'; ENTRY_BYTES],
        );
    }
    let archive = write_bundle_archive(&files).unwrap();
    assert!(archive.len() as u64 > 20 * 1024 * 1024);
    assert!(archive.len() as u64 <= 22 * 1024 * 1024);
    let source = root.0.join("maximum.jskill");
    std::fs::write(&source, archive).unwrap();

    let imported = filesystem.read_bundle_source(&source).unwrap();
    assert_eq!(
        validate_bundle(&imported).unwrap().content_hash(),
        validate_bundle(&files).unwrap().content_hash()
    );
}
