use async_trait::async_trait;
use jameskills_core::{
    AppError, AppResult,
    domain::{CapabilitySupport, PortablePath, Scope, validate_bundle},
    ports::{
        agent::{AgentAvailability, AgentPort, ApprovedAgentExecutable, DetectionContext},
        process::{
            ApprovedExecutable, ApprovedRoot, ExecutableFingerprint, ProcessIdentity,
            ProcessOutput, ProcessPermission, ProcessPort, ProcessSpec,
        },
    },
};
use jameskills_infra::{
    agents::{AgentRegistry, codex::CodexAdapter},
    process::fingerprint_executable,
};
use semver::Version;
use std::{
    collections::{BTreeMap, VecDeque},
    ffi::OsString,
    sync::{Arc, Mutex},
};

fn files() -> BTreeMap<PortablePath, Vec<u8>> {
    [
        (
            "jameskills.toml",
            "schema_version = 1\nid = \"f9c0199f-c4ce-4b04-85dd-ae12a7db292b\"\nslug = \"portable-demo\"\ndisplay_name = \"Portable Demo\"\nversion = \"1.0.0\"\ndescription = \"Test fixture\"\nlicense = \"Apache-2.0\"\nminimum_app_version = \"0.1.0\"\npolicy_files = []\nguidance_files = []\n",
        ),
        (
            "SKILL.md",
            "---\nname: portable-demo\ndescription: Test fixture\n---\n\nInstructions remain data.\n",
        ),
        ("assets/readme.txt", "Preserve exact bytes and relative paths.\n"),
    ]
    .into_iter()
    .map(|(path, contents)| {
        (
            PortablePath::new(path.to_owned()).unwrap(),
            contents.as_bytes().to_vec(),
        )
    })
    .collect()
}

struct ProcessCall {
    identity_is_codex: bool,
    args: Vec<OsString>,
    permission: ProcessPermission,
    fingerprint: Option<ExecutableFingerprint>,
}

struct FakeProcess {
    responses: Mutex<VecDeque<AppResult<ProcessOutput>>>,
    calls: Mutex<Vec<ProcessCall>>,
}

impl FakeProcess {
    fn new(responses: Vec<AppResult<ProcessOutput>>) -> Arc<Self> {
        Arc::new(Self {
            responses: Mutex::new(responses.into()),
            calls: Mutex::new(Vec::new()),
        })
    }
}

#[async_trait]
impl ProcessPort for FakeProcess {
    async fn run(&self, spec: ProcessSpec) -> AppResult<ProcessOutput> {
        self.calls.lock().unwrap().push(ProcessCall {
            identity_is_codex: spec.process_identity()
                == ProcessIdentity::Agent(jameskills_core::domain::agent::AgentId::Codex),
            args: spec.args().to_vec(),
            permission: spec.permission(),
            fingerprint: spec.approved_executable_fingerprint().copied(),
        });
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| Err(AppError::Cancelled))
    }
}

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(future)
}

fn approved_profile(responses: Vec<AppResult<ProcessOutput>>) -> (CodexAdapter, Arc<FakeProcess>) {
    let registry = AgentRegistry::built_in().unwrap();
    let process = FakeProcess::new(responses);
    let home = std::env::temp_dir().join("Codex User Home With Spaces");
    let adapter = CodexAdapter::new(
        registry
            .get(jameskills_core::domain::agent::AgentId::Codex)
            .unwrap()
            .clone(),
        process.clone(),
        home,
    )
    .unwrap();
    (adapter, process)
}

#[test]
fn codex_adapter_rejects_a_different_agents_profile() {
    let registry = AgentRegistry::built_in().unwrap();
    let process = FakeProcess::new(vec![]);

    assert!(
        CodexAdapter::new(
            registry
                .get(jameskills_core::domain::agent::AgentId::OpenCode)
                .unwrap()
                .clone(),
            process,
            std::env::temp_dir().join("Codex Home"),
        )
        .is_err()
    );
}

#[test]
fn plan_artifact_uses_documented_scope_roots_and_preserves_bundle_bytes() {
    let files = files();
    let validated = validate_bundle(&files).unwrap();
    let (adapter, _) = approved_profile(vec![]);
    let home = std::env::temp_dir().join("Codex User Home With Spaces");

    let user = adapter
        .plan_artifact(&validated, files.clone(), &home, None, Scope::User)
        .unwrap();
    let expected_user_target = home.join(".agents").join("skills").join("portable-demo");
    assert_eq!(user.target(), expected_user_target.as_path());
    assert_eq!(user.files(), &files);
    assert!(
        !user
            .files()
            .contains_key(&PortablePath::new("agents/openai.yaml".to_owned()).unwrap())
    );

    let project_root = std::env::temp_dir().join("Repository With Spaces");
    let approved_root = ApprovedRoot::from_absolute_path(project_root.clone()).unwrap();
    let project = adapter
        .plan_artifact(
            &validated,
            files.clone(),
            &home,
            Some(&approved_root),
            Scope::Project,
        )
        .unwrap();
    let expected_project_target = project_root
        .join(".agents")
        .join("skills")
        .join("portable-demo");
    assert_eq!(project.target(), expected_project_target.as_path());
    assert_eq!(project.files(), &files);
}

#[test]
fn plan_artifact_rejects_bytes_that_differ_from_validated_bundle() {
    let files = files();
    let validated = validate_bundle(&files).unwrap();
    let mut changed = files;
    changed.insert(
        PortablePath::new("SKILL.md".to_owned()).unwrap(),
        b"---\nname: portable-demo\ndescription: changed\n---\n".to_vec(),
    );
    let (adapter, _) = approved_profile(vec![]);

    assert!(matches!(
        adapter.plan_artifact(
            &validated,
            changed,
            &std::env::temp_dir().join("Codex Home"),
            None,
            Scope::User,
        ),
        Err(AppError::Conflict { .. })
    ));
}

#[test]
fn project_artifact_requires_an_approved_root() {
    let files = files();
    let validated = validate_bundle(&files).unwrap();
    let (adapter, _) = approved_profile(vec![]);

    assert!(
        adapter
            .plan_artifact(
                &validated,
                files,
                &std::env::temp_dir().join("Codex Home"),
                None,
                Scope::Project,
            )
            .is_err()
    );
}

#[test]
fn codex_version_probe_uses_only_confirmed_binary_and_fixed_read_only_argv() {
    let output = ProcessOutput::new(Some(0), b"codex 0.100.1\n".to_vec(), Vec::new());
    let (adapter, process) = approved_profile(vec![Ok(output)]);
    let executable_path = std::env::current_exe().unwrap();
    let fingerprint = fingerprint_executable(&executable_path).unwrap();
    let executable = ApprovedExecutable::from_absolute_path(executable_path).unwrap();
    let approved = ApprovedAgentExecutable::after_explicit_fingerprint_confirmation(
        executable,
        fingerprint,
        &fingerprint,
    )
    .unwrap();
    let context = DetectionContext::new(Scope::User, None)
        .unwrap()
        .with_approved_executable(approved);

    let detection = block_on(adapter.detect(context)).unwrap();

    assert_eq!(detection.availability(), AgentAvailability::Verified);
    assert_eq!(
        detection.version(),
        Some(&Version::parse("0.100.1").unwrap())
    );
    assert!(detection.evidence().iter().any(|evidence| {
        evidence.source_id() == "codex-cli-source"
            && evidence.tested_version() == Some(&Version::parse("0.100.1").unwrap())
            && evidence.fixture_id().is_none()
    }));
    assert_eq!(
        detection
            .capabilities()
            .get(jameskills_core::domain::agent::AgentCapabilityId::UserInstall),
        CapabilitySupport::NeedsVerification
    );
    let calls = process.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert!(calls[0].identity_is_codex);
    assert_eq!(calls[0].args, [OsString::from("--version")]);
    assert!(calls[0].permission == ProcessPermission::ReadOnlyCheck);
    assert!(calls[0].fingerprint == Some(fingerprint));
}

#[test]
fn codex_candidate_detection_does_not_spawn_without_executable_confirmation() {
    let (adapter, process) = approved_profile(vec![]);
    let detection =
        block_on(adapter.detect(DetectionContext::new(Scope::User, None).unwrap())).unwrap();

    assert!(matches!(
        detection.availability(),
        AgentAvailability::Missing | AgentAvailability::Candidate
    ));
    assert!(process.calls.lock().unwrap().is_empty());
}

#[test]
fn unknown_codex_version_output_blocks_detection_without_claiming_support() {
    let output = ProcessOutput::new(Some(0), b"Codex development build\n".to_vec(), Vec::new());
    let (adapter, _) = approved_profile(vec![Ok(output)]);
    let executable_path = std::env::current_exe().unwrap();
    let fingerprint = fingerprint_executable(&executable_path).unwrap();
    let executable = ApprovedExecutable::from_absolute_path(executable_path).unwrap();
    let approved = ApprovedAgentExecutable::after_explicit_fingerprint_confirmation(
        executable,
        fingerprint,
        &fingerprint,
    )
    .unwrap();
    let context = DetectionContext::new(Scope::User, None)
        .unwrap()
        .with_approved_executable(approved);

    let detection = block_on(adapter.detect(context)).unwrap();

    assert_eq!(detection.availability(), AgentAvailability::Blocked);
    assert!(detection.version().is_none());
    assert_eq!(
        detection
            .capabilities()
            .get(jameskills_core::domain::agent::AgentCapabilityId::UserInstall),
        CapabilitySupport::NeedsVerification
    );
}

#[test]
fn current_user_codex_home_is_resolved_from_platform_directories() {
    let files = files();
    let validated = validate_bundle(&files).unwrap();
    let (adapter, _) = approved_profile(vec![]);
    let home = jameskills_infra::platform::user_home_directory().unwrap();

    let artifact = adapter
        .plan_artifact_for_current_user(&validated, files, None, Scope::User)
        .unwrap();
    assert!(home.is_absolute());
    assert_eq!(
        artifact.target(),
        home.join(".agents").join("skills").join("portable-demo")
    );
}
