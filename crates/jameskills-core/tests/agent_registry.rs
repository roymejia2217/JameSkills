use jameskills_core::{
    domain::Scope,
    domain::agent::{
        AgentCapabilities, AgentCapabilityAssessment, AgentCapabilityId, AgentId, AgentInstallMode,
        AgentProfile, CapabilityEvidence, CapabilitySupport,
    },
    ports::agent::{AgentAvailability, AgentDetection, ApprovedAgentExecutable, DetectionContext},
    ports::process::{ApprovedExecutable, ApprovedRoot, ExecutableFingerprint},
};
use semver::Version;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct AgentWire {
    agent: AgentId,
}

fn assessment(support: CapabilitySupport) -> AgentCapabilityAssessment {
    let evidence =
        CapabilityEvidence::new("agent-profile-source", "2026-10-06T12:00:00Z", None, None)
            .unwrap();
    AgentCapabilityAssessment::new(support, Some(evidence)).unwrap()
}

fn capabilities(
    user: CapabilitySupport,
    project: CapabilitySupport,
    plugin: CapabilitySupport,
) -> AgentCapabilities {
    AgentCapabilities::new(vec![
        (AgentCapabilityId::UserInstall, assessment(user)),
        (AgentCapabilityId::ProjectInstall, assessment(project)),
        (
            AgentCapabilityId::DiscoveryVerification,
            assessment(CapabilitySupport::NeedsVerification),
        ),
        (AgentCapabilityId::VendorPluginInstall, assessment(plugin)),
        (
            AgentCapabilityId::OpenAiMetadata,
            assessment(CapabilitySupport::Unsupported),
        ),
    ])
    .unwrap()
}

#[test]
fn agent_ids_are_a_closed_stable_registry() {
    let agents = [
        (AgentId::Codex, "codex"),
        (AgentId::OpenCode, "opencode"),
        (AgentId::Pi, "pi"),
        (AgentId::Antigravity, "antigravity"),
        (AgentId::Grok, "grok"),
    ];

    for (agent, id) in agents {
        assert_eq!(agent.as_str(), id);
        assert_eq!(AgentId::parse(id), Some(agent));
        let serialized = toml::to_string(&AgentWire { agent }).unwrap();
        assert!(serialized.contains(&format!("agent = \"{id}\"")));
        assert_eq!(
            toml::from_str::<AgentWire>(&serialized).unwrap().agent,
            agent
        );
    }
    assert_eq!(AgentId::parse("codex.exe"), None);
    assert_eq!(AgentId::parse("unknown"), None);
    assert!(toml::from_str::<AgentWire>("agent = \"unknown\"").is_err());
}

#[test]
fn capability_states_remain_independent_and_project_support_is_explicit() {
    let profile = AgentProfile::new(
        AgentId::Antigravity,
        AgentInstallMode::VendorPlugin,
        capabilities(
            CapabilitySupport::NeedsVerification,
            CapabilitySupport::Unsupported,
            CapabilitySupport::NeedsVerification,
        ),
    )
    .unwrap();

    assert_eq!(profile.install_mode(), AgentInstallMode::VendorPlugin);
    assert_eq!(
        profile.capabilities().get(AgentCapabilityId::UserInstall),
        CapabilitySupport::NeedsVerification
    );
    assert_eq!(
        profile
            .capabilities()
            .get(AgentCapabilityId::ProjectInstall),
        CapabilitySupport::Unsupported
    );
    assert_eq!(
        profile
            .capabilities()
            .get(AgentCapabilityId::VendorPluginInstall),
        CapabilitySupport::NeedsVerification
    );
}

#[test]
fn file_copy_profile_uses_closed_executable_name_and_explicit_scopes() {
    let profile = AgentProfile::new(
        AgentId::Codex,
        AgentInstallMode::FileCopy,
        capabilities(
            CapabilitySupport::NeedsVerification,
            CapabilitySupport::Unsupported,
            CapabilitySupport::Unsupported,
        ),
    )
    .unwrap();

    assert_eq!(profile.id(), AgentId::Codex);
    assert_eq!(profile.cli_name(), "codex");
    assert_eq!(
        profile.supports_scope(Scope::User),
        CapabilitySupport::NeedsVerification
    );
    assert_eq!(
        profile.supports_scope(Scope::Project),
        CapabilitySupport::Unsupported
    );
}

#[test]
fn vendor_plugin_profiles_cannot_claim_project_install() {
    assert!(
        AgentProfile::new(
            AgentId::Antigravity,
            AgentInstallMode::VendorPlugin,
            capabilities(
                CapabilitySupport::NeedsVerification,
                CapabilitySupport::NeedsVerification,
                CapabilitySupport::NeedsVerification,
            ),
        )
        .is_err()
    );
    assert!(
        AgentProfile::new(
            AgentId::Antigravity,
            AgentInstallMode::FileCopy,
            capabilities(
                CapabilitySupport::NeedsVerification,
                CapabilitySupport::Unsupported,
                CapabilitySupport::Unsupported,
            ),
        )
        .is_err()
    );
    assert!(
        AgentProfile::new(
            AgentId::Codex,
            AgentInstallMode::VendorPlugin,
            capabilities(
                CapabilitySupport::NeedsVerification,
                CapabilitySupport::Unsupported,
                CapabilitySupport::NeedsVerification,
            ),
        )
        .is_err()
    );
}

#[test]
fn supported_capability_requires_tested_version_and_fixture_evidence() {
    assert!(AgentCapabilityAssessment::new(CapabilitySupport::Supported, None).is_err());
    assert!(AgentCapabilityAssessment::new(CapabilitySupport::NeedsVerification, None).is_err());
    assert!(AgentCapabilityAssessment::new(CapabilitySupport::Unsupported, None).is_err());
    let evidence = CapabilityEvidence::new(
        "codex-skills",
        "2026-10-06T12:00:00Z",
        Some(Version::parse("1.2.3").unwrap()),
        Some("codex-discovery-v1"),
    )
    .unwrap();
    assert!(AgentCapabilityAssessment::new(CapabilitySupport::Supported, Some(evidence)).is_ok());
    assert!(
        CapabilityEvidence::new(
            "../unregistered",
            "2026-10-06T12:00:00Z",
            Some(Version::parse("1.2.3").unwrap()),
            Some("fixture-1"),
        )
        .is_err()
    );
    assert!(
        CapabilityEvidence::new(
            "codex-skills",
            "2026-99-06T12:00:00Z",
            Some(Version::parse("1.2.3").unwrap()),
            Some("fixture-1"),
        )
        .is_err()
    );
}

#[test]
fn verified_detection_requires_approved_executable_fingerprint_and_version() {
    let missing = AgentDetection::new(
        AgentId::Codex,
        None,
        None,
        None,
        None,
        AgentAvailability::Verified,
        capabilities(
            CapabilitySupport::NeedsVerification,
            CapabilitySupport::NeedsVerification,
            CapabilitySupport::Unsupported,
        ),
        vec![],
    );
    assert!(missing.is_err());

    let executable =
        ApprovedExecutable::from_absolute_path(std::env::current_exe().unwrap()).unwrap();
    let verified = AgentDetection::new(
        AgentId::Codex,
        Some(executable),
        Some(ExecutableFingerprint::from_sha256([1; 32])),
        Some(Version::parse("1.2.3").unwrap()),
        None,
        AgentAvailability::Verified,
        capabilities(
            CapabilitySupport::NeedsVerification,
            CapabilitySupport::NeedsVerification,
            CapabilitySupport::Unsupported,
        ),
        vec![],
    )
    .unwrap();
    assert_eq!(verified.availability(), AgentAvailability::Verified);
}

#[test]
fn project_detection_requires_an_approved_root() {
    assert!(DetectionContext::new(Scope::Project, None).is_err());
    let root = ApprovedRoot::from_absolute_path(std::env::current_dir().unwrap()).unwrap();
    assert!(DetectionContext::new(Scope::Project, Some(root)).is_ok());
}

#[test]
fn agent_version_probe_requires_explicit_confirmation_of_executable_fingerprint() {
    let executable =
        ApprovedExecutable::from_absolute_path(std::env::current_exe().unwrap()).unwrap();
    let observed = ExecutableFingerprint::from_sha256([1; 32]);
    let different = ExecutableFingerprint::from_sha256([2; 32]);

    assert!(
        ApprovedAgentExecutable::after_explicit_fingerprint_confirmation(
            ApprovedExecutable::from_absolute_path(std::env::current_exe().unwrap()).unwrap(),
            observed,
            &different,
        )
        .is_err()
    );

    let approved = ApprovedAgentExecutable::after_explicit_fingerprint_confirmation(
        executable, observed, &observed,
    )
    .unwrap();
    let context = DetectionContext::new(Scope::User, None)
        .unwrap()
        .with_approved_executable(approved);
    assert!(context.approved_executable().unwrap().fingerprint() == observed);
}

#[test]
fn capability_matrix_rejects_missing_and_duplicate_entries() {
    let entries = || {
        [
            AgentCapabilityId::UserInstall,
            AgentCapabilityId::ProjectInstall,
            AgentCapabilityId::DiscoveryVerification,
            AgentCapabilityId::VendorPluginInstall,
            AgentCapabilityId::OpenAiMetadata,
        ]
        .into_iter()
        .map(|id| (id, assessment(CapabilitySupport::NeedsVerification)))
        .collect::<Vec<_>>()
    };
    let mut incomplete = entries();
    incomplete.pop();
    assert!(AgentCapabilities::new(incomplete).is_err());

    let mut duplicated = entries();
    duplicated.push(duplicated[0].clone());
    assert!(AgentCapabilities::new(duplicated).is_err());
}
