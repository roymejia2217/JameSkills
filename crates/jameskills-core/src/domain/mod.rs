pub mod agent;
pub mod assets;
pub mod bundle;
pub mod guidance;
mod ids;
pub mod import;
pub mod library;
pub mod policy;
pub mod repo_change;
mod scope;
pub mod skill;
pub mod sync;

pub use agent::{
    AgentCapabilities, AgentCapabilityAssessment, AgentCapabilityId, AgentId, AgentInstallMode,
    AgentProfile, CapabilityEvidence, CapabilitySupport,
};
pub use assets::{
    AssetFiles, AssetPreview, AssetPreviewKind, add_asset, preview_asset, remove_asset,
    rename_bundle_path, replace_asset,
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
pub use import::{
    ImportClassification, ImportFiles, ImportPreview, ImportResolution, ImportResult,
    ImportScanStatus, ImportSourceKind, TrustState,
};
pub use library::{
    CreateSkill, DeleteRequest, ForkRequest, MAX_BINDING_REPORT_CHECKS,
    MAX_REPOSITORY_BINDINGS_PER_SKILL, RepositoryBinding, RepositoryBindingCheck,
    RepositoryBindingEvidence, RepositoryBindingReport, RepositoryProfile, RestoreRevisionRequest,
    RevisionKind, RevisionRecord, RevisionTrust, SaveRevisionRequest, SaveRevisionResult,
    SkillDraft, compute_revision,
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
pub use sync::{
    BACKUP_AEAD_TAG_LEN, BACKUP_ARGON2_ITERATIONS, BACKUP_ARGON2_MEMORY_KIB,
    BACKUP_ARGON2_PARALLELISM, BACKUP_CIPHER_XCHACHA20_POLY1305, BACKUP_HEADER_V1_LEN,
    BACKUP_HEADER_V1_VERSION, BACKUP_KDF_ARGON2ID, BACKUP_NONCE_LEN, BACKUP_SALT_LEN,
    BACKUP_WRAPPED_MASTER_LEN, BackupHeaderV1, CANONICAL_HASH_VERSION, DeviceId, EncryptedSnapshot,
    MAX_ENCRYPTED_PAYLOAD_BYTES, MAX_SNAPSHOT_ARCHIVES_BYTES, MAX_SNAPSHOT_BUNDLE_BYTES,
    MAX_SNAPSHOT_METADATA_ENTRIES, MAX_SNAPSHOT_PARENTS, SNAPSHOT_SCHEMA_VERSION, SnapshotBundle,
    SnapshotHead, SnapshotId, SnapshotPayload, SnapshotRevision, SnapshotRevisionKind,
    UnlockedVault, VaultId, VerifiedSnapshot, WrappingKey,
};
