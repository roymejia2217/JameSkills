use super::import::TrustState;
use super::{ContentHash, PortablePath, RevisionId, SkillId, ValidatedBundle};
use crate::{AppError, AppResult, Diagnostic};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

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
    expected_draft_generation: Option<u64>,
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
        }
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
