use super::import::{ImportSourceKind, TrustState};
use super::{
    ContentHash, PortablePath, RevisionId, SkillId, ValidatedBundle, policy::RepositoryHead,
};
use crate::{AppError, AppResult, Diagnostic};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

/// Domain tag opening every revision digest (SPEC-skill-format, canonical hashing).
const REVISION_TAG: &[u8] = b"JAMESKILLS-REVISION-V1\0";
/// Kind byte for a content revision; tombstones use [`KIND_TOMBSTONE`].
const KIND_CONTENT: u8 = 0x01;
/// Kind byte for a tombstone revision; the bundle slot digests as zeros.
const KIND_TOMBSTONE: u8 = 0x02;
/// Upper bound for a user-confirmed causal delete request.
pub const MAX_DELETE_EXPECTED_HEADS: usize = 128;
pub const MAX_REPOSITORY_BINDINGS_PER_SKILL: usize = 128;
pub const MAX_BINDING_REPORT_CHECKS: usize = 512;
const MAX_BINDING_REPORT_EVIDENCE: usize = 32;

/// Causal kind of a revision. Tombstones carry the heads they observed so a
/// delete never silently resurrects a concurrent content revision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RevisionKind {
    Content,
    Tombstone { observed_heads: Vec<RevisionId> },
}

/// Local trust/provenance attached to a committed revision. It is deliberately
/// excluded from the content hash and remote revision identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RevisionTrust {
    state: TrustState,
    source_kind: Option<ImportSourceKind>,
}

impl RevisionTrust {
    pub const fn reviewed() -> Self {
        Self {
            state: TrustState::Reviewed,
            source_kind: None,
        }
    }

    pub const fn quarantined(source_kind: ImportSourceKind) -> Self {
        Self {
            state: TrustState::Quarantined,
            source_kind: Some(source_kind),
        }
    }

    pub const fn state(self) -> TrustState {
        self.state
    }

    pub const fn source_kind(self) -> Option<ImportSourceKind> {
        self.source_kind
    }
}

/// Explicit soft-delete intent tied to the exact active heads the user saw.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeleteRequest {
    skill_id: SkillId,
    expected_heads: Vec<RevisionId>,
}

impl DeleteRequest {
    pub fn new(skill_id: SkillId, expected_heads: Vec<RevisionId>) -> AppResult<Self> {
        if expected_heads.is_empty() || expected_heads.len() > MAX_DELETE_EXPECTED_HEADS {
            return Err(AppError::Validation(vec![Diagnostic::error(
                "library.delete.expected_heads.invalid",
                "Delete requires a bounded, nonempty set of observed revision heads.",
            )]));
        }
        let expected_heads = sorted_unique(expected_heads);
        Ok(Self {
            skill_id,
            expected_heads,
        })
    }

    pub fn skill_id(&self) -> SkillId {
        self.skill_id
    }

    pub fn expected_heads(&self) -> &[RevisionId] {
        &self.expected_heads
    }
}

/// Explicit content rollback: copy an older content revision into a new
/// revision at `next_version`, causally descending every head the user saw.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RestoreRevisionRequest {
    skill_id: SkillId,
    source_revision_id: RevisionId,
    expected_heads: Vec<RevisionId>,
    next_version: String,
}

impl RestoreRevisionRequest {
    pub fn new(
        skill_id: SkillId,
        source_revision_id: RevisionId,
        expected_heads: Vec<RevisionId>,
        next_version: String,
    ) -> AppResult<Self> {
        if expected_heads.is_empty()
            || expected_heads.len() > MAX_DELETE_EXPECTED_HEADS
            || next_version.len() > 128
            || next_version.parse::<semver::Version>().is_err()
        {
            return Err(AppError::Validation(vec![Diagnostic::error(
                "library.restore.request.invalid",
                "Restore requires a valid version and a bounded, nonempty observed head set.",
            )]));
        }
        Ok(Self {
            skill_id,
            source_revision_id,
            expected_heads: sorted_unique(expected_heads),
            next_version,
        })
    }

    pub fn skill_id(&self) -> SkillId {
        self.skill_id
    }

    pub fn source_revision_id(&self) -> &RevisionId {
        &self.source_revision_id
    }

    pub fn expected_heads(&self) -> &[RevisionId] {
        &self.expected_heads
    }

    pub fn next_version(&self) -> &str {
        &self.next_version
    }
}

/// Clones one immutable content revision into a new local skill identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForkRequest {
    source_skill_id: SkillId,
    source_revision_id: RevisionId,
    new_slug: String,
    new_display_name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepositoryProfile {
    Rust,
    Node,
    Generic,
}

impl RepositoryProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Node => "node",
            Self::Generic => "generic",
        }
    }

    pub fn parse(value: &str) -> AppResult<Self> {
        match value {
            "rust" => Ok(Self::Rust),
            "node" => Ok(Self::Node),
            "generic" => Ok(Self::Generic),
            _ => Err(AppError::Validation(vec![Diagnostic::error(
                "library.binding.profile.invalid",
                "Repository profile is not registered.",
            )])),
        }
    }
}

/// Local-only link from one repository path to one exact suite revision and
/// profile. This type is never serialized into a portable bundle or backup.
#[derive(Clone, PartialEq, Eq)]
pub struct RepositoryBinding {
    id: String,
    skill_id: SkillId,
    suite_revision: RevisionId,
    repository_root: PathBuf,
    profile: RepositoryProfile,
    strict: bool,
    repository_head: RepositoryHead,
    environment_fingerprint: String,
}

impl RepositoryBinding {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        skill_id: SkillId,
        suite_revision: RevisionId,
        repository_root: PathBuf,
        profile: RepositoryProfile,
        strict: bool,
        repository_head: RepositoryHead,
        environment_fingerprint: String,
    ) -> AppResult<Self> {
        Self::from_storage(
            uuid::Uuid::new_v4().to_string(),
            skill_id,
            suite_revision,
            repository_root,
            profile,
            strict,
            repository_head,
            environment_fingerprint,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_storage(
        id: String,
        skill_id: SkillId,
        suite_revision: RevisionId,
        repository_root: PathBuf,
        profile: RepositoryProfile,
        strict: bool,
        repository_head: RepositoryHead,
        environment_fingerprint: String,
    ) -> AppResult<Self> {
        let path_is_valid = repository_root.is_absolute()
            && repository_root.as_os_str().len() <= 4096
            && repository_root.to_str().is_some()
            && !repository_root
                .components()
                .any(|component| matches!(component, Component::CurDir | Component::ParentDir));
        let fingerprint_is_valid =
            environment_fingerprint
                .strip_prefix("sha256:")
                .is_some_and(|digest| {
                    digest.len() == 64
                        && digest
                            .bytes()
                            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                });
        let parsed_id = uuid::Uuid::parse_str(&id).ok();
        if !parsed_id.is_some_and(|parsed| !parsed.is_nil() && parsed.to_string() == id)
            || !path_is_valid
            || !fingerprint_is_valid
        {
            return Err(AppError::Validation(vec![Diagnostic::error(
                "library.binding.invalid",
                "Repository binding identity is invalid or outside its bounds.",
            )]));
        }
        Ok(Self {
            id,
            skill_id,
            suite_revision,
            repository_root,
            profile,
            strict,
            repository_head,
            environment_fingerprint,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn skill_id(&self) -> SkillId {
        self.skill_id
    }

    pub fn suite_revision(&self) -> &RevisionId {
        &self.suite_revision
    }

    pub fn repository_root(&self) -> &Path {
        &self.repository_root
    }

    pub fn profile(&self) -> RepositoryProfile {
        self.profile
    }

    pub fn strict(&self) -> bool {
        self.strict
    }

    pub fn repository_head(&self) -> &RepositoryHead {
        &self.repository_head
    }

    pub fn environment_fingerprint(&self) -> &str {
        &self.environment_fingerprint
    }
}

/// Redacted result metadata retained locally so offline/moved bindings can
/// display the last run as stale instead of silently discarding it.
#[derive(Clone, PartialEq, Eq)]
pub struct RepositoryBindingReport {
    binding_id: String,
    skill_id: SkillId,
    suite_revision: RevisionId,
    repository_head: RepositoryHead,
    environment_fingerprint: String,
    profile: RepositoryProfile,
    strict: bool,
    observed_at: String,
    strict_passed: bool,
    checks: Vec<RepositoryBindingCheck>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepositoryBindingCheck {
    requirement_id: String,
    status: String,
    severity: String,
    enforcement: Option<String>,
    guidance_id: Option<String>,
    evidence: Vec<RepositoryBindingEvidence>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepositoryBindingEvidence {
    source_id: String,
    observed_at: String,
    revision: Option<RevisionId>,
    environment_fingerprint: String,
    summary: String,
}

impl RepositoryBindingReport {
    pub fn new(
        binding: &RepositoryBinding,
        observed_at: String,
        strict_passed: bool,
        checks: Vec<RepositoryBindingCheck>,
    ) -> AppResult<Self> {
        Self::from_storage(
            binding.id().to_owned(),
            binding.skill_id(),
            binding.suite_revision().clone(),
            binding.repository_head().clone(),
            binding.environment_fingerprint().to_owned(),
            binding.profile(),
            binding.strict(),
            observed_at,
            strict_passed,
            checks,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_storage(
        binding_id: String,
        skill_id: SkillId,
        suite_revision: RevisionId,
        repository_head: RepositoryHead,
        environment_fingerprint: String,
        profile: RepositoryProfile,
        strict: bool,
        observed_at: String,
        strict_passed: bool,
        checks: Vec<RepositoryBindingCheck>,
    ) -> AppResult<Self> {
        let parsed_id = uuid::Uuid::parse_str(&binding_id).ok();
        if !parsed_id.is_some_and(|parsed| !parsed.is_nil() && parsed.to_string() == binding_id)
            || !valid_binding_fingerprint(&environment_fingerprint)
            || observed_at.is_empty()
            || observed_at.len() > 40
            || observed_at.chars().any(char::is_control)
            || checks.len() > MAX_BINDING_REPORT_CHECKS
        {
            return Err(binding_report_error());
        }
        let mut requirement_ids = std::collections::BTreeSet::new();
        if checks
            .iter()
            .any(|check| !requirement_ids.insert(check.requirement_id.as_str()))
        {
            return Err(binding_report_error());
        }
        Ok(Self {
            binding_id,
            skill_id,
            suite_revision,
            repository_head,
            environment_fingerprint,
            profile,
            strict,
            observed_at,
            strict_passed,
            checks,
        })
    }

    pub fn is_stale_for(&self, binding: &RepositoryBinding) -> bool {
        self.binding_id != binding.id()
            || self.skill_id != binding.skill_id()
            || self.suite_revision != *binding.suite_revision()
            || self.repository_head != *binding.repository_head()
            || self.environment_fingerprint != binding.environment_fingerprint()
            || self.profile != binding.profile()
            || self.strict != binding.strict()
    }

    pub fn binding_id(&self) -> &str {
        &self.binding_id
    }

    pub fn skill_id(&self) -> SkillId {
        self.skill_id
    }

    pub fn suite_revision(&self) -> &RevisionId {
        &self.suite_revision
    }

    pub fn repository_head(&self) -> &RepositoryHead {
        &self.repository_head
    }

    pub fn environment_fingerprint(&self) -> &str {
        &self.environment_fingerprint
    }

    pub fn profile(&self) -> RepositoryProfile {
        self.profile
    }

    pub fn strict(&self) -> bool {
        self.strict
    }

    pub fn observed_at(&self) -> &str {
        &self.observed_at
    }

    pub fn strict_passed(&self) -> bool {
        self.strict_passed
    }

    pub fn checks(&self) -> &[RepositoryBindingCheck] {
        &self.checks
    }
}

impl RepositoryBindingCheck {
    pub fn new(
        requirement_id: String,
        status: String,
        severity: String,
        enforcement: Option<String>,
        guidance_id: Option<String>,
        evidence: Vec<RepositoryBindingEvidence>,
    ) -> AppResult<Self> {
        if requirement_id.is_empty()
            || requirement_id.len() > 128
            || requirement_id.chars().any(char::is_control)
            || !matches!(
                status.as_str(),
                "pass" | "fail" | "blocked" | "unknown" | "unsupported" | "not-applicable"
            )
            || !matches!(severity.as_str(), "info" | "warning" | "error")
            || enforcement.as_deref().is_some_and(|value| {
                !matches!(
                    value,
                    "instruction" | "local-check" | "local-hook" | "required-ci" | "host-rule"
                )
            })
            || guidance_id
                .as_deref()
                .is_some_and(|value| value.is_empty() || value.len() > 128)
            || evidence.len() > MAX_BINDING_REPORT_EVIDENCE
        {
            return Err(binding_report_error());
        }
        Ok(Self {
            requirement_id,
            status,
            severity,
            enforcement,
            guidance_id,
            evidence,
        })
    }

    pub fn requirement_id(&self) -> &str {
        &self.requirement_id
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn severity(&self) -> &str {
        &self.severity
    }

    pub fn enforcement(&self) -> Option<&str> {
        self.enforcement.as_deref()
    }

    pub fn guidance_id(&self) -> Option<&str> {
        self.guidance_id.as_deref()
    }

    pub fn evidence(&self) -> &[RepositoryBindingEvidence] {
        &self.evidence
    }
}

impl RepositoryBindingEvidence {
    pub fn new(
        source_id: String,
        observed_at: String,
        revision: Option<RevisionId>,
        environment_fingerprint: String,
        summary: String,
    ) -> AppResult<Self> {
        if source_id.is_empty()
            || source_id.len() > 128
            || source_id.chars().any(char::is_control)
            || observed_at.is_empty()
            || observed_at.len() > 40
            || observed_at.chars().any(char::is_control)
            || !valid_binding_fingerprint(&environment_fingerprint)
            || summary.is_empty()
            || summary.len() > 256
            || summary.chars().any(char::is_control)
        {
            return Err(binding_report_error());
        }
        Ok(Self {
            source_id,
            observed_at,
            revision,
            environment_fingerprint,
            summary,
        })
    }

    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    pub fn observed_at(&self) -> &str {
        &self.observed_at
    }

    pub fn revision(&self) -> Option<&RevisionId> {
        self.revision.as_ref()
    }

    pub fn environment_fingerprint(&self) -> &str {
        &self.environment_fingerprint
    }

    pub fn summary(&self) -> &str {
        &self.summary
    }
}

fn valid_binding_fingerprint(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn binding_report_error() -> AppError {
    AppError::Validation(vec![Diagnostic::error(
        "library.binding.report.invalid",
        "Binding result is invalid or exceeds its local retention limits.",
    )])
}

impl ForkRequest {
    pub fn new(
        source_skill_id: SkillId,
        source_revision_id: RevisionId,
        new_slug: String,
        new_display_name: String,
    ) -> AppResult<Self> {
        let portable = PortablePath::new(new_slug.clone()).map_err(|_| {
            AppError::Validation(vec![Diagnostic::error(
                "library.fork.slug.invalid",
                "Fork slug must be one portable path component.",
            )])
        })?;
        if portable.as_str().contains('/')
            || new_display_name.trim() != new_display_name
            || new_display_name.is_empty()
            || new_display_name.len() > 128
            || new_display_name.chars().any(char::is_control)
        {
            return Err(AppError::Validation(vec![Diagnostic::error(
                "library.fork.metadata.invalid",
                "Fork metadata is invalid.",
            )]));
        }
        Ok(Self {
            source_skill_id,
            source_revision_id,
            new_slug,
            new_display_name,
        })
    }

    pub fn source_skill_id(&self) -> SkillId {
        self.source_skill_id
    }

    pub fn source_revision_id(&self) -> &RevisionId {
        &self.source_revision_id
    }

    pub fn new_slug(&self) -> &str {
        &self.new_slug
    }

    pub fn new_display_name(&self) -> &str {
        &self.new_display_name
    }
}

/// Immutable revision of one skill: content hash plus causal parents.
/// Timestamps never participate; see [`compute_revision`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevisionRecord {
    id: RevisionId,
    skill_id: SkillId,
    bundle_hash: ContentHash,
    parents: Vec<RevisionId>,
    kind: RevisionKind,
    semantic_version: String,
}

impl RevisionRecord {
    /// Builds a record, sorting and deduplicating parents; tombstones store a
    /// zero bundle hash and sorted observed heads. The caller resolves which
    /// version a tombstone carries (last content version or `"0.0.0"`).
    pub fn new(
        skill_id: SkillId,
        bundle_hash: Option<ContentHash>,
        parents: Vec<RevisionId>,
        kind: RevisionKind,
        semantic_version: String,
    ) -> AppResult<Self> {
        let parents = sorted_unique(parents);
        let kind = match kind {
            RevisionKind::Content => RevisionKind::Content,
            RevisionKind::Tombstone { observed_heads } => RevisionKind::Tombstone {
                observed_heads: sorted_unique(observed_heads),
            },
        };
        let id = compute_revision(
            skill_id,
            &kind,
            bundle_hash.as_ref(),
            &semantic_version,
            &parents,
        )?;
        Ok(Self {
            id,
            skill_id,
            bundle_hash: bundle_hash.unwrap_or_else(zero_hash),
            parents,
            kind,
            semantic_version,
        })
    }

    pub fn id(&self) -> &RevisionId {
        &self.id
    }

    pub fn skill_id(&self) -> SkillId {
        self.skill_id
    }

    pub fn bundle_hash(&self) -> &ContentHash {
        &self.bundle_hash
    }

    pub fn parents(&self) -> &[RevisionId] {
        &self.parents
    }

    pub fn kind(&self) -> &RevisionKind {
        &self.kind
    }

    pub fn semantic_version(&self) -> &str {
        &self.semantic_version
    }
}

/// Computes the causal revision id over the exact SPEC-skill-format bytes:
/// tag, skill UUID16, kind byte, bundle hash32 (zeros for tombstones),
/// length-prefixed version, sorted unique parent hashes with count, then
/// sorted observed heads with count for tombstones. Ancestry across skills is
/// a commit-time check; every parent is hashed, never trusted by position.
pub fn compute_revision(
    skill_id: SkillId,
    kind: &RevisionKind,
    bundle_hash: Option<&ContentHash>,
    semantic_version: &str,
    parents: &[RevisionId],
) -> AppResult<RevisionId> {
    parse_version(semantic_version)?;
    let mut digest = Sha256::new();
    digest.update(REVISION_TAG);
    digest.update(skill_id.as_uuid().into_bytes());
    match kind {
        RevisionKind::Content => {
            digest.update([KIND_CONTENT]);
            let bundle = bundle_hash.ok_or(AppError::CryptoInvalid)?;
            digest.update(decode_digest(bundle.as_str())?);
        }
        RevisionKind::Tombstone { .. } => {
            digest.update([KIND_TOMBSTONE]);
            digest.update([0u8; 32]);
        }
    }
    digest.update(count_bytes(semantic_version.len())?);
    digest.update(semantic_version.as_bytes());
    let parents = sorted_unique(parents.to_vec());
    digest.update(count_bytes(parents.len())?);
    for parent in &parents {
        digest.update(decode_digest(parent.as_str())?);
    }
    if let RevisionKind::Tombstone { observed_heads } = kind {
        let heads = sorted_unique(observed_heads.clone());
        digest.update(count_bytes(heads.len())?);
        for head in &heads {
            digest.update(decode_digest(head.as_str())?);
        }
    }
    Ok(RevisionId::from_digest(digest.finalize().into()))
}

fn parse_version(semantic_version: &str) -> AppResult<semver::Version> {
    semantic_version.parse::<semver::Version>().map_err(|_| {
        AppError::Validation(vec![Diagnostic::error(
            "revision.version.invalid",
            "revision version is not valid semver",
        )])
    })
}

fn count_bytes(len: usize) -> AppResult<[u8; 4]> {
    u32::try_from(len)
        .map(|count| count.to_be_bytes())
        .map_err(|_| AppError::CryptoInvalid)
}

fn sorted_unique(mut ids: Vec<RevisionId>) -> Vec<RevisionId> {
    ids.sort();
    ids.dedup();
    ids
}

#[cfg(test)]
mod tests {
    use super::{DeleteRequest, ForkRequest, MAX_DELETE_EXPECTED_HEADS, RestoreRevisionRequest};
    use crate::domain::{RevisionId, SkillId};

    #[test]
    fn delete_request_canonicalizes_the_observed_heads() {
        let first = RevisionId::from_digest([1; 32]);
        let second = RevisionId::from_digest([2; 32]);
        let request = DeleteRequest::new(
            SkillId::new(),
            vec![second.clone(), first.clone(), second.clone()],
        )
        .unwrap();

        assert_eq!(request.expected_heads(), &[first.clone(), second]);
    }

    #[test]
    fn delete_request_rejects_empty_or_unbounded_observations() {
        let skill_id = SkillId::new();
        assert!(DeleteRequest::new(skill_id, Vec::new()).is_err());
        let repeated = vec![RevisionId::from_digest([1; 32]); MAX_DELETE_EXPECTED_HEADS + 1];
        assert!(DeleteRequest::new(skill_id, repeated).is_err());
        let heads = (0..=MAX_DELETE_EXPECTED_HEADS)
            .map(|index| {
                let mut bytes = [0; 32];
                bytes[..8].copy_from_slice(&(index as u64).to_be_bytes());
                RevisionId::from_digest(bytes)
            })
            .collect();
        assert!(DeleteRequest::new(skill_id, heads).is_err());
    }

    #[test]
    fn restore_request_canonicalizes_heads_and_requires_valid_version() {
        let skill_id = SkillId::new();
        let source = RevisionId::from_digest([9; 32]);
        let first = RevisionId::from_digest([1; 32]);
        let second = RevisionId::from_digest([2; 32]);
        let request = RestoreRevisionRequest::new(
            skill_id,
            source.clone(),
            vec![second.clone(), first.clone(), second.clone()],
            "2.0.0".to_owned(),
        )
        .unwrap();
        assert_eq!(request.skill_id(), skill_id);
        assert_eq!(request.source_revision_id(), &source);
        assert_eq!(request.expected_heads(), &[first.clone(), second]);
        assert_eq!(request.next_version(), "2.0.0");
        assert!(
            RestoreRevisionRequest::new(skill_id, source.clone(), vec![], "2.0.0".to_owned())
                .is_err()
        );
        assert!(
            RestoreRevisionRequest::new(skill_id, source, vec![first], "not-semver".to_owned())
                .is_err()
        );
    }

    #[test]
    fn fork_request_accepts_only_a_portable_slug_and_bounded_display_name() {
        let skill_id = SkillId::new();
        let revision_id = RevisionId::from_digest([7; 32]);
        let request = ForkRequest::new(
            skill_id,
            revision_id.clone(),
            "portable-fork".to_owned(),
            "Portable Fork".to_owned(),
        )
        .unwrap();
        assert_eq!(request.source_skill_id(), skill_id);
        assert_eq!(request.source_revision_id(), &revision_id);
        assert_eq!(request.new_slug(), "portable-fork");
        assert_eq!(request.new_display_name(), "Portable Fork");
        assert!(
            ForkRequest::new(
                skill_id,
                revision_id.clone(),
                "nested/fork".to_owned(),
                "Portable Fork".to_owned(),
            )
            .is_err()
        );
        assert!(
            ForkRequest::new(
                skill_id,
                revision_id,
                "portable-fork".to_owned(),
                " trailing".to_owned(),
            )
            .is_err()
        );
    }
}

fn zero_hash() -> ContentHash {
    ContentHash::from_digest([0u8; 32])
}

/// Decodes a validated 64-char lowercase hex digest. Construction-time
/// parsing guarantees the shape; anything else is corrupt crypto input.
fn decode_digest(hex: &str) -> AppResult<[u8; 32]> {
    let bytes = hex.as_bytes();
    if bytes.len() != 64 {
        return Err(AppError::CryptoInvalid);
    }
    let mut out = [0u8; 32];
    for (index, chunk) in bytes.chunks(2).enumerate() {
        let pair = std::str::from_utf8(chunk).map_err(|_| AppError::CryptoInvalid)?;
        out[index] = u8::from_str_radix(pair, 16).map_err(|_| AppError::CryptoInvalid)?;
    }
    Ok(out)
}

/// Request to persist one revision. The skill row must already exist;
/// creating skills belongs to authoring, so a missing skill fails instead
/// of inventing metadata. Tombstones carry no bundle hash.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveRevisionRequest {
    skill_id: SkillId,
    bundle_hash: Option<ContentHash>,
    parents: Vec<RevisionId>,
    kind: RevisionKind,
    semantic_version: String,
    schema_version: u32,
    expected_heads: Vec<RevisionId>,
    catalog_tags: Vec<String>,
    catalog_capabilities: Vec<String>,
    expected_draft_generation: Option<u64>,
    trust: RevisionTrust,
}

impl SaveRevisionRequest {
    pub fn new(
        skill_id: SkillId,
        bundle_hash: Option<ContentHash>,
        parents: Vec<RevisionId>,
        kind: RevisionKind,
        semantic_version: String,
        schema_version: u32,
        expected_heads: Vec<RevisionId>,
    ) -> Self {
        Self {
            skill_id,
            bundle_hash,
            parents,
            kind,
            semantic_version,
            schema_version,
            expected_heads,
            catalog_tags: Vec::new(),
            catalog_capabilities: Vec::new(),
            expected_draft_generation: None,
            trust: RevisionTrust::reviewed(),
        }
    }

    pub fn with_trust(mut self, trust: RevisionTrust) -> AppResult<Self> {
        if trust.state() == TrustState::Quarantined && !matches!(&self.kind, RevisionKind::Content)
        {
            return Err(AppError::Validation(vec![Diagnostic::error(
                "revision.trust.tombstone.invalid",
                "A causal tombstone cannot carry imported content quarantine state.",
            )]));
        }
        self.trust = trust;
        Ok(self)
    }

    pub fn with_expected_draft_generation(mut self, generation: u64) -> AppResult<Self> {
        if generation == 0 || generation > i64::MAX as u64 {
            return Err(AppError::Validation(vec![Diagnostic::error(
                "revision.draft_generation.invalid",
                "Publish request must target a valid saved draft generation.",
            )]));
        }
        self.expected_draft_generation = Some(generation);
        Ok(self)
    }

    /// Binds catalog filters to metadata from the same validated bundle bytes
    /// that this content revision publishes.
    pub fn with_validated_bundle(mut self, bundle: &ValidatedBundle) -> AppResult<Self> {
        let manifest = bundle.manifest();
        if !matches!(&self.kind, RevisionKind::Content)
            || self.skill_id != manifest.id()
            || self.bundle_hash.as_ref() != Some(bundle.content_hash())
            || self.semantic_version != manifest.version().to_string()
            || self.schema_version != manifest.schema_version()
        {
            return Err(AppError::Validation(vec![Diagnostic::error(
                "revision.bundle.metadata_mismatch",
                "Catalog metadata must come from the exact validated content revision.",
            )]));
        }
        let normalize_values = |values: &[String]| {
            let mut normalized = values
                .iter()
                .map(|value| value.trim().chars().flat_map(char::to_lowercase).collect())
                .collect::<Vec<String>>();
            normalized.sort();
            normalized.dedup();
            normalized
        };
        self.catalog_tags = normalize_values(manifest.tags());
        self.catalog_capabilities = normalize_values(
            &manifest
                .capabilities()
                .iter()
                .map(|capability| capability.id().to_owned())
                .collect::<Vec<_>>(),
        );
        Ok(self)
    }

    pub fn skill_id(&self) -> SkillId {
        self.skill_id
    }

    pub fn bundle_hash(&self) -> Option<&ContentHash> {
        self.bundle_hash.as_ref()
    }

    pub fn parents(&self) -> &[RevisionId] {
        &self.parents
    }

    pub fn kind(&self) -> &RevisionKind {
        &self.kind
    }

    pub fn semantic_version(&self) -> &str {
        &self.semantic_version
    }

    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn expected_heads(&self) -> &[RevisionId] {
        &self.expected_heads
    }

    pub fn catalog_tags(&self) -> &[String] {
        &self.catalog_tags
    }

    pub fn catalog_capabilities(&self) -> &[String] {
        &self.catalog_capabilities
    }

    pub fn expected_draft_generation(&self) -> Option<u64> {
        self.expected_draft_generation
    }

    pub fn trust(&self) -> RevisionTrust {
        self.trust
    }
}

const MAX_DRAFT_FILE_COUNT: usize = 2_000;
const MAX_DRAFT_BYTES: usize = 20 * 1024 * 1024;
const MAX_DRAFT_GENERATION: u64 = i64::MAX as u64;

/// Editable local bytes are retained even when bundle semantics are invalid;
/// only portable file paths and hard resource bounds are required for a draft.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkillDraft {
    skill_id: SkillId,
    base_head: Option<RevisionId>,
    generation: u64,
    files: BTreeMap<PortablePath, Vec<u8>>,
    trust_state: TrustState,
}

/// Application-owned starting metadata for a new editable skill. Construction
/// generates identity once; subsequent edits belong to its draft, not a new ID.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateSkill {
    skill_id: SkillId,
    slug: String,
    display_name: String,
    description: String,
}

impl CreateSkill {
    pub fn new(slug: String, display_name: String) -> AppResult<Self> {
        Self::new_with_description(slug, display_name, "A new skill suite.".to_owned())
    }

    pub fn new_with_description(
        slug: String,
        display_name: String,
        description: String,
    ) -> AppResult<Self> {
        let portable = PortablePath::new(slug.clone()).map_err(|_| {
            AppError::Validation(vec![Diagnostic::error(
                "library.create.slug.invalid",
                "Skill slug must be one portable path component.",
            )])
        })?;
        if portable.as_str().contains('/')
            || display_name.trim() != display_name
            || display_name.is_empty()
            || display_name.len() > 128
            || display_name.chars().any(char::is_control)
            || description.trim() != description
            || description.is_empty()
            || description.len() > 1024
            || description.chars().any(char::is_control)
        {
            return Err(AppError::Validation(vec![Diagnostic::error(
                "library.create.metadata.invalid",
                "New skill metadata is invalid.",
            )]));
        }
        Ok(Self {
            skill_id: SkillId::new(),
            slug,
            display_name,
            description,
        })
    }

    pub fn skill_id(&self) -> SkillId {
        self.skill_id
    }
    pub fn slug(&self) -> &str {
        &self.slug
    }
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    /// Creates a valid editable template; the user may subsequently make and
    /// save semantically invalid edits without losing their draft.
    pub fn initial_draft(&self) -> AppResult<SkillDraft> {
        let quoted_slug = quote_toml_string(&self.slug);
        let quoted_description = quote_toml_string(&self.description);
        let skill = format!(
            "---\nname: {quoted_slug}\ndescription: {quoted_description}\n---\n# {}\n\nDescribe this skill and its intended use.\n",
            self.display_name,
        );
        self.build_draft(skill.into_bytes(), TrustState::Reviewed)
    }

    /// Turns a standard SKILL.md into an editable, quarantined instructions-only
    /// draft. The instruction bytes are preserved exactly and never executed.
    pub fn from_plain_skill_source(source: Vec<u8>) -> AppResult<(Self, SkillDraft)> {
        Self::from_plain_skill_source_with_id(source, SkillId::new())
    }

    pub(crate) fn from_plain_skill_source_with_id(
        source: Vec<u8>,
        skill_id: SkillId,
    ) -> AppResult<(Self, SkillDraft)> {
        let frontmatter = super::skill::parse_frontmatter(&source).map_err(AppError::Validation)?;
        let slug = frontmatter.name().to_owned();
        let mut create =
            Self::new_with_description(slug.clone(), slug, frontmatter.description().to_owned())?;
        create.skill_id = skill_id;
        let draft = create.build_draft(source, TrustState::Quarantined)?;
        Ok((create, draft))
    }

    fn build_draft(&self, skill: Vec<u8>, trust_state: TrustState) -> AppResult<SkillDraft> {
        let quoted_slug = quote_toml_string(&self.slug);
        let quoted_name = quote_toml_string(&self.display_name);
        let quoted_description = quote_toml_string(&self.description);
        let manifest = format!(
            "schema_version = 1\nid = \"{}\"\nslug = {quoted_slug}\ndisplay_name = {quoted_name}\nversion = \"0.1.0\"\ndescription = {quoted_description}\nlicense = \"UNLICENSED\"\nminimum_app_version = \"0.1.0\"\n",
            self.skill_id.as_uuid()
        );
        let files = [
            (
                PortablePath::new("SKILL.md".to_owned())
                    .map_err(|_| draft_error("library.create.template.invalid"))?,
                skill,
            ),
            (
                PortablePath::new("jameskills.toml".to_owned())
                    .map_err(|_| draft_error("library.create.template.invalid"))?,
                manifest.into_bytes(),
            ),
        ]
        .into_iter()
        .collect();
        let validated = super::validate_bundle(&files).map_err(AppError::Validation)?;
        let draft = match trust_state {
            TrustState::Quarantined => SkillDraft::new_quarantined(self.skill_id, None, 1, files)?,
            TrustState::Reviewed => SkillDraft::new(self.skill_id, None, 1, files)?,
        };
        if validated.manifest().id() == draft.skill_id() {
            Ok(draft)
        } else {
            Err(draft_error("library.create.template.invalid"))
        }
    }
}

fn quote_toml_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

impl SkillDraft {
    pub fn new(
        skill_id: SkillId,
        base_head: Option<RevisionId>,
        generation: u64,
        files: BTreeMap<PortablePath, Vec<u8>>,
    ) -> AppResult<Self> {
        Self::new_with_trust(skill_id, base_head, generation, files, TrustState::Reviewed)
    }

    pub fn new_quarantined(
        skill_id: SkillId,
        base_head: Option<RevisionId>,
        generation: u64,
        files: BTreeMap<PortablePath, Vec<u8>>,
    ) -> AppResult<Self> {
        Self::new_with_trust(
            skill_id,
            base_head,
            generation,
            files,
            TrustState::Quarantined,
        )
    }

    fn new_with_trust(
        skill_id: SkillId,
        base_head: Option<RevisionId>,
        generation: u64,
        files: BTreeMap<PortablePath, Vec<u8>>,
        trust_state: TrustState,
    ) -> AppResult<Self> {
        validate_draft_files(&files)?;
        if generation == 0 || generation > MAX_DRAFT_GENERATION {
            return Err(draft_error("library.draft.generation.invalid"));
        }
        Ok(Self {
            skill_id,
            base_head,
            generation,
            files,
            trust_state,
        })
    }

    pub fn replace_files(&self, files: BTreeMap<PortablePath, Vec<u8>>) -> AppResult<Self> {
        let generation = self
            .generation
            .checked_add(1)
            .ok_or_else(|| draft_error("library.draft.generation.overflow"))?;
        Self::new_with_trust(
            self.skill_id,
            self.base_head.clone(),
            generation,
            files,
            self.trust_state,
        )
    }

    pub fn skill_id(&self) -> SkillId {
        self.skill_id
    }
    pub fn base_head(&self) -> Option<&RevisionId> {
        self.base_head.as_ref()
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn files(&self) -> &BTreeMap<PortablePath, Vec<u8>> {
        &self.files
    }
    pub fn trust_state(&self) -> TrustState {
        self.trust_state
    }
}

fn validate_draft_files(files: &BTreeMap<PortablePath, Vec<u8>>) -> AppResult<()> {
    if files.len() > MAX_DRAFT_FILE_COUNT {
        return Err(draft_error("library.draft.files.limit"));
    }
    let total_bytes = files
        .values()
        .try_fold(0usize, |total, bytes| total.checked_add(bytes.len()));
    if total_bytes.is_none_or(|total| total > MAX_DRAFT_BYTES) {
        return Err(draft_error("library.draft.bytes.limit"));
    }
    Ok(())
}

fn draft_error(code: &'static str) -> AppError {
    AppError::Validation(vec![Diagnostic::error(
        code,
        "Skill draft is invalid or exceeds its bounded resource limits.",
    )])
}

/// Outcome of a committed revision: the stored record plus the new head set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveRevisionResult {
    revision: RevisionRecord,
    new_heads: Vec<RevisionId>,
    no_op: bool,
}

impl SaveRevisionResult {
    pub fn new(revision: RevisionRecord, new_heads: Vec<RevisionId>) -> Self {
        Self {
            revision,
            new_heads,
            no_op: false,
        }
    }

    pub fn no_op(revision: RevisionRecord, heads: Vec<RevisionId>) -> Self {
        Self {
            revision,
            new_heads: heads,
            no_op: true,
        }
    }

    pub fn revision(&self) -> &RevisionRecord {
        &self.revision
    }

    pub fn new_heads(&self) -> &[RevisionId] {
        &self.new_heads
    }

    pub fn is_no_op(&self) -> bool {
        self.no_op
    }
}
