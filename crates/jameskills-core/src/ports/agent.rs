use crate::{
    AppResult, Diagnostic,
    domain::Scope,
    domain::agent::{AgentCapabilities, AgentId, AgentProfile, CapabilityEvidence},
    ports::process::{ApprovedExecutable, ApprovedRoot, ExecutableFingerprint},
};
use semver::Version;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentAvailability {
    Missing,
    Candidate,
    Verified,
    Blocked,
}

pub struct ApprovedAgentExecutable {
    executable: ApprovedExecutable,
    fingerprint: ExecutableFingerprint,
}

impl ApprovedAgentExecutable {
    pub fn after_explicit_fingerprint_confirmation(
        executable: ApprovedExecutable,
        observed_fingerprint: ExecutableFingerprint,
        confirmed_fingerprint: &ExecutableFingerprint,
    ) -> Result<Self, Vec<Diagnostic>> {
        if &observed_fingerprint != confirmed_fingerprint {
            return Err(agent_port_diagnostic(
                "agent.executable.confirmation.mismatch",
                "Agent executable fingerprint confirmation did not match the observed executable.",
            ));
        }
        Ok(Self {
            executable,
            fingerprint: observed_fingerprint,
        })
    }

    pub fn executable(&self) -> &ApprovedExecutable {
        &self.executable
    }

    pub fn fingerprint(&self) -> ExecutableFingerprint {
        self.fingerprint
    }
}

pub struct DetectionContext {
    scope: Scope,
    project_root: Option<ApprovedRoot>,
    approved_executable: Option<ApprovedAgentExecutable>,
}

impl DetectionContext {
    pub fn new(scope: Scope, project_root: Option<ApprovedRoot>) -> Result<Self, Vec<Diagnostic>> {
        if scope == Scope::Project && project_root.is_none() {
            return Err(agent_port_diagnostic(
                "agent.detection.project-root.required",
                "Project-scoped detection requires an approved repository root.",
            ));
        }
        Ok(Self {
            scope,
            project_root,
            approved_executable: None,
        })
    }

    pub fn with_approved_executable(mut self, executable: ApprovedAgentExecutable) -> Self {
        self.approved_executable = Some(executable);
        self
    }

    pub fn scope(&self) -> Scope {
        self.scope
    }

    pub fn project_root(&self) -> Option<&ApprovedRoot> {
        self.project_root.as_ref()
    }

    pub fn approved_executable(&self) -> Option<&ApprovedAgentExecutable> {
        self.approved_executable.as_ref()
    }
}

pub struct AgentDetection {
    id: AgentId,
    executable: Option<ApprovedExecutable>,
    executable_fingerprint: Option<ExecutableFingerprint>,
    version: Option<Version>,
    profile_root: Option<PathBuf>,
    availability: AgentAvailability,
    capabilities: AgentCapabilities,
    evidence: Vec<CapabilityEvidence>,
}

impl AgentDetection {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: AgentId,
        executable: Option<ApprovedExecutable>,
        executable_fingerprint: Option<ExecutableFingerprint>,
        version: Option<Version>,
        profile_root: Option<PathBuf>,
        availability: AgentAvailability,
        capabilities: AgentCapabilities,
        evidence: Vec<CapabilityEvidence>,
    ) -> Result<Self, Vec<Diagnostic>> {
        let availability_is_consistent = match availability {
            AgentAvailability::Missing | AgentAvailability::Candidate => {
                executable.is_none() && executable_fingerprint.is_none() && version.is_none()
            }
            AgentAvailability::Verified => {
                executable.is_some() && executable_fingerprint.is_some() && version.is_some()
            }
            AgentAvailability::Blocked => true,
        };
        if !availability_is_consistent
            || profile_root
                .as_deref()
                .is_some_and(|path| !path.is_absolute())
        {
            return Err(agent_port_diagnostic(
                "agent.detection.invalid",
                "Agent detection identity, availability, or profile root is inconsistent.",
            ));
        }
        Ok(Self {
            id,
            executable,
            executable_fingerprint,
            version,
            profile_root,
            availability,
            capabilities,
            evidence,
        })
    }

    pub fn id(&self) -> AgentId {
        self.id
    }

    pub fn executable(&self) -> Option<&ApprovedExecutable> {
        self.executable.as_ref()
    }

    pub fn executable_fingerprint(&self) -> Option<ExecutableFingerprint> {
        self.executable_fingerprint
    }

    pub fn version(&self) -> Option<&Version> {
        self.version.as_ref()
    }

    pub fn profile_root(&self) -> Option<&Path> {
        self.profile_root.as_deref()
    }

    pub fn availability(&self) -> AgentAvailability {
        self.availability
    }

    pub fn capabilities(&self) -> &AgentCapabilities {
        &self.capabilities
    }

    pub fn evidence(&self) -> &[CapabilityEvidence] {
        &self.evidence
    }
}

/// Port implemented by each registered agent adapter. Implementations expose
/// only app-owned profiles; imported skill metadata cannot add agents or probes.
#[async_trait::async_trait]
pub trait AgentPort: Send + Sync {
    fn profile(&self) -> &AgentProfile;

    async fn detect(&self, context: DetectionContext) -> AppResult<AgentDetection>;
}

fn agent_port_diagnostic(code: &'static str, message: &'static str) -> Vec<Diagnostic> {
    vec![Diagnostic::error(code, message)]
}
