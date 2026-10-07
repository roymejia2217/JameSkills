use super::{ContentHash, RevisionId, SkillId};
use crate::{AppError, AppResult, Diagnostic};
use sha2::{Digest, Sha256};

/// Domain tag opening every revision digest (SPEC-skill-format, canonical hashing).
const REVISION_TAG: &[u8] = b"JAMESKILLS-REVISION-V1\0";
/// Kind byte for a content revision; tombstones use [`KIND_TOMBSTONE`].
const KIND_CONTENT: u8 = 0x01;
/// Kind byte for a tombstone revision; the bundle slot digests as zeros.
const KIND_TOMBSTONE: u8 = 0x02;

/// Causal kind of a revision. Tombstones carry the heads they observed so a
/// delete never silently resurrects a concurrent content revision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RevisionKind {
    Content,
    Tombstone { observed_heads: Vec<RevisionId> },
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
        }
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
}

/// Outcome of a committed revision: the stored record plus the new head set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveRevisionResult {
    revision: RevisionRecord,
    new_heads: Vec<RevisionId>,
}

impl SaveRevisionResult {
    pub fn new(revision: RevisionRecord, new_heads: Vec<RevisionId>) -> Self {
        Self {
            revision,
            new_heads,
        }
    }

    pub fn revision(&self) -> &RevisionRecord {
        &self.revision
    }

    pub fn new_heads(&self) -> &[RevisionId] {
        &self.new_heads
    }
}
