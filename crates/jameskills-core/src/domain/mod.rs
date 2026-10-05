pub mod bundle;
pub mod guidance;
mod ids;
pub mod library;
pub mod policy;
mod scope;
pub mod skill;

pub use bundle::{
    BundleEntry, EntryKind, ValidatedFile, ValidatedInventory, validate_bundle_inventory,
};
pub use guidance::{
    GuidanceAction, GuidanceCondition, GuidancePlan, GuidanceStep, OfficialGuidanceSource,
    ToolAvailability, ToolCapabilitySupport, ToolDetection, ToolEvidence, ToolVersionStatus,
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
pub use scope::Scope;
pub use skill::{
    CapabilityDeclaration, SkillFrontmatter, SkillManifest, ValidatedBundle, canonical_inventory,
    hash_bundle, validate_bundle,
};
