use super::policy::ApplicabilityFact;
use super::policy::{ToolId, ToolOperation};
use crate::{Diagnostic, DiagnosticSeverity};
use semver::{Version, VersionReq};
use std::collections::BTreeMap;

const MAX_EVIDENCE_SOURCE_ID: usize = 128;
const MAX_EVIDENCE_SUMMARY: usize = 256;
const MAX_CAPABILITIES_PER_TOOL: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolAvailability {
    Missing,
    Candidate,
    Verified,
    Blocked,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolVersionStatus {
    Compatible,
    Incompatible,
    Unknown,
    NotApplicable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolCapabilitySupport {
    Supported,
    NeedsVerification,
    Unsupported,
}

/// App-authored provenance for one bounded tool observation. Summary cannot
/// contain arbitrary process output; it is a static, redacted explanation.
#[derive(Clone, PartialEq, Eq)]
pub struct ToolEvidence {
    source_id: String,
    observed_at: String,
    tested_version: Option<Version>,
    summary: &'static str,
}

impl ToolEvidence {
    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    pub fn observed_at(&self) -> &str {
        &self.observed_at
    }

    pub fn tested_version(&self) -> Option<&Version> {
        self.tested_version.as_ref()
    }

    pub fn summary(&self) -> &'static str {
        self.summary
    }
}

/// Presence, version and per-operation support remain separate so a found
/// executable or malformed version output cannot become a verified capability.
#[derive(Clone, PartialEq, Eq)]
pub struct ToolDetection {
    tool_id: ToolId,
    availability: ToolAvailability,
    version: Option<Version>,
    version_status: ToolVersionStatus,
    capabilities: BTreeMap<ToolOperation, ToolCapabilitySupport>,
    evidence: ToolEvidence,
}

impl ToolDetection {
    #[allow(clippy::too_many_arguments)]
    pub fn from_probe(
        tool_id: ToolId,
        availability: ToolAvailability,
        version: Option<Version>,
        supported_range: &VersionReq,
        operations: &[ToolOperation],
        source_id: &str,
        observed_at: &str,
        summary: &'static str,
    ) -> Result<Self, Vec<Diagnostic>> {
        if !valid_source_id(source_id)
            || !valid_timestamp(observed_at)
            || summary.is_empty()
            || summary.len() > MAX_EVIDENCE_SUMMARY
            || operations.is_empty()
            || operations.len() > MAX_CAPABILITIES_PER_TOOL
        {
            return Err(tool_diagnostic(
                "tool.evidence.invalid",
                "Tool evidence metadata is invalid or outside its limits.",
            ));
        }
        let version_status = match (availability, version.as_ref()) {
            (ToolAvailability::Missing, None) => ToolVersionStatus::NotApplicable,
            (ToolAvailability::Missing, Some(_)) => {
                return Err(tool_diagnostic(
                    "tool.observation.inconsistent",
                    "A missing executable cannot have a tested version.",
                ));
            }
            (ToolAvailability::Blocked | ToolAvailability::Unknown, None)
            | (ToolAvailability::Candidate, None) => ToolVersionStatus::Unknown,
            (ToolAvailability::Blocked | ToolAvailability::Unknown, Some(_)) => {
                return Err(tool_diagnostic(
                    "tool.observation.inconsistent",
                    "Blocked or unknown presence cannot carry a tested version.",
                ));
            }
            (ToolAvailability::Candidate | ToolAvailability::Verified, Some(version))
                if supported_range.matches(version) =>
            {
                ToolVersionStatus::Compatible
            }
            (ToolAvailability::Candidate | ToolAvailability::Verified, Some(_)) => {
                ToolVersionStatus::Incompatible
            }
            (ToolAvailability::Verified, None) => {
                return Err(tool_diagnostic(
                    "tool.observation.inconsistent",
                    "A verified executable must include a tested version.",
                ));
            }
        };
        let support = match (availability, version_status) {
            (_, ToolVersionStatus::Incompatible | ToolVersionStatus::NotApplicable) => {
                ToolCapabilitySupport::Unsupported
            }
            (ToolAvailability::Verified, ToolVersionStatus::Compatible) => {
                ToolCapabilitySupport::Supported
            }
            _ => ToolCapabilitySupport::NeedsVerification,
        };
        let capabilities: BTreeMap<_, _> = operations
            .iter()
            .copied()
            .map(|operation| (operation, support))
            .collect();
        if capabilities.len() != operations.len() {
            return Err(tool_diagnostic(
                "tool.capability.duplicate",
                "Tool capability operations must be unique.",
            ));
        }
        Ok(Self {
            tool_id,
            availability,
            version: version.clone(),
            version_status,
            capabilities,
            evidence: ToolEvidence {
                source_id: source_id.to_owned(),
                observed_at: observed_at.to_owned(),
                tested_version: version,
                summary,
            },
        })
    }

    pub fn tool_id(&self) -> ToolId {
        self.tool_id
    }

    pub fn availability(&self) -> ToolAvailability {
        self.availability
    }

    pub fn version(&self) -> Option<&Version> {
        self.version.as_ref()
    }

    pub fn version_status(&self) -> ToolVersionStatus {
        self.version_status
    }

    pub fn capability(&self, operation: ToolOperation) -> Option<ToolCapabilitySupport> {
        self.capabilities.get(&operation).copied()
    }

    pub fn capabilities(&self) -> &BTreeMap<ToolOperation, ToolCapabilitySupport> {
        &self.capabilities
    }

    pub fn evidence(&self) -> &ToolEvidence {
        &self.evidence
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum OfficialGuidanceSource {
    GitInstall,
    Gitleaks,
    ConventionalCommits,
    GithubCli,
    GithubRulesets,
}

#[derive(Clone, PartialEq, Eq)]
pub enum GuidanceAction {
    ManualInstruction,
    OpenOfficialUrl {
        source: OfficialGuidanceSource,
    },
    CopyApprovedCommand {
        tool_id: ToolId,
        operation: ToolOperation,
    },
    SelectLocalPath {
        purpose: String,
    },
    AnswerChoice {
        choices: Vec<String>,
    },
    Recheck,
}

#[derive(Clone, PartialEq, Eq)]
pub struct GuidanceCondition {
    fact: ApplicabilityFact,
    equals: String,
}

impl GuidanceCondition {
    pub(super) fn from_validated(fact: ApplicabilityFact, equals: String) -> Self {
        Self { fact, equals }
    }

    pub fn fact(&self) -> ApplicabilityFact {
        self.fact
    }

    pub fn equals(&self) -> &str {
        &self.equals
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct GuidanceStep {
    id: String,
    prompt_es: String,
    requires: Vec<String>,
    verification_requirement_ids: Vec<String>,
    applies_when: Option<GuidanceCondition>,
    action: GuidanceAction,
}

impl GuidanceStep {
    pub(super) fn from_validated(
        id: String,
        prompt_es: String,
        requires: Vec<String>,
        verification_requirement_ids: Vec<String>,
        applies_when: Option<GuidanceCondition>,
        action: GuidanceAction,
    ) -> Self {
        Self {
            id,
            prompt_es,
            requires,
            verification_requirement_ids,
            applies_when,
            action,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn prompt_es(&self) -> &str {
        &self.prompt_es
    }

    pub fn requires(&self) -> &[String] {
        &self.requires
    }

    pub fn verification_requirement_ids(&self) -> &[String] {
        &self.verification_requirement_ids
    }

    pub fn applies_when(&self) -> Option<&GuidanceCondition> {
        self.applies_when.as_ref()
    }

    pub fn action(&self) -> &GuidanceAction {
        &self.action
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct GuidancePlan {
    id: String,
    requirement_ids: Vec<String>,
    steps: Vec<GuidanceStep>,
}

impl GuidancePlan {
    pub(super) fn from_validated(
        id: String,
        requirement_ids: Vec<String>,
        steps: Vec<GuidanceStep>,
    ) -> Self {
        Self {
            id,
            requirement_ids,
            steps,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn requirement_ids(&self) -> &[String] {
        &self.requirement_ids
    }

    pub fn steps(&self) -> &[GuidanceStep] {
        &self.steps
    }
}

fn valid_source_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_EVIDENCE_SOURCE_ID
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-')
        })
        && value
            .split(['.', '-'])
            .all(|component| !component.is_empty())
}

fn valid_timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() < 20
        || bytes.len() > 40
        || !value.is_ascii()
        || bytes.get(4) != Some(&b'-')
        || bytes.get(7) != Some(&b'-')
        || bytes.get(10) != Some(&b'T')
        || bytes.get(13) != Some(&b':')
        || bytes.get(16) != Some(&b':')
    {
        return false;
    }
    let Some(year) = decimal(bytes, 0, 4) else {
        return false;
    };
    let Some(month) = decimal(bytes, 5, 7) else {
        return false;
    };
    let Some(day) = decimal(bytes, 8, 10) else {
        return false;
    };
    let Some(hour) = decimal(bytes, 11, 13) else {
        return false;
    };
    let Some(minute) = decimal(bytes, 14, 16) else {
        return false;
    };
    let Some(second) = decimal(bytes, 17, 19) else {
        return false;
    };
    let leap_year = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap_year => 29,
        2 => 28,
        _ => return false,
    };
    if day == 0 || day > max_day || hour > 23 || minute > 59 || second > 60 {
        return false;
    }

    let mut suffix = &value[19..];
    if suffix.starts_with('.') {
        let timezone = suffix[1..]
            .find(['Z', '+', '-'])
            .map(|index| index + 1)
            .unwrap_or(suffix.len());
        let fraction = &suffix[1..timezone];
        if fraction.is_empty()
            || fraction.len() > 9
            || !fraction.bytes().all(|byte| byte.is_ascii_digit())
        {
            return false;
        }
        suffix = &suffix[timezone..];
    }
    if suffix == "Z" {
        return true;
    }
    if suffix.len() != 6
        || !matches!(suffix.as_bytes()[0], b'+' | b'-')
        || suffix.as_bytes()[3] != b':'
    {
        return false;
    }
    decimal(suffix.as_bytes(), 1, 3).is_some_and(|hours| hours <= 23)
        && decimal(suffix.as_bytes(), 4, 6).is_some_and(|minutes| minutes <= 59)
}

fn decimal(bytes: &[u8], start: usize, end: usize) -> Option<u32> {
    let value = bytes.get(start..end)?;
    if value.is_empty() || !value.iter().all(u8::is_ascii_digit) {
        return None;
    }
    std::str::from_utf8(value).ok()?.parse().ok()
}

fn tool_diagnostic(code: &'static str, message: &'static str) -> Vec<Diagnostic> {
    vec![Diagnostic::new(
        code,
        None,
        None,
        None,
        message,
        DiagnosticSeverity::Error,
    )]
}
