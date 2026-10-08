use async_trait::async_trait;
use jameskills_core::{
    AppError, PortablePath,
    application::policy::{CheckContext, CheckRequest, PolicyCheckProvider, PolicyService},
    domain::{
        ToolId,
        policy::{CheckObservation, CheckStatus, parse_policy},
    },
    ports::ClockPort,
    ports::process::{
        ApprovedEnv, ApprovedExecutable, ApprovedRoot, ExecutableFingerprint, ProcessOutput,
        ProcessPermission, ProcessPort, ProcessSpec,
    },
};
use jameskills_infra::{
    fs::{
        ApprovedRepositoryTool, GitleaksReportStatus, LocalFileSystem,
        RepositoryPolicyCheckProvider, parse_gitleaks_report,
    },
    platform::{
        HostPlatform, ToolCandidate, ToolProfile, find_tool_candidates, load_tool_profiles,
    },
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

const NOW: &str = "2026-10-04T12:00:00Z";
const ENVIRONMENT: &str = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
static NEXT_REPO_ID: AtomicU64 = AtomicU64::new(0);

struct FakeGit {
    responses: Mutex<VecDeque<ProcessOutput>>,
    args: Mutex<Vec<Vec<OsString>>>,
}

impl FakeGit {
    fn new(responses: Vec<ProcessOutput>) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
            args: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl ProcessPort for FakeGit {
    async fn run(&self, spec: ProcessSpec) -> Result<ProcessOutput, AppError> {
        assert!(spec.tool_id() == Some(ToolId::Git));
        assert!(matches!(
            spec.permission(),
            ProcessPermission::ReadOnlyCheck
        ));
        assert!(spec.approved_executable_fingerprint().is_some());
        self.args.lock().unwrap().push(spec.args().to_vec());
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .ok_or(AppError::Cancelled)
    }
}

struct FakeGitleaks {
    responses: Mutex<VecDeque<ProcessOutput>>,
    invocations: Mutex<Vec<GitleaksInvocation>>,
}

struct GitleaksInvocation {
    tool_id: ToolId,
    args: Vec<OsString>,
    fingerprint: Option<ExecutableFingerprint>,
    config_path: Option<PathBuf>,
    config_contents: Option<String>,
}

impl FakeGitleaks {
    fn new(responses: Vec<ProcessOutput>) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
            invocations: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl ProcessPort for FakeGitleaks {
    async fn run(&self, spec: ProcessSpec) -> Result<ProcessOutput, AppError> {
        let config_path = spec
            .args()
            .windows(2)
            .find(|pair| pair[0] == "--config")
            .map(|pair| PathBuf::from(&pair[1]));
        let config_contents = config_path
            .as_ref()
            .and_then(|path| std::fs::read_to_string(path).ok());
        self.invocations.lock().unwrap().push(GitleaksInvocation {
            tool_id: spec
                .tool_id()
                .expect("gitleaks check uses a registered ToolId"),
            args: spec.args().to_vec(),
            fingerprint: spec.approved_executable_fingerprint().copied(),
            config_path,
            config_contents,
        });
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .ok_or(AppError::Cancelled)
    }
}

fn gitleaks_candidate(root: &TestRoot) -> (ToolProfile, ToolCandidate) {
    let profiles = load_tool_profiles().unwrap();
    let profile = profiles
        .into_iter()
        .find(|profile| profile.tool_id() == ToolId::Gitleaks)
        .unwrap();
    std::fs::write(root.0.join("gitleaks"), b"synthetic Gitleaks candidate").unwrap();
    let search_path = std::fs::canonicalize(&root.0).unwrap();
    let candidate = find_tool_candidates(
        std::slice::from_ref(&profile),
        std::slice::from_ref(&search_path),
        HostPlatform::Linux,
    )
    .into_iter()
    .next()
    .unwrap();
    (profile, candidate)
}

fn git_ignore_output(line: &str, pattern: &str, path: &str) -> ProcessOutput {
    let mut stdout = Vec::new();
    for field in [".gitignore", line, pattern, path] {
        stdout.extend_from_slice(field.as_bytes());
        stdout.push(0);
    }
    ProcessOutput::new(Some(0), stdout, Vec::new())
}

struct TestRoot(PathBuf);

impl TestRoot {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "jameskills repo checks-{}-{}",
            std::process::id(),
            NEXT_REPO_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
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

fn required_headings() -> Vec<String> {
    ["Inicio rápido", "Arquitectura", "Contribuir"]
        .into_iter()
        .map(str::to_owned)
        .collect()
}

fn observation(fs: &LocalFileSystem, root: &ApprovedRoot, headings: &[String]) -> CheckObservation {
    fs.check_readme_sections(
        root,
        &PortablePath::new("README.md".to_owned()).unwrap(),
        headings,
        NOW,
        ENVIRONMENT,
    )
    .unwrap()
}

#[test]
fn readme_ast_accepts_required_nonempty_sections_and_returns_safe_evidence() {
    let root = TestRoot::new();
    std::fs::write(
        root.0.join("README.md"),
        include_str!("../../../tests/fixtures/repo-policy/README.md"),
    )
    .unwrap();
    let fs = LocalFileSystem;

    let result = observation(&fs, &root.approved(), &required_headings());

    assert_eq!(result.status(), CheckStatus::Pass);
    assert!(result.enforcement() == Some(jameskills_core::domain::policy::Enforcement::LocalCheck));
    assert_eq!(result.evidence().len(), 1);
    assert_eq!(result.evidence()[0].source_id(), "repo.readme");
    assert_eq!(
        result.evidence()[0].summary(),
        "Required README sections were checked."
    );
}

#[test]
fn heading_text_inside_code_is_not_a_section_and_empty_sections_fail() {
    let root = TestRoot::new();
    std::fs::write(
        root.0.join("README.md"),
        "```markdown\n# Inicio rápido\n```\n\n# Arquitectura\n\n## Contribuir\n# Siguiente\nTexto.\n",
    )
    .unwrap();
    let fs = LocalFileSystem;
    let result = observation(&fs, &root.approved(), &required_headings());

    assert_eq!(result.status(), CheckStatus::Fail);
}

#[test]
fn readme_check_blocks_non_regular_and_oversized_files() {
    let fs = LocalFileSystem;
    let headings = required_headings();

    let directory = TestRoot::new();
    std::fs::create_dir(directory.0.join("README.md")).unwrap();
    assert_eq!(
        observation(&fs, &directory.approved(), &headings).status(),
        CheckStatus::Blocked
    );

    let oversized = TestRoot::new();
    std::fs::write(oversized.0.join("README.md"), vec![b'x'; 1024 * 1024 + 1]).unwrap();
    assert_eq!(
        observation(&fs, &oversized.approved(), &headings).status(),
        CheckStatus::Blocked
    );
}

#[test]
fn gitignore_check_uses_registered_synthetic_paths_and_fixed_argv() {
    let root = TestRoot::new();
    std::fs::create_dir(root.0.join(".git")).unwrap();
    std::fs::write(
        root.0.join(".gitignore"),
        include_str!("../../../tests/fixtures/repo-policy/.gitignore"),
    )
    .unwrap();
    let patterns = [".env", "*.key", "target/"]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let fake = FakeGit::new(vec![
        git_ignore_output("1", ".env", ".env"),
        git_ignore_output("2", "*.key", "jameskills-policy-probe.key"),
        git_ignore_output("3", "target/", "target/jameskills-policy-probe.txt"),
    ]);
    let executable =
        ApprovedExecutable::from_absolute_path(std::env::current_exe().unwrap()).unwrap();
    let fingerprint = ExecutableFingerprint::from_sha256([0x44; 32]);
    let environment = ApprovedEnv::new(BTreeMap::new()).unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let observation = runtime
        .block_on(LocalFileSystem.check_gitignore_patterns(
            &root.approved(),
            &PortablePath::new(".gitignore".to_owned()).unwrap(),
            &patterns,
            Some((&executable, fingerprint)),
            &environment,
            &fake,
            NOW,
            ENVIRONMENT,
        ))
        .unwrap();

    assert_eq!(observation.status(), CheckStatus::Pass);
    assert_eq!(
        observation
            .evidence()
            .iter()
            .map(|item| item.source_id())
            .collect::<Vec<_>>(),
        [
            "repo.gitignore.env",
            "repo.gitignore.key",
            "repo.gitignore.target"
        ]
    );
    let args = fake.args.lock().unwrap();
    assert_eq!(args.len(), 3);
    assert_eq!(
        args[0],
        ["check-ignore", "--no-index", "-v", "-z", "--", ".env"].map(OsString::from)
    );
    assert_eq!(args[1][0], OsString::from("check-ignore"));
    assert_eq!(
        args[2].last(),
        Some(&OsString::from("target/jameskills-policy-probe.txt"))
    );
}

#[test]
fn gitignore_check_without_approved_git_or_with_unknown_pattern_never_spawns() {
    let root = TestRoot::new();
    std::fs::write(
        root.0.join(".gitignore"),
        include_str!("../../../tests/fixtures/repo-policy/.gitignore"),
    )
    .unwrap();
    let environment = ApprovedEnv::new(BTreeMap::new()).unwrap();
    let missing_git = FakeGit::new(Vec::new());
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let blocked = runtime
        .block_on(LocalFileSystem.check_gitignore_patterns(
            &root.approved(),
            &PortablePath::new(".gitignore".to_owned()).unwrap(),
            &[".env".to_owned()],
            None,
            &environment,
            &missing_git,
            NOW,
            ENVIRONMENT,
        ))
        .unwrap();
    assert_eq!(blocked.status(), CheckStatus::Blocked);
    assert!(missing_git.args.lock().unwrap().is_empty());

    let executable =
        ApprovedExecutable::from_absolute_path(std::env::current_exe().unwrap()).unwrap();
    let unknown = FakeGit::new(Vec::new());
    let unsupported = runtime
        .block_on(LocalFileSystem.check_gitignore_patterns(
            &root.approved(),
            &PortablePath::new(".gitignore".to_owned()).unwrap(),
            &["$(not-a-shell)".to_owned()],
            Some((&executable, ExecutableFingerprint::from_sha256([0x44; 32]))),
            &environment,
            &unknown,
            NOW,
            ENVIRONMENT,
        ))
        .unwrap();
    assert_eq!(unsupported.status(), CheckStatus::Unsupported);
    assert!(unknown.args.lock().unwrap().is_empty());
}

#[test]
fn gitleaks_scan_profile_is_redacted_bounded_and_has_distinct_exit_codes() {
    let profiles = load_tool_profiles().unwrap();
    let gitleaks = profiles
        .iter()
        .find(|profile| profile.tool_id() == ToolId::Gitleaks)
        .unwrap();
    let scan = gitleaks
        .scan_probe()
        .expect("registered Gitleaks scan probe");

    assert_eq!(
        scan.args(),
        [
            "dir",
            "--config",
            "{APP_GITLEAKS_CONFIG}",
            "--redact",
            "--no-banner",
            "--no-color",
            "--report-format",
            "json",
            "--exit-code",
            "3"
        ]
    );
    assert_eq!(scan.clean_exit_code(), 0);
    assert_eq!(scan.findings_exit_code(), 3);
    assert_eq!(scan.output_schema(), "gitleaks-json-array-v1");
    assert!(scan.output_limit_bytes() <= 64 * 1024);
    assert!(
        profiles
            .iter()
            .filter(|profile| profile.tool_id() != ToolId::Gitleaks)
            .all(|profile| profile.scan_probe().is_none())
    );
}

#[test]
fn gitleaks_json_parser_returns_only_redacted_findings_state() {
    assert_eq!(
        parse_gitleaks_report(b"[]"),
        GitleaksReportStatus::NoFindings
    );
    assert_eq!(
        parse_gitleaks_report(include_bytes!(
            "../../../tests/fixtures/repo-policy/gitleaks-findings.json"
        )),
        GitleaksReportStatus::Findings
    );
    assert_eq!(
        parse_gitleaks_report(b"{not-json}"),
        GitleaksReportStatus::Unknown
    );
    assert_eq!(
        parse_gitleaks_report(&vec![b' '; 65 * 1024]),
        GitleaksReportStatus::Unknown
    );
}

#[test]
fn gitleaks_scan_rechecks_compatible_version_and_withholds_finding_data() {
    let root = TestRoot::new();
    let (profile, candidate) = gitleaks_candidate(&root);
    let fingerprint = ExecutableFingerprint::from_sha256([0x55; 32]);
    let environment = ApprovedEnv::new(BTreeMap::new()).unwrap();
    let fake = FakeGitleaks::new(vec![
        ProcessOutput::new(Some(0), b"8.30.1\n".to_vec(), Vec::new()),
        ProcessOutput::new(
            Some(3),
            include_bytes!("../../../tests/fixtures/repo-policy/gitleaks-findings.json").to_vec(),
            Vec::new(),
        ),
    ]);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let observation = runtime
        .block_on(LocalFileSystem.check_tracked_secrets(
            &root.approved(),
            &profile,
            &candidate,
            Some(fingerprint),
            false,
            &environment,
            &fake,
            NOW,
            ENVIRONMENT,
        ))
        .unwrap();

    assert_eq!(observation.status(), CheckStatus::Fail);
    assert!(
        observation.evidence()[0]
            .summary()
            .contains("redacted details withheld")
    );
    assert!(observation.evidence()[0].summary().contains("8.30.1"));
    assert!(observation.evidence()[0].summary().contains("65536 bytes"));
    assert!(
        !observation.evidence()[0]
            .summary()
            .contains("REDACTED-FIXTURE")
    );
    let invocations = fake.invocations.lock().unwrap();
    assert_eq!(invocations.len(), 2);
    assert!(invocations.iter().all(|invocation| {
        invocation.tool_id == ToolId::Gitleaks && invocation.fingerprint == Some(fingerprint)
    }));
    assert_eq!(
        invocations[1].config_contents.as_deref(),
        Some("[extend]\nuseDefault = true\n")
    );
    assert!(
        !invocations[1]
            .config_path
            .as_ref()
            .unwrap()
            .starts_with(&root.0)
    );
    let config_path = invocations[1].config_path.as_ref().unwrap();
    assert!(
        !config_path.exists(),
        "ephemeral config is removed after scan"
    );
    let mut expected_args = profile
        .scan_probe()
        .unwrap()
        .args()
        .iter()
        .cloned()
        .map(OsString::from)
        .collect::<Vec<_>>();
    let config_index = expected_args
        .iter()
        .position(|argument| argument == "{APP_GITLEAKS_CONFIG}")
        .unwrap();
    expected_args[config_index] = config_path.as_os_str().to_os_string();
    assert_eq!(invocations[1].args, expected_args);
}

#[test]
fn gitleaks_scan_blocks_unapproved_or_incompatible_and_keeps_bad_reports_unknown() {
    let root = TestRoot::new();
    let (profile, candidate) = gitleaks_candidate(&root);
    let environment = ApprovedEnv::new(BTreeMap::new()).unwrap();
    let approved = ExecutableFingerprint::from_sha256([0x66; 32]);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();

    let no_approval = FakeGitleaks::new(Vec::new());
    let blocked = runtime
        .block_on(LocalFileSystem.check_tracked_secrets(
            &root.approved(),
            &profile,
            &candidate,
            None,
            false,
            &environment,
            &no_approval,
            NOW,
            ENVIRONMENT,
        ))
        .unwrap();
    assert_eq!(blocked.status(), CheckStatus::Blocked);
    assert!(no_approval.invocations.lock().unwrap().is_empty());

    let incompatible = FakeGitleaks::new(vec![ProcessOutput::new(
        Some(0),
        b"9.0.0\n".to_vec(),
        Vec::new(),
    )]);
    let blocked = runtime
        .block_on(LocalFileSystem.check_tracked_secrets(
            &root.approved(),
            &profile,
            &candidate,
            Some(approved),
            false,
            &environment,
            &incompatible,
            NOW,
            ENVIRONMENT,
        ))
        .unwrap();
    assert_eq!(blocked.status(), CheckStatus::Blocked);
    assert_eq!(incompatible.invocations.lock().unwrap().len(), 1);

    let unverified_minor = FakeGitleaks::new(vec![ProcessOutput::new(
        Some(0),
        b"8.29.0\n".to_vec(),
        Vec::new(),
    )]);
    let blocked = runtime
        .block_on(LocalFileSystem.check_tracked_secrets(
            &root.approved(),
            &profile,
            &candidate,
            Some(approved),
            false,
            &environment,
            &unverified_minor,
            NOW,
            ENVIRONMENT,
        ))
        .unwrap();
    assert_eq!(blocked.status(), CheckStatus::Blocked);
    assert_eq!(unverified_minor.invocations.lock().unwrap().len(), 1);

    let malformed = FakeGitleaks::new(vec![
        ProcessOutput::new(Some(0), b"8.30.1\n".to_vec(), Vec::new()),
        ProcessOutput::new(Some(0), b"not-json".to_vec(), Vec::new()),
    ]);
    let unknown = runtime
        .block_on(LocalFileSystem.check_tracked_secrets(
            &root.approved(),
            &profile,
            &candidate,
            Some(approved),
            false,
            &environment,
            &malformed,
            NOW,
            ENVIRONMENT,
        ))
        .unwrap();
    assert_eq!(unknown.status(), CheckStatus::Unknown);

    let history = FakeGitleaks::new(Vec::new());
    let unsupported = runtime
        .block_on(LocalFileSystem.check_tracked_secrets(
            &root.approved(),
            &profile,
            &candidate,
            Some(approved),
            true,
            &environment,
            &history,
            NOW,
            ENVIRONMENT,
        ))
        .unwrap();
    assert_eq!(unsupported.status(), CheckStatus::Unsupported);
    assert!(history.invocations.lock().unwrap().is_empty());
}

struct RepoCheckClock;

impl ClockPort for RepoCheckClock {
    fn now_utc(&self) -> String {
        NOW.to_owned()
    }

    fn monotonic_ms(&self) -> u64 {
        50
    }
}

#[test]
fn gitleaks_scan_blocks_repository_ignore_file_before_spawning() {
    let root = TestRoot::new();
    let (profile, candidate) = gitleaks_candidate(&root);
    std::fs::write(root.0.join(".gitleaksignore"), "synthetic-fingerprint\n").unwrap();
    let environment = ApprovedEnv::new(BTreeMap::new()).unwrap();
    let approved = ExecutableFingerprint::from_sha256([0x77; 32]);
    let fake = FakeGitleaks::new(vec![
        ProcessOutput::new(Some(0), b"8.30.1\n".to_vec(), Vec::new()),
        ProcessOutput::new(Some(0), b"[]".to_vec(), Vec::new()),
    ]);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();

    let observation = runtime
        .block_on(LocalFileSystem.check_tracked_secrets(
            &root.approved(),
            &profile,
            &candidate,
            Some(approved),
            false,
            &environment,
            &fake,
            NOW,
            ENVIRONMENT,
        ))
        .unwrap();

    assert_eq!(observation.status(), CheckStatus::Blocked);
    assert!(fake.invocations.lock().unwrap().is_empty());
}

#[test]
fn repository_policy_provider_dispatches_documents_and_secrets_without_leaking_output() {
    const POLICY: &str = r#"
schema_version = 1
profile = "repo-document-checks"

[[tool_requirements]]
tool_id = "git"
operation = "ignore-check"
version = ">=2.0.0"

[[tool_requirements]]
tool_id = "gitleaks"
operation = "scan-tracked"
version = ">=8.0.0"

[[requirements]]
id = "repo.readme"
description = "README has the declared sections."
severity = "error"
required = true
phase = "pre-install"
enforcement = "local-check"
[requirements.check]
kind = "readme-sections"
path = "README.md"
headings = ["Inicio rápido", "Arquitectura", "Contribuir"]

[[requirements]]
id = "repo.ignore"
description = "Ignore rules cover secret/build paths."
severity = "error"
required = true
phase = "ci"
enforcement = "local-check"
[requirements.check]
kind = "gitignore-patterns"
path = ".gitignore"
patterns = [".env", "*.key", "target/"]

[[requirements]]
id = "repo.secrets"
description = "No tracked secret was detected."
severity = "error"
required = true
phase = "pull-request"
enforcement = "local-check"
[requirements.check]
kind = "tracked-secrets"
include_history = false
"#;

    let root = TestRoot::new();
    std::fs::create_dir(root.0.join(".git")).unwrap();
    std::fs::write(
        root.0.join("README.md"),
        include_str!("../../../tests/fixtures/repo-policy/README.md"),
    )
    .unwrap();
    std::fs::write(
        root.0.join(".gitignore"),
        include_str!("../../../tests/fixtures/repo-policy/.gitignore"),
    )
    .unwrap();
    let gitleaks_name = if cfg!(windows) {
        "gitleaks.exe"
    } else {
        "gitleaks"
    };
    let gitleaks_path = root.0.join(gitleaks_name);
    std::fs::write(&gitleaks_path, b"synthetic Gitleaks executable").unwrap();

    let git = ApprovedRepositoryTool::new(
        ApprovedExecutable::from_absolute_path(std::env::current_exe().unwrap()).unwrap(),
        ExecutableFingerprint::from_sha256([0x33; 32]),
    );
    let gitleaks = ApprovedRepositoryTool::new(
        ApprovedExecutable::from_absolute_path(std::fs::canonicalize(gitleaks_path).unwrap())
            .unwrap(),
        ExecutableFingerprint::from_sha256([0x55; 32]),
    );
    let environment = ApprovedEnv::new(BTreeMap::new()).unwrap();
    let process = Arc::new(FakeGitleaks::new(vec![
        git_ignore_output("1", ".env", ".env"),
        git_ignore_output("2", "*.key", "jameskills-policy-probe.key"),
        git_ignore_output("3", "target/", "target/jameskills-policy-probe.txt"),
        ProcessOutput::new(Some(0), b"8.30.1\n".to_vec(), Vec::new()),
        ProcessOutput::new(Some(0), b"[]".to_vec(), Vec::new()),
    ]));
    let provider = RepositoryPolicyCheckProvider::new(
        root.approved(),
        Some(git),
        Some(gitleaks),
        environment,
        process.clone(),
        Arc::new(RepoCheckClock),
        ENVIRONMENT.to_owned(),
    );
    let service = PolicyService::new(Arc::new(provider), Arc::new(RepoCheckClock));
    let report = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(service.check(CheckRequest::new(
            parse_policy(POLICY.as_bytes()).unwrap(),
            CheckContext::default(),
        )))
        .unwrap();

    assert_eq!(report.results().len(), 3);
    assert!(
        report
            .results()
            .iter()
            .all(|result| result.status() == CheckStatus::Pass)
    );
    assert_eq!(report.strict_exit(), 0);
    assert_eq!(process.invocations.lock().unwrap().len(), 5);
}

#[test]
#[ignore = "executes the explicitly approved Gitleaks binary against a clean temporary repository fixture"]
fn real_gitleaks_repository_policy_check_passes_on_an_isolated_root() {
    let executable = PathBuf::from(
        std::env::var_os("JAMESKILLS_GITLEAKS_EXE")
            .expect("set the explicitly approved absolute Gitleaks executable path"),
    );
    let approved_sha256 = std::env::var("JAMESKILLS_GITLEAKS_SHA256")
        .expect("set the approved lowercase SHA-256 fingerprint");
    let fingerprint = fingerprint_executable(&executable).expect("fingerprint Gitleaks binary");
    let actual_sha256 = fingerprint
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(actual_sha256, approved_sha256);

    let profiles = load_tool_profiles().unwrap();
    let profile = profiles
        .iter()
        .find(|profile| profile.tool_id() == ToolId::Gitleaks)
        .unwrap();
    let canonical_executable = std::fs::canonicalize(&executable).unwrap();
    let parent = canonical_executable.parent().unwrap();
    let platform = if cfg!(windows) {
        HostPlatform::Windows
    } else {
        HostPlatform::Linux
    };
    let candidate = find_tool_candidates(
        std::slice::from_ref(profile),
        &[parent.to_path_buf()],
        platform,
    )
    .into_iter()
    .find(|candidate| {
        candidate
            .path()
            .and_then(|path| std::fs::canonicalize(path).ok())
            .is_some_and(|path| path == canonical_executable)
    })
    .expect("selected executable matches the registered native Gitleaks candidate");
    assert_eq!(
        candidate.kind(),
        jameskills_infra::platform::ToolCandidateKind::NativeExecutable
    );

    let root = TestRoot::new();
    std::fs::write(
        root.0.join("README.md"),
        include_str!("../../../tests/fixtures/repo-policy/README.md"),
    )
    .unwrap();
    std::fs::write(
        root.0.join(".gitignore"),
        include_str!("../../../tests/fixtures/repo-policy/.gitignore"),
    )
    .unwrap();
    let gitleaks = ApprovedRepositoryTool::new(
        ApprovedExecutable::from_absolute_path(canonical_executable).unwrap(),
        fingerprint,
    );
    let environment = ApprovedEnv::new(BTreeMap::new()).unwrap();
    let provider = RepositoryPolicyCheckProvider::new(
        root.approved(),
        None,
        Some(gitleaks),
        environment,
        Arc::new(SystemProcessPort),
        Arc::new(RepoCheckClock),
        ENVIRONMENT.to_owned(),
    );
    let policy = parse_policy(include_bytes!(
        "../../../examples/repository-foundation/policies/repository.toml"
    ))
    .unwrap();
    let requirement = policy
        .requirements()
        .iter()
        .find(|requirement| requirement.id() == "no-tracked-secrets")
        .unwrap();
    let observation = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(provider.observe(requirement))
        .expect("native Gitleaks provider completes on the isolated root");

    assert_eq!(observation.status(), CheckStatus::Pass);
    assert!(matches!(
        observation.enforcement(),
        Some(jameskills_core::domain::policy::Enforcement::LocalCheck)
    ));
    assert_eq!(observation.evidence().len(), 1);
    assert_eq!(observation.evidence()[0].source_id(), "tool.gitleaks.scan");
    assert_eq!(
        observation.evidence()[0].summary(),
        "Gitleaks 8.30.1 checked current tree; history excluded; JSON cap 65536 bytes."
    );
}
