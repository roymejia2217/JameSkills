use async_trait::async_trait;
use jameskills_core::{
    AppError,
    application::policy::{CheckContext, CheckRequest, PolicyService},
    domain::{
        ToolId,
        policy::{CheckStatus, Enforcement, parse_policy},
    },
    ports::ClockPort,
    ports::process::{
        ApprovedEnv, ApprovedExecutable, ApprovedRoot, ApprovedScript, ProcessOutput, ProcessPort,
        ProcessSpec,
    },
};
use jameskills_infra::{
    fs::{
        ApprovedCommitlint, ApprovedCommitlintNode, ApprovedRepositoryTool, LocalFileSystem,
        RepositoryPolicyCheckProvider, commitlint_cli_entrypoint_for_candidate,
    },
    platform::{HostPlatform, ToolCandidateKind, find_tool_candidates, load_tool_profiles},
    process::{SystemProcessPort, fingerprint_executable},
};
use std::{
    collections::{BTreeMap, VecDeque},
    ffi::OsString,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

const NOW: &str = "2026-10-04T00:00:00Z";
const ENVIRONMENT: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const CONVENTIONAL_COMMIT_POLICY: &str = r#"
schema_version = 1
profile = "repository-foundation"
scope = "project"

[[tool_requirements]]
tool_id = "commitlint"
operation = "lint-message"
version = "=21.2.2"

[[requirements]]
id = "commit-message"
description = "The current commit uses Conventional Commits."
severity = "error"
required = true
phase = "commit"
enforcement = "local-check"
depends_on = []
[requirements.check]
kind = "conventional-commit"
"#;
static NEXT_ROOT_ID: AtomicU64 = AtomicU64::new(0);

struct TestRoot(PathBuf);

impl TestRoot {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "jameskills-commit-check-{}-{}",
            std::process::id(),
            NEXT_ROOT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("bin")).unwrap();
        Self(root)
    }

    fn tool(&self, name: &str) -> ApprovedRepositoryTool {
        let executable_name = if cfg!(windows) {
            format!("{name}.exe")
        } else {
            name.to_owned()
        };
        self.tool_named(&executable_name)
    }

    fn tool_named(&self, executable_name: &str) -> ApprovedRepositoryTool {
        let path = self.0.join("bin").join(executable_name);
        std::fs::write(&path, b"synthetic approved executable").unwrap();
        let path = std::fs::canonicalize(path).unwrap();
        ApprovedRepositoryTool::new(
            ApprovedExecutable::from_absolute_path(path.clone()).unwrap(),
            fingerprint_executable(&path).unwrap(),
        )
    }

    fn approved(&self) -> ApprovedRoot {
        ApprovedRoot::from_absolute_path(std::fs::canonicalize(&self.0).unwrap()).unwrap()
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn commitlint_command_shim_resolves_only_the_registered_cli_js_entrypoint() {
    let root = TestRoot::new();
    let launcher_dir = root.0.join("project").join("node_modules").join(".bin");
    let package_dir = root
        .0
        .join("project")
        .join("node_modules")
        .join("@commitlint")
        .join("cli");
    std::fs::create_dir_all(&launcher_dir).unwrap();
    std::fs::create_dir_all(&package_dir).unwrap();
    let launcher = launcher_dir.join("commitlint.cmd");
    let entrypoint = package_dir.join("cli.js");
    std::fs::write(&launcher, b"@echo off\n").unwrap();
    std::fs::write(&entrypoint, b"// reviewed package fixture\n").unwrap();
    let profile = load_tool_profiles()
        .unwrap()
        .into_iter()
        .find(|profile| profile.tool_id() == ToolId::Commitlint)
        .unwrap();
    let candidate = find_tool_candidates(
        &[profile],
        &[std::fs::canonicalize(&launcher_dir).unwrap()],
        HostPlatform::Windows,
    )
    .into_iter()
    .next()
    .unwrap();

    assert_eq!(candidate.kind(), ToolCandidateKind::CommandShim);
    assert_eq!(
        commitlint_cli_entrypoint_for_candidate(&candidate).as_deref(),
        Some(std::fs::canonicalize(entrypoint).unwrap().as_path())
    );
    assert!(!launcher_dir.join("commitlint.js").exists());
}

struct TestClock;

impl ClockPort for TestClock {
    fn now_utc(&self) -> String {
        NOW.to_owned()
    }

    fn monotonic_ms(&self) -> u64 {
        10
    }
}

struct Invocation {
    tool_id: ToolId,
    executable: PathBuf,
    args: Vec<OsString>,
    cwd: PathBuf,
    edit_path: Option<PathBuf>,
    message: Option<Vec<u8>>,
    config_path: Option<PathBuf>,
    config: Option<Vec<u8>>,
    path_environment: Option<OsString>,
    approved_script: Option<PathBuf>,
    approved_script_fingerprint: Option<Vec<u8>>,
}

struct FakeProcess {
    outputs: Mutex<VecDeque<ProcessOutput>>,
    invocations: Mutex<Vec<Invocation>>,
}

impl FakeProcess {
    fn new(outputs: Vec<ProcessOutput>) -> Self {
        Self {
            outputs: Mutex::new(outputs.into()),
            invocations: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl ProcessPort for FakeProcess {
    async fn run(&self, spec: ProcessSpec) -> Result<ProcessOutput, AppError> {
        let edit_path = spec
            .args()
            .windows(2)
            .find(|pair| pair[0] == "--edit")
            .map(|pair| PathBuf::from(&pair[1]));
        let config_path = spec
            .args()
            .windows(2)
            .find(|pair| pair[0] == "--config")
            .map(|pair| PathBuf::from(&pair[1]));
        let message = edit_path.as_ref().and_then(|path| std::fs::read(path).ok());
        let config = config_path
            .as_ref()
            .and_then(|path| std::fs::read(path).ok());
        self.invocations.lock().unwrap().push(Invocation {
            tool_id: spec
                .tool_id()
                .expect("commit checks use a registered ToolId"),
            executable: spec.executable().path().to_path_buf(),
            args: spec.args().to_vec(),
            cwd: spec.cwd().path().to_path_buf(),
            edit_path,
            message,
            config_path,
            config,
            path_environment: spec.env().entries().get(&OsString::from("PATH")).cloned(),
            approved_script: spec
                .approved_script()
                .map(|(script, _)| script.path().to_path_buf()),
            approved_script_fingerprint: spec
                .approved_script()
                .map(|(_, fingerprint)| fingerprint.as_bytes().to_vec()),
        });
        self.outputs
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| AppError::ExternalTool {
                tool_id: "test-process".to_owned(),
                exit_code: None,
            })
    }
}

fn output(exit_code: i32, stdout: &[u8]) -> ProcessOutput {
    ProcessOutput::new(Some(exit_code), stdout.to_vec(), Vec::new())
}

fn run_check(
    root: &TestRoot,
    git: &ApprovedRepositoryTool,
    commitlint: ApprovedRepositoryTool,
    process: Arc<FakeProcess>,
) -> jameskills_core::domain::policy::CheckObservation {
    let environment = ApprovedEnv::new(BTreeMap::new()).unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    runtime
        .block_on(LocalFileSystem.check_conventional_commit(
            &root.approved(),
            Some(git),
            Some(&ApprovedCommitlint::Native(commitlint)),
            &environment,
            process.as_ref(),
            NOW,
            ENVIRONMENT,
        ))
        .unwrap()
}

#[test]
fn conventional_commit_check_uses_private_message_file_and_builtin_rules() {
    let root = TestRoot::new();
    let hook_directory = root.0.join(".git/hooks");
    std::fs::create_dir_all(&hook_directory).unwrap();
    let hook_path = hook_directory.join("commit-msg");
    std::fs::write(
        &hook_path,
        b"#!/bin/sh\nprintf 'hook must not execute' > hook-marker\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&hook_path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let git = root.tool("git");
    let commitlint = root.tool("commitlint");
    let message =
        b"feat(policy-engine): verify commit messages\n\nUse the approved commitlint driver.\n";
    let process = Arc::new(FakeProcess::new(vec![
        output(0, b"git version 2.55.0\n"),
        output(0, b"@commitlint/cli@21.2.2\n"),
        output(0, message),
        output(0, b""),
        output(0, b".git/hooks/commit-msg\n"),
    ]));

    let status = run_check(&root, &git, commitlint, Arc::clone(&process));

    assert_eq!(status.status(), CheckStatus::Pass);
    assert!(matches!(
        status.enforcement(),
        Some(Enforcement::LocalCheck)
    ));
    let hook_evidence = status
        .evidence()
        .iter()
        .find(|evidence| evidence.source_id() == "repo.commit-hook")
        .unwrap();
    assert!(hook_evidence.summary().contains("content-hashed"));
    assert!(hook_evidence.summary().contains("invocation are unproven"));
    assert!(!root.0.join("hook-marker").exists());
    let invocations = process.invocations.lock().unwrap();
    assert_eq!(invocations.len(), 5);
    assert!(matches!(invocations[2].tool_id, ToolId::Git));
    assert_eq!(
        invocations[2].args,
        ["--no-pager", "log", "-1", "--format=%B"]
    );
    assert!(matches!(invocations[3].tool_id, ToolId::Commitlint));
    assert_eq!(
        invocations[4].args,
        ["rev-parse", "--git-path", "hooks/commit-msg"]
    );
    assert!(
        invocations[3]
            .args
            .iter()
            .any(|arg| arg == "--default-config")
    );
    assert_eq!(invocations[3].message.as_deref(), Some(message.as_slice()));
    let edit_path = invocations[3].edit_path.as_ref().unwrap();
    let config_path = invocations[3].config_path.as_ref().unwrap();
    assert!(!edit_path.starts_with(&root.0));
    assert!(!config_path.starts_with(&root.0));
    assert!(!invocations[3].cwd.starts_with(&root.0));
    assert!(!edit_path.exists());
    assert!(!config_path.exists());
    assert_eq!(
        invocations[3].config.as_deref(),
        Some(b"{\"rules\":{}}\n".as_slice())
    );
    let path_entries = std::env::split_paths(invocations[3].path_environment.as_ref().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        path_entries[0],
        std::fs::canonicalize(root.0.join("bin")).unwrap()
    );
}

#[test]
fn conventional_commit_failure_withholds_message_and_tool_output() {
    let root = TestRoot::new();
    let git = root.tool("git");
    let commitlint = root.tool("commitlint");
    let private_text = b"feat: synthetic secret-marker-42";
    let process = Arc::new(FakeProcess::new(vec![
        output(0, b"git version 2.55.0\n"),
        output(0, b"@commitlint/cli@21.2.2\n"),
        output(0, private_text),
        output(1, b"secret-marker-42 invalid commit message"),
        output(0, b".git/hooks/commit-msg\n"),
    ]));

    let observation = run_check(&root, &git, commitlint, process);

    assert_eq!(observation.status(), CheckStatus::Fail);
    assert!(
        observation
            .evidence()
            .iter()
            .all(|evidence| { !evidence.summary().contains("secret-marker-42") })
    );
}

#[test]
fn conventional_commit_blocks_unreviewed_version_and_unregistered_executable() {
    let root = TestRoot::new();
    let git = root.tool("git");
    let commitlint = root.tool("commitlint");
    let unreviewed = Arc::new(FakeProcess::new(vec![
        output(0, b"git version 2.55.0\n"),
        output(0, b"@commitlint/cli@21.2.3\n"),
    ]));
    let observation = run_check(&root, &git, commitlint, Arc::clone(&unreviewed));
    assert_eq!(observation.status(), CheckStatus::Blocked);
    assert_eq!(unreviewed.invocations.lock().unwrap().len(), 2);

    let root = TestRoot::new();
    let git = root.tool("git");
    let commitlint = root.tool_named("commitlint-helper.exe");
    let blocked_process = Arc::new(FakeProcess::new(Vec::new()));
    let observation = run_check(&root, &git, commitlint, Arc::clone(&blocked_process));
    assert_eq!(observation.status(), CheckStatus::Blocked);
    assert!(blocked_process.invocations.lock().unwrap().is_empty());
}

#[test]
fn provider_runs_reviewed_commitlint_cli_through_approved_node_as_local_check() {
    let root = TestRoot::new();
    let git = root.tool("git");
    let node = root.tool("node");
    let cli_directory = root.0.join("node_modules").join("@commitlint").join("cli");
    std::fs::create_dir_all(&cli_directory).unwrap();
    let cli_path = cli_directory.join("cli.js");
    std::fs::write(&cli_path, b"reviewed Commitlint entrypoint fixture").unwrap();
    let cli_path = std::fs::canonicalize(cli_path).unwrap();
    let cli_fingerprint = fingerprint_executable(&cli_path).unwrap();
    let process = Arc::new(FakeProcess::new(vec![
        output(0, b"git version 2.55.0\n"),
        output(0, b"v24.18.0\n"),
        output(0, b"@commitlint/cli@21.2.2\n"),
        output(0, b"feat(policy-engine): verify node entrypoint\n"),
        output(0, b""),
        output(0, b".git/hooks/commit-msg\n"),
    ]));
    let report = run_node_policy_check(&root, git, node, &cli_path, process.clone());

    assert_eq!(report.results().len(), 1);
    assert_eq!(report.results()[0].status(), CheckStatus::Pass);
    assert!(matches!(
        report.results()[0].enforcement(),
        Some(Enforcement::LocalCheck)
    ));
    assert_eq!(report.strict_exit(), 0);
    let invocations = process.invocations.lock().unwrap();
    assert_eq!(invocations.len(), 6);
    assert!(matches!(invocations[1].tool_id, ToolId::Node));
    assert!(matches!(invocations[2].tool_id, ToolId::Commitlint));
    assert_eq!(invocations[2].executable, node_path(&invocations));
    assert!(invocations[2].args[0].to_string_lossy().ends_with("cli.js"));
    assert!(
        !invocations[2].args[0]
            .to_string_lossy()
            .starts_with(r"\\?\")
    );
    assert_eq!(invocations[2].args[1], "--version");
    assert!(matches!(invocations[4].tool_id, ToolId::Commitlint));
    assert_eq!(invocations[4].executable, node_path(&invocations));
    assert!(invocations[4].args[0].to_string_lossy().ends_with("cli.js"));
    assert!(
        !invocations[4].args[0]
            .to_string_lossy()
            .starts_with(r"\\?\")
    );
    assert_eq!(invocations[4].args[1], "--cwd");
    assert!(
        invocations[4].args[2]
            .to_string_lossy()
            .ends_with(root.0.file_name().unwrap().to_string_lossy().as_ref())
    );
    assert!(
        invocations[4]
            .args
            .iter()
            .any(|arg| arg == "--default-config")
    );
    assert_eq!(
        invocations[4].approved_script.as_deref(),
        Some(cli_path.as_path())
    );
    assert_eq!(
        invocations[4].approved_script_fingerprint.as_deref(),
        Some(cli_fingerprint.as_bytes().as_slice())
    );
}

#[test]
fn provider_blocks_node_below_commitlint_engine_minimum() {
    let root = TestRoot::new();
    let git = root.tool("git");
    let node = root.tool("node");
    let cli_directory = root.0.join("node_modules").join("@commitlint").join("cli");
    std::fs::create_dir_all(&cli_directory).unwrap();
    let cli_path = cli_directory.join("cli.js");
    std::fs::write(&cli_path, b"reviewed Commitlint entrypoint fixture").unwrap();
    let process = Arc::new(FakeProcess::new(vec![
        output(0, b"git version 2.55.0\n"),
        output(0, b"v22.11.0\n"),
    ]));

    let report = run_node_policy_check(&root, git, node, &cli_path, process.clone());

    assert_eq!(report.results()[0].status(), CheckStatus::Blocked);
    assert_eq!(report.strict_exit(), 1);
    assert_eq!(process.invocations.lock().unwrap().len(), 2);
}

#[test]
fn provider_blocks_when_commitlint_driver_is_not_approved() {
    let root = TestRoot::new();
    let process = Arc::new(FakeProcess::new(Vec::new()));
    let provider = RepositoryPolicyCheckProvider::new(
        root.approved(),
        None,
        None,
        ApprovedEnv::new(BTreeMap::new()).unwrap(),
        process.clone(),
        Arc::new(TestClock),
        ENVIRONMENT.to_owned(),
    );
    let service = PolicyService::new(Arc::new(provider), Arc::new(TestClock));
    let report = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(service.check(CheckRequest::new(
            parse_policy(CONVENTIONAL_COMMIT_POLICY.as_bytes()).unwrap(),
            CheckContext::default(),
        )))
        .unwrap();

    assert_eq!(report.results()[0].status(), CheckStatus::Blocked);
    assert_eq!(report.strict_exit(), 1);
    assert!(process.invocations.lock().unwrap().is_empty());
}

fn run_node_policy_check(
    root: &TestRoot,
    git: ApprovedRepositoryTool,
    node: ApprovedRepositoryTool,
    cli_path: &std::path::Path,
    process: Arc<FakeProcess>,
) -> jameskills_core::domain::policy::CheckReport {
    let cli_path = std::fs::canonicalize(cli_path).unwrap();
    let cli_fingerprint = fingerprint_executable(&cli_path).unwrap();
    let provider = RepositoryPolicyCheckProvider::new(
        root.approved(),
        Some(git),
        None,
        ApprovedEnv::new(BTreeMap::new()).unwrap(),
        process,
        Arc::new(TestClock),
        ENVIRONMENT.to_owned(),
    )
    .with_commitlint(ApprovedCommitlint::Node(ApprovedCommitlintNode::new(
        node,
        ApprovedScript::from_absolute_path(cli_path).unwrap(),
        cli_fingerprint,
    )));
    let service = PolicyService::new(Arc::new(provider), Arc::new(TestClock));
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(service.check(CheckRequest::new(
            parse_policy(CONVENTIONAL_COMMIT_POLICY.as_bytes()).unwrap(),
            CheckContext::default(),
        )))
        .unwrap()
}

#[test]
#[ignore = "requires explicitly installed Node 22.12+ and locked Commitlint 21.2.2 package"]
fn real_node_commitlint_package_passes_through_repository_policy_provider() {
    let repository = std::fs::canonicalize(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("infra crate has a workspace root"),
    )
    .unwrap();
    let cli_path = repository.join("node_modules/@commitlint/cli/cli.js");
    if !cli_path.is_file() {
        panic!("run npm ci before explicitly running this ignored integration test");
    }
    let cli_path = std::fs::canonicalize(cli_path).unwrap();
    let git_path = find_executable("git").expect("approved Git must be available");
    let node_path = find_executable("node").expect("approved Node must be available");
    let git = ApprovedRepositoryTool::new(
        ApprovedExecutable::from_absolute_path(git_path.clone()).unwrap(),
        fingerprint_executable(&git_path).unwrap(),
    );
    let node = ApprovedRepositoryTool::new(
        ApprovedExecutable::from_absolute_path(node_path.clone()).unwrap(),
        fingerprint_executable(&node_path).unwrap(),
    );
    let commitlint = ApprovedCommitlint::Node(ApprovedCommitlintNode::new(
        node,
        ApprovedScript::from_absolute_path(cli_path.clone()).unwrap(),
        fingerprint_executable(&cli_path).unwrap(),
    ));
    let mut environment = BTreeMap::new();
    for key in [
        "PATH",
        "HOME",
        "USERPROFILE",
        "SYSTEMROOT",
        "WINDIR",
        "LANG",
        "LC_ALL",
        "LANGUAGE",
        "TMP",
        "TEMP",
    ] {
        if let Some(value) = std::env::var_os(key) {
            environment.insert(OsString::from(key), value);
        }
    }
    let provider = RepositoryPolicyCheckProvider::new(
        ApprovedRoot::from_absolute_path(repository).unwrap(),
        Some(git),
        None,
        ApprovedEnv::new(environment).unwrap(),
        Arc::new(SystemProcessPort),
        Arc::new(TestClock),
        ENVIRONMENT.to_owned(),
    )
    .with_commitlint(commitlint);
    let service = PolicyService::new(Arc::new(provider), Arc::new(TestClock));
    let report = service.check(CheckRequest::new(
        parse_policy(CONVENTIONAL_COMMIT_POLICY.as_bytes()).unwrap(),
        CheckContext::default(),
    ));
    let report = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(report)
        .unwrap();

    assert_eq!(report.results()[0].status(), CheckStatus::Pass);
    assert!(matches!(
        report.results()[0].enforcement(),
        Some(Enforcement::LocalCheck)
    ));
    assert!(report.results()[0].evidence().iter().any(|evidence| {
        evidence.source_id() == "repo.commit-hook" && evidence.summary().contains("content-hashed")
    }));
    assert_eq!(report.strict_exit(), 0);
}

fn find_executable(name: &str) -> Option<PathBuf> {
    let filename = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    };
    std::env::split_paths(&std::env::var_os("PATH")?)
        .filter(|directory| directory.is_absolute())
        .map(|directory| directory.join(&filename))
        .find_map(|path| {
            let metadata = std::fs::symlink_metadata(&path).ok()?;
            if !metadata.file_type().is_file() {
                return None;
            }
            std::fs::canonicalize(path).ok()
        })
}

fn node_path(invocations: &[Invocation]) -> PathBuf {
    invocations[1].executable.clone()
}
