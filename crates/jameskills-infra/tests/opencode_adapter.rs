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
    agents::{AgentRegistry, opencode::OpenCodeAdapter},
    process::fingerprint_executable,
};
use semver::Version;
use std::{
    collections::{BTreeMap, VecDeque},
    ffi::OsString,
    path::PathBuf,
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
        ("assets/readme.txt", "Keep this file byte-for-byte.\n"),
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
    identity_is_opencode: bool,
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
            identity_is_opencode: spec.process_identity()
                == ProcessIdentity::Agent(jameskills_core::domain::agent::AgentId::OpenCode),
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

fn adapter(
    responses: Vec<AppResult<ProcessOutput>>,
    xdg_config_home: Option<PathBuf>,
    config_directory_override: Option<PathBuf>,
) -> (OpenCodeAdapter, Arc<FakeProcess>, PathBuf) {
    let registry = AgentRegistry::built_in().unwrap();
    let process = FakeProcess::new(responses);
    let home = std::env::temp_dir().join("OpenCode User Home With Spaces");
    let adapter = OpenCodeAdapter::with_paths(
        registry
            .get(jameskills_core::domain::agent::AgentId::OpenCode)
            .unwrap()
            .clone(),
        process.clone(),
        home.clone(),
        xdg_config_home,
        config_directory_override,
    )
    .unwrap();
    (adapter, process, home)
}

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(future)
}

#[test]
fn opencode_artifact_uses_xdg_config_and_project_roots_with_exact_bundle_bytes() {
    let files = files();
    let validated = validate_bundle(&files).unwrap();
    let xdg = std::env::temp_dir().join("XDG Config Root With Spaces");
    let (adapter, _, _) = adapter(vec![], Some(xdg.clone()), None);

    let user = adapter
        .plan_artifact(&validated, files.clone(), None, Scope::User)
        .unwrap();
    let expected_user = xdg.join("opencode").join("skills").join("portable-demo");
    assert_eq!(user.target(), expected_user.as_path());
    assert_eq!(user.files(), &files);

    let repository = std::env::temp_dir().join("OpenCode Project With Spaces");
    let approved_root = ApprovedRoot::from_absolute_path(repository.clone()).unwrap();
    let project = adapter
        .plan_artifact(
            &validated,
            files.clone(),
            Some(&approved_root),
            Scope::Project,
        )
        .unwrap();
    let expected_project = repository
        .join(".opencode")
        .join("skills")
        .join("portable-demo");
    assert_eq!(project.target(), expected_project.as_path());
    assert_eq!(project.files(), &files);
}

#[test]
fn opencode_custom_config_directory_overrides_xdg_without_appdata_guessing() {
    let files = files();
    let validated = validate_bundle(&files).unwrap();
    let xdg = std::env::temp_dir().join("XDG Config Root");
    let custom = std::env::temp_dir().join("OpenCode Custom Config Root");
    let (adapter, _, _) = adapter(vec![], Some(xdg), Some(custom.clone()));

    let artifact = adapter
        .plan_artifact(&validated, files, None, Scope::User)
        .unwrap();
    assert_eq!(
        artifact.target(),
        custom.join("skills").join("portable-demo").as_path()
    );

    let registry = AgentRegistry::built_in().unwrap();
    assert!(
        OpenCodeAdapter::with_paths(
            registry
                .get(jameskills_core::domain::agent::AgentId::OpenCode)
                .unwrap()
                .clone(),
            FakeProcess::new(vec![]),
            std::env::temp_dir().join("OpenCode Home"),
            None,
            Some(PathBuf::from("relative/custom-config")),
        )
        .is_err()
    );
}

#[test]
fn opencode_version_probe_is_fingerprinted_read_only_and_feature_conservative() {
    let output = ProcessOutput::new(Some(0), b"opencode 1.2.3\n".to_vec(), Vec::new());
    let (adapter, process, _) = adapter(vec![Ok(output)], None, None);
    let executable_path = std::env::current_exe().unwrap();
    let fingerprint = fingerprint_executable(&executable_path).unwrap();
    let approved = ApprovedAgentExecutable::after_explicit_fingerprint_confirmation(
        ApprovedExecutable::from_absolute_path(executable_path).unwrap(),
        fingerprint,
        &fingerprint,
    )
    .unwrap();

    let detection = block_on(
        adapter.detect(
            DetectionContext::new(Scope::User, None)
                .unwrap()
                .with_approved_executable(approved),
        ),
    )
    .unwrap();

    assert_eq!(detection.availability(), AgentAvailability::Verified);
    assert_eq!(detection.version(), Some(&Version::parse("1.2.3").unwrap()));
    assert_eq!(
        detection
            .capabilities()
            .get(jameskills_core::domain::agent::AgentCapabilityId::UserInstall),
        CapabilitySupport::NeedsVerification
    );
    let calls = process.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert!(calls[0].identity_is_opencode);
    assert_eq!(calls[0].args, [OsString::from("--version")]);
    assert!(calls[0].permission == ProcessPermission::ReadOnlyCheck);
    assert!(calls[0].fingerprint == Some(fingerprint));
}

#[test]
fn opencode_unknown_version_output_blocks_and_unapproved_candidate_never_spawns() {
    let (candidate_adapter, process, _) = adapter(vec![], None, None);
    let candidate =
        block_on(candidate_adapter.detect(DetectionContext::new(Scope::User, None).unwrap()))
            .unwrap();
    assert!(matches!(
        candidate.availability(),
        AgentAvailability::Candidate | AgentAvailability::Missing
    ));
    assert!(process.calls.lock().unwrap().is_empty());

    let output = ProcessOutput::new(
        Some(0),
        b"unexpected opencode output\n".to_vec(),
        Vec::new(),
    );
    let (adapter, _, _) = adapter(vec![Ok(output)], None, None);
    let executable_path = std::env::current_exe().unwrap();
    let fingerprint = fingerprint_executable(&executable_path).unwrap();
    let approved = ApprovedAgentExecutable::after_explicit_fingerprint_confirmation(
        ApprovedExecutable::from_absolute_path(executable_path).unwrap(),
        fingerprint,
        &fingerprint,
    )
    .unwrap();
    let blocked = block_on(
        adapter.detect(
            DetectionContext::new(Scope::User, None)
                .unwrap()
                .with_approved_executable(approved),
        ),
    )
    .unwrap();
    assert_eq!(blocked.availability(), AgentAvailability::Blocked);
    assert!(blocked.version().is_none());
}
