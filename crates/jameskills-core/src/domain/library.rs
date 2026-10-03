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
