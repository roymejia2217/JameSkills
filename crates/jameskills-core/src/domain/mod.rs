pub mod agent;
pub mod bundle;
pub mod guidance;
mod ids;
pub mod library;
pub mod policy;
pub mod repo_change;
mod scope;
pub mod skill;
pub use agent::{
    AgentCapabilities, AgentCapabilityAssessment, AgentCapabilityId, AgentId, AgentInstallMode,
    AgentProfile, CapabilityEvidence, CapabilitySupport,
};

pub use bundle::{
    BundleEntry, EntryKind, ValidatedFile, ValidatedInventory, validate_bundle_inventory,
};
pub use guidance::{
    GuidanceAction, GuidanceAnswer, GuidanceCondition, GuidanceDecision, GuidanceFactObservation,
    GuidanceFacts, GuidancePlan, GuidanceProgressStatus, GuidanceStep, GuidanceStepProgress,
    GuidanceStepStatus, OfficialGuidanceSource, ToolAvailability, ToolCapabilitySupport,
    ToolDetection, ToolEvidence, ToolVersionStatus, next_step,
};
pub use ids::{
    ContentHash, IdValidationError, OperationId, PathValidationError, PortablePath, RevisionId,
    SkillId,
};
pub use library::{
    RevisionKind, RevisionRecord, SaveRevisionRequest, SaveRevisionResult, compute_revision,
};
pub use policy::{
    Check, Enforcement, Phase, Policy, Requirement, Severity, ToolId, ToolOperation,
    ToolRequirement, parse_policy,
};
pub use repo_change::{ApprovedRepoChange, RepoChangePlan, RepoTemplateId};
pub use scope::Scope;
pub use skill::{
    CapabilityDeclaration, SkillFrontmatter, SkillManifest, ValidatedBundle, canonical_inventory,
    hash_bundle, validate_bundle,
};
