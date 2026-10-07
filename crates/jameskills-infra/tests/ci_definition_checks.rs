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
        ApprovedEnv, ApprovedExecutable, ApprovedRoot, ProcessOutput, ProcessPermission,
        ProcessPort, ProcessSpec,
    },
};
use jameskills_infra::{
    fs::{ApprovedRepositoryTool, RepositoryPolicyCheckProvider},
    process::{SystemProcessPort, fingerprint_executable},
};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

const NOW: &str = "2026-10-05T12:00:00Z";
const ENVIRONMENT: &str = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const GITHUB_REMOTE: &[u8] = b"origin\thttps://github.com/example/repository.git (fetch)\norigin\thttps://github.com/example/repository.git (push)\n";
const CI_POLICY: &str = r#"
schema_version = 1
profile = "repository-foundation"
scope = "project"

[[requirements]]
id = "ci-contract"
description = "The GitHub workflow runs CI on pushes and pull requests."
severity = "error"
required = true
phase = "ci"
enforcement = "local-check"
depends_on = []
[requirements.check]
kind = "ci-contract"
workflow_paths = [".github/workflows/ci.yml"]
required_jobs = ["quality"]
"#;
const CI_EVIDENCE_POLICY: &str = r#"
schema_version = 1
profile = "repository-foundation"
scope = "project"

[[tool_requirements]]
tool_id = "gh"
operation = "check-runs"
version = ">=2.0.0"

[[requirements]]
id = "ci-current"
description = "The current repository SHA has passed required CI."
severity = "error"
required = true
phase = "pre-release"
enforcement = "required-ci"
depends_on = []
[requirements.check]
kind = "ci-evidence"
required_checks = ["Required CI"]
"#;
static NEXT_ROOT_ID: AtomicU64 = AtomicU64::new(0);

struct TestRoot(PathBuf);

impl TestRoot {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "jameskills-ci-contract-{}-{}",
            std::process::id(),
            NEXT_ROOT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(path.join(".github/workflows")).unwrap();
        std::fs::create_dir_all(path.join("bin")).unwrap();
        Self(path)
    }

    fn workflow(&self, source: &str) {
        std::fs::write(self.0.join(".github/workflows/ci.yml"), source).unwrap();
    }

    fn git(&self) -> ApprovedRepositoryTool {
        let path = self
            .0
            .join("bin")
            .join(if cfg!(windows) { "git.exe" } else { "git" });
        std::fs::write(&path, b"synthetic approved Git").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let path = std::fs::canonicalize(path).unwrap();
        ApprovedRepositoryTool::new(
            ApprovedExecutable::from_absolute_path(path.clone()).unwrap(),
            fingerprint_executable(&path).unwrap(),
        )
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct TestClock;

impl ClockPort for TestClock {
    fn now_utc(&self) -> String {
        NOW.to_owned()
    }

    fn monotonic_ms(&self) -> u64 {
        1
    }
}

struct FakeGit {
    remote_output: Vec<u8>,
    invocations: Mutex<Vec<Vec<OsString>>>,
}

impl FakeGit {
    fn new(remote_output: &[u8]) -> Self {
        Self {
            remote_output: remote_output.to_vec(),
            invocations: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl ProcessPort for FakeGit {
    async fn run(&self, spec: ProcessSpec) -> Result<ProcessOutput, AppError> {
        assert!(matches!(spec.tool_id(), Some(ToolId::Git)));
        assert!(matches!(
            spec.permission(),
            ProcessPermission::ReadOnlyCheck
        ));
        assert!(spec.approved_executable_fingerprint().is_some());
        self.invocations.lock().unwrap().push(spec.args().to_vec());
        let stdout = if spec.args() == [OsString::from("--version")] {
            b"git version 2.55.0\n".to_vec()
        } else if spec.args() == [OsString::from("remote"), OsString::from("-v")] {
            self.remote_output.clone()
        } else {
            panic!("unexpected Git argv in CI host observation")
        };
        Ok(ProcessOutput::new(Some(0), stdout, Vec::new()))
    }
}

fn inspect(root: &TestRoot) -> jameskills_core::domain::policy::CheckReport {
    inspect_with_policy(root, CI_POLICY)
}

fn inspect_with_policy(
    root: &TestRoot,
    policy_source: &str,
) -> jameskills_core::domain::policy::CheckReport {
    inspect_with_remote(root, policy_source, GITHUB_REMOTE).0
}

fn inspect_with_remote(
    root: &TestRoot,
    policy_source: &str,
    remote_output: &[u8],
) -> (jameskills_core::domain::policy::CheckReport, Arc<FakeGit>) {
    let approved_root =
        ApprovedRoot::from_absolute_path(std::fs::canonicalize(&root.0).unwrap()).unwrap();
    let environment = ApprovedEnv::new(BTreeMap::new()).unwrap();
    let process = Arc::new(FakeGit::new(remote_output));
    let provider = RepositoryPolicyCheckProvider::new(
        approved_root,
        Some(root.git()),
        None,
        environment,
        process.clone(),
        Arc::new(TestClock),
        ENVIRONMENT.to_owned(),
    );
    let report = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(
            PolicyService::new(Arc::new(provider), Arc::new(TestClock)).check(CheckRequest::new(
                parse_policy(policy_source.as_bytes()).unwrap(),
                CheckContext::default(),
            )),
        )
        .unwrap();
    (report, process)
}

const VALID_WORKFLOW: &str = r#"
name: CI
on:
  push:
  pull_request:
permissions:
  contents: read
jobs:
  quality:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@d23441a48e516b6c34aea4fa41551a30e30af803
      - run: cargo test --workspace --locked
"#;

#[test]
fn valid_workflow_proves_only_a_local_ci_contract() {
    let root = TestRoot::new();
    root.workflow(VALID_WORKFLOW);

    let report = inspect(&root);

    assert_eq!(report.results().len(), 1);
    assert_eq!(report.results()[0].status(), CheckStatus::Pass);
    assert!(matches!(
        report.results()[0].enforcement(),
        Some(Enforcement::LocalCheck)
    ));
    assert_eq!(report.strict_exit(), 0);
}

#[test]
fn checked_in_ci_workflow_has_the_registered_required_aggregator() {
    let root = TestRoot::new();
    root.workflow(include_str!("../../../.github/workflows/ci.yml"));
    let policy = CI_POLICY.replace("\"quality\"", "\"required-ci\"");

    let report = inspect_with_policy(&root, &policy);

    assert_eq!(report.results()[0].status(), CheckStatus::Pass);
    assert!(matches!(
        report.results()[0].enforcement(),
        Some(Enforcement::LocalCheck)
    ));
}

#[test]
fn non_github_or_missing_remote_keeps_workflow_unknown() {
    for remote_output in [
        b"origin\thttps://gitlab.com/example/repository.git (fetch)\n".as_slice(),
        b"origin\thttps://github.enterprise.example/example/repository.git (fetch)\n".as_slice(),
        b"origin\thttps://github.com/example/repository.git (fetch)\nupstream\thttps://gitlab.com/example/repository.git (fetch)\n".as_slice(),
        b"".as_slice(),
    ] {
        let root = TestRoot::new();
        root.workflow(VALID_WORKFLOW);

        let (report, _) = inspect_with_remote(&root, CI_POLICY, remote_output);

        assert_eq!(report.results()[0].status(), CheckStatus::Unknown);
        assert!(report.results()[0].enforcement().is_none());
    }
}

#[test]
fn github_remote_host_is_observed_without_leaking_url_credentials() {
    let root = TestRoot::new();
    root.workflow(VALID_WORKFLOW);
    let remote =
        b"origin\thttps://secret-marker:credential@github.com/example/repository.git (fetch)\n";

    let (report, process) = inspect_with_remote(&root, CI_POLICY, remote);

    assert_eq!(report.results()[0].status(), CheckStatus::Pass);
    assert!(matches!(
        report.results()[0].enforcement(),
        Some(Enforcement::LocalCheck)
    ));
    assert!(
        report.results()[0]
            .evidence()
            .iter()
            .all(|evidence| !evidence.summary().contains("secret-marker"))
    );
    let invocations = process.invocations.lock().unwrap();
    assert_eq!(invocations.len(), 2);
    assert_eq!(invocations[0], [OsString::from("--version")]);
    assert_eq!(
        invocations[1],
        [OsString::from("remote"), OsString::from("-v")]
    );
}

#[test]
fn missing_pull_request_trigger_fails_the_local_contract() {
    let root = TestRoot::new();
    root.workflow(&VALID_WORKFLOW.replace("  pull_request:\n", ""));

    let report = inspect(&root);

    assert_eq!(report.results()[0].status(), CheckStatus::Fail);
    assert!(matches!(
        report.results()[0].enforcement(),
        Some(Enforcement::LocalCheck)
    ));
}

#[test]
fn pull_request_path_filter_fails_closed_for_required_checks() {
    let root = TestRoot::new();
    root.workflow(&VALID_WORKFLOW.replace(
        "  pull_request:\n",
        "  pull_request:\n    paths: ['src/**']\n",
    ));

    let report = inspect(&root);

    assert_eq!(report.results()[0].status(), CheckStatus::Fail);
}

#[test]
fn pull_request_activity_filter_must_check_new_and_updated_prs() {
    let root = TestRoot::new();
    root.workflow(&VALID_WORKFLOW.replace(
        "  pull_request:\n",
        "  pull_request:\n    types: [opened, closed]\n",
    ));

    let report = inspect(&root);

    assert_eq!(report.results()[0].status(), CheckStatus::Fail);
}

#[test]
fn tag_only_push_trigger_does_not_claim_branch_ci() {
    let root = TestRoot::new();
    root.workflow(&VALID_WORKFLOW.replace("  push:\n", "  push:\n    tags: ['v*']\n"));

    let report = inspect(&root);

    assert_eq!(report.results()[0].status(), CheckStatus::Fail);
}

#[test]
fn pull_request_branch_filter_is_unknown_without_protected_branch_scope() {
    let root = TestRoot::new();
    root.workflow(&VALID_WORKFLOW.replace(
        "  pull_request:\n",
        "  pull_request:\n    branches: [main]\n",
    ));

    let report = inspect(&root);

    assert_eq!(report.results()[0].status(), CheckStatus::Unknown);
    assert!(report.results()[0].enforcement().is_none());
}

#[test]
fn continue_on_error_on_required_job_fails_the_local_contract() {
    let root = TestRoot::new();
    root.workflow(&VALID_WORKFLOW.replace(
        "  quality:\n    runs-on:",
        "  quality:\n    continue-on-error: true\n    runs-on:",
    ));

    let report = inspect(&root);

    assert_eq!(report.results()[0].status(), CheckStatus::Fail);
}

#[test]
fn missing_required_job_fails_the_local_contract() {
    let root = TestRoot::new();
    root.workflow(VALID_WORKFLOW);
    let policy = CI_POLICY.replace("\"quality\"", "\"release-check\"");

    let report = inspect_with_policy(&root, &policy);

    assert_eq!(report.results()[0].status(), CheckStatus::Fail);
}

#[test]
fn workflow_job_without_runner_fails_the_local_contract() {
    let root = TestRoot::new();
    root.workflow(&VALID_WORKFLOW.replace("    runs-on: ubuntu-latest\n", ""));

    let report = inspect(&root);

    assert_eq!(report.results()[0].status(), CheckStatus::Fail);
}

#[test]
fn workflow_job_with_unknown_dependency_fails_the_local_contract() {
    let root = TestRoot::new();
    root.workflow(&VALID_WORKFLOW.replace("  quality:\n", "  quality:\n    needs: missing-job\n"));

    let report = inspect(&root);

    assert_eq!(report.results()[0].status(), CheckStatus::Fail);
}

#[test]
fn dynamic_condition_on_required_job_is_unknown() {
    let root = TestRoot::new();
    root.workflow(&VALID_WORKFLOW.replace(
        "  quality:\n",
        "  quality:\n    if: github.ref == 'refs/heads/main'\n",
    ));

    let report = inspect(&root);

    assert_eq!(report.results()[0].status(), CheckStatus::Unknown);
    assert!(report.results()[0].enforcement().is_none());
}

#[test]
fn conditional_required_step_remains_unknown() {
    let root = TestRoot::new();
    root.workflow(&VALID_WORKFLOW.replace("      - run:", "      - if: failure()\n        run:"));

    let report = inspect(&root);

    assert_eq!(report.results()[0].status(), CheckStatus::Unknown);
    assert!(report.results()[0].enforcement().is_none());
}

#[test]
fn malformed_or_unsupported_workflow_is_unknown_not_pass() {
    let root = TestRoot::new();
    root.workflow("name: [not a scalar\n");

    let report = inspect(&root);

    assert_eq!(report.results()[0].status(), CheckStatus::Unknown);
    assert_eq!(report.strict_exit(), 1);
}

#[test]
fn duplicate_keys_and_aliases_remain_unknown() {
    for source in [
        "name: CI\nname: duplicate\non: [push, pull_request]\n",
        "name: CI\nshared: &event push\non: [*event, pull_request]\n",
    ] {
        let root = TestRoot::new();
        root.workflow(source);

        let report = inspect(&root);

        assert_eq!(report.results()[0].status(), CheckStatus::Unknown);
        assert!(report.results()[0].enforcement().is_none());
    }
}

#[test]
fn oversized_workflow_remains_unknown_without_full_yaml_parse() {
    let root = TestRoot::new();
    root.workflow(&format!("#{}", "x".repeat(256 * 1024)));

    let report = inspect(&root);

    assert_eq!(report.results()[0].status(), CheckStatus::Unknown);
    assert!(report.results()[0].enforcement().is_none());
}

#[test]
fn an_unpinned_external_action_fails_the_local_contract() {
    let root = TestRoot::new();
    root.workflow(&VALID_WORKFLOW.replace(
        "actions/checkout@d23441a48e516b6c34aea4fa41551a30e30af803",
        "actions/checkout@v6",
    ));

    let report = inspect(&root);

    assert_eq!(report.results()[0].status(), CheckStatus::Fail);
}

#[test]
fn write_scoped_workflow_permissions_fail_the_local_contract() {
    let root = TestRoot::new();
    root.workflow(&VALID_WORKFLOW.replace("contents: read", "contents: write"));

    let report = inspect(&root);

    assert_eq!(report.results()[0].status(), CheckStatus::Fail);
}

#[test]
fn implicit_workflow_permissions_remain_unknown() {
    let root = TestRoot::new();
    root.workflow(&VALID_WORKFLOW.replace("permissions:\n  contents: read\n", ""));

    let report = inspect(&root);

    assert_eq!(report.results()[0].status(), CheckStatus::Unknown);
    assert!(report.results()[0].enforcement().is_none());
}

#[test]
fn unregistered_workflow_permission_remains_unknown() {
    let root = TestRoot::new();
    root.workflow(&VALID_WORKFLOW.replace("  contents: read", "  future-scope: read"));

    let report = inspect(&root);

    assert_eq!(report.results()[0].status(), CheckStatus::Unknown);
    assert!(report.results()[0].enforcement().is_none());
}

#[test]
fn local_workflow_configuration_never_proves_required_ci_for_a_sha() {
    let root = TestRoot::new();

    let (report, process) = inspect_with_remote(&root, CI_EVIDENCE_POLICY, GITHUB_REMOTE);

    assert_eq!(report.results()[0].status(), CheckStatus::Unknown);
    assert!(report.results()[0].enforcement().is_none());
    assert_eq!(report.strict_exit(), 1);
    assert!(process.invocations.lock().unwrap().is_empty());
}

#[test]
#[ignore = "explicitly reads the selected repository's local Git remotes and workflow"]
fn real_git_remote_and_checked_in_workflow_produce_localcheck_only() {
    let repository_root = std::fs::canonicalize(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("infra crate has a workspace root"),
    )
    .unwrap();
    let git_path = find_executable("git").expect("Git is needed for this explicit test");
    let git = ApprovedRepositoryTool::new(
        ApprovedExecutable::from_absolute_path(git_path.clone()).unwrap(),
        fingerprint_executable(&git_path).unwrap(),
    );
    let mut environment = BTreeMap::new();
    for key in [
        "PATH",
        "HOME",
        "USERPROFILE",
        "SYSTEMROOT",
        "WINDIR",
        "LANG",
    ] {
        if let Some(value) = std::env::var_os(key) {
            environment.insert(OsString::from(key), value);
        }
    }
    let provider = RepositoryPolicyCheckProvider::new(
        ApprovedRoot::from_absolute_path(repository_root).unwrap(),
        Some(git),
        None,
        ApprovedEnv::new(environment).unwrap(),
        Arc::new(SystemProcessPort),
        Arc::new(TestClock),
        ENVIRONMENT.to_owned(),
    );
    let policy = CI_POLICY.replace("\"quality\"", "\"required-ci\"");
    let report = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(
            PolicyService::new(Arc::new(provider), Arc::new(TestClock)).check(CheckRequest::new(
                parse_policy(policy.as_bytes()).unwrap(),
                CheckContext::default(),
            )),
        )
        .unwrap();

    assert_eq!(report.results()[0].status(), CheckStatus::Pass);
    assert!(matches!(
        report.results()[0].enforcement(),
        Some(Enforcement::LocalCheck)
    ));
}

fn find_executable(name: &str) -> Option<PathBuf> {
    let executable_name = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    };
    std::env::split_paths(&std::env::var_os("PATH")?)
        .filter(|directory| directory.is_absolute())
        .map(|directory| directory.join(&executable_name))
        .find_map(|path| {
            let metadata = std::fs::symlink_metadata(&path).ok()?;
            if !metadata.file_type().is_file() {
                return None;
            }
            std::fs::canonicalize(path).ok()
        })
}
