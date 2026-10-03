use super::{ContentHash, PathValidationError, PortablePath, SkillId, ValidatedInventory};
use crate::{AppError, AppResult, Diagnostic, DiagnosticSeverity};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
    str,
};

const MAX_MANIFEST_BYTES: usize = 64 * 1024;
const MAX_FRONTMATTER_BYTES: usize = 16 * 1024;
const MAX_SKILL_BYTES: usize = 256 * 1024;
const MAX_CAPABILITIES: usize = 128;
const MAX_RESOURCE_PATHS: usize = 256;
const MAX_TAGS: usize = 64;

#[derive(Clone, PartialEq, Eq)]
pub struct SkillManifest {
    schema_version: u32,
    id: SkillId,
    slug: String,
    display_name: String,
    version: semver::Version,
    description: String,
    license: String,
    minimum_app_version: semver::Version,
    policy_files: Vec<PortablePath>,
    guidance_files: Vec<PortablePath>,
    tags: Vec<String>,
    capabilities: Vec<CapabilityDeclaration>,
    extensions: BTreeMap<String, String>,
}

impl SkillManifest {
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }
    pub fn id(&self) -> SkillId {
        self.id
    }
    pub fn slug(&self) -> &str {
        &self.slug
    }
    pub fn display_name(&self) -> &str {
        &self.display_name
    }
    pub fn version(&self) -> &semver::Version {
        &self.version
    }
    pub fn description(&self) -> &str {
        &self.description
    }
    pub fn license(&self) -> &str {
        &self.license
    }
    pub fn minimum_app_version(&self) -> &semver::Version {
        &self.minimum_app_version
    }
    pub fn policy_files(&self) -> &[PortablePath] {
        &self.policy_files
    }
    pub fn guidance_files(&self) -> &[PortablePath] {
        &self.guidance_files
    }
    pub fn tags(&self) -> &[String] {
        &self.tags
    }
    pub fn capabilities(&self) -> &[CapabilityDeclaration] {
        &self.capabilities
    }
    pub fn extensions(&self) -> &BTreeMap<String, String> {
        &self.extensions
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct CapabilityDeclaration {
    id: String,
    required: bool,
    guidance_id: Option<String>,
}

impl CapabilityDeclaration {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn required(&self) -> bool {
        self.required
    }
    pub fn guidance_id(&self) -> Option<&str> {
        self.guidance_id.as_deref()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct SkillFrontmatter {
    name: String,
    description: String,
    compatibility: Option<String>,
    metadata: BTreeMap<String, String>,
    source: String,
    body: String,
}

impl SkillFrontmatter {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn description(&self) -> &str {
        &self.description
    }
    pub fn compatibility(&self) -> Option<&str> {
        self.compatibility.as_deref()
    }
    pub fn metadata(&self) -> &BTreeMap<String, String> {
        &self.metadata
    }
    pub fn source(&self) -> &str {
        &self.source
    }
    pub fn body(&self) -> &str {
        &self.body
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSkillManifest {
    schema_version: toml::Spanned<u32>,
    id: toml::Spanned<String>,
    slug: toml::Spanned<String>,
    display_name: String,
    version: toml::Spanned<String>,
    description: String,
    license: String,
    minimum_app_version: toml::Spanned<String>,
    #[serde(default)]
    policy_files: Vec<String>,
    #[serde(default)]
    guidance_files: Vec<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    capabilities: Vec<RawCapabilityDeclaration>,
    #[serde(default)]
    extensions: BTreeMap<String, String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCapabilityDeclaration {
    id: toml::Spanned<String>,
    required: bool,
    #[serde(default)]
    guidance_id: Option<toml::Spanned<String>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSkillFrontmatter {
    name: String,
    description: String,
    #[serde(default)]
    compatibility: Option<String>,
    #[serde(default)]
    metadata: BTreeMap<String, String>,
}

pub fn parse_manifest(source: &str) -> Result<SkillManifest, Vec<Diagnostic>> {
    if source.len() > MAX_MANIFEST_BYTES {
        return Err(vec![diagnostic(
            "manifest.too_large",
            None,
            None,
            "Manifest exceeds the size limit.",
        )]);
    }

    let raw: RawSkillManifest = toml::from_str(source).map_err(|error: toml::de::Error| {
        let location = error.span().map(|span| line_column(source, span.start));
        vec![diagnostic(
            "manifest.invalid",
            location.map(|value| value.0),
            location.map(|value| value.1),
            "Manifest is invalid or contains an unsupported field.",
        )]
    })?;

    let schema_span = raw.schema_version.span();
    let schema_version = raw.schema_version.into_inner();
    if schema_version != 1 {
        return Err(vec![span_diagnostic(
            "manifest.unsupported_schema",
            source,
            schema_span,
            "Manifest schema version is not supported.",
        )]);
    }

    let id_span = raw.id.span();
    let id_value = raw.id.into_inner();
    let id = SkillId::parse(&id_value).map_err(|_| {
        vec![span_diagnostic(
            "manifest.id.invalid",
            source,
            id_span,
            "Manifest ID must be a UUID.",
        )]
    })?;

    let slug_span = raw.slug.span();
    let slug = raw.slug.into_inner();
    validate_slug(&slug).map_err(|_| {
        vec![span_diagnostic(
            "manifest.slug.invalid",
            source,
            slug_span,
            "Manifest slug is invalid.",
        )]
    })?;

    let version = parse_version(
        raw.version,
        "manifest.version.invalid",
        source,
        "Suite version must be valid SemVer.",
    )?;
    let minimum_app_version = parse_version(
        raw.minimum_app_version,
        "manifest.minimum_app_version.invalid",
        source,
        "Minimum app version must be valid SemVer.",
    )?;

    validate_text(
        &raw.display_name,
        128,
        false,
        "manifest.display_name.invalid",
    )?;
    validate_text(
        &raw.description,
        1024,
        false,
        "manifest.description.invalid",
    )?;
    validate_text(&raw.license, 128, false, "manifest.license.invalid")?;

    if raw.policy_files.len() > MAX_RESOURCE_PATHS || raw.guidance_files.len() > MAX_RESOURCE_PATHS
    {
        return Err(vec![diagnostic(
            "manifest.resource_limit",
            None,
            None,
            "Manifest has too many resource references.",
        )]);
    }
    let policy_files = parse_resource_paths(raw.policy_files)?;
    let guidance_files = parse_resource_paths(raw.guidance_files)?;

    if raw.tags.len() > MAX_TAGS {
        return Err(vec![diagnostic(
            "manifest.tag_limit",
            None,
            None,
            "Manifest has too many tags.",
        )]);
    }
    let mut tags_seen = BTreeSet::new();
    for tag in &raw.tags {
        validate_slug(tag).map_err(|_| {
            vec![diagnostic(
                "manifest.tag.invalid",
                None,
                None,
                "Manifest tag is invalid.",
            )]
        })?;
        if !tags_seen.insert(tag) {
            return Err(vec![diagnostic(
                "manifest.tag.duplicate",
                None,
                None,
                "Manifest contains a duplicate tag.",
            )]);
        }
    }

    if raw.capabilities.len() > MAX_CAPABILITIES {
        return Err(vec![diagnostic(
            "manifest.capability_limit",
            None,
            None,
            "Manifest has too many capability declarations.",
        )]);
    }
    let mut capabilities = Vec::with_capacity(raw.capabilities.len());
    let mut capability_ids = BTreeSet::new();
    for raw_capability in raw.capabilities {
        let id_span = raw_capability.id.span();
        let id = raw_capability.id.into_inner();
        if !valid_namespaced_id(&id) {
            return Err(vec![span_diagnostic(
                "manifest.capability.invalid",
                source,
                id_span,
                "Capability identifier is invalid.",
            )]);
        }
        if !capability_ids.insert(id.clone()) {
            return Err(vec![diagnostic(
                "manifest.capability.duplicate",
                None,
                None,
                "Manifest contains a duplicate capability.",
            )]);
        }
        let guidance_id = match raw_capability.guidance_id {
            Some(value) => {
                let span = value.span();
                let value = value.into_inner();
                if !valid_namespaced_id(&value) {
                    return Err(vec![span_diagnostic(
                        "manifest.guidance_id.invalid",
                        source,
                        span,
                        "Capability guidance ID is invalid.",
                    )]);
                }
                Some(value)
            }
            None => None,
        };
        capabilities.push(CapabilityDeclaration {
            id,
            required: raw_capability.required,
            guidance_id,
        });
    }

    if raw.extensions.len() > 128
        || raw
            .extensions
            .iter()
            .any(|(key, value)| !valid_namespaced_id(key) || value.chars().count() > 1024)
    {
        return Err(vec![diagnostic(
            "manifest.extensions.invalid",
            None,
            None,
            "Manifest extensions must be bounded string metadata.",
        )]);
    }

    Ok(SkillManifest {
        schema_version,
        id,
        slug,
        display_name: raw.display_name,
        version,
        description: raw.description,
        license: raw.license,
        minimum_app_version,
        policy_files,
        guidance_files,
        tags: raw.tags,
        capabilities,
        extensions: raw.extensions,
    })
}

pub fn parse_frontmatter(source_bytes: &[u8]) -> Result<SkillFrontmatter, Vec<Diagnostic>> {
    if source_bytes.len() > MAX_SKILL_BYTES {
        return Err(vec![diagnostic(
            "frontmatter.too_large",
            None,
            None,
            "SKILL.md exceeds the size limit.",
        )]);
    }
    let source = str::from_utf8(source_bytes).map_err(|_| {
        vec![diagnostic(
            "frontmatter.invalid_utf8",
            Some(1),
            Some(1),
            "SKILL.md must be UTF-8.",
        )]
    })?;

    let first_line_end = source.find('\n').map_or(source.len(), |index| index + 1);
    let first_line = source[..first_line_end].trim_end_matches(['\r', '\n']);
    if first_line != "---" {
        return Err(vec![diagnostic(
            "frontmatter.missing",
            Some(1),
            Some(1),
            "SKILL.md must begin with YAML frontmatter.",
        )]);
    }

    let header_start = first_line_end;
    let mut offset = header_start;
    let mut closing_end = None;
    for line in source[header_start..].split_inclusive('\n') {
        let line_content = line.trim_end_matches(['\r', '\n']);
        offset += line.len();
        if line_content == "---" {
            closing_end = Some(offset);
            break;
        }
        if offset - header_start > MAX_FRONTMATTER_BYTES {
            return Err(vec![diagnostic(
                "frontmatter.header_too_large",
                Some(2),
                Some(1),
                "YAML frontmatter exceeds the size limit.",
            )]);
        }
    }
    let closing_end = closing_end.ok_or_else(|| {
        vec![diagnostic(
            "frontmatter.unclosed",
            None,
            None,
            "YAML frontmatter is not closed.",
        )]
    })?;
    let header_end = closing_end
        - source[header_start..closing_end]
            .split_inclusive('\n')
            .next_back()
            .map_or(0, str::len);
    let header = &source[header_start..header_end];
    let body = &source[closing_end..];
    if body.trim().is_empty() {
        return Err(vec![diagnostic(
            "frontmatter.invalid",
            None,
            None,
            "SKILL.md instruction body must not be empty.",
        )]);
    }
    if header.len() > MAX_FRONTMATTER_BYTES {
        return Err(vec![diagnostic(
            "frontmatter.header_too_large",
            None,
            None,
            "YAML frontmatter exceeds the size limit.",
        )]);
    }

    let raw: RawSkillFrontmatter = serde_saphyr::from_str_with_options(
        header,
        serde_saphyr::options! {
            budget: serde_saphyr::budget! {
                flow_nesting_limit: 16,
                max_events: 4096,
                max_aliases: 0,
                max_anchors: 0,
                max_recorded_anchor_events: 0,
                max_recorded_anchor_bytes: 0,
                max_depth: 16,
                max_inclusion_depth: 0,
                max_documents: 1,
                max_nodes: 1024,
                max_total_scalar_bytes: MAX_FRONTMATTER_BYTES,
                max_total_comment_bytes: 0,
                max_merge_keys: 0,
                max_total_property_interpolation_work: 0,
            },
            duplicate_keys: serde_saphyr::DuplicateKeyPolicy::Error,
            merge_keys: serde_saphyr::MergeKeyPolicy::Error,
            alias_limits: serde_saphyr::alias_limits! {
                max_total_replayed_events: 0,
                max_replay_stack_depth: 0,
                max_alias_expansions_per_anchor: 0,
            },
            emit_comments: false,
            strict_booleans: true,
            no_schema: true,
            reject_unsupported_tags: true,
            with_snippet: false,
        },
    )
    .map_err(|error| vec![frontmatter_parser_diagnostic(error)])?;

    let name = raw.name;
    validate_slug(&name).map_err(|_| {
        vec![diagnostic(
            "frontmatter.name.invalid",
            None,
            None,
            "Frontmatter name is invalid.",
        )]
    })?;
    let description = raw.description;
    validate_text(&description, 1024, false, "frontmatter.description.invalid")?;
    let compatibility = raw.compatibility;
    if compatibility
        .as_ref()
        .is_some_and(|value| value.chars().count() > 500)
    {
        return Err(vec![diagnostic(
            "frontmatter.compatibility.invalid",
            None,
            None,
            "Compatibility text exceeds the size limit.",
        )]);
    }
    if raw.metadata.len() > 128
        || raw.metadata.iter().any(|(key, value)| {
            key.is_empty() || key.chars().count() > 64 || value.chars().count() > 1024
        })
    {
        return Err(vec![diagnostic(
            "frontmatter.metadata_invalid",
            None,
            None,
            "Frontmatter metadata is outside the size limits.",
        )]);
    }
    let metadata = raw.metadata;

    Ok(SkillFrontmatter {
        name,
        description,
        compatibility,
        metadata,
        source: source.to_owned(),
        body: body.to_owned(),
    })
}

pub fn validate_skill_pair(
    manifest: &SkillManifest,
    frontmatter: &SkillFrontmatter,
) -> Result<(), Vec<Diagnostic>> {
    if manifest.slug != frontmatter.name {
        return Err(vec![diagnostic(
            "frontmatter.name_mismatch",
            None,
            None,
            "Frontmatter name must equal the manifest slug.",
        )]);
    }
    if manifest.description != frontmatter.description {
        return Err(vec![diagnostic(
            "frontmatter.description_mismatch",
            None,
            None,
            "Manifest and frontmatter descriptions must match exactly.",
        )]);
    }
    if let Some(id) = frontmatter.metadata.get("jameskills-id") {
        let metadata_id = SkillId::parse(id).map_err(|_| {
            vec![diagnostic(
                "frontmatter.id.invalid",
                None,
                None,
                "JameSkills metadata ID must be a UUID.",
            )]
        })?;
        if metadata_id != manifest.id {
            return Err(vec![diagnostic(
                "frontmatter.id_mismatch",
                None,
                None,
                "JameSkills metadata ID does not match the manifest.",
            )]);
        }
    }
    if let Some(version) = frontmatter.metadata.get("jameskills-version") {
        let metadata_version = semver::Version::parse(version).map_err(|_| {
            vec![diagnostic(
                "frontmatter.version.invalid",
                None,
                None,
                "JameSkills metadata version must be SemVer.",
            )]
        })?;
        if metadata_version != manifest.version {
            return Err(vec![diagnostic(
                "frontmatter.version_mismatch",
                None,
                None,
                "JameSkills metadata version does not match the manifest.",
            )]);
        }
    }
    Ok(())
}

fn parse_version(
    value: toml::Spanned<String>,
    code: &'static str,
    source: &str,
    message: &'static str,
) -> Result<semver::Version, Vec<Diagnostic>> {
    let span = value.span();
    semver::Version::parse(value.get_ref())
        .map_err(|_| vec![span_diagnostic(code, source, span, message)])
}

fn parse_resource_paths(values: Vec<String>) -> Result<Vec<PortablePath>, Vec<Diagnostic>> {
    values
        .into_iter()
        .map(|value| {
            PortablePath::new(value).map_err(|_: PathValidationError| {
                vec![diagnostic(
                    "manifest.path.invalid",
                    None,
                    None,
                    "Manifest resource path is not portable.",
                )]
            })
        })
        .collect()
}

fn frontmatter_parser_diagnostic(error: serde_saphyr::Error) -> Diagnostic {
    use serde_saphyr::{Error, budget::BudgetBreach};

    let code = match &error {
        Error::MergeKeyNotAllowed { .. } => "frontmatter.merge_key.unsupported",
        Error::DuplicateMappingKey { .. } => "frontmatter.duplicate_key",
        Error::SerdeUnknownField { .. } => "frontmatter.unknown_field",
        Error::SerdeMissingField { .. } => "frontmatter.required_field",
        Error::UnsupportedTag { .. } => "frontmatter.tag.unsupported",
        Error::Budget {
            breach: BudgetBreach::Aliases { .. },
            ..
        } => "frontmatter.alias.unsupported",
        Error::Budget {
            breach: BudgetBreach::Anchors { .. },
            ..
        } => "frontmatter.anchor.unsupported",
        Error::Budget {
            breach: BudgetBreach::Depth { .. },
            ..
        } => "frontmatter.depth_limit",
        Error::Budget { .. } => "frontmatter.resource_limit",
        _ => "frontmatter.invalid",
    };
    let location = error.location();
    diagnostic(
        code,
        location.and_then(|value| u32::try_from(value.line()).ok()),
        location.and_then(|value| u32::try_from(value.column()).ok()),
        match code {
            "frontmatter.merge_key.unsupported" => "YAML merge keys are not supported.",
            "frontmatter.duplicate_key" => "Frontmatter contains a duplicate key.",
            "frontmatter.unknown_field" => "Frontmatter contains an unsupported field.",
            "frontmatter.required_field" => "Frontmatter is missing a required field.",
            "frontmatter.tag.unsupported" => "YAML tags are not supported.",
            "frontmatter.alias.unsupported" => "YAML aliases are not supported.",
            "frontmatter.anchor.unsupported" => "YAML anchors are not supported.",
            "frontmatter.depth_limit" => "YAML frontmatter exceeds the nesting limit.",
            "frontmatter.resource_limit" => "YAML frontmatter exceeds a resource limit.",
            _ => "YAML frontmatter is invalid.",
        },
    )
}

fn validate_text(
    value: &str,
    max_chars: usize,
    allow_empty: bool,
    code: &'static str,
) -> Result<(), Vec<Diagnostic>> {
    let chars = value.chars().count();
    if chars > max_chars || (!allow_empty && value.trim().is_empty()) {
        return Err(vec![diagnostic(
            code,
            None,
            None,
            "Text field is empty or exceeds its size limit.",
        )]);
    }
    Ok(())
}

fn validate_slug(value: &str) -> Result<(), ()> {
    let bytes = value.as_bytes();
    if bytes.is_empty()
        || bytes.len() > 64
        || !bytes[0].is_ascii_lowercase() && !bytes[0].is_ascii_digit()
    {
        return Err(());
    }
    let mut previous_hyphen = false;
    for (index, byte) in bytes.iter().copied().enumerate() {
        if byte == b'-' {
            if index == 0 || index + 1 == bytes.len() || previous_hyphen {
                return Err(());
            }
            previous_hyphen = true;
        } else if byte.is_ascii_lowercase() || byte.is_ascii_digit() {
            previous_hyphen = false;
        } else {
            return Err(());
        }
    }
    Ok(())
}

fn valid_namespaced_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.split('.').all(|part| validate_slug(part).is_ok())
}

fn diagnostic(
    code: &'static str,
    line: Option<u32>,
    column: Option<u32>,
    message: &'static str,
) -> Diagnostic {
    Diagnostic::new(code, None, line, column, message, DiagnosticSeverity::Error)
}

fn span_diagnostic(
    code: &'static str,
    source: &str,
    span: Range<usize>,
    message: &'static str,
) -> Diagnostic {
    let (line, column) = line_column(source, span.start);
    diagnostic(code, Some(line), Some(column), message)
}

fn line_column(source: &str, index: usize) -> (u32, u32) {
    let prefix = &source[..index.min(source.len())];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() as u32 + 1;
    let column = prefix
        .rsplit('\n')
        .next()
        .unwrap_or_default()
        .chars()
        .count() as u32
        + 1;
    (line, column)
}

/// Canonical path order of a validated inventory: UTF-8 byte order, the same
/// order `validate_bundle_inventory` stores. Library, install and backup
/// share this order instead of re-sorting it.
pub fn canonical_inventory(inventory: &ValidatedInventory) -> Vec<PortablePath> {
    inventory
        .files()
        .iter()
        .map(|file| file.path().clone())
        .collect()
}

/// Hashes raw bundle bytes in canonical order over the exact SPEC-skill-format
/// bytes: tag, then per path u32be length plus path bytes plus u64be length
/// plus raw bytes. Line endings hash as-is, so different bytes are a different
/// revision; sizes and other metadata never reach the digest. The byte map
/// must match the inventory exactly: missing and unexpected entries fail.
pub fn hash_bundle(
    inventory: &ValidatedInventory,
    files: &BTreeMap<PortablePath, Vec<u8>>,
) -> AppResult<ContentHash> {
    let canonical = canonical_inventory(inventory);
    let mut digest = Sha256::new();
    digest.update(b"JAMESKILLS-BUNDLE-V1\0");
    for path in &canonical {
        let content = files.get(path).ok_or_else(|| {
            AppError::Validation(vec![diagnostic(
                "bundle.hash.missing_bytes",
                None,
                None,
                "Bundle bytes are missing for a validated path.",
            )])
        })?;
        let path_bytes = path.as_str().as_bytes();
        digest.update(bundle_len(path_bytes.len())?);
        digest.update(path_bytes);
        digest.update((content.len() as u64).to_be_bytes());
        digest.update(content);
    }
    if files.len() != canonical.len() {
        return Err(AppError::Validation(vec![diagnostic(
            "bundle.hash.unexpected_bytes",
            None,
            None,
            "Bundle bytes cover paths outside the validated inventory.",
        )]));
    }
    Ok(ContentHash::from_digest(digest.finalize().into()))
}

fn bundle_len(len: usize) -> AppResult<[u8; 4]> {
    u32::try_from(len)
        .map(|count| count.to_be_bytes())
        .map_err(|_| AppError::CryptoInvalid)
}
