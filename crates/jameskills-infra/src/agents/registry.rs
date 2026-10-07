use jameskills_core::{
    Diagnostic,
    domain::agent::{
        AgentCapabilities, AgentCapabilityAssessment, AgentCapabilityId, AgentId, AgentInstallMode,
        AgentProfile, CapabilityEvidence, CapabilitySupport,
    },
};
use std::collections::BTreeMap;

const PROFILE_REVIEWED_AT: &str = "2026-10-06T00:00:00Z";

pub struct AgentRegistry {
    profiles: BTreeMap<AgentId, AgentProfile>,
}

impl AgentRegistry {
    pub fn built_in() -> Result<Self, Vec<Diagnostic>> {
        let profiles = AgentId::ALL
            .into_iter()
            .map(profile_for)
            .collect::<Result<Vec<_>, _>>()?;
        Self::new(profiles)
    }

    pub fn new(profiles: Vec<AgentProfile>) -> Result<Self, Vec<Diagnostic>> {
        let mut by_id = BTreeMap::new();
        for profile in profiles {
            let id = profile.id();
            if by_id.insert(id, profile).is_some() {
                return Err(agent_registry_diagnostic(
                    "agent.registry.duplicate",
                    "Agent profile IDs must be unique.",
                ));
            }
        }
        if AgentId::ALL.iter().any(|id| !by_id.contains_key(id))
            || by_id.len() != AgentId::ALL.len()
        {
            return Err(agent_registry_diagnostic(
                "agent.registry.incomplete",
                "The built-in registry must declare every closed agent profile.",
            ));
        }
        Ok(Self { profiles: by_id })
    }

    pub fn get(&self, id: AgentId) -> Option<&AgentProfile> {
        self.profiles.get(&id)
    }

    pub fn profiles(&self) -> impl Iterator<Item = &AgentProfile> {
        self.profiles.values()
    }
}

fn profile_for(id: AgentId) -> Result<AgentProfile, Vec<Diagnostic>> {
    let source_id = match id {
        AgentId::Codex => "codex-skills",
        AgentId::OpenCode => "opencode-skills",
        AgentId::Pi => "pi-skills",
        AgentId::Antigravity => "antigravity-cli-plugins",
        AgentId::Grok => "grok-cli-reference",
    };
    let antigravity = id == AgentId::Antigravity;
    let codex = id == AgentId::Codex;
    let capabilities = AgentCapabilities::new(vec![
        (
            AgentCapabilityId::UserInstall,
            assessment(CapabilitySupport::NeedsVerification, source_id)?,
        ),
        (
            AgentCapabilityId::ProjectInstall,
            assessment(
                if antigravity {
                    CapabilitySupport::Unsupported
                } else {
                    CapabilitySupport::NeedsVerification
                },
                source_id,
            )?,
        ),
        (
            AgentCapabilityId::DiscoveryVerification,
            assessment(CapabilitySupport::NeedsVerification, source_id)?,
        ),
        (
            AgentCapabilityId::VendorPluginInstall,
            assessment(
                if antigravity {
                    CapabilitySupport::NeedsVerification
                } else {
                    CapabilitySupport::Unsupported
                },
                source_id,
            )?,
        ),
        (
            AgentCapabilityId::OpenAiMetadata,
            assessment(
                if codex {
                    CapabilitySupport::NeedsVerification
                } else {
                    CapabilitySupport::Unsupported
                },
                source_id,
            )?,
        ),
    ])?;
    let install_mode = if antigravity {
        AgentInstallMode::VendorPlugin
    } else {
        AgentInstallMode::FileCopy
    };
    AgentProfile::new(id, install_mode, capabilities)
}

fn assessment(
    support: CapabilitySupport,
    source_id: &str,
) -> Result<AgentCapabilityAssessment, Vec<Diagnostic>> {
    let evidence = CapabilityEvidence::new(source_id, PROFILE_REVIEWED_AT, None, None)?;
    AgentCapabilityAssessment::new(support, Some(evidence))
}

fn agent_registry_diagnostic(code: &'static str, message: &'static str) -> Vec<Diagnostic> {
    vec![Diagnostic::error(code, message)]
}
