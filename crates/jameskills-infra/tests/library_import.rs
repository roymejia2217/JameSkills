use jameskills_core::{
    domain::{
        ImportClassification, ImportResolution, ImportResult, ImportSourceKind, PortablePath,
    },
    ports::{BundleFiles, filesystem::FileSystemPort, write_bundle_archive},
};
use jameskills_infra::{
    composition::build_services, fs::LocalFileSystem, platform::UserDirectories,
};
use std::io::Write as _;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static CASE_COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_root() -> PathBuf {
    let id = CASE_COUNTER.fetch_add(1, Ordering::SeqCst);
    let root = std::env::temp_dir().join(format!(
        "jameskills-import-source-{}-{id}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn bundle_source_reads_regular_directories_and_bounded_jskill_archives_as_data() {
    let root = temp_root();
    let directory = root.join("directory-bundle");
    std::fs::create_dir(&directory).unwrap();
    let skill = b"invalid semantic draft is still inspectable\r\n";
    std::fs::write(directory.join("SKILL.md"), skill).unwrap();
    let from_directory = LocalFileSystem.read_bundle_source(&directory).unwrap();
    assert_eq!(
        from_directory[&PortablePath::new("SKILL.md".to_owned()).unwrap()],
        skill
    );

    let files: BundleFiles = [(
        PortablePath::new("SKILL.md".to_owned()).unwrap(),
        skill.to_vec(),
    )]
    .into_iter()
    .collect();
    let archive = write_bundle_archive(&files).unwrap();
    let archive_path = root.join("portable.jskill");
    std::fs::write(&archive_path, archive).unwrap();
    assert_eq!(
        LocalFileSystem.read_bundle_source(&archive_path).unwrap(),
        files
    );

    let malformed = root.join("malformed.jskill");
    std::fs::write(&malformed, b"not a zip archive").unwrap();
    assert!(LocalFileSystem.read_bundle_source(&malformed).is_err());
    let wrong_extension = root.join("portable.zip");
    std::fs::write(&wrong_extension, b"not imported").unwrap();
    assert!(
        LocalFileSystem
            .read_bundle_source(&wrong_extension)
            .is_err()
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn bundle_source_never_follows_a_source_symlink_and_handles_linked_file_policy() {
    let root = temp_root();
    let archive_path = root.join("source.jskill");
    std::fs::write(
        &archive_path,
        write_bundle_archive(
            &[(
                PortablePath::new("SKILL.md".to_owned()).unwrap(),
                b"# inert\n".to_vec(),
            )]
            .into_iter()
            .collect(),
        )
        .unwrap(),
    )
    .unwrap();

    let hard_link = root.join("linked.jskill");
    if std::fs::hard_link(&archive_path, &hard_link).is_ok() {
        #[cfg(unix)]
        assert!(LocalFileSystem.read_bundle_source(&hard_link).is_err());
        #[cfg(windows)]
        assert!(LocalFileSystem.read_bundle_source(&hard_link).is_ok());
    }
    #[cfg(unix)]
    {
        let symlink = root.join("symlink.jskill");
        std::os::unix::fs::symlink(&archive_path, &symlink).unwrap();
        assert!(LocalFileSystem.read_bundle_source(&symlink).is_err());
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn bundle_source_reads_a_standalone_plain_skill_as_exact_inert_bytes() {
    let root = temp_root();
    let source = root.join("SKILL.md");
    let bytes = b"---\nname: plain-instructions\ndescription: Plain imported guidance.\n---\n# Instructions\n";
    std::fs::write(&source, bytes).unwrap();

    let files = LocalFileSystem.read_bundle_source(&source).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(
        files[&PortablePath::new("SKILL.md".to_owned()).unwrap()],
        bytes
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn runtime_import_preview_classifies_new_duplicate_and_conflicting_identity_without_writes() {
    let root = temp_root();
    let source = root.join("source");
    std::fs::create_dir_all(source.join("policies")).unwrap();
    std::fs::create_dir_all(source.join("guidance")).unwrap();
    for (path, bytes) in [
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
        std::fs::write(source.join(path), bytes).unwrap();
    }
    let app_root = root.join("app");
    let directories = UserDirectories {
        config: app_root.join("config"),
        data: app_root.join("data"),
        cache: app_root.join("cache"),
    };
    let runtime = build_services(directories.clone()).unwrap();
    let executor = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let preview = executor
        .block_on(
            runtime
                .library()
                .preview_import(&source, ImportSourceKind::Directory),
        )
        .unwrap();
    assert_eq!(preview.classification(), &ImportClassification::NewSkill);
    assert_eq!(
        preview.trust_state(),
        jameskills_core::domain::TrustState::Quarantined
    );

    let imported = executor
        .block_on(
            runtime
                .library()
                .apply_import(preview.clone(), ImportResolution::AddConcurrentRoot),
        )
        .unwrap();
    let first_revision = match imported {
        ImportResult::Imported {
            revision,
            heads,
            trust_state,
        } => {
            assert_eq!(heads, vec![revision.id().clone()]);
            assert_eq!(
                trust_state,
                jameskills_core::domain::TrustState::Quarantined
            );
            revision.id().clone()
        }
        _ => panic!("new identity must create its first quarantined revision"),
    };

    let duplicate = executor
        .block_on(
            runtime
                .library()
                .preview_import(&source, ImportSourceKind::Directory),
        )
        .unwrap();
    assert!(matches!(
        duplicate.classification(),
        ImportClassification::Identical { .. }
    ));
    assert!(matches!(
        executor
            .block_on(runtime.library().apply_import(
                duplicate,
                ImportResolution::KeepExisting,
            ))
            .unwrap(),
        ImportResult::Duplicate { revision_id, .. } if revision_id == first_revision
    ));

    std::fs::OpenOptions::new()
        .append(true)
        .open(source.join("SKILL.md"))
        .unwrap()
        .write_all(b"\nA changed imported instruction.\n")
        .unwrap();
    let conflict = executor
        .block_on(
            runtime
                .library()
                .preview_import(&source, ImportSourceKind::Directory),
        )
        .unwrap();
    assert!(matches!(
        conflict.classification(),
        ImportClassification::Conflict { current_heads } if current_heads.len() == 1
    ));
    assert_eq!(
        conflict.trust_state(),
        jameskills_core::domain::TrustState::Quarantined
    );
    assert!(matches!(
        executor
            .block_on(runtime.library().apply_import(
                conflict.clone(),
                ImportResolution::KeepExisting,
            ))
            .unwrap(),
        ImportResult::KeptExisting { heads } if heads == vec![first_revision.clone()]
    ));
    let concurrent = executor
        .block_on(
            runtime
                .library()
                .apply_import(conflict, ImportResolution::AddConcurrentRoot),
        )
        .unwrap();
    let concurrent_head = match concurrent {
        ImportResult::Imported {
            revision,
            heads,
            trust_state,
        } => {
            assert_eq!(heads.len(), 2);
            assert!(heads.contains(&first_revision));
            assert_eq!(
                trust_state,
                jameskills_core::domain::TrustState::Quarantined
            );
            revision.id().clone()
        }
        _ => panic!("different content must be preserved as a concurrent root"),
    };
    assert_ne!(concurrent_head, first_revision);
    let connection = rusqlite::Connection::open(directories.data.join("library.sqlite3")).unwrap();
    let revision_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM revisions", [], |row| row.get(0))
        .unwrap();
    let head_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM skill_heads", [], |row| row.get(0))
        .unwrap();
    let parent_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM revision_parents", [], |row| {
            row.get(0)
        })
        .unwrap();
    let quarantined_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM revision_trust WHERE trust_state = 'quarantined'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(revision_count, 2);
    assert_eq!(head_count, 2, "existing heads must remain visible");
    assert_eq!(
        parent_count, 0,
        "individual .jskill imports invent no ancestry"
    );
    assert_eq!(quarantined_count, 2);
    drop(connection);
    drop(runtime);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn runtime_plain_skill_import_creates_only_a_quarantined_instructions_draft() {
    let root = temp_root();
    let source = root.join("SKILL.md");
    let bytes = b"---\r\nname: imported-guidance\r\ndescription: Imported instructions.\r\n---\r\n# Keep these bytes\r\n";
    std::fs::write(&source, bytes).unwrap();
    let app_root = root.join("app");
    let runtime = build_services(UserDirectories {
        config: app_root.join("config"),
        data: app_root.join("data"),
        cache: app_root.join("cache"),
    })
    .unwrap();
    let executor = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let preview = executor
        .block_on(
            runtime
                .library()
                .preview_import(&source, ImportSourceKind::PlainSkill),
        )
        .unwrap();
    assert_eq!(preview.source_kind(), ImportSourceKind::PlainSkill);
    assert_eq!(preview.classification(), &ImportClassification::NewSkill);
    assert_eq!(
        preview.trust_state(),
        jameskills_core::domain::TrustState::Quarantined
    );
    let skill_id = preview.skill_id();
    let generated_manifest =
        preview.files()[&PortablePath::new("jameskills.toml".to_owned()).unwrap()].clone();

    assert!(matches!(
        executor.block_on(runtime.library().apply_import(
            preview,
            ImportResolution::CreateQuarantinedDraft,
        )),
        Ok(ImportResult::DraftCreated {
            skill_id: imported_id,
            generation: 1,
            trust_state: jameskills_core::domain::TrustState::Quarantined,
        }) if imported_id == skill_id
    ));

    let draft = executor
        .block_on(runtime.library().load_draft(skill_id))
        .unwrap()
        .unwrap();
    assert_eq!(
        draft.trust_state(),
        jameskills_core::domain::TrustState::Quarantined
    );
    assert_eq!(
        draft.files()[&PortablePath::new("SKILL.md".to_owned()).unwrap()],
        bytes
    );
    assert_eq!(
        draft.files()[&PortablePath::new("jameskills.toml".to_owned()).unwrap()],
        generated_manifest
    );
    let detail = executor
        .block_on(runtime.library().load_skill(skill_id))
        .unwrap()
        .unwrap();
    assert!(detail.heads().is_empty());
    let publish = jameskills_core::application::PublishDraft::new(skill_id, 1, vec![]).unwrap();
    assert!(matches!(
        executor.block_on(runtime.library().publish(publish)),
        Err(jameskills_core::AppError::UntrustedInput { code })
            if code == "library.draft.review_required"
    ));
    let history = executor
        .block_on(runtime.library().load_history(
            jameskills_core::ports::LibraryHistoryQuery::new(skill_id, None, 10).unwrap(),
        ))
        .unwrap();
    assert!(history.entries().is_empty());
    assert!(!app_root.join("data").join("blobs").exists());
    drop(runtime);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn import_failure_rolls_back_skill_revision_head_and_trust_rows() {
    let root = temp_root();
    let source = root.join("source");
    std::fs::create_dir_all(source.join("policies")).unwrap();
    std::fs::create_dir_all(source.join("guidance")).unwrap();
    for (path, bytes) in [
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
        std::fs::write(source.join(path), bytes).unwrap();
    }
    let app = root.join("app");
    let directories = UserDirectories {
        config: app.join("config"),
        data: app.join("data"),
        cache: app.join("cache"),
    };
    let runtime = build_services(directories.clone()).unwrap();
    let executor = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let preview = executor
        .block_on(
            runtime
                .library()
                .preview_import(&source, ImportSourceKind::Directory),
        )
        .unwrap();
    rusqlite::Connection::open(directories.data.join("library.sqlite3"))
        .unwrap()
        .execute_batch(
            "CREATE TRIGGER reject_quarantine BEFORE INSERT ON revision_trust BEGIN SELECT RAISE(ABORT, 'injected'); END;",
        )
        .unwrap();

    assert!(matches!(
        executor.block_on(runtime.library().apply_import(
            preview,
            ImportResolution::AddConcurrentRoot,
        )),
        Err(jameskills_core::AppError::Storage { ref code })
            if code == "storage.import.write.failed"
    ));
    let connection = rusqlite::Connection::open(directories.data.join("library.sqlite3")).unwrap();
    for table in [
        "skills",
        "library_catalog",
        "revisions",
        "skill_heads",
        "revision_trust",
    ] {
        let count: i64 = connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0, "partial import left rows in {table}");
    }
    drop(connection);
    let store =
        jameskills_infra::sqlite::SqliteStore::open(&directories.data.join("library.sqlite3"))
            .unwrap();
    assert_eq!(store.orphan_blob_hashes().unwrap().len(), 1);
    drop(store);
    drop(runtime);
    std::fs::remove_dir_all(root).unwrap();
}
