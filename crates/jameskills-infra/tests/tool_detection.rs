use async_trait::async_trait;
use jameskills_core::{
    AppError,
    domain::{ToolAvailability, ToolCapabilitySupport, ToolId, ToolOperation, ToolVersionStatus},
    ports::process::{
        ApprovedEnv, ApprovedRoot, ExecutableFingerprint, ProcessOutput, ProcessPort, ProcessSpec,
    },
};
use jameskills_infra::platform::{
    HostPlatform, ProjectStack, ToolCandidate, ToolCandidateKind, ToolProfile, detect_tools,
    find_tool_candidates, inspect_project_manifests, load_tool_profiles, parse_tool_version_output,
    probe_registered_tool_version,
};
use std::{
    collections::{BTreeMap, VecDeque},
    ffi::OsString,
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

static NEXT_CANDIDATE_ID: AtomicU64 = AtomicU64::new(0);

fn project_root() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "jameskills project manifests-{}-{}",
        std::process::id(),
        NEXT_CANDIDATE_ID.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    root
}

struct Invocation {
    tool_id: ToolId,
    args: Vec<OsString>,
    fingerprint: Option<ExecutableFingerprint>,
}

struct FakeProcessPort {
    responses: Mutex<VecDeque<Result<ProcessOutput, AppError>>>,
    invocations: Mutex<Vec<Invocation>>,
}

impl FakeProcessPort {
    fn new(responses: Vec<Result<ProcessOutput, AppError>>) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
            invocations: Mutex::new(Vec::new()),
        }
    }

    fn invocation_count(&self) -> usize {
        self.invocations.lock().unwrap().len()
    }
}

#[async_trait]
impl ProcessPort for FakeProcessPort {
    async fn run(&self, spec: ProcessSpec) -> Result<ProcessOutput, AppError> {
        self.invocations.lock().unwrap().push(Invocation {
            tool_id: spec
                .tool_id()
                .expect("tool detection uses a registered ToolId"),
            args: spec.args().to_vec(),
            fingerprint: spec.approved_executable_fingerprint().copied(),
        });
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .expect("test response configured")
    }
}

fn output(stdout: &[u8], exit_code: Option<i32>) -> Result<ProcessOutput, AppError> {
    Ok(ProcessOutput::new(exit_code, stdout.to_vec(), Vec::new()))
}

fn run_probe(
    profile: &ToolProfile,
    candidate: &ToolCandidate,
    fingerprint: Option<ExecutableFingerprint>,
    process: &FakeProcessPort,
) -> Result<jameskills_core::domain::ToolDetection, AppError> {
    let cwd = ApprovedRoot::from_absolute_path(std::fs::canonicalize(".").unwrap()).unwrap();
    let environment = ApprovedEnv::new(Default::default()).unwrap();
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(probe_registered_tool_version(
            profile,
            candidate,
            fingerprint,
            &cwd,
            &environment,
            process,
            "2026-10-04T00:00:00Z",
        ))
}

fn profile(profiles: &[ToolProfile], tool_id: ToolId) -> &ToolProfile {
    profiles
        .iter()
        .find(|profile| profile.tool_id() == tool_id)
        .unwrap()
}

fn candidate_for(profile: &ToolProfile, filename: &str, host: HostPlatform) -> ToolCandidate {
    let root = std::env::temp_dir().join(format!(
        "jameskills probe candidate-{}-{}-{filename}",
        std::process::id(),
        NEXT_CANDIDATE_ID.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join(filename), b"candidate binary fingerprint fixture").unwrap();
    let candidates = find_tool_candidates(
        std::slice::from_ref(profile),
        std::slice::from_ref(&root),
        host,
    );
    candidates.into_iter().next().unwrap()
}

fn remove_candidate(candidate: &ToolCandidate) {
    if let Some(root) = candidate.path().and_then(Path::parent) {
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn app_owned_tool_profiles_register_supported_tools_and_fixed_probes() {
    let profiles = load_tool_profiles().expect("built-in profiles are valid");
    assert_eq!(profiles.len(), 10);

    let expected = [
        (ToolId::Git, "git"),
        (ToolId::Cargo, "cargo"),
        (ToolId::Npm, "npm"),
        (ToolId::Node, "node"),
        (ToolId::Rustc, "rustc"),
        (ToolId::Gh, "gh"),
        (ToolId::Gitleaks, "gitleaks"),
        (ToolId::Commitlint, "commitlint"),
        (ToolId::CargoAudit, "cargo-audit"),
        (ToolId::CargoDeny, "cargo-deny"),
    ];
    for (tool_id, executable) in expected {
        let profile = profiles
            .iter()
            .find(|profile| profile.tool_id() == tool_id)
            .expect("registered tool profile");
        assert_eq!(profile.executable_name(), executable);
        assert!(!profile.source_id().is_empty());
        assert!(profile.operations().contains(&ToolOperation::Version));
        assert!(!profile.version_args().is_empty());
        assert!(profile.version_args().iter().all(|argument| {
            !argument.contains('\0') && !argument.contains('\n') && !argument.contains('\r')
        }));
    }

    let audit = profiles
        .iter()
        .find(|profile| profile.tool_id() == ToolId::CargoAudit)
        .unwrap();
    assert_eq!(audit.executable_name(), "cargo-audit");
    assert_eq!(audit.version_args(), ["--version"]);
    assert!(audit.operations().contains(&ToolOperation::Audit));

    let deny = profiles
        .iter()
        .find(|profile| profile.tool_id() == ToolId::CargoDeny)
        .unwrap();
    assert_eq!(deny.executable_name(), "cargo-deny");
    assert_eq!(deny.version_args(), ["--version"]);
    assert!(deny.operations().contains(&ToolOperation::Deny));
}

#[test]
fn every_registered_tool_has_app_owned_windows_and_linux_install_guides() {
    let profiles = load_tool_profiles().unwrap();
    for profile in &profiles {
        let guides = profile.install_guides();
        for platform in [HostPlatform::Windows, HostPlatform::Linux] {
            let guide = guides
                .for_platform(platform)
                .expect("guide for supported OS");
            assert!(guide.source_id().ends_with(platform_suffix(platform)));
            assert!(guide.url().starts_with("https://"));
        }
        assert!(guides.for_platform(HostPlatform::Other).is_none());
    }

    let git = profile(&profiles, ToolId::Git).install_guides();
    assert_eq!(
        git.for_platform(HostPlatform::Windows).unwrap().source_id(),
        "git-install-windows"
    );
    assert_eq!(
        git.for_platform(HostPlatform::Linux).unwrap().source_id(),
        "git-install-linux"
    );
}

#[test]
fn project_stack_comes_from_validated_manifests_not_skill_metadata() {
    let rust = project_root();
    std::fs::write(
        rust.join("Cargo.toml"),
        "[package]\nname = \"fixture-rust\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let facts = inspect_project_manifests(&ApprovedRoot::from_absolute_path(rust.clone()).unwrap())
        .unwrap();
    assert_eq!(facts.stack(), ProjectStack::Rust);
    assert!(facts.node_script_names().is_empty());
    std::fs::remove_dir_all(rust).unwrap();

    let node = project_root();
    std::fs::write(
        node.join("package.json"),
        r#"{"name":"fixture-node","scripts":{"test":"echo safe fixture","build":"echo build"}}"#,
    )
    .unwrap();
    let facts = inspect_project_manifests(&ApprovedRoot::from_absolute_path(node.clone()).unwrap())
        .unwrap();
    assert_eq!(facts.stack(), ProjectStack::Node);
    assert_eq!(
        facts.node_script_names(),
        &["build".to_owned(), "test".to_owned()]
    );
    std::fs::remove_dir_all(node).unwrap();

    let mixed = project_root();
    std::fs::write(mixed.join("Cargo.toml"), "[workspace]\nmembers = []\n").unwrap();
    std::fs::write(mixed.join("package.json"), r#"{"name":"fixture-mixed"}"#).unwrap();
    let facts =
        inspect_project_manifests(&ApprovedRoot::from_absolute_path(mixed.clone()).unwrap())
            .unwrap();
    assert_eq!(facts.stack(), ProjectStack::RustAndNode);
    std::fs::remove_dir_all(mixed).unwrap();
}

#[test]
fn malformed_or_oversized_manifests_remain_unknown_and_absence_is_generic() {
    let empty = project_root();
    let facts =
        inspect_project_manifests(&ApprovedRoot::from_absolute_path(empty.clone()).unwrap())
            .unwrap();
    assert_eq!(facts.stack(), ProjectStack::Generic);
    std::fs::remove_dir_all(empty).unwrap();

    let malformed = project_root();
    std::fs::write(malformed.join("package.json"), b"{not-json").unwrap();
    let facts =
        inspect_project_manifests(&ApprovedRoot::from_absolute_path(malformed.clone()).unwrap())
            .unwrap();
    assert_eq!(facts.stack(), ProjectStack::Unknown);
    std::fs::remove_dir_all(malformed).unwrap();

    let oversized = project_root();
    std::fs::write(oversized.join("package.json"), vec![b' '; 1024 * 1024 + 1]).unwrap();
    let facts =
        inspect_project_manifests(&ApprovedRoot::from_absolute_path(oversized.clone()).unwrap())
            .unwrap();
    assert_eq!(facts.stack(), ProjectStack::Unknown);
    std::fs::remove_dir_all(oversized).unwrap();

    let non_regular = project_root();
    std::fs::create_dir(non_regular.join("Cargo.toml")).unwrap();
    let facts =
        inspect_project_manifests(&ApprovedRoot::from_absolute_path(non_regular.clone()).unwrap())
            .unwrap();
    assert_eq!(facts.stack(), ProjectStack::Unknown);
    std::fs::remove_dir_all(non_regular).unwrap();
}

fn platform_suffix(platform: HostPlatform) -> &'static str {
    match platform {
        HostPlatform::Windows => "windows",
        HostPlatform::Linux => "linux",
        HostPlatform::Other => "other",
    }
}

#[test]
fn candidate_discovery_never_executes_path_entries_and_marks_windows_shims() {
    let root =
        std::env::temp_dir().join(format!("jameskills tool candidates-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    for file in ["git.exe", "npm.cmd", "cargo-audit.exe"] {
        std::fs::write(root.join(file), b"candidate fixture").unwrap();
    }

    let profiles = load_tool_profiles().unwrap();
    let candidates = find_tool_candidates(
        &profiles,
        std::slice::from_ref(&root),
        HostPlatform::Windows,
    );
    let git = candidates
        .iter()
        .find(|candidate| candidate.tool_id() == ToolId::Git)
        .unwrap();
    assert_eq!(git.kind(), ToolCandidateKind::NativeExecutable);
    let expected_git = std::fs::canonicalize(root.join("git.exe")).unwrap();
    assert_eq!(git.path(), Some(expected_git.as_path()));

    let npm = candidates
        .iter()
        .find(|candidate| candidate.tool_id() == ToolId::Npm)
        .unwrap();
    assert_eq!(npm.kind(), ToolCandidateKind::CommandShim);
    assert_eq!(
        npm.path().unwrap().file_name(),
        Some(std::ffi::OsStr::new("npm.cmd"))
    );

    let node = candidates
        .iter()
        .find(|candidate| candidate.tool_id() == ToolId::Node)
        .unwrap();
    assert_eq!(node.kind(), ToolCandidateKind::Missing);
    assert_eq!(node.path(), None);
    assert!(root.is_absolute());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn commitlint_profile_supports_the_reviewed_default_config_cli() {
    let profiles = load_tool_profiles().unwrap();
    let commitlint = profile(&profiles, ToolId::Commitlint);
    let reviewed = semver::Version::parse("21.2.2").unwrap();
    let prior_major = semver::Version::parse("20.2.0").unwrap();
    let unreviewed_minor = semver::Version::parse("21.2.3").unwrap();

    assert!(
        commitlint.version_range().matches(&reviewed),
        "the reviewed --default-config CLI must be within the app-owned range"
    );
    assert!(!commitlint.version_range().matches(&prior_major));
    assert!(!commitlint.version_range().matches(&unreviewed_minor));
}

#[test]
fn node_profile_covers_commitlint_supported_node_24_runtime() {
    let profiles = load_tool_profiles().unwrap();
    let node = profile(&profiles, ToolId::Node);
    let supported_lts = semver::Version::parse("24.18.0").unwrap();
    let unsupported_major = semver::Version::parse("25.0.0").unwrap();

    assert!(
        node.version_range().matches(&supported_lts),
        "the registered Node range must include the supported Node 24 LTS runtime"
    );
    assert!(!node.version_range().matches(&unsupported_major));
}

#[test]
fn version_parser_accepts_registered_formats_and_rejects_unknown_output() {
    let profiles = load_tool_profiles().unwrap();
    let git = profiles
        .iter()
        .find(|profile| profile.tool_id() == ToolId::Git)
        .unwrap();
    assert_eq!(
        parse_tool_version_output(git, b"git version 2.51.0.windows.1\n"),
        Some(semver::Version::new(2, 51, 0))
    );

    let node = profiles
        .iter()
        .find(|profile| profile.tool_id() == ToolId::Node)
        .unwrap();
    assert_eq!(
        parse_tool_version_output(node, b"v22.3.0\n"),
        Some(semver::Version::new(22, 3, 0))
    );
    assert_eq!(parse_tool_version_output(git, b"not a version\n"), None);
    assert_eq!(parse_tool_version_output(git, b"cargo 2.51.0\n"), None);
    assert_eq!(parse_tool_version_output(git, &vec![b'1'; 65 * 1024]), None);
}

#[test]
fn relative_search_paths_are_not_used_for_candidate_discovery() {
    let profiles = load_tool_profiles().unwrap();
    let candidates = find_tool_candidates(
        &profiles,
        &[PathBuf::from("relative-tools")],
        HostPlatform::Linux,
    );
    assert!(
        candidates
            .iter()
            .all(|candidate| candidate.kind() == ToolCandidateKind::Missing)
    );
}

#[test]
fn candidate_without_fingerprint_is_not_executed() {
    let profiles = load_tool_profiles().unwrap();
    let node = profile(&profiles, ToolId::Node);
    let candidate = candidate_for(node, "node", HostPlatform::Linux);
    let process = FakeProcessPort::new(vec![output(b"v22.3.0\n", Some(0))]);

    let detection = run_probe(node, &candidate, None, &process).unwrap();

    assert_eq!(detection.availability(), ToolAvailability::Candidate);
    assert_eq!(detection.version_status(), ToolVersionStatus::Unknown);
    assert_eq!(detection.version(), None);
    assert_eq!(
        detection.capability(ToolOperation::Version),
        Some(ToolCapabilitySupport::NeedsVerification)
    );
    assert_eq!(process.invocation_count(), 0);
    remove_candidate(&candidate);
}

#[test]
fn approved_native_probe_uses_only_registered_argv_and_unknown_output_stays_unknown() {
    let profiles = load_tool_profiles().unwrap();
    let node = profile(&profiles, ToolId::Node);
    let candidate = candidate_for(node, "node", HostPlatform::Linux);
    let fingerprint = ExecutableFingerprint::from_sha256([0x42; 32]);
    let process = FakeProcessPort::new(vec![output(b"unrecognized version text\n", Some(0))]);

    let detection = run_probe(node, &candidate, Some(fingerprint), &process).unwrap();

    assert_eq!(detection.version_status(), ToolVersionStatus::Unknown);
    assert_eq!(
        detection.capability(ToolOperation::Version),
        Some(ToolCapabilitySupport::NeedsVerification)
    );
    let invocations = process.invocations.lock().unwrap();
    assert_eq!(invocations.len(), 1);
    assert!(invocations[0].tool_id == ToolId::Node);
    assert_eq!(invocations[0].args, [OsString::from("--version")]);
    assert!(invocations[0].fingerprint == Some(fingerprint));
    drop(invocations);
    remove_candidate(&candidate);
}

#[test]
fn missing_incompatible_and_windows_shim_tools_never_report_supported() {
    let profiles = load_tool_profiles().unwrap();
    let node = profile(&profiles, ToolId::Node);
    let missing = find_tool_candidates(std::slice::from_ref(node), &[], HostPlatform::Linux)
        .into_iter()
        .next()
        .unwrap();
    let process = FakeProcessPort::new(Vec::new());
    let missing_detection = run_probe(node, &missing, None, &process).unwrap();
    assert_eq!(missing_detection.availability(), ToolAvailability::Missing);
    assert_eq!(
        missing_detection.version_status(),
        ToolVersionStatus::NotApplicable
    );

    let candidate = candidate_for(node, "node", HostPlatform::Linux);
    let process = FakeProcessPort::new(vec![output(b"v25.1.0\n", Some(0))]);
    let incompatible = run_probe(
        node,
        &candidate,
        Some(ExecutableFingerprint::from_sha256([0x24; 32])),
        &process,
    )
    .unwrap();
    assert_eq!(
        incompatible.version_status(),
        ToolVersionStatus::Incompatible
    );
    assert_eq!(
        incompatible.capability(ToolOperation::Version),
        Some(ToolCapabilitySupport::Unsupported)
    );
    remove_candidate(&candidate);

    let changed = candidate_for(node, "node", HostPlatform::Linux);
    let process = FakeProcessPort::new(vec![Err(AppError::PermissionDenied {
        operation: "process.executable.identity_changed".to_owned(),
    })]);
    let blocked_identity = run_probe(
        node,
        &changed,
        Some(ExecutableFingerprint::from_sha256([0x24; 32])),
        &process,
    )
    .unwrap();
    assert_eq!(blocked_identity.availability(), ToolAvailability::Blocked);
    assert_eq!(
        blocked_identity.capability(ToolOperation::Version),
        Some(ToolCapabilitySupport::NeedsVerification)
    );
    remove_candidate(&changed);

    let npm = profile(&profiles, ToolId::Npm);
    let shim = candidate_for(npm, "npm.cmd", HostPlatform::Windows);
    assert_eq!(shim.kind(), ToolCandidateKind::CommandShim);
    let process = FakeProcessPort::new(Vec::new());
    let blocked = run_probe(
        npm,
        &shim,
        Some(ExecutableFingerprint::from_sha256([0x11; 32])),
        &process,
    )
    .unwrap();
    assert_eq!(blocked.availability(), ToolAvailability::Blocked);
    assert_eq!(process.invocation_count(), 0);
    remove_candidate(&shim);
}

#[test]
fn detect_tools_orchestrates_profiles_without_running_unapproved_candidates() {
    let root = project_root();
    std::fs::write(root.join("node"), b"candidate node executable").unwrap();
    let cwd = ApprovedRoot::from_absolute_path(std::fs::canonicalize(".").unwrap()).unwrap();
    let environment = ApprovedEnv::new(BTreeMap::new()).unwrap();
    let process = FakeProcessPort::new(Vec::new());
    let detections = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(detect_tools(
            std::slice::from_ref(&root),
            HostPlatform::Linux,
            &BTreeMap::new(),
            &cwd,
            &environment,
            &process,
            "2026-10-04T00:00:00Z",
        ))
        .unwrap();

    assert_eq!(detections.len(), 10);
    let node = detections
        .iter()
        .find(|detection| detection.tool_id() == ToolId::Node)
        .unwrap();
    assert_eq!(node.availability(), ToolAvailability::Candidate);
    assert_eq!(node.version_status(), ToolVersionStatus::Unknown);
    assert_eq!(process.invocation_count(), 0);
    std::fs::remove_dir_all(root).unwrap();
}
