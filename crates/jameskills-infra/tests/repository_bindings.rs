use jameskills_core::{
    AppError,
    domain::{
        ImportResult, RepositoryBinding, RepositoryBindingCheck, RepositoryBindingEvidence,
        RepositoryBindingReport, RepositoryProfile, RevisionId, policy::RepositoryHead,
    },
    ports::StoragePort,
};
use jameskills_infra::{
    composition::{RepositoryCheckTools, build_services},
    fs::ApprovedRepositoryTool,
    platform::UserDirectories,
    process::fingerprint_executable,
    sqlite::SqliteStore,
};
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

struct TestRoot(PathBuf);

impl TestRoot {
    fn new() -> Self {
        let id = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "jameskills-repository-bindings-{}-{id}",
            std::process::id()
        ));
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

fn approved_git() -> (ApprovedRepositoryTool, PathBuf) {
    let executable_name = if cfg!(windows) { "git.exe" } else { "git" };
    let path = std::env::split_paths(&std::env::var_os("PATH").expect("PATH is available"))
        .map(|directory| directory.join(executable_name))
        .find(|candidate| candidate.is_file())
        .expect("Git is available for the native repository-binding test");
    let path = std::fs::canonicalize(path).unwrap();
    let executable =
        jameskills_core::ports::process::ApprovedExecutable::from_absolute_path(path.clone())
            .unwrap();
    let fingerprint = fingerprint_executable(executable.path()).unwrap();
    (ApprovedRepositoryTool::new(executable, fingerprint), path)
}

fn git_fixture_command(git_path: &Path, root: &Path, args: &[&str]) {
    let hooks = root.join("hooks-disabled-for-test");
    std::fs::create_dir_all(&hooks).unwrap();
    let output = Command::new(git_path)
        .arg("-c")
        .arg(format!("core.hooksPath={}", hooks.display()))
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success());
}

#[test]
fn sqlite_roundtrips_and_updates_a_binding_without_replacing_its_identity() {
    let root = TestRoot::new();
    let runtime_services = services(&root.0);
    let executor = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let (skill_id, suite_revision) = executor.block_on(async {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("docs")
            .join("examples")
            .join("repository-foundation");
        let preview = runtime_services
            .library()
            .preview_import(
                &source,
                jameskills_core::domain::ImportSourceKind::Directory,
            )
            .await
            .unwrap();
        let skill_id = preview.skill_id();
        let imported = runtime_services
            .library()
            .apply_import(
                preview,
                jameskills_core::domain::ImportResolution::AddConcurrentRoot,
            )
            .await
            .unwrap();
        let revision_id = match imported {
            ImportResult::Imported { revision, .. } => revision.id().clone(),
            _ => panic!("fixture import did not create a revision"),
        };
        (skill_id, revision_id)
    });
    let repository = root.0.join("repo");
    std::fs::create_dir_all(&repository).unwrap();
    let repository = std::fs::canonicalize(repository).unwrap();
    let binding = RepositoryBinding::new(
        skill_id,
        suite_revision.clone(),
        repository.clone(),
        RepositoryProfile::Rust,
        true,
        RepositoryHead::parse(&"a".repeat(40)).unwrap(),
        format!("sha256:{}", "b".repeat(64)),
    )
    .unwrap();
    let binding_id = binding.id().to_owned();
    executor
        .block_on(runtime_services.library().bind_repository(binding.clone()))
        .unwrap();
    let service_bindings = executor
        .block_on(
            runtime_services
                .library()
                .list_repository_bindings(skill_id),
        )
        .unwrap();
    assert_eq!(service_bindings.len(), 1);
    assert!(service_bindings[0] == binding);
    drop(runtime_services);

    let store = SqliteStore::open(&root.0.join("data").join("library.sqlite3")).unwrap();

    executor.block_on(async {
        StoragePort::save_repository_binding(&store, binding.clone())
            .await
            .unwrap();
        let listed = StoragePort::list_repository_bindings(&store, skill_id)
            .await
            .unwrap();
        assert_eq!(listed.len(), 1);
        assert!(listed[0] == binding);
        assert!(
            StoragePort::load_repository_binding(&store, &binding_id)
                .await
                .unwrap()
                .is_some_and(|loaded| loaded == binding)
        );

        let check = RepositoryBindingCheck::new(
            "readme-structure".to_owned(),
            "pass".to_owned(),
            "error".to_owned(),
            Some("local-check".to_owned()),
            None,
            vec![
                RepositoryBindingEvidence::new(
                    "policy.readme".to_owned(),
                    "2026-10-07T00:00:00.000Z".to_owned(),
                    None,
                    binding.environment_fingerprint().to_owned(),
                    "README sections were inspected.".to_owned(),
                )
                .unwrap(),
            ],
        )
        .unwrap();
        let prior_report = RepositoryBindingReport::new(
            &binding,
            "2026-10-07T00:00:00.000Z".to_owned(),
            true,
            vec![check],
        )
        .unwrap();
        StoragePort::save_repository_binding_report(&store, &binding, prior_report.clone())
            .await
            .unwrap();
        assert!(
            StoragePort::load_repository_binding_report(&store, &binding_id)
                .await
                .unwrap()
                .is_some_and(|report| report == prior_report)
        );

        let updated = RepositoryBinding::from_storage(
            binding_id.clone(),
            skill_id,
            suite_revision.clone(),
            repository.clone(),
            RepositoryProfile::Node,
            false,
            RepositoryHead::parse(&"c".repeat(40)).unwrap(),
            format!("sha256:{}", "d".repeat(64)),
        )
        .unwrap();
        StoragePort::save_repository_binding(&store, updated.clone())
            .await
            .unwrap();
        let listed = StoragePort::list_repository_bindings(&store, skill_id)
            .await
            .unwrap();
        assert_eq!(listed.len(), 1);
        assert!(listed[0] == updated);
        let saved_prior = StoragePort::load_repository_binding_report(&store, &binding_id)
            .await
            .unwrap()
            .unwrap();
        assert!(saved_prior.is_stale_for(&updated));
        assert!(matches!(
            StoragePort::save_repository_binding_report(&store, &updated, saved_prior.clone())
                .await,
            Err(AppError::Conflict { .. })
        ));
        let fresh_report = RepositoryBindingReport::new(
            &updated,
            "2026-10-07T00:01:00.000Z".to_owned(),
            false,
            Vec::new(),
        )
        .unwrap();
        StoragePort::save_repository_binding_report(&store, &updated, fresh_report.clone())
            .await
            .unwrap();
        assert!(
            StoragePort::load_repository_binding_report(&store, &binding_id)
                .await
                .unwrap()
                .is_some_and(|report| report == fresh_report)
        );

        let stale_suite = RepositoryBinding::new(
            skill_id,
            RevisionId::from_digest([9; 32]),
            repository,
            RepositoryProfile::Rust,
            true,
            RepositoryHead::parse(&"e".repeat(40)).unwrap(),
            format!("sha256:{}", "f".repeat(64)),
        )
        .unwrap();
        assert!(matches!(
            StoragePort::save_repository_binding(&store, stale_suite).await,
            Err(AppError::Conflict { .. })
        ));
        assert!(
            StoragePort::load_repository_binding(&store, "00000000-0000-0000-0000-000000000001")
                .await
                .unwrap()
                .is_none()
        );
    });
}

#[test]
fn runtime_binding_observation_tracks_git_head_and_repository_move() {
    let root = TestRoot::new();
    let repository = root.0.join("project");
    std::fs::create_dir_all(&repository).unwrap();
    let repository = std::fs::canonicalize(repository).unwrap();
    let (git, git_path) = approved_git();
    git_fixture_command(
        &git_path,
        &repository,
        &["init", "--quiet", "--initial-branch=main"],
    );
    git_fixture_command(&git_path, &repository, &["config", "user.name", "Fixture"]);
    git_fixture_command(
        &git_path,
        &repository,
        &["config", "user.email", "fixture@example.invalid"],
    );
    std::fs::write(
        repository.join("Cargo.toml"),
        "[package]\nname = \"binding-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    git_fixture_command(&git_path, &repository, &["add", "Cargo.toml"]);
    git_fixture_command(
        &git_path,
        &repository,
        &["commit", "--quiet", "-m", "first"],
    );

    let services = services(&root.0);
    let executor = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let (skill_id, suite_revision) = executor.block_on(async {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("docs")
            .join("examples")
            .join("repository-foundation");
        let preview = services
            .library()
            .preview_import(
                &source,
                jameskills_core::domain::ImportSourceKind::Directory,
            )
            .await
            .unwrap();
        let skill_id = preview.skill_id();
        let imported = services
            .library()
            .apply_import(
                preview,
                jameskills_core::domain::ImportResolution::AddConcurrentRoot,
            )
            .await
            .unwrap();
        let revision_id = match imported {
            ImportResult::Imported { revision, .. } => revision.id().clone(),
            _ => panic!("fixture import did not create a revision"),
        };
        (skill_id, revision_id)
    });

    let first = executor
        .block_on(services.observe_repository_binding(
            skill_id,
            suite_revision.clone(),
            &repository,
            RepositoryProfile::Rust,
            true,
            &git,
        ))
        .unwrap();
    assert!(first.repository_head().as_str().len() == 40);
    executor
        .block_on(services.library().bind_repository(first.clone()))
        .unwrap();
    assert!(
        executor
            .block_on(
                services
                    .library()
                    .load_validated_revision(skill_id, &suite_revision)
            )
            .is_ok()
    );
    assert!(
        services
            .repository_check_context(&repository, "rust")
            .is_ok()
    );
    assert!(
        services
            .policy_for_repository(
                &repository,
                RepositoryCheckTools::default().with_git(approved_git().0),
            )
            .is_ok()
    );
    let first_evaluation = executor
        .block_on(services.evaluate_repository_binding(
            first.id(),
            RepositoryCheckTools::default().with_git(approved_git().0),
        ))
        .unwrap();
    assert!(
        !first_evaluation.is_stale(),
        "unexpected initial stale reason: {:?}",
        first_evaluation.stale_reason()
    );
    let first_report = first_evaluation.report().unwrap().clone();
    assert!(!first_report.checks().is_empty());

    std::fs::write(
        repository.join("Cargo.toml"),
        "[package]\nname = \"binding-fixture\"\nversion = \"0.2.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    git_fixture_command(&git_path, &repository, &["add", "Cargo.toml"]);
    git_fixture_command(
        &git_path,
        &repository,
        &["commit", "--quiet", "-m", "second"],
    );
    let after_head = executor
        .block_on(services.evaluate_repository_binding(
            first.id(),
            RepositoryCheckTools::default().with_git(approved_git().0),
        ))
        .unwrap();
    assert!(!after_head.is_stale());
    let after_head_report = after_head.report().unwrap().clone();
    assert_ne!(
        first_report.repository_head().as_str(),
        after_head_report.repository_head().as_str()
    );
    assert_eq!(
        first_report.environment_fingerprint(),
        after_head_report.environment_fingerprint()
    );
    let refreshed = executor
        .block_on(services.library().load_repository_binding(first.id()))
        .unwrap()
        .unwrap();
    assert_eq!(refreshed.id(), first.id());
    assert_eq!(
        refreshed.repository_head().as_str(),
        after_head_report.repository_head().as_str()
    );
    let saved_after_head = executor
        .block_on(
            services
                .library()
                .load_repository_binding_report(first.id()),
        )
        .unwrap()
        .unwrap();
    assert!(saved_after_head == after_head_report);

    let unapproved_git = executor
        .block_on(services.evaluate_repository_binding(first.id(), RepositoryCheckTools::default()))
        .unwrap();
    assert_eq!(
        unapproved_git.stale_reason(),
        Some(
            jameskills_core::application::policy::RepositoryBindingStaleReason::GitApprovalRequired
        )
    );
    assert_eq!(
        unapproved_git.next_step(),
        Some(jameskills_core::application::policy::RepositoryBindingNextStep::ApproveGit)
    );
    assert!(unapproved_git.report().unwrap() == &saved_after_head);
    assert_eq!(
        executor
            .block_on(services.library().list_repository_bindings(skill_id))
            .unwrap()
            .len(),
        1
    );

    let moved_path = root.0.join("moved-project");
    std::fs::rename(&repository, &moved_path).unwrap();
    let after_move = executor
        .block_on(services.evaluate_repository_binding(
            first.id(),
            RepositoryCheckTools::default().with_git(approved_git().0),
        ))
        .unwrap();
    assert!(after_move.is_stale());
    assert_eq!(
        after_move.stale_reason(),
        Some(jameskills_core::application::policy::RepositoryBindingStaleReason::RepositoryUnavailable)
    );
    assert_eq!(
        after_move.next_step(),
        Some(jameskills_core::application::policy::RepositoryBindingNextStep::LocateRepository)
    );
    assert!(after_move.report().unwrap() == &saved_after_head);
    drop(services);
}
