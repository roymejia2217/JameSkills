use async_trait::async_trait;
use jameskills_core::{
    AppError, AppResult, ContentHash,
    application::{RepoPolicyRequest, RepositoryChangeService},
    domain::{RepoTemplateId, policy::RepositoryHead},
    ports::{
        ApprovedRepoGit, ClockPort, OperationJournalPort, RepoChangeJournalState,
        process::{
            ApprovedEnv, ApprovedExecutable, ApprovedRoot, CancellationToken,
            ExecutableFingerprint, ProcessOutput, ProcessPermission, ProcessPort, ProcessSpec,
        },
    },
};
use jameskills_infra::{
    fs::{
        LocalRepoChangePort, plan_repo_template, plan_repo_template_with_approved_git,
        repository_root_fingerprint,
    },
    process::fingerprint_executable,
    sqlite::SqliteStore,
};
use std::{
    collections::VecDeque,
    future::Future,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    task::{Context, Poll, Wake, Waker},
};

static NEXT_REPOSITORY: AtomicU64 = AtomicU64::new(0);

struct TempRepository(PathBuf);

impl TempRepository {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "jameskills-template-plan-{}-{}",
            std::process::id(),
            NEXT_REPOSITORY.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn approved_root(&self) -> ApprovedRoot {
        ApprovedRoot::from_absolute_path(std::fs::canonicalize(&self.0).unwrap()).unwrap()
    }
}

impl Drop for TempRepository {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn head() -> RepositoryHead {
    RepositoryHead::parse(&"a".repeat(40)).unwrap()
}

fn approved_git(repository: &TempRepository) -> (ApprovedRepoGit, ExecutableFingerprint) {
    let bin = repository.path().join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let executable = bin.join(if cfg!(windows) { "git.exe" } else { "git" });
    std::fs::write(&executable, b"fake native Git for fixed-argv tests").unwrap();
    let fingerprint = fingerprint_executable(&executable).unwrap();
    let approved = ApprovedRepoGit::after_explicit_fingerprint_confirmation(
        ApprovedExecutable::from_absolute_path(std::fs::canonicalize(executable).unwrap()).unwrap(),
        fingerprint,
        &fingerprint,
        ApprovedEnv::new(Default::default()).unwrap(),
    )
    .unwrap();
    (approved, fingerprint)
}

fn repo_policy_request(repository: &TempRepository, git: ApprovedRepoGit) -> RepoPolicyRequest {
    let root = repository.approved_root();
    let root_fingerprint = repository_root_fingerprint(&root).unwrap();
    RepoPolicyRequest::new(root, root_fingerprint, head(), RepoTemplateId::RustCi, git)
}

struct FakeProcessPort {
    approved_fingerprint: ExecutableFingerprint,
    responses: Mutex<VecDeque<Result<ProcessOutput, AppError>>>,
    invocations: Mutex<Vec<(Vec<String>, ProcessPermission)>>,
}

impl FakeProcessPort {
    fn new(approved_fingerprint: ExecutableFingerprint, responses: Vec<ProcessOutput>) -> Self {
        Self::new_with_results(
            approved_fingerprint,
            responses.into_iter().map(Ok).collect(),
        )
    }

    fn new_with_results(
        approved_fingerprint: ExecutableFingerprint,
        responses: Vec<Result<ProcessOutput, AppError>>,
    ) -> Self {
        Self {
            approved_fingerprint,
            responses: Mutex::new(responses.into()),
            invocations: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl ProcessPort for FakeProcessPort {
    async fn run(&self, spec: ProcessSpec) -> AppResult<ProcessOutput> {
        if spec.tool_id() != jameskills_core::domain::ToolId::Git
            || spec.permission() != ProcessPermission::ReadOnlyCheck
            || spec.approved_executable_fingerprint() != Some(&self.approved_fingerprint)
        {
            return Err(AppError::PermissionDenied {
                operation: "test.git.approval.mismatch".to_owned(),
            });
        }
        self.invocations.lock().unwrap().push((
            spec.args()
                .iter()
                .map(|argument| argument.to_string_lossy().into_owned())
                .collect(),
            spec.permission(),
        ));
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| AppError::ExternalTool {
                tool_id: "git".to_owned(),
                exit_code: None,
            })?
    }
}

fn git_output(text: &str) -> ProcessOutput {
    ProcessOutput::new(Some(0), text.as_bytes().to_vec(), Vec::new())
}

fn git_preview_outputs(head: char, cycles: usize) -> Vec<ProcessOutput> {
    let head_output = format!("{}\n", head.to_string().repeat(40));
    let mut outputs = Vec::new();
    for _ in 0..cycles {
        outputs.push(git_output("git version 2.50.1\n"));
        outputs.push(git_output(&head_output));
        outputs.push(git_output(&head_output));
    }
    outputs
}

fn change_service(
    repository: &TempRepository,
    fingerprint: ExecutableFingerprint,
    responses: Vec<ProcessOutput>,
) -> (RepositoryChangeService, Arc<SqliteStore>) {
    let process = Arc::new(FakeProcessPort::new(fingerprint, responses));
    let store = Arc::new(SqliteStore::open(&repository.path().join("jameskills.sqlite3")).unwrap());
    let port = Arc::new(LocalRepoChangePort::new(
        process,
        store.clone(),
        Arc::new(TestClock),
    ));
    (RepositoryChangeService::new(port), store)
}

struct TestClock;

impl ClockPort for TestClock {
    fn now_utc(&self) -> String {
        "2026-10-05T12:00:00Z".to_owned()
    }

    fn monotonic_ms(&self) -> u64 {
        1
    }
}

fn block_on<F: Future>(future: F) -> F::Output {
    struct Noop;
    impl Wake for Noop {
        fn wake(self: std::sync::Arc<Self>) {}
    }
    let waker = Waker::from(std::sync::Arc::new(Noop));
    let mut context = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

#[test]
fn missing_target_preview_is_bounded_and_does_not_create_files_or_directories() {
    let repository = TempRepository::new();
    let root = repository.approved_root();
    let fingerprint = repository_root_fingerprint(&root).unwrap();
    let preview = plan_repo_template(&root, &fingerprint, &head(), RepoTemplateId::RustCi).unwrap();
    let target = repository.path().join(preview.target().as_str());

    assert!(
        preview
            .diff()
            .contains("+++ b/.github/workflows/jameskills-ci.yml")
    );
    assert!(preview.diff().contains("+name: JameSkills Rust CI"));
    assert!(
        preview
            .diff()
            .contains("actions/checkout@d23441a48e516b6c34aea4fa41551a30e30af803")
    );
    assert!(
        preview
            .diff()
            .contains("dtolnay/rust-toolchain@6bed0761d98439e5a578e2877258200ad565ba87 # v1.98.1")
    );
    assert!(preview.diff().contains("toolchain: 1.95.0"));
    assert!(!preview.diff().contains("TEMPLATE DE PLAN"));
    assert!(!preview.diff().contains("run: exit 1"));
    assert!(!target.exists());
    assert!(!repository.path().join(".github").exists());
}

#[test]
fn existing_unowned_target_is_conflict_and_remains_byte_identical() {
    let repository = TempRepository::new();
    let target = repository.path().join(RepoTemplateId::RustCi.target());
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    let original = b"user-owned workflow\n";
    std::fs::write(&target, original).unwrap();
    let root = repository.approved_root();
    let fingerprint = repository_root_fingerprint(&root).unwrap();

    assert!(matches!(
        plan_repo_template(&root, &fingerprint, &head(), RepoTemplateId::RustCi),
        Err(AppError::Conflict { .. })
    ));
    assert_eq!(std::fs::read(target).unwrap(), original);
}

#[test]
fn root_fingerprint_mismatch_is_rejected_before_preview() {
    let repository = TempRepository::new();
    let root = repository.approved_root();
    let wrong_fingerprint = ContentHash::parse_hex(&"b".repeat(64)).unwrap();

    assert!(matches!(
        plan_repo_template(&root, &wrong_fingerprint, &head(), RepoTemplateId::RustCi),
        Err(AppError::Conflict { .. })
    ));
    assert!(!repository.path().join(".github").exists());
}

#[test]
fn stale_git_head_blocks_preview_after_fixed_read_only_argv() {
    let repository = TempRepository::new();
    let (git, fingerprint) = approved_git(&repository);
    let request = repo_policy_request(&repository, git);
    let process = FakeProcessPort::new(
        fingerprint,
        vec![
            git_output("git version 2.50.1\n"),
            git_output(&format!("{}\n", "b".repeat(40))),
        ],
    );

    assert!(matches!(
        block_on(plan_repo_template_with_approved_git(
            &request,
            &process,
            "2026-10-05T12:00:00Z",
            CancellationToken::new(),
        )),
        Err(AppError::Conflict { .. })
    ));
    let invocations = process.invocations.lock().unwrap();
    assert_eq!(invocations.len(), 2);
    assert_eq!(invocations[0].0, ["--version"]);
    assert!(matches!(invocations[0].1, ProcessPermission::ReadOnlyCheck));
    assert_eq!(invocations[1].0, ["rev-parse", "HEAD"]);
    assert!(matches!(invocations[1].1, ProcessPermission::ReadOnlyCheck));
    assert!(!repository.path().join(".github").exists());
}

#[test]
fn compatible_git_and_matching_head_produce_preview_with_read_only_probes() {
    let repository = TempRepository::new();
    let (git, fingerprint) = approved_git(&repository);
    let request = repo_policy_request(&repository, git);
    let expected_head = format!("{}\n", "a".repeat(40));
    let process = FakeProcessPort::new(
        fingerprint,
        vec![
            git_output("git version 2.50.1\n"),
            git_output(&expected_head),
            git_output(&expected_head),
        ],
    );

    let preview = block_on(plan_repo_template_with_approved_git(
        &request,
        &process,
        "2026-10-05T12:00:00Z",
        CancellationToken::new(),
    ))
    .unwrap();

    assert!(preview.diff().contains("+name: JameSkills Rust CI"));
    assert_eq!(process.invocations.lock().unwrap().len(), 3);
    assert!(!repository.path().join(".github").exists());
}

#[test]
fn incompatible_git_version_stops_before_head_probe_or_filesystem_change() {
    let repository = TempRepository::new();
    let (git, fingerprint) = approved_git(&repository);
    let request = repo_policy_request(&repository, git);
    let process = FakeProcessPort::new(fingerprint, vec![git_output("git version 1.9.0\n")]);

    assert!(matches!(
        block_on(plan_repo_template_with_approved_git(
            &request,
            &process,
            "2026-10-05T12:00:00Z",
            CancellationToken::new(),
        )),
        Err(AppError::CapabilityUnavailable { .. })
    ));
    assert_eq!(process.invocations.lock().unwrap().len(), 1);
    assert!(!repository.path().join(".github").exists());
}

#[test]
fn changed_git_executable_fingerprint_blocks_before_any_probe_or_write() {
    let repository = TempRepository::new();
    let (_approved_git, actual_fingerprint) = approved_git(&repository);
    let executable_path =
        repository
            .path()
            .join("bin")
            .join(if cfg!(windows) { "git.exe" } else { "git" });
    let wrong_fingerprint = ExecutableFingerprint::from_sha256([0xee; 32]);
    let wrong_git = ApprovedRepoGit::after_explicit_fingerprint_confirmation(
        ApprovedExecutable::from_absolute_path(std::fs::canonicalize(executable_path).unwrap())
            .unwrap(),
        wrong_fingerprint,
        &wrong_fingerprint,
        ApprovedEnv::new(Default::default()).unwrap(),
    )
    .unwrap();
    let request = repo_policy_request(&repository, wrong_git);
    let process =
        FakeProcessPort::new(actual_fingerprint, vec![git_output("git version 2.50.1\n")]);

    assert!(matches!(
        block_on(plan_repo_template_with_approved_git(
            &request,
            &process,
            "2026-10-05T12:00:00Z",
            CancellationToken::new(),
        )),
        Err(AppError::CapabilityUnavailable { .. })
    ));
    assert!(process.invocations.lock().unwrap().is_empty());
    assert!(!repository.path().join(".github").exists());
}

#[test]
fn approved_apply_creates_only_the_registered_template_and_commits_its_journal() {
    let repository = TempRepository::new();
    let workflow_directory = repository.path().join(".github/workflows");
    std::fs::create_dir_all(&workflow_directory).unwrap();
    let (git, fingerprint) = approved_git(&repository);
    let (service, store) = change_service(&repository, fingerprint, git_preview_outputs('a', 4));
    let preview = block_on(service.plan_repo_changes(
        &repo_policy_request(&repository, git),
        CancellationToken::new(),
    ))
    .unwrap();
    let operation_id = preview.operation_id();
    let confirmed_digest = preview.confirmation_digest().clone();

    let receipt = block_on(service.apply_repo_changes(
        repo_policy_request(&repository, approved_git(&repository).0),
        preview,
        &confirmed_digest,
        CancellationToken::new(),
    ))
    .unwrap();

    let target = repository.path().join(RepoTemplateId::RustCi.target());
    assert_eq!(
        std::fs::read_to_string(&target).unwrap(),
        include_str!("../../../examples/repository-foundation/templates/ci-rust.yml")
    );
    assert!(
        !workflow_directory
            .join(format!(".jameskills-{}.stage", operation_id.as_uuid()))
            .exists()
    );
    assert_eq!(receipt.operation_id(), operation_id);
    assert_eq!(
        store.load_operation(operation_id).unwrap().unwrap().state(),
        RepoChangeJournalState::Committed
    );
    assert!(store.pending_operations().unwrap().is_empty());
}

#[test]
fn cancellation_during_revalidation_removes_only_the_verified_stage() {
    let repository = TempRepository::new();
    let workflow_directory = repository.path().join(".github/workflows");
    std::fs::create_dir_all(&workflow_directory).unwrap();
    let (git, fingerprint) = approved_git(&repository);
    let mut responses = git_preview_outputs('a', 2)
        .into_iter()
        .map(Ok)
        .collect::<Vec<_>>();
    responses.push(Ok(git_output("git version 2.50.1\n")));
    responses.push(Ok(git_output(&format!("{}\n", "a".repeat(40)))));
    responses.push(Err(AppError::Cancelled));
    let process = Arc::new(FakeProcessPort::new_with_results(fingerprint, responses));
    let store = Arc::new(SqliteStore::open(&repository.path().join("jameskills.sqlite3")).unwrap());
    let port = Arc::new(LocalRepoChangePort::new(
        process,
        store.clone(),
        Arc::new(TestClock),
    ));
    let service = RepositoryChangeService::new(port);
    let preview = block_on(service.plan_repo_changes(
        &repo_policy_request(&repository, git),
        CancellationToken::new(),
    ))
    .unwrap();
    let operation_id = preview.operation_id();
    let digest = preview.confirmation_digest().clone();
    let stage = workflow_directory.join(format!(".jameskills-{}.stage", operation_id.as_uuid()));
    let target = repository.path().join(RepoTemplateId::RustCi.target());

    assert!(matches!(
        block_on(service.apply_repo_changes(
            repo_policy_request(&repository, approved_git(&repository).0),
            preview,
            &digest,
            CancellationToken::new(),
        )),
        Err(AppError::Cancelled)
    ));
    assert!(!stage.exists());
    assert!(!target.exists());
    assert_eq!(
        store.load_operation(operation_id).unwrap().unwrap().state(),
        RepoChangeJournalState::Recovered
    );
    assert!(store.pending_operations().unwrap().is_empty());
}

#[test]
fn destination_changed_after_preview_is_preserved_and_apply_is_rejected() {
    let repository = TempRepository::new();
    let workflow_directory = repository.path().join(".github/workflows");
    std::fs::create_dir_all(&workflow_directory).unwrap();
    let (git, fingerprint) = approved_git(&repository);
    let (service, store) = change_service(
        &repository,
        fingerprint,
        vec![
            git_output("git version 2.50.1\n"),
            git_output(&format!("{}\n", "a".repeat(40))),
            git_output(&format!("{}\n", "a".repeat(40))),
            git_output("git version 2.50.1\n"),
            git_output(&format!("{}\n", "a".repeat(40))),
        ],
    );
    let preview = block_on(service.plan_repo_changes(
        &repo_policy_request(&repository, git),
        CancellationToken::new(),
    ))
    .unwrap();
    let operation_id = preview.operation_id();
    let digest = preview.confirmation_digest().clone();
    let target = repository.path().join(RepoTemplateId::RustCi.target());
    let user_bytes = b"concurrent user workflow\n";
    std::fs::write(&target, user_bytes).unwrap();

    assert!(matches!(
        block_on(service.apply_repo_changes(
            repo_policy_request(&repository, approved_git(&repository).0),
            preview,
            &digest,
            CancellationToken::new(),
        )),
        Err(AppError::Conflict { .. })
    ));
    assert_eq!(std::fs::read(&target).unwrap(), user_bytes);
    assert!(store.load_operation(operation_id).unwrap().is_none());
}

#[test]
fn stale_head_after_preview_blocks_apply_without_creating_the_target() {
    let repository = TempRepository::new();
    std::fs::create_dir_all(repository.path().join(".github/workflows")).unwrap();
    let (git, fingerprint) = approved_git(&repository);
    let (service, store) = change_service(
        &repository,
        fingerprint,
        vec![
            git_output("git version 2.50.1\n"),
            git_output(&format!("{}\n", "a".repeat(40))),
            git_output(&format!("{}\n", "a".repeat(40))),
            git_output("git version 2.50.1\n"),
            git_output(&format!("{}\n", "b".repeat(40))),
        ],
    );
    let preview = block_on(service.plan_repo_changes(
        &repo_policy_request(&repository, git),
        CancellationToken::new(),
    ))
    .unwrap();
    let operation_id = preview.operation_id();
    let digest = preview.confirmation_digest().clone();
    let target = repository.path().join(RepoTemplateId::RustCi.target());

    assert!(matches!(
        block_on(service.apply_repo_changes(
            repo_policy_request(&repository, approved_git(&repository).0),
            preview,
            &digest,
            CancellationToken::new(),
        )),
        Err(AppError::Conflict { .. })
    ));
    assert!(!target.exists());
    assert!(store.load_operation(operation_id).unwrap().is_none());
}

#[test]
fn preexisting_stage_name_is_not_overwritten_or_journaled() {
    let repository = TempRepository::new();
    let workflow_directory = repository.path().join(".github/workflows");
    std::fs::create_dir_all(&workflow_directory).unwrap();
    let (git, fingerprint) = approved_git(&repository);
    let (service, store) = change_service(&repository, fingerprint, git_preview_outputs('a', 2));
    let preview = block_on(service.plan_repo_changes(
        &repo_policy_request(&repository, git),
        CancellationToken::new(),
    ))
    .unwrap();
    let operation_id = preview.operation_id();
    let digest = preview.confirmation_digest().clone();
    let stage = workflow_directory.join(format!(".jameskills-{}.stage", operation_id.as_uuid()));
    let user_stage_bytes = b"unowned stage path\n";
    std::fs::write(&stage, user_stage_bytes).unwrap();

    assert!(matches!(
        block_on(service.apply_repo_changes(
            repo_policy_request(&repository, approved_git(&repository).0),
            preview,
            &digest,
            CancellationToken::new(),
        )),
        Err(AppError::Conflict { .. })
    ));
    assert_eq!(std::fs::read(stage).unwrap(), user_stage_bytes);
    assert!(store.load_operation(operation_id).unwrap().is_none());
    assert!(
        !repository
            .path()
            .join(RepoTemplateId::RustCi.target())
            .exists()
    );
}

#[test]
fn missing_parent_directory_blocks_apply_without_creating_scaffolding() {
    let repository = TempRepository::new();
    let (git, fingerprint) = approved_git(&repository);
    let (service, store) = change_service(&repository, fingerprint, git_preview_outputs('a', 2));
    let preview = block_on(service.plan_repo_changes(
        &repo_policy_request(&repository, git),
        CancellationToken::new(),
    ))
    .unwrap();
    let operation_id = preview.operation_id();
    let digest = preview.confirmation_digest().clone();

    assert!(matches!(
        block_on(service.apply_repo_changes(
            repo_policy_request(&repository, approved_git(&repository).0),
            preview,
            &digest,
            CancellationToken::new(),
        )),
        Err(AppError::CapabilityUnavailable { .. })
    ));
    assert!(!repository.path().join(".github").exists());
    assert!(store.load_operation(operation_id).unwrap().is_none());
}

#[cfg(unix)]
#[test]
fn symlinked_target_parent_is_blocked_without_writing_outside_root() {
    use std::os::unix::fs::symlink;

    let repository = TempRepository::new();
    let outside = TempRepository::new();
    symlink(outside.path(), repository.path().join(".github")).unwrap();
    let root = repository.approved_root();
    let fingerprint = repository_root_fingerprint(&root).unwrap();

    assert!(matches!(
        plan_repo_template(&root, &fingerprint, &head(), RepoTemplateId::RustCi),
        Err(AppError::PermissionDenied { .. })
    ));
    assert!(std::fs::read_dir(outside.path()).unwrap().next().is_none());
}

#[cfg(windows)]
#[test]
#[ignore = "this Windows host lacks symlink/junction creation privilege (ERROR_PRIVILEGE_NOT_HELD)"]
fn reparse_target_parent_is_blocked_without_writing_outside_root() {
    use std::os::windows::fs::symlink_dir;

    let repository = TempRepository::new();
    let outside = TempRepository::new();
    symlink_dir(outside.path(), repository.path().join(".github")).unwrap();
    let root = repository.approved_root();
    let fingerprint = repository_root_fingerprint(&root).unwrap();

    assert!(matches!(
        plan_repo_template(&root, &fingerprint, &head(), RepoTemplateId::RustCi),
        Err(AppError::PermissionDenied { .. })
    ));
    assert!(std::fs::read_dir(outside.path()).unwrap().next().is_none());
}
