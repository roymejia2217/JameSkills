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
    agents::{AgentRegistry, pi::PiAdapter},
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
            "---\nname: portable-demo\ndescription: Test fixture\n---\n\nInstructions remain inert data.\n",
        ),
        ("references/example.txt", "Keep exact bytes and relative paths.\n"),
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
    identity_is_pi: bool,
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
            identity_is_pi: spec.process_identity()
                == ProcessIdentity::Agent(jameskills_core::domain::agent::AgentId::Pi),
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
    user_home: PathBuf,
    agent_directory_override: Option<PathBuf>,
) -> (PiAdapter, Arc<FakeProcess>) {
    let registry = AgentRegistry::built_in().unwrap();
    let process = FakeProcess::new(responses);
    let adapter = PiAdapter::with_paths(
        registry
            .get(jameskills_core::domain::agent::AgentId::Pi)
            .unwrap()
            .clone(),
        process.clone(),
        user_home,
        agent_directory_override,
    )
    .unwrap();
    (adapter, process)
}

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(future)
}

#[test]
fn pi_user_override_and_project_roots_preserve_validated_bundle() {
    let files = files();
    let validated = validate_bundle(&files).unwrap();
    let home = std::env::temp_dir().join("Pi Home With Spaces");
    let (default_adapter, _) = adapter(vec![], home.clone(), None);

    let default_user = default_adapter
        .plan_artifact(&validated, files.clone(), None, Scope::User)
        .unwrap();
    let expected_default = home
        .join(".pi")
        .join("agent")
        .join("skills")
        .join("portable-demo");
    assert_eq!(default_user.target(), expected_default.as_path());
    assert_eq!(default_user.files(), &files);

    let agent_directory = std::env::temp_dir().join("Pi Custom Agent Directory");
    let (overridden_adapter, _) = adapter(vec![], home.clone(), Some(agent_directory.clone()));
    let overridden = overridden_adapter
        .plan_artifact(&validated, files.clone(), None, Scope::User)
        .unwrap();
    assert_eq!(
        overridden.target(),
        agent_directory
            .join("skills")
            .join("portable-demo")
            .as_path()
    );

    let repository = std::env::temp_dir().join("Pi Project With Spaces");
    let root = ApprovedRoot::from_absolute_path(repository.clone()).unwrap();
    let project = default_adapter
        .plan_artifact(&validated, files.clone(), Some(&root), Scope::Project)
        .unwrap();
    assert_eq!(
        project.target(),
        repository
            .join(".pi")
            .join("skills")
            .join("portable-demo")
            .as_path()
    );
    assert_eq!(project.files(), &files);
}

#[test]
fn pi_rejects_empty_or_relative_agent_directory_override() {
    let registry = AgentRegistry::built_in().unwrap();
    let profile = registry
        .get(jameskills_core::domain::agent::AgentId::Pi)
        .unwrap()
        .clone();
    let process = FakeProcess::new(vec![]);

    for override_dir in [PathBuf::new(), PathBuf::from("relative/pi-agent")] {
        assert!(
            PiAdapter::with_paths(
                profile.clone(),
                process.clone(),
                std::env::temp_dir().join("Pi Home"),
                Some(override_dir),
            )
            .is_err()
        );
    }
}

#[test]
fn pi_version_probe_uses_exact_release_output_and_approved_agent_identity() {
    let output = ProcessOutput::new(Some(0), b"1.0.4\n".to_vec(), Vec::new());
    let home = std::env::temp_dir().join("Pi Home");
    let (adapter, process) = adapter(vec![Ok(output)], home, None);
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
    assert_eq!(detection.version(), Some(&Version::parse("1.0.4").unwrap()));
    assert_eq!(
        detection
            .capabilities()
            .get(jameskills_core::domain::agent::AgentCapabilityId::UserInstall),
        CapabilitySupport::NeedsVerification
    );
    let calls = process.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert!(calls[0].identity_is_pi);
    assert_eq!(calls[0].args, [OsString::from("--version")]);
    assert!(calls[0].permission == ProcessPermission::ReadOnlyCheck);
    assert!(calls[0].fingerprint == Some(fingerprint));
}

#[test]
fn pi_version_output_from_other_cli_is_blocked_and_unapproved_candidate_is_not_run() {
    let home = std::env::temp_dir().join("Pi Home");
    let (candidate_adapter, process) = adapter(vec![], home.clone(), None);
    let candidate =
        block_on(candidate_adapter.detect(DetectionContext::new(Scope::User, None).unwrap()))
            .unwrap();
    assert!(matches!(
        candidate.availability(),
        AgentAvailability::Candidate | AgentAvailability::Missing
    ));
    assert!(process.calls.lock().unwrap().is_empty());

    for output_text in ["pi 1.0.4", "1.0.5"] {
        let output =
            ProcessOutput::new(Some(0), format!("{output_text}\n").into_bytes(), Vec::new());
        let (adapter, _) = adapter(vec![Ok(output)], home.clone(), None);
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
}
