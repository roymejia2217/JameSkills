use async_trait::async_trait;
use jameskills_core::{
    AppError, AppResult,
    application::policy::PolicyCheckProvider,
    domain::{
        ToolId,
        policy::{CheckStatus, parse_policy},
    },
    ports::ClockPort,
    ports::process::{
        ApprovedEnv, ApprovedExecutable, ApprovedRoot, ProcessOutput, ProcessPermission,
        ProcessPort, ProcessSpec,
    },
};
use jameskills_infra::{
    fs::{ApprovedRepositoryTool, RepositoryPolicyCheckProvider},
    platform::{PlatformFacts, ToolCandidateKind, find_tool_candidates, load_tool_profiles},
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
const CI_EVIDENCE_POLICY: &str = r#"
schema_version = 1
profile = "repository-foundation"
scope = "project"

[[tool_requirements]]
tool_id = "gh"
operation = "check-runs"
version = "=2.102.0"

[[requirements]]
id = "repo.ci-evidence"
description = "Required checks passed for this exact commit."
severity = "error"
required = true
phase = "pre-release"
enforcement = "required-ci"
depends_on = []
[requirements.check]
kind = "ci-evidence"
required_checks = ["quality"]
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

fn api_json(body: String) -> ProcessOutput {
    output(
        0,
        format!("HTTP/2 200 OK\r\ncontent-type: application/json\r\n\r\n{body}").as_bytes(),
    )
}

fn ci_evidence_outputs(
    returned_sha: &str,
    host_app_id: Option<i64>,
    run_app_id: Option<i64>,
    legacy_status: Option<&str>,
) -> Vec<ProcessOutput> {
    ci_evidence_outputs_with_run_state(
        returned_sha,
        host_app_id,
        run_app_id,
        legacy_status,
        "completed",
        "success",
    )
}

fn ci_evidence_outputs_with_run_state(
    returned_sha: &str,
    host_app_id: Option<i64>,
    run_app_id: Option<i64>,
    legacy_status: Option<&str>,
    run_status: &str,
    run_conclusion: &str,
) -> Vec<ProcessOutput> {
    let host_app = host_app_id.map_or_else(|| "null".to_owned(), |id| id.to_string());
    let run_app = run_app_id.map_or_else(|| "null".to_owned(), |id| format!("{{\"id\":{id}}}"));
    let effective = format!(
        "[{{\"type\":\"required_status_checks\",\"parameters\":{{\"required_status_checks\":[{{\"context\":\"quality\",\"integration_id\":{host_app}}}]}}}}]"
    );
    let classic = format!(
        "{{\"required_status_checks\":{{\"contexts\":[],\"checks\":[{{\"context\":\"quality\",\"app_id\":{host_app}}}]}},\"required_pull_request_reviews\":null,\"enforce_admins\":{{\"enabled\":true}}}}"
    );
    let check_runs = if run_app_id.is_some() {
        format!(
            "{{\"total_count\":1,\"check_runs\":[{{\"head_sha\":\"{returned_sha}\",\"name\":\"quality\",\"status\":\"{run_status}\",\"conclusion\":\"{run_conclusion}\",\"started_at\":\"2026-10-05T12:00:00Z\",\"app\":{run_app}}}]}}"
        )
    } else {
        "{\"total_count\":0,\"check_runs\":[]}".to_owned()
    };
    let mut outputs = vec![
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
        api_json(effective.clone()),
        api_json(classic.clone()),
        api_json(
            "[{\"target\":\"branch\",\"enforcement\":\"active\",\"conditions\":{\"ref_name\":{\"include\":[\"refs/heads/main\"],\"exclude\":[]}},\"bypass_actors\":[],\"current_user_can_bypass\":\"never\"}]".to_owned(),
        ),
        api_json(effective),
        api_json(classic),
        api_json(check_runs),
    ];
    if let Some(status) = legacy_status {
        outputs.push(api_json(format!(
            "{{\"sha\":\"{HEAD}\",\"state\":\"{status}\",\"total_count\":1,\"statuses\":[{{\"context\":\"quality\",\"state\":\"{status}\"}}]}}"
        )));
    }
    outputs
}

fn observe_ci(
    outputs: impl IntoIterator<Item = ProcessOutput>,
) -> (
    jameskills_core::domain::policy::CheckObservation,
    Vec<Vec<OsString>>,
) {
    let process = Arc::new(FakeProcess::new(outputs));
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
    let policy = parse_policy(CI_EVIDENCE_POLICY.as_bytes()).unwrap();
    let result = run(provider.observe(&policy.requirements()[0])).unwrap();
    (result, process.args())
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

fn approved_host_tool(tool_id: ToolId) -> ApprovedRepositoryTool {
    let profiles = load_tool_profiles().unwrap();
    let profile = profiles
        .iter()
        .find(|profile| profile.tool_id() == tool_id)
        .unwrap();
    let search_paths = std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .unwrap_or_default();
    let candidate = find_tool_candidates(
        std::slice::from_ref(profile),
        &search_paths,
        PlatformFacts::detect().platform,
    )
    .into_iter()
    .find(|candidate| candidate.kind() == ToolCandidateKind::NativeExecutable)
    .expect("native tool must be installed for opt-in GitHub host test");
    let path = std::fs::canonicalize(candidate.path().unwrap()).unwrap();
    ApprovedRepositoryTool::new(
        ApprovedExecutable::from_absolute_path(path.clone()).unwrap(),
        fingerprint_executable(&path).unwrap(),
    )
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
fn ci_evidence_requires_active_host_rule_and_successful_check_for_exact_head_sha() {
    let (observation, calls) = observe_ci(ci_evidence_outputs(HEAD, Some(42), Some(42), None));

    assert_eq!(observation.status(), CheckStatus::Pass);
    assert!(matches!(
        observation.enforcement(),
        Some(jameskills_core::domain::policy::Enforcement::RequiredCi)
    ));
    assert!(observation.evidence()[0].summary().contains(HEAD));
    assert_eq!(calls.len(), 11);
    assert!(calls[10].iter().any(|argument| {
        argument
            .to_string_lossy()
            .contains(&format!("commits/{HEAD}/check-runs"))
    }));
}

#[test]
#[ignore = "read-only request for selected host rules and current local SHA; requires an authenticated gh profile"]
fn real_ci_evidence_binds_its_result_to_the_selected_local_sha() {
    let repository_root = std::fs::canonicalize(std::env::current_dir().unwrap()).unwrap();
    let mut environment = BTreeMap::new();
    for key in [
        "PATH",
        "HOME",
        "USERPROFILE",
        "APPDATA",
        "SYSTEMROOT",
        "TEMP",
        "TMP",
    ] {
        if let Some(value) = std::env::var_os(key) {
            environment.insert(OsString::from(key), value);
        }
    }
    let process: Arc<dyn ProcessPort> = Arc::new(SystemProcessPort);
    let provider = RepositoryPolicyCheckProvider::new(
        ApprovedRoot::from_absolute_path(repository_root).unwrap(),
        Some(approved_host_tool(ToolId::Git)),
        None,
        ApprovedEnv::new(environment).unwrap(),
        process,
        Arc::new(TestClock),
        ENVIRONMENT.to_owned(),
    )
    .with_github_cli(approved_host_tool(ToolId::Gh));
    let policy = parse_policy(CI_EVIDENCE_POLICY.as_bytes()).unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let observation = runtime
        .block_on(provider.observe(&policy.requirements()[0]))
        .unwrap();

    let evidence = observation
        .evidence()
        .first()
        .expect("selected GitHub repo should return sanitized evidence");
    assert_eq!(evidence.source_id(), "github.ci-evidence");
    let sha = evidence
        .summary()
        .split(";sha=")
        .nth(1)
        .and_then(|tail| tail.split(';').next())
        .unwrap_or_default();
    assert!(matches!(sha.len(), 40 | 64));
    assert!(
        sha.bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    );
    if observation.status() == CheckStatus::Pass {
        assert!(matches!(
            observation.enforcement(),
            Some(jameskills_core::domain::policy::Enforcement::RequiredCi)
        ));
    }
}

#[test]
fn stale_check_run_sha_fails_even_when_the_current_branch_rule_is_valid() {
    let other_sha = "b1b2c3d4e5f60123456789abcdef0123456789ab";
    let (observation, _) = observe_ci(ci_evidence_outputs(other_sha, Some(42), Some(42), None));
    assert_eq!(observation.status(), CheckStatus::Fail);
}

#[test]
fn a_check_run_from_a_different_required_app_does_not_satisfy_the_context() {
    let (observation, _) = observe_ci(ci_evidence_outputs(HEAD, Some(42), Some(99), None));
    assert_eq!(observation.status(), CheckStatus::Fail);
}

#[test]
fn pending_exact_sha_check_is_blocked_not_pass() {
    let (observation, _) = observe_ci(ci_evidence_outputs_with_run_state(
        HEAD,
        Some(42),
        Some(42),
        None,
        "in_progress",
        "",
    ));
    assert_eq!(observation.status(), CheckStatus::Blocked);
    assert!(matches!(
        observation.enforcement(),
        Some(jameskills_core::domain::policy::Enforcement::RequiredCi)
    ));
}

#[test]
fn insufficient_rules_api_permission_blocks_host_rule_claim() {
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
            1,
            b"HTTP/2 403 Forbidden\r\ncontent-type: application/json\r\n\r\n{\"message\":\"permission denied\"}",
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
    let observation = run(provider.observe(&policy.requirements()[0])).unwrap();

    assert_eq!(observation.status(), CheckStatus::Blocked);
    assert!(observation.enforcement().is_none());
    assert_eq!(process.args().len(), 6);
}

#[test]
fn successful_sha_check_does_not_become_required_ci_without_a_host_rule() {
    let mut outputs = ci_evidence_outputs(HEAD, Some(42), Some(42), None);
    outputs[5] = api_json("[]".to_owned());
    outputs[6] = api_json(
        "{\"required_status_checks\":null,\"required_pull_request_reviews\":null,\"enforce_admins\":{\"enabled\":false}}".to_owned(),
    );
    let (observation, calls) = observe_ci(outputs);

    assert_eq!(observation.status(), CheckStatus::Fail);
    assert!(observation.enforcement().is_none());
    assert_eq!(
        calls.len(),
        8,
        "check-run endpoint must not be queried without a host rule"
    );
}

#[test]
fn legacy_commit_status_can_satisfy_an_unbound_required_context_for_the_exact_sha() {
    let (observation, calls) = observe_ci(ci_evidence_outputs(HEAD, None, None, Some("success")));
    assert_eq!(observation.status(), CheckStatus::Pass);
    assert!(matches!(
        observation.enforcement(),
        Some(jameskills_core::domain::policy::Enforcement::RequiredCi)
    ));
    assert!(calls[11].iter().any(|argument| {
        argument
            .to_string_lossy()
            .contains(&format!("commits/{HEAD}/status"))
    }));
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
