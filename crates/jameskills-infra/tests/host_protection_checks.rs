use async_trait::async_trait;
use jameskills_core::{
    AppError, AppResult,
    application::policy::PolicyCheckProvider,
    domain::policy::{CheckStatus, parse_policy},
    ports::ClockPort,
    ports::process::{
        ApprovedEnv, ApprovedExecutable, ApprovedRoot, ProcessOutput, ProcessPermission,
        ProcessPort, ProcessSpec,
    },
};
use jameskills_infra::{
    fs::{ApprovedRepositoryTool, RepositoryPolicyCheckProvider},
    process::fingerprint_executable,
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

const NOW: &str = "2026-10-05T12:00:00Z";
const ENVIRONMENT: &str = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const HEAD: &str = "a1b2c3d4e5f60123456789abcdef0123456789ab";
const POLICY: &str = r#"
schema_version = 1
profile = "repository-foundation"
scope = "project"

[[tool_requirements]]
tool_id = "gh"
operation = "branch-rules"
version = "=2.102.0"

[[requirements]]
id = "repo.main-protection"
description = "Main requires a PR and the current required checks."
severity = "error"
required = true
phase = "pre-release"
enforcement = "host-rule"
depends_on = []
[requirements.check]
kind = "github-branch-policy"
branch = "main"
require_pull_request = true
required_checks = ["quality"]
require_no_bypass = false
"#;

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

struct TestClock;

impl ClockPort for TestClock {
    fn now_utc(&self) -> String {
        NOW.to_owned()
    }

    fn monotonic_ms(&self) -> u64 {
        10
    }
}

struct FakeProcess {
    output: Mutex<VecDeque<ProcessOutput>>,
    args: Mutex<Vec<Vec<OsString>>>,
}

impl FakeProcess {
    fn new(outputs: impl IntoIterator<Item = ProcessOutput>) -> Self {
        Self {
            output: Mutex::new(outputs.into_iter().collect()),
            args: Mutex::new(Vec::new()),
        }
    }

    fn args(&self) -> Vec<Vec<OsString>> {
        self.args.lock().unwrap().clone()
    }
}

#[async_trait]
impl ProcessPort for FakeProcess {
    async fn run(&self, spec: ProcessSpec) -> AppResult<ProcessOutput> {
        if spec.permission() != ProcessPermission::ReadOnlyCheck {
            return Err(AppError::PermissionDenied {
                operation: "test.expected-read-only".into(),
            });
        }
        self.args.lock().unwrap().push(spec.args().to_vec());
        self.output
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| AppError::ExternalTool {
                tool_id: "github-test".into(),
                exit_code: None,
            })
    }
}

struct TestRoot(PathBuf);

impl TestRoot {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "jameskills-host-rules-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        for tool in ["git", "gh"] {
            let path = path.join(if cfg!(windows) {
                format!("{tool}.exe")
            } else {
                tool.to_owned()
            });
            std::fs::write(&path, format!("fixture {tool} executable")).unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
            }
        }
        Self(path)
    }

    fn tool(&self, tool: &str) -> ApprovedRepositoryTool {
        let path = self.0.join(if cfg!(windows) {
            format!("{tool}.exe")
        } else {
            tool.to_owned()
        });
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

fn output(code: i32, stdout: &[u8]) -> ProcessOutput {
    ProcessOutput::new(Some(code), stdout.to_vec(), Vec::new())
}

fn run<F: std::future::Future>(future: F) -> F::Output {
    use std::task::{Context, Poll, Wake, Waker};
    struct Noop;
    impl Wake for Noop {
        fn wake(self: Arc<Self>) {}
    }
    let waker = Waker::from(Arc::new(Noop));
    let mut context = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(result) => return result,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

#[test]
fn explicit_empty_effective_rules_and_classic_protection_fail_main_policy() {
    let process = Arc::new(FakeProcess::new([
        output(0, b"git version 2.55.0\n"),
        output(
            0,
            b"origin\thttps://github.com/owner/repo.git (fetch)\norigin\thttps://github.com/owner/repo.git (push)\n",
        ),
        output(0, format!("{HEAD}\n").as_bytes()),
        output(0, b"gh version 2.102.0 (2026-09-30)\n"),
        output(
            0,
            br#"{"hosts":{"github.com":[{"host":"github.com","active":true,"state":"success"}]}}"#,
        ),
        output(
            0,
            b"HTTP/2 200 OK\r\ncontent-type: application/json\r\n\r\n[]",
        ),
        output(
            0,
            b"HTTP/2 200 OK\r\ncontent-type: application/json\r\n\r\n{\"required_status_checks\":null,\"required_pull_request_reviews\":null,\"enforce_admins\":{\"enabled\":false}}",
        ),
        output(
            0,
            b"HTTP/2 200 OK\r\ncontent-type: application/json\r\n\r\n[]",
        ),
    ]));
    let root = TestRoot::new();
    let mut env = BTreeMap::new();
    env.insert(OsString::from("PATH"), OsString::from("test-path"));
    let provider = RepositoryPolicyCheckProvider::new(
        ApprovedRoot::from_absolute_path(root.0.clone()).unwrap(),
        Some(root.tool("git")),
        None,
        ApprovedEnv::new(env).unwrap(),
        process.clone(),
        Arc::new(TestClock),
        ENVIRONMENT.to_owned(),
    )
    .with_github_cli(root.tool("gh"));
    let policy = parse_policy(POLICY.as_bytes()).unwrap();
    let result = run(provider.observe(&policy.requirements()[0]));

    let observation = result.unwrap();
    assert_eq!(observation.status(), CheckStatus::Fail);
    assert!(
        observation.evidence()[0]
            .summary()
            .contains("bypass=present")
    );
    assert_eq!(process.args().len(), 8);
}

#[test]
fn active_effective_pr_and_required_check_rule_observe_host_rule() {
    let process = Arc::new(FakeProcess::new([
        output(0, b"git version 2.55.0\n"),
        output(
            0,
            b"origin\thttps://github.com/owner/repo.git (fetch)\norigin\thttps://github.com/owner/repo.git (push)\n",
        ),
        output(0, format!("{HEAD}\n").as_bytes()),
        output(0, b"gh version 2.102.0 (2026-09-30)\n"),
        output(
            0,
            br#"{"hosts":{"github.com":[{"host":"github.com","active":true,"state":"success"}]}}"#,
        ),
        output(
            0,
            b"HTTP/2 200 OK\r\ncontent-type: application/json\r\n\r\n[{\"type\":\"pull_request\",\"parameters\":{\"required_approving_review_count\":1}},{\"type\":\"required_status_checks\",\"parameters\":{\"required_status_checks\":[{\"context\":\"quality\",\"integration_id\":42}]}}]",
        ),
        output(
            0,
            b"HTTP/2 200 OK\r\ncontent-type: application/json\r\n\r\n{\"required_status_checks\":{\"contexts\":[\"quality\"],\"checks\":[{\"context\":\"quality\",\"app_id\":42}]},\"required_pull_request_reviews\":{\"required_approving_review_count\":1,\"bypass_pull_request_allowances\":{\"users\":[],\"teams\":[],\"apps\":[]}},\"enforce_admins\":{\"enabled\":true}}",
        ),
        output(
            0,
            b"HTTP/2 200 OK\r\ncontent-type: application/json\r\n\r\n[]",
        ),
    ]));
    let root = TestRoot::new();
    let mut env = BTreeMap::new();
    env.insert(OsString::from("PATH"), OsString::from("test-path"));
    let provider = RepositoryPolicyCheckProvider::new(
        ApprovedRoot::from_absolute_path(root.0.clone()).unwrap(),
        Some(root.tool("git")),
        None,
        ApprovedEnv::new(env).unwrap(),
        process,
        Arc::new(TestClock),
        ENVIRONMENT.to_owned(),
    )
    .with_github_cli(root.tool("gh"));
    let policy_source = POLICY.replace("require_no_bypass = false", "require_no_bypass = true");
    let policy = parse_policy(policy_source.as_bytes()).unwrap();
    let observation = run(provider.observe(&policy.requirements()[0])).unwrap();

    assert_eq!(observation.status(), CheckStatus::Pass);
    assert!(matches!(
        observation.enforcement(),
        Some(jameskills_core::domain::policy::Enforcement::HostRule)
    ));
    assert!(observation.evidence()[0].summary().contains(HEAD));
    assert!(
        observation.evidence()[0]
            .summary()
            .contains("bypass=none-visible")
    );
}

#[test]
fn visible_ruleset_bypass_actor_fails_no_bypass_without_leaking_actor_details() {
    let process = Arc::new(FakeProcess::new([
        output(0, b"git version 2.55.0\n"),
        output(
            0,
            b"origin\thttps://github.com/owner/repo.git (fetch)\norigin\thttps://github.com/owner/repo.git (push)\n",
        ),
        output(0, format!("{HEAD}\n").as_bytes()),
        output(0, b"gh version 2.102.0 (2026-09-30)\n"),
        output(
            0,
            br#"{"hosts":{"github.com":[{"host":"github.com","active":true,"state":"success"}]}}"#,
        ),
        output(
            0,
            b"HTTP/2 200 OK\r\ncontent-type: application/json\r\n\r\n[{\"type\":\"pull_request\"},{\"type\":\"required_status_checks\",\"parameters\":{\"required_status_checks\":[{\"context\":\"quality\"}]}}]",
        ),
        output(
            0,
            b"HTTP/2 200 OK\r\ncontent-type: application/json\r\n\r\n{\"required_status_checks\":{\"contexts\":[\"quality\"],\"checks\":[]},\"required_pull_request_reviews\":{\"bypass_pull_request_allowances\":{\"users\":[],\"teams\":[],\"apps\":[]}},\"enforce_admins\":{\"enabled\":true}}",
        ),
        output(
            0,
            b"HTTP/2 200 OK\r\ncontent-type: application/json\r\n\r\n[{\"target\":\"branch\",\"enforcement\":\"active\",\"conditions\":{\"ref_name\":{\"include\":[\"~ALL\"],\"exclude\":[]}},\"bypass_actors\":[{\"actor_type\":\"OrganizationAdmin\",\"bypass_mode\":\"always\"}],\"current_user_can_bypass\":\"always\"}]",
        ),
    ]));
    let root = TestRoot::new();
    let mut env = BTreeMap::new();
    env.insert(OsString::from("PATH"), OsString::from("test-path"));
    let provider = RepositoryPolicyCheckProvider::new(
        ApprovedRoot::from_absolute_path(root.0.clone()).unwrap(),
        Some(root.tool("git")),
        None,
        ApprovedEnv::new(env).unwrap(),
        process,
        Arc::new(TestClock),
        ENVIRONMENT.to_owned(),
    )
    .with_github_cli(root.tool("gh"));
    let policy_source = POLICY.replace("require_no_bypass = false", "require_no_bypass = true");
    let policy = parse_policy(policy_source.as_bytes()).unwrap();
    let observation = run(provider.observe(&policy.requirements()[0])).unwrap();

    assert_eq!(observation.status(), CheckStatus::Fail);
    assert!(
        observation.evidence()[0]
            .summary()
            .contains("bypass=present")
    );
    assert!(
        !observation.evidence()[0]
            .summary()
            .contains("OrganizationAdmin")
    );
}

#[test]
fn active_ruleset_can_prove_positive_branch_policy_when_classic_endpoint_is_absent() {
    let process = Arc::new(FakeProcess::new([
        output(0, b"git version 2.55.0\n"),
        output(
            0,
            b"origin\thttps://github.com/owner/repo.git (fetch)\norigin\thttps://github.com/owner/repo.git (push)\n",
        ),
        output(0, format!("{HEAD}\n").as_bytes()),
        output(0, b"gh version 2.102.0 (2026-09-30)\n"),
        output(
            0,
            br#"{"hosts":{"github.com":[{"host":"github.com","active":true,"state":"success"}]}}"#,
        ),
        output(
            0,
            b"HTTP/2 200 OK\r\ncontent-type: application/json\r\n\r\n[{\"type\":\"pull_request\"},{\"type\":\"required_status_checks\",\"parameters\":{\"required_status_checks\":[{\"context\":\"quality\"}]}}]",
        ),
        output(
            0,
            b"HTTP/2 404 Not Found\r\ncontent-type: application/json\r\n\r\n{}",
        ),
        output(
            0,
            b"HTTP/2 200 OK\r\ncontent-type: application/json\r\n\r\n[{\"target\":\"branch\",\"enforcement\":\"active\",\"conditions\":{\"ref_name\":{\"include\":[\"refs/heads/main\"],\"exclude\":[]}},\"bypass_actors\":[],\"current_user_can_bypass\":\"never\"}]",
        ),
    ]));
    let root = TestRoot::new();
    let mut env = BTreeMap::new();
    env.insert(OsString::from("PATH"), OsString::from("test-path"));
    let provider = RepositoryPolicyCheckProvider::new(
        ApprovedRoot::from_absolute_path(root.0.clone()).unwrap(),
        Some(root.tool("git")),
        None,
        ApprovedEnv::new(env).unwrap(),
        process,
        Arc::new(TestClock),
        ENVIRONMENT.to_owned(),
    )
    .with_github_cli(root.tool("gh"));
    let policy = parse_policy(POLICY.as_bytes()).unwrap();
    let observation = run(provider.observe(&policy.requirements()[0])).unwrap();

    assert_eq!(observation.status(), CheckStatus::Pass);
    assert!(matches!(
        observation.enforcement(),
        Some(jameskills_core::domain::policy::Enforcement::HostRule)
    ));
    assert!(
        observation.evidence()[0]
            .summary()
            .contains("bypass=unknown")
    );
}
