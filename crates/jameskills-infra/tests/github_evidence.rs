use async_trait::async_trait;
use jameskills_core::{
    AppError, AppResult,
    application::policy::PolicyCheckProvider,
    domain::{
        ToolId,
        policy::{CheckStatus, Enforcement, RepositoryHead, parse_policy},
    },
    ports::ClockPort,
    ports::process::{
        ApprovedEnv, ApprovedExecutable, ApprovedRoot, ProcessOutput, ProcessPermission,
        ProcessPort, ProcessSpec,
    },
};
use jameskills_infra::{
    fs::{ApprovedRepositoryTool, RepositoryPolicyCheckProvider},
    github::{GithubEvidenceDriver, GithubRepository},
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
static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

const GITHUB_ACCESS_POLICY: &str = r#"
schema_version = 1
profile = "repository-foundation"
scope = "project"

[[tool_requirements]]
tool_id = "gh"
operation = "repository-read"
version = "=2.102.0"

[[requirements]]
id = "repo.github-access"
description = "The selected GitHub repository can be read."
severity = "error"
required = true
phase = "ci"
enforcement = "local-check"
depends_on = []
[requirements.check]
kind = "github-access"
"#;

struct TestClock;

impl ClockPort for TestClock {
    fn now_utc(&self) -> String {
        NOW.to_owned()
    }

    fn monotonic_ms(&self) -> u64 {
        100
    }
}

struct FakeProcess {
    outputs: Mutex<VecDeque<ProcessOutput>>,
    invocations: Mutex<Vec<Vec<OsString>>>,
}

impl FakeProcess {
    fn new(outputs: impl IntoIterator<Item = ProcessOutput>) -> Self {
        Self {
            outputs: Mutex::new(outputs.into_iter().collect()),
            invocations: Mutex::new(Vec::new()),
        }
    }

    fn invocations(&self) -> Vec<Vec<OsString>> {
        self.invocations.lock().unwrap().clone()
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
        self.invocations.lock().unwrap().push(spec.args().to_vec());
        self.outputs
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| AppError::ExternalTool {
                tool_id: "gh".into(),
                exit_code: None,
            })
    }
}

struct TestRoot {
    path: PathBuf,
    gh: ApprovedRepositoryTool,
    approved_root: ApprovedRoot,
    environment: ApprovedEnv,
    clock: TestClock,
}

impl TestRoot {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "jameskills-gh-evidence-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        let executable = path.join(if cfg!(windows) { "gh.exe" } else { "gh" });
        std::fs::write(&executable, b"fixture executable identity").unwrap();
        let git_executable = path.join(if cfg!(windows) { "git.exe" } else { "git" });
        std::fs::write(&git_executable, b"fixture git executable identity").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755)).unwrap();
            std::fs::set_permissions(&git_executable, std::fs::Permissions::from_mode(0o755))
                .unwrap();
        }
        let executable = std::fs::canonicalize(executable).unwrap();
        let gh = ApprovedRepositoryTool::new(
            ApprovedExecutable::from_absolute_path(executable.clone()).unwrap(),
            fingerprint_executable(&executable).unwrap(),
        );
        let mut entries = BTreeMap::new();
        entries.insert(OsString::from("PATH"), OsString::from("test-path"));
        Self {
            approved_root: ApprovedRoot::from_absolute_path(path.clone()).unwrap(),
            path,
            gh,
            environment: ApprovedEnv::new(entries).unwrap(),
            clock: TestClock,
        }
    }

    fn driver<'a>(&'a self, process: &'a dyn ProcessPort) -> GithubEvidenceDriver<'a> {
        GithubEvidenceDriver::new(
            &self.gh,
            &self.approved_root,
            &self.environment,
            process,
            &self.clock,
            ENVIRONMENT,
        )
    }

    fn tool(&self, name: &str) -> ApprovedRepositoryTool {
        let executable = self.path.join(if cfg!(windows) {
            format!("{name}.exe")
        } else {
            name.to_owned()
        });
        let executable = std::fs::canonicalize(executable).unwrap();
        ApprovedRepositoryTool::new(
            ApprovedExecutable::from_absolute_path(executable.clone()).unwrap(),
            fingerprint_executable(&executable).unwrap(),
        )
    }
}

#[test]
fn repository_policy_provider_observes_remote_and_head_before_github_get() {
    let process = Arc::new(FakeProcess::new([
        process_output(0, b"git version 2.55.0\n", b""),
        process_output(
            0,
            b"origin\thttps://github.com/owner/repo.git (fetch)\norigin\thttps://github.com/owner/repo.git (push)\n",
            b"",
        ),
        process_output(0, format!("{HEAD}\n").as_bytes(), b""),
        process_output(
            0,
            b"gh version 2.102.0 (2026-09-30)\n",
            b"",
        ),
        process_output(
            0,
            br#"{"hosts":{"github.com":[{"host":"github.com","active":true,"state":"success"}]}}"#,
            b"",
        ),
        process_output(
            0,
            b"HTTP/2 200 OK\r\ncontent-type: application/json\r\n\r\n{\"full_name\":\"owner/repo\",\"name\":\"repo\",\"owner\":{\"login\":\"owner\"}}",
            b"",
        ),
    ]));
    let root = TestRoot::new();
    let mut entries = BTreeMap::new();
    entries.insert(OsString::from("PATH"), OsString::from("test-path"));
    let environment = ApprovedEnv::new(entries).unwrap();
    let provider = RepositoryPolicyCheckProvider::new(
        ApprovedRoot::from_absolute_path(root.path.clone()).unwrap(),
        Some(root.tool("git")),
        None,
        environment,
        process.clone(),
        Arc::new(TestClock),
        ENVIRONMENT.to_owned(),
    )
    .with_github_cli(root.tool("gh"));
    let policy = parse_policy(GITHUB_ACCESS_POLICY.as_bytes()).unwrap();
    let result = run(provider.observe(&policy.requirements()[0]));
    assert_eq!(process.invocations().len(), 6);
    let observation = result.unwrap();

    assert_eq!(observation.status(), CheckStatus::Pass);
    assert!(observation.evidence()[0].summary().contains(HEAD));
    let calls = process.invocations();
    let args = calls
        .iter()
        .map(|call| {
            call.iter()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    assert_eq!(args[0], ["--version"]);
    assert_eq!(args[1], ["remote", "-v"]);
    assert_eq!(args[2], ["rev-parse", "HEAD"]);
    assert_eq!(args[3], ["--version"]);
    assert_eq!(args[4][0], "auth");
    assert_eq!(args[5][0], "api");
    assert_eq!(args.len(), 6);
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn process_output(exit_code: i32, stdout: &[u8], stderr: &[u8]) -> ProcessOutput {
    ProcessOutput::new(Some(exit_code), stdout.to_vec(), stderr.to_vec())
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
    .expect("native tool must be installed for opt-in GitHub evidence test");
    let path = std::fs::canonicalize(candidate.path().unwrap()).unwrap();
    ApprovedRepositoryTool::new(
        ApprovedExecutable::from_absolute_path(path.clone()).unwrap(),
        fingerprint_executable(&path).unwrap(),
    )
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
            Poll::Ready(output) => return output,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

#[test]
fn mixed_or_unsupported_remotes_do_not_trigger_github_auth_or_api_requests() {
    let process = Arc::new(FakeProcess::new([
        process_output(0, b"git version 2.55.0\n", b""),
        process_output(
            0,
            b"origin\thttps://github.com/owner/repo.git (fetch)\nupstream\thttps://gitlab.com/owner/repo.git (fetch)\n",
            b"",
        ),
    ]));
    let root = TestRoot::new();
    let mut entries = BTreeMap::new();
    entries.insert(OsString::from("PATH"), OsString::from("test-path"));
    let environment = ApprovedEnv::new(entries).unwrap();
    let provider = RepositoryPolicyCheckProvider::new(
        ApprovedRoot::from_absolute_path(root.path.clone()).unwrap(),
        Some(root.tool("git")),
        None,
        environment,
        process.clone(),
        Arc::new(TestClock),
        ENVIRONMENT.to_owned(),
    )
    .with_github_cli(root.tool("gh"));
    let policy = parse_policy(GITHUB_ACCESS_POLICY.as_bytes()).unwrap();
    let observation = run(provider.observe(&policy.requirements()[0])).unwrap();

    assert_eq!(observation.status(), CheckStatus::Unknown);
    let calls = process.invocations();
    assert_eq!(calls.len(), 2);
    assert!(calls.iter().all(|args| {
        args.first()
            .is_some_and(|arg| arg.to_string_lossy() != "api" && arg.to_string_lossy() != "auth")
    }));
}

#[test]
fn repository_check_uses_fixed_github_get_and_binds_evidence_without_auth_data() {
    let process = Arc::new(FakeProcess::new([
        process_output(0, b"gh version 2.102.0 (2026-09-30)\n", b""),
        process_output(
            0,
            br#"{"hosts":{"github.com":[{"host":"github.com","active":true,"state":"success","login":"private-user","token":"must-not-leak"}]}}"#,
            b"",
        ),
        process_output(
            0,
            b"HTTP/2 200 OK\r\ncontent-type: application/json\r\n\r\n{\"full_name\":\"owner/repo\",\"name\":\"repo\",\"owner\":{\"login\":\"owner\"}}",
            b"",
        ),
    ]));
    let root = TestRoot::new();
    let driver = root.driver(process.as_ref());
    let repository =
        GithubRepository::from_remote_url("https://github.com/owner/repo.git").unwrap();
    let revision = RepositoryHead::parse(HEAD).unwrap();
    let observation = run(driver.identify_repository(&repository, &revision)).unwrap();

    assert_eq!(observation.status(), CheckStatus::Pass);
    assert!(matches!(
        observation.enforcement(),
        Some(Enforcement::LocalCheck)
    ));
    let evidence = &observation.evidence()[0];
    assert!(evidence.summary().contains("owner/repo"));
    assert!(evidence.summary().contains(HEAD));
    assert!(evidence.summary().contains("cap=repo-read"));
    assert_eq!(evidence.source_id(), "github.repository.identity");
    assert!(!evidence.summary().contains("private-user"));
    assert!(!evidence.summary().contains("must-not-leak"));

    let calls = process.invocations();
    assert_eq!(calls.len(), 3);
    let version = calls[0]
        .iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert_eq!(version, ["--version"]);
    let auth = calls[1]
        .iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert_eq!(
        auth,
        [
            "auth",
            "status",
            "--active",
            "--hostname",
            "github.com",
            "--json",
            "hosts"
        ]
    );
    let api = calls[2]
        .iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert_eq!(
        api,
        [
            "api",
            "--hostname",
            "github.com",
            "--method",
            "GET",
            "--include",
            "repos/owner/repo"
        ]
    );
}

#[test]
fn unauthenticated_status_stops_before_any_repository_request() {
    let process = Arc::new(FakeProcess::new([process_output(
        0,
        b"gh version 2.102.0 (2026-09-30)\n",
        b"",
    ), process_output(
        0,
        br#"{"hosts":{"github.com":[{"host":"github.com","active":true,"state":"error","login":"private-user","token":"must-not-leak"}]}}"#,
        b"secret diagnostic must not surface",
    )]));
    let root = TestRoot::new();
    let driver = root.driver(process.as_ref());
    let repository = GithubRepository::from_remote_url("git@github.com:owner/repo.git").unwrap();
    let revision = RepositoryHead::parse(HEAD).unwrap();
    let observation = run(driver.identify_repository(&repository, &revision)).unwrap();

    assert_eq!(observation.status(), CheckStatus::Blocked);
    assert!(observation.enforcement().is_none());
    assert_eq!(process.invocations().len(), 2);
    assert!(!observation.evidence()[0].summary().contains("private-user"));
    assert!(
        !observation.evidence()[0]
            .summary()
            .contains("must-not-leak")
    );
}

#[test]
fn unverified_gh_release_stays_unknown_without_auth_or_repository_requests() {
    let process = Arc::new(FakeProcess::new([process_output(
        0,
        b"gh version 2.103.0 (unreviewed)\n",
        b"",
    )]));
    let root = TestRoot::new();
    let driver = root.driver(process.as_ref());
    let repository =
        GithubRepository::from_remote_url("https://github.com/owner/repo.git").unwrap();
    let revision = RepositoryHead::parse(HEAD).unwrap();
    let observation = run(driver.identify_repository(&repository, &revision)).unwrap();

    assert_eq!(observation.status(), CheckStatus::Unknown);
    let calls = process.invocations();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0], [OsString::from("--version")]);
}

#[test]
#[ignore = "explicitly performs gh auth status and one read-only GET on the selected repository"]
fn real_github_access_verifies_the_checked_out_repository_without_exporting_auth_data() {
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
    let environment = ApprovedEnv::new(environment).unwrap();
    let process: Arc<dyn ProcessPort> = Arc::new(SystemProcessPort);
    let provider = RepositoryPolicyCheckProvider::new(
        ApprovedRoot::from_absolute_path(repository_root).unwrap(),
        Some(approved_host_tool(ToolId::Git)),
        None,
        environment,
        process,
        Arc::new(TestClock),
        ENVIRONMENT.to_owned(),
    )
    .with_github_cli(approved_host_tool(ToolId::Gh));
    let policy = parse_policy(GITHUB_ACCESS_POLICY.as_bytes()).unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let observation = runtime
        .block_on(provider.observe(&policy.requirements()[0]))
        .unwrap();

    assert_eq!(observation.status(), CheckStatus::Pass);
    assert!(
        observation.evidence()[0]
            .summary()
            .contains("cap=repo-read")
    );
    assert_eq!(
        observation.evidence()[0].source_id(),
        "github.repository.identity"
    );
}
