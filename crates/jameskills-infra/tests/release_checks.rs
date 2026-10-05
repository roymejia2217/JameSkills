use async_trait::async_trait;
use jameskills_core::{
    AppError, AppResult,
    application::policy::PolicyCheckProvider,
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
    platform::{PlatformFacts, ToolCandidateKind, find_tool_candidates, load_tool_profiles},
    process::{SystemProcessPort, fingerprint_executable},
};
use std::{
    collections::{BTreeMap, VecDeque},
    ffi::OsString,
    future::Future,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    task::{Context, Poll, Wake, Waker},
};

const HEAD: &str = "a1b2c3d4e5f60123456789abcdef0123456789ab";
const ENVIRONMENT: &str = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const NOW: &str = "2026-10-05T12:00:00Z";
static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

const RELEASE_POLICY: &str = r#"
schema_version = 1
profile = "repository-foundation"
scope = "project"

[[requirements]]
id = "release-ready"
description = "The current project version has a published release."
severity = "error"
required = true
phase = "pre-release"
enforcement = "local-check"
depends_on = []
guidance_id = "release-setup"
[requirements.check]
kind = "release-contract"
require_changelog = true
require_checksums = true
require_signature = false
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
    arguments: Mutex<Vec<Vec<OsString>>>,
}

impl FakeProcess {
    fn new(outputs: impl IntoIterator<Item = ProcessOutput>) -> Self {
        Self {
            outputs: Mutex::new(outputs.into_iter().collect()),
            arguments: Mutex::new(Vec::new()),
        }
    }

    fn arguments(&self) -> Vec<Vec<OsString>> {
        self.arguments.lock().unwrap().clone()
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
        self.arguments.lock().unwrap().push(spec.args().to_vec());
        self.outputs
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| AppError::ExternalTool {
                tool_id: "test-process".into(),
                exit_code: None,
            })
    }
}

struct TestRoot(PathBuf);

impl TestRoot {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "jameskills-release-checks-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(
            path.join("Cargo.toml"),
            "[workspace]\nmembers = []\n[workspace.package]\nversion = \"1.2.3\"\n",
        )
        .unwrap();
        std::fs::write(
            path.join("CHANGELOG.md"),
            "# Changelog\n\n## [1.2.3]\n\n- Release notes.\n",
        )
        .unwrap();
        Self(path)
    }

    fn write_node_version(&self, version: &str) {
        std::fs::write(
            self.0.join("package.json"),
            format!("{{\"name\":\"fixture\",\"version\":\"{version}\"}}"),
        )
        .unwrap();
    }

    fn write_cargo_manifest(&self, source: &str) {
        std::fs::write(self.0.join("Cargo.toml"), source).unwrap();
    }

    fn write_changelog(&self, source: &str) {
        std::fs::write(self.0.join("CHANGELOG.md"), source).unwrap();
    }

    fn tool(&self, name: &str) -> ApprovedRepositoryTool {
        let executable = self.0.join(if cfg!(windows) {
            format!("{name}.exe")
        } else {
            name.to_owned()
        });
        std::fs::write(&executable, b"fixture executable identity").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let executable = std::fs::canonicalize(executable).unwrap();
        ApprovedRepositoryTool::new(
            ApprovedExecutable::from_absolute_path(executable.clone()).unwrap(),
            fingerprint_executable(&executable).unwrap(),
        )
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn output(code: i32, body: &[u8]) -> ProcessOutput {
    ProcessOutput::new(Some(code), body.to_vec(), Vec::new())
}

fn api(body: &str) -> ProcessOutput {
    output(
        0,
        format!("HTTP/2 200 OK\r\ncontent-type: application/json\r\n\r\n{body}").as_bytes(),
    )
}

fn outputs() -> Vec<ProcessOutput> {
    outputs_with_release(
        r#"[{"tag_name":"v1.2.3","draft":false,"prerelease":false,"assets":[{"name":"app.zip","digest":"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}]}]"#,
    )
}

fn outputs_with_release(releases: &str) -> Vec<ProcessOutput> {
    vec![
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
        api(r#"{"full_name":"owner/repo","name":"repo","owner":{"login":"owner"}}"#),
        api(releases),
    ]
}

fn observe(
    root: &TestRoot,
    outputs: impl IntoIterator<Item = ProcessOutput>,
    policy_source: &str,
) -> (
    jameskills_core::domain::policy::CheckObservation,
    Vec<Vec<OsString>>,
) {
    let process = Arc::new(FakeProcess::new(outputs));
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
    let policy = parse_policy(policy_source.as_bytes()).unwrap();
    assert_eq!(
        policy.requirements()[0].guidance_id(),
        Some("release-setup")
    );
    let observation = run(provider.observe(&policy.requirements()[0])).unwrap();
    (observation, process.arguments())
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
    .expect("native host tool must be installed for the opt-in release test");
    let executable = std::fs::canonicalize(candidate.path().unwrap()).unwrap();
    ApprovedRepositoryTool::new(
        ApprovedExecutable::from_absolute_path(executable.clone()).unwrap(),
        fingerprint_executable(&executable).unwrap(),
    )
}

fn run<F: Future>(future: F) -> F::Output {
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
fn current_project_version_requires_a_published_matching_release_with_asset_digest() {
    let root = TestRoot::new();
    let (observation, calls) = observe(&root, outputs(), RELEASE_POLICY);

    assert_eq!(observation.status(), CheckStatus::Pass);
    assert!(matches!(
        observation.enforcement(),
        Some(Enforcement::LocalCheck)
    ));
    assert!(observation.evidence()[0].summary().contains("1.2.3"));
    assert_eq!(calls.len(), 7);
    assert_eq!(
        calls[6],
        [
            "api",
            "--hostname",
            "github.com",
            "--method",
            "GET",
            "--include",
            "repos/owner/repo/releases?per_page=100",
        ]
        .map(OsString::from)
    );
}

#[test]
fn mismatched_tag_and_published_prerelease_fail() {
    let root = TestRoot::new();
    let (mismatch, _) = observe(
        &root,
        outputs_with_release(
            r#"[{"tag_name":"v1.2.4","draft":false,"prerelease":false,"assets":[]}]"#,
        ),
        RELEASE_POLICY,
    );
    assert_eq!(mismatch.status(), CheckStatus::Fail);

    let (prerelease, _) = observe(
        &root,
        outputs_with_release(
            r#"[{"tag_name":"v1.2.3","draft":false,"prerelease":true,"assets":[]}]"#,
        ),
        RELEASE_POLICY,
    );
    assert_eq!(prerelease.status(), CheckStatus::Fail);

    let (draft, _) = observe(
        &root,
        outputs_with_release(
            r#"[{"tag_name":"v1.2.3","draft":true,"prerelease":false,"assets":[]}]"#,
        ),
        RELEASE_POLICY,
    );
    assert_eq!(draft.status(), CheckStatus::Fail);
}

#[test]
fn complete_release_list_without_version_fails_but_access_errors_remain_unknown_or_blocked() {
    let root = TestRoot::new();
    let (absent, _) = observe(&root, outputs_with_release("[]"), RELEASE_POLICY);
    assert_eq!(absent.status(), CheckStatus::Fail);

    let mut forbidden = outputs();
    *forbidden.last_mut().unwrap() = output(
        1,
        b"HTTP/2 403 Forbidden\r\ncontent-type: application/json\r\n\r\n{\"message\":\"permission denied\"}",
    );
    let (denied, _) = observe(&root, forbidden, RELEASE_POLICY);
    assert_eq!(denied.status(), CheckStatus::Blocked);

    let mut not_found = outputs();
    *not_found.last_mut().unwrap() = output(
        1,
        b"HTTP/2 404 Not Found\r\ncontent-type: application/json\r\n\r\n{\"message\":\"not found\"}",
    );
    let (ambiguous, _) = observe(&root, not_found, RELEASE_POLICY);
    assert_eq!(ambiguous.status(), CheckStatus::Unknown);
}

#[test]
fn release_list_at_api_page_limit_is_unknown_without_following_pagination() {
    let root = TestRoot::new();
    let entry = r#"{"tag_name":"v0.0.1","draft":false,"prerelease":false,"assets":[]}"#;
    let response = format!("[{}]", vec![entry; 100].join(","));
    let (observation, calls) = observe(&root, outputs_with_release(&response), RELEASE_POLICY);
    assert_eq!(observation.status(), CheckStatus::Unknown);
    assert_eq!(calls.len(), 7);
}

#[test]
fn missing_asset_digest_fails_and_missing_digest_field_is_unknown() {
    let root = TestRoot::new();
    let (missing, _) = observe(
        &root,
        outputs_with_release(
            r#"[{"tag_name":"1.2.3","draft":false,"prerelease":false,"assets":[{"name":"app.zip","digest":null}]}]"#,
        ),
        RELEASE_POLICY,
    );
    assert_eq!(missing.status(), CheckStatus::Fail);

    let (unavailable, _) = observe(
        &root,
        outputs_with_release(
            r#"[{"tag_name":"1.2.3","draft":false,"prerelease":false,"assets":[{"name":"app.zip"}]}]"#,
        ),
        RELEASE_POLICY,
    );
    assert_eq!(unavailable.status(), CheckStatus::Unknown);
}

#[test]
fn conflicting_project_manifests_fail_before_any_host_request() {
    let root = TestRoot::new();
    root.write_node_version("2.0.0");
    let (observation, calls) = observe(&root, [], RELEASE_POLICY);
    assert_eq!(observation.status(), CheckStatus::Fail);
    assert!(calls.is_empty());
}

#[test]
fn inherited_cargo_and_node_project_versions_match_without_reading_skill_version() {
    let root = TestRoot::new();
    root.write_cargo_manifest(
        "[workspace]\nmembers = []\n[workspace.package]\nversion = \"1.2.3\"\n[package]\nname = \"fixture\"\nversion.workspace = true\n",
    );
    root.write_node_version("1.2.3");
    std::fs::write(
        root.0.join("jameskills.toml"),
        "semantic_version = \"9.9.9\"\n",
    )
    .unwrap();
    let (observation, _) = observe(&root, outputs(), RELEASE_POLICY);
    assert_eq!(observation.status(), CheckStatus::Pass);
}

#[test]
fn metadata_only_node_manifest_does_not_conflict_with_cargo_project_version() {
    let root = TestRoot::new();
    std::fs::write(
        root.0.join("package.json"),
        "{\"name\":\"governance-only\",\"private\":true}",
    )
    .unwrap();
    let (observation, _) = observe(&root, outputs(), RELEASE_POLICY);
    assert_eq!(observation.status(), CheckStatus::Pass);
}

#[test]
fn duplicate_node_version_fields_remain_unknown_without_host_queries() {
    let root = TestRoot::new();
    std::fs::write(
        root.0.join("package.json"),
        "{\"version\":\"1.2.3\",\"version\":\"9.9.9\"}",
    )
    .unwrap();
    let (observation, calls) = observe(&root, [], RELEASE_POLICY);
    assert_eq!(observation.status(), CheckStatus::Unknown);
    assert!(calls.is_empty());
}

#[test]
fn required_changelog_must_have_a_matching_markdown_heading() {
    let root = TestRoot::new();
    root.write_changelog("# Changelog\n\n## [1.2.2]\nOld version only.\n");
    let (observation, calls) = observe(&root, [], RELEASE_POLICY);
    assert_eq!(observation.status(), CheckStatus::Fail);
    assert!(calls.is_empty());

    root.write_changelog("# Changelog\n\n## [1.2.3]\n");
    let (empty, calls) = observe(&root, [], RELEASE_POLICY);
    assert_eq!(empty.status(), CheckStatus::Fail);
    assert!(calls.is_empty());
}

#[test]
fn signature_requirement_reads_only_annotated_verified_tag_metadata() {
    let root = TestRoot::new();
    let signed_policy =
        RELEASE_POLICY.replace("require_signature = false", "require_signature = true");
    let mut signed = outputs();
    signed.push(api(
        r#"{"ref":"refs/tags/v1.2.3","object":{"type":"tag","sha":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}}"#,
    ));
    signed.push(api(
        r#"{"tag":"v1.2.3","object":{"type":"commit","sha":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"},"verification":{"verified":true,"reason":"valid","signature":"private signature","payload":"private payload"}}"#,
    ));
    let (verified, _) = observe(&root, signed, &signed_policy);
    assert_eq!(verified.status(), CheckStatus::Pass);
    assert!(
        !verified.evidence()[0]
            .summary()
            .contains("private signature")
    );
    assert!(!verified.evidence()[0].summary().contains("private payload"));

    let mut unsigned = outputs();
    unsigned.push(api(
        r#"{"ref":"refs/tags/v1.2.3","object":{"type":"tag","sha":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}}"#,
    ));
    unsigned.push(api(
        r#"{"tag":"v1.2.3","object":{"type":"commit","sha":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"},"verification":{"verified":false,"reason":"unsigned","signature":null,"payload":null}}"#,
    ));
    let (unverified, _) = observe(&root, unsigned, &signed_policy);
    assert_eq!(unverified.status(), CheckStatus::Fail);

    let mut lightweight = outputs();
    lightweight.push(api(
        r#"{"ref":"refs/tags/v1.2.3","object":{"type":"commit","sha":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}}"#,
    ));
    let (lightweight, _) = observe(&root, lightweight, &signed_policy);
    assert_eq!(lightweight.status(), CheckStatus::Fail);
}

#[test]
fn release_without_required_artifacts_can_pass_when_the_profile_does_not_request_them() {
    let root = TestRoot::new();
    let minimal_policy = RELEASE_POLICY
        .replace("require_changelog = true", "require_changelog = false")
        .replace("require_checksums = true", "require_checksums = false");
    let (observation, _) = observe(
        &root,
        outputs_with_release(
            r#"[{"tag_name":"1.2.3","draft":false,"prerelease":false,"assets":[]}]"#,
        ),
        &minimal_policy,
    );
    assert_eq!(observation.status(), CheckStatus::Pass);
}

#[test]
#[ignore = "read-only GitHub release-list query for the selected checkout; requires authenticated gh"]
fn real_release_evidence_is_read_only_and_reports_selected_project_version() {
    let repository_root =
        std::fs::canonicalize(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
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
    let policy_source = RELEASE_POLICY
        .replace("require_changelog = true", "require_changelog = false")
        .replace("require_checksums = true", "require_checksums = false");
    let policy = parse_policy(policy_source.as_bytes()).unwrap();
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
        .expect("authenticated release query should return sanitized evidence");
    assert_eq!(
        evidence.source_id(),
        "github.release-contract",
        "{}",
        evidence.summary()
    );
    assert!(evidence.summary().contains("v=0.1.0"));
}
