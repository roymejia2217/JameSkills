use super::{PortablePath, RevisionId, Scope};
use crate::{Diagnostic, DiagnosticSeverity};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

const MAX_POLICY_BYTES: usize = 256 * 1024;
const MAX_REQUIREMENTS: usize = 512;
const MAX_TOOL_REQUIREMENTS: usize = 64;
const MAX_LIST_ITEMS: usize = 128;
const MAX_CHECK_EVIDENCE: usize = 32;
const MAX_CHECK_SUMMARY_BYTES: usize = 256;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    PreInstall,
    Commit,
    PullRequest,
    Ci,
    PreRelease,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Enforcement {
    Instruction,
    LocalCheck,
    LocalHook,
    RequiredCi,
    HostRule,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ApplicabilityFact {
    Os,
    Architecture,
    Stack,
    Host,
    Context,
    Capability,
}

impl ApplicabilityFact {
    pub fn accepts_value(self, value: &str) -> bool {
        let values: &[&str] = match self {
            Self::Os => &["windows", "linux", "other"],
            Self::Architecture => &["x86_64", "aarch64", "x86", "arm", "other"],
            Self::Stack => &["rust", "node", "rust-node", "generic"],
            Self::Host => &["github", "other"],
            Self::Context => &["user", "project", "repository"],
            Self::Capability => &["supported", "needs-verification", "unsupported"],
        };
        values.contains(&value)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ApplicabilityCondition {
    fact: ApplicabilityFact,
    equals: String,
}

impl ApplicabilityCondition {
    pub fn fact(&self) -> ApplicabilityFact {
        self.fact
    }

    pub fn equals(&self) -> &str {
        &self.equals
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ToolId {
    Git,
    Gitleaks,
    Commitlint,
    Gh,
    Cargo,
    Npm,
    Node,
    Rustc,
    CargoAudit,
    CargoDeny,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ToolOperation {
    RepositoryRoot,
    IgnoreCheck,
    ScanTracked,
    LintMessage,
    BranchRules,
    CheckRuns,
    QualitySuite,
    Version,
    Audit,
    Deny,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Policy {
    schema_version: u32,
    profile: String,
    scope: Scope,
    requirements: Vec<Requirement>,
    tool_requirements: Vec<ToolRequirement>,
}

impl Policy {
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }
    pub fn profile(&self) -> &str {
        &self.profile
    }
    pub fn scope(&self) -> Scope {
        self.scope
    }
    pub fn requirements(&self) -> &[Requirement] {
        &self.requirements
    }
    pub fn tool_requirements(&self) -> &[ToolRequirement] {
        &self.tool_requirements
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Requirement {
    id: String,
    description: String,
    severity: Severity,
    required: bool,
    phase: Phase,
    enforcement: Enforcement,
    depends_on: Vec<String>,
    guidance_id: Option<String>,
    applies_when: Option<ApplicabilityCondition>,
    check: Check,
}

impl Requirement {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn description(&self) -> &str {
        &self.description
    }
    pub fn severity(&self) -> Severity {
        self.severity
    }
    pub fn required(&self) -> bool {
        self.required
    }
    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn enforcement(&self) -> Enforcement {
        self.enforcement
    }
    pub fn depends_on(&self) -> &[String] {
        &self.depends_on
    }
    pub fn guidance_id(&self) -> Option<&str> {
        self.guidance_id.as_deref()
    }
    pub fn applies_when(&self) -> Option<&ApplicabilityCondition> {
        self.applies_when.as_ref()
    }
    pub fn check(&self) -> &Check {
        &self.check
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ToolRequirement {
    tool_id: ToolId,
    operation: ToolOperation,
    version: semver::VersionReq,
}

impl ToolRequirement {
    pub fn tool_id(&self) -> ToolId {
        self.tool_id
    }
    pub fn operation(&self) -> ToolOperation {
        self.operation
    }
    pub fn version(&self) -> &semver::VersionReq {
        &self.version
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum Check {
    GitRepository,
    GitignorePatterns {
        path: PortablePath,
        patterns: Vec<String>,
    },
    TrackedSecrets {
        include_history: bool,
    },
    ReadmeSections {
        path: PortablePath,
        headings: Vec<String>,
    },
    ConventionalCommit,
    ProtectedMainLocal {
        branch: String,
    },
    GithubBranchPolicy {
        branch: String,
        require_pull_request: bool,
        required_checks: Vec<String>,
        require_no_bypass: bool,
    },
    CiContract {
        workflow_paths: Vec<PortablePath>,
        required_jobs: Vec<String>,
    },
    CiEvidence {
        required_checks: Vec<String>,
    },
    ReleaseContract {
        require_changelog: bool,
        require_checksums: bool,
        require_signature: bool,
    },
    ToolchainVersion {
        tool_id: ToolId,
        version_range: semver::VersionReq,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TestSuiteKind {
    CargoTest,
    NodeLint,
    NodeTest,
    NodeBuild,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TestSuiteDeclaration {
    Declared,
    Missing,
    Unknown,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TestSuiteExecution {
    NotRun,
    Blocked,
    Passed,
    Failed,
}

#[derive(Clone, PartialEq, Eq)]
pub struct RepositoryHead(String);

impl RepositoryHead {
    pub fn parse(value: &str) -> Result<Self, Vec<Diagnostic>> {
        if !matches!(value.len(), 40 | 64)
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(vec![Diagnostic::error(
                "policy.repository_head.invalid",
                "Repository HEAD must be a lowercase Git object ID.",
            )]);
        }
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Keeps suite presence distinct from whether an explicit execution passed.
#[derive(Clone, PartialEq, Eq)]
pub struct TestSuiteRunResult {
    suite: TestSuiteKind,
    declaration: TestSuiteDeclaration,
    execution: TestSuiteExecution,
    exit_code: Option<i32>,
}

impl TestSuiteRunResult {
    pub fn new(
        suite: TestSuiteKind,
        declaration: TestSuiteDeclaration,
        execution: TestSuiteExecution,
        exit_code: Option<i32>,
    ) -> Result<Self, Vec<Diagnostic>> {
        let valid = match execution {
            TestSuiteExecution::NotRun | TestSuiteExecution::Blocked => exit_code.is_none(),
            TestSuiteExecution::Passed => {
                declaration != TestSuiteDeclaration::Missing && exit_code == Some(0)
            }
            TestSuiteExecution::Failed => {
                declaration != TestSuiteDeclaration::Missing
                    && exit_code.is_some_and(|code| code != 0)
            }
        };
        if !valid {
            return Err(vec![Diagnostic::error(
                "policy.test_suite_result.inconsistent",
                "Suite declaration, execution state, and exit code are inconsistent.",
            )]);
        }
        Ok(Self {
            suite,
            declaration,
            execution,
            exit_code,
        })
    }

    pub fn suite(&self) -> TestSuiteKind {
        self.suite
    }

    pub fn declaration(&self) -> TestSuiteDeclaration {
        self.declaration
    }

    pub fn execution(&self) -> TestSuiteExecution {
        self.execution
    }

    pub fn exit_code(&self) -> Option<i32> {
        self.exit_code
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPolicy {
    schema_version: u32,
    profile: String,
    #[serde(default = "project_scope")]
    scope: String,
    #[serde(default)]
    tool_requirements: Vec<RawToolRequirement>,
    requirements: Vec<RawRequirement>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawToolRequirement {
    tool_id: String,
    operation: String,
    version: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRequirement {
    id: String,
    description: String,
    severity: String,
    required: bool,
    phase: String,
    enforcement: String,
    #[serde(default)]
    depends_on: Vec<String>,
    #[serde(default)]
    guidance_id: Option<String>,
    #[serde(default)]
    applies_when: Option<RawApplicabilityCondition>,
    check: RawCheck,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckStatus {
    Pass,
    Fail,
    Blocked,
    Unknown,
    Unsupported,
    NotApplicable,
}

#[derive(Clone, PartialEq, Eq)]
pub struct CheckEvidence {
    source_id: String,
    observed_at: String,
    revision: Option<RevisionId>,
    environment_fingerprint: String,
    summary: &'static str,
    expires_at_monotonic_ms: Option<u64>,
}

impl CheckEvidence {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source_id: &str,
        observed_at: &str,
        revision: Option<RevisionId>,
        environment_fingerprint: &str,
        summary: &'static str,
        expires_at_monotonic_ms: Option<u64>,
    ) -> Result<Self, Vec<Diagnostic>> {
        if !valid_evidence_source(source_id)
            || !valid_evidence_timestamp(observed_at)
            || !valid_environment_fingerprint(environment_fingerprint)
            || summary.is_empty()
            || summary.len() > MAX_CHECK_SUMMARY_BYTES
            || summary.chars().any(char::is_control)
        {
            return Err(vec![diagnostic(
                "policy.evidence.invalid",
                "Check evidence metadata is invalid or outside its limits.",
            )]);
        }
        Ok(Self {
            source_id: source_id.to_owned(),
            observed_at: observed_at.to_owned(),
            revision,
            environment_fingerprint: environment_fingerprint.to_owned(),
            summary,
            expires_at_monotonic_ms,
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

    pub fn summary(&self) -> &'static str {
        self.summary
    }

    pub fn expires_at_monotonic_ms(&self) -> Option<u64> {
        self.expires_at_monotonic_ms
    }

    pub(crate) fn is_expired_at(&self, now_monotonic_ms: u64) -> bool {
        self.expires_at_monotonic_ms
            .is_some_and(|expires| now_monotonic_ms >= expires)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct CheckObservation {
    status: CheckStatus,
    enforcement: Option<Enforcement>,
    evidence: Vec<CheckEvidence>,
}

impl CheckObservation {
    pub fn new(
        status: CheckStatus,
        enforcement: Option<Enforcement>,
        evidence: Vec<CheckEvidence>,
    ) -> Result<Self, Vec<Diagnostic>> {
        if evidence.len() > MAX_CHECK_EVIDENCE {
            return Err(vec![diagnostic(
                "policy.evidence.limit",
                "Check returned too many evidence entries.",
            )]);
        }
        Ok(Self {
            status,
            enforcement,
            evidence,
        })
    }

    pub fn unknown() -> Self {
        Self {
            status: CheckStatus::Unknown,
            enforcement: None,
            evidence: Vec::new(),
        }
    }

    pub fn status(&self) -> CheckStatus {
        self.status
    }

    pub fn enforcement(&self) -> Option<Enforcement> {
        self.enforcement
    }

    pub fn evidence(&self) -> &[CheckEvidence] {
        &self.evidence
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct CheckResult {
    requirement_id: String,
    status: CheckStatus,
    severity: Severity,
    enforcement: Option<Enforcement>,
    evidence: Vec<CheckEvidence>,
    guidance_id: Option<String>,
}

impl CheckResult {
    pub fn requirement_id(&self) -> &str {
        &self.requirement_id
    }

    pub fn status(&self) -> CheckStatus {
        self.status
    }

    pub fn severity(&self) -> Severity {
        self.severity
    }

    pub fn enforcement(&self) -> Option<Enforcement> {
        self.enforcement
    }

    pub fn evidence(&self) -> &[CheckEvidence] {
        &self.evidence
    }

    pub fn guidance_id(&self) -> Option<&str> {
        self.guidance_id.as_deref()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct CheckReport {
    results: Vec<CheckResult>,
    required_ids: BTreeSet<String>,
}

impl CheckReport {
    pub(crate) fn new(results: Vec<CheckResult>, required_ids: BTreeSet<String>) -> Self {
        Self {
            results,
            required_ids,
        }
    }

    pub fn results(&self) -> &[CheckResult] {
        &self.results
    }

    pub fn strict_exit(&self) -> u8 {
        strict_exit(self)
    }
}

pub fn evaluate_predicate(
    requirement: &Requirement,
    observation: &CheckObservation,
    now_monotonic_ms: u64,
) -> CheckResult {
    let evidence_expired = observation
        .evidence
        .iter()
        .any(|evidence| evidence.is_expired_at(now_monotonic_ms));
    let evidence_missing = observation.evidence.is_empty()
        && matches!(
            observation.status,
            CheckStatus::Pass | CheckStatus::Fail | CheckStatus::NotApplicable
        );
    let status =
        if evidence_expired || evidence_missing || observation.status == CheckStatus::NotApplicable
        {
            CheckStatus::Unknown
        } else if observation.status == CheckStatus::Pass
            && !enforcement_satisfies(requirement.enforcement, observation.enforcement)
        {
            CheckStatus::Blocked
        } else {
            observation.status
        };
    CheckResult {
        requirement_id: requirement.id.clone(),
        status,
        severity: requirement.severity,
        enforcement: (!evidence_expired && !observation.evidence.is_empty())
            .then_some(observation.enforcement)
            .flatten(),
        evidence: observation.evidence.clone(),
        guidance_id: requirement.guidance_id.clone(),
    }
}

pub(crate) fn not_applicable_result(
    requirement: &Requirement,
    evidence: CheckEvidence,
    now_monotonic_ms: u64,
) -> CheckResult {
    CheckResult {
        requirement_id: requirement.id.clone(),
        status: if evidence.is_expired_at(now_monotonic_ms) {
            CheckStatus::Unknown
        } else {
            CheckStatus::NotApplicable
        },
        severity: requirement.severity,
        enforcement: None,
        evidence: vec![evidence],
        guidance_id: requirement.guidance_id.clone(),
    }
}

pub fn strict_exit(report: &CheckReport) -> u8 {
    if report.required_ids.iter().any(|id| {
        report
            .results
            .iter()
            .find(|result| result.requirement_id == *id)
            .is_none_or(|result| {
                !matches!(
                    result.status,
                    CheckStatus::Pass | CheckStatus::NotApplicable
                )
            })
    }) {
        1
    } else {
        0
    }
}

fn enforcement_satisfies(required: Enforcement, observed: Option<Enforcement>) -> bool {
    match required {
        Enforcement::Instruction => observed.is_some(),
        Enforcement::LocalCheck => matches!(
            observed,
            Some(
                Enforcement::LocalCheck
                    | Enforcement::LocalHook
                    | Enforcement::RequiredCi
                    | Enforcement::HostRule
            )
        ),
        Enforcement::LocalHook => observed == Some(Enforcement::LocalHook),
        Enforcement::RequiredCi => {
            matches!(
                observed,
                Some(Enforcement::RequiredCi | Enforcement::HostRule)
            )
        }
        Enforcement::HostRule => observed == Some(Enforcement::HostRule),
    }
}

fn valid_evidence_source(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-')
        })
        && value
            .split(['.', '-'])
            .all(|component| !component.is_empty())
}

fn valid_environment_fingerprint(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn valid_evidence_timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    if !(20..=40).contains(&bytes.len())
        || !value.is_ascii()
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
    {
        return false;
    }
    let Some(year) = evidence_decimal(bytes, 0, 4) else {
        return false;
    };
    let Some(month) = evidence_decimal(bytes, 5, 7) else {
        return false;
    };
    let Some(day) = evidence_decimal(bytes, 8, 10) else {
        return false;
    };
    let Some(hour) = evidence_decimal(bytes, 11, 13) else {
        return false;
    };
    let Some(minute) = evidence_decimal(bytes, 14, 16) else {
        return false;
    };
    let Some(second) = evidence_decimal(bytes, 17, 19) else {
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

    let mut suffix = &bytes[19..];
    if suffix.starts_with(b".") {
        let timezone = suffix
            .iter()
            .position(|byte| matches!(byte, b'Z' | b'+' | b'-'))
            .unwrap_or(suffix.len());
        let fraction = &suffix[1..timezone];
        if fraction.is_empty() || fraction.len() > 9 || !fraction.iter().all(u8::is_ascii_digit) {
            return false;
        }
        suffix = &suffix[timezone..];
    }
    if suffix == b"Z" {
        return true;
    }
    suffix.len() == 6
        && matches!(suffix[0], b'+' | b'-')
        && suffix[3] == b':'
        && evidence_decimal(suffix, 1, 3).is_some_and(|hours| hours <= 23)
        && evidence_decimal(suffix, 4, 6).is_some_and(|minutes| minutes <= 59)
}

fn evidence_decimal(bytes: &[u8], start: usize, end: usize) -> Option<u32> {
    let digits = bytes.get(start..end)?;
    if digits.is_empty() || !digits.iter().all(u8::is_ascii_digit) {
        return None;
    }
    std::str::from_utf8(digits).ok()?.parse().ok()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawApplicabilityCondition {
    fact: String,
    equals: String,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
enum RawCheck {
    GitRepository,
    GitignorePatterns {
        path: PortablePath,
        patterns: Vec<String>,
    },
    TrackedSecrets {
        #[serde(default)]
        include_history: bool,
    },
    ReadmeSections {
        path: PortablePath,
        headings: Vec<String>,
    },
    ConventionalCommit,
    ProtectedMainLocal {
        branch: String,
    },
    GithubBranchPolicy {
        branch: String,
        require_pull_request: bool,
        required_checks: Vec<String>,
        require_no_bypass: bool,
    },
    CiContract {
        workflow_paths: Vec<PortablePath>,
        required_jobs: Vec<String>,
    },
    CiEvidence {
        required_checks: Vec<String>,
    },
    ReleaseContract {
        require_changelog: bool,
        require_checksums: bool,
        require_signature: bool,
    },
    ToolchainVersion {
        tool_id: String,
        version_range: String,
    },
}

pub fn parse_policy(source: &[u8]) -> Result<Policy, Vec<Diagnostic>> {
    if source.len() > MAX_POLICY_BYTES {
        return Err(vec![diagnostic(
            "policy.too_large",
            "Policy exceeds the size limit.",
        )]);
    }
    let source = std::str::from_utf8(source)
        .map_err(|_| vec![diagnostic("policy.invalid_utf8", "Policy must be UTF-8.")])?;
    let raw: RawPolicy = toml::from_str(source).map_err(|error: toml::de::Error| {
        let location = error.span().map(|span| line_column(source, span.start));
        vec![located_diagnostic(
            "policy.invalid",
            location,
            "Policy is invalid or contains an unsupported field.",
        )]
    })?;
    if raw.schema_version != 1 {
        return Err(vec![diagnostic(
            "policy.unsupported_schema",
            "Policy schema version is not supported.",
        )]);
    }
    validate_profile(&raw.profile)?;
    let scope = parse_scope(&raw.scope)?;
    if raw.requirements.is_empty() || raw.requirements.len() > MAX_REQUIREMENTS {
        return Err(vec![diagnostic(
            "policy.requirement_limit",
            "Policy must contain a bounded non-empty requirement list.",
        )]);
    }
    if raw.tool_requirements.len() > MAX_TOOL_REQUIREMENTS {
        return Err(vec![diagnostic(
            "policy.tool_limit",
            "Policy contains too many tool requirements.",
        )]);
    }

    let mut tool_requirements = Vec::with_capacity(raw.tool_requirements.len());
    let mut tool_pairs = BTreeSet::new();
    for item in raw.tool_requirements {
        let tool_id = parse_tool_id(&item.tool_id)?;
        let operation = parse_tool_operation(&item.operation)?;
        if !operation_allowed(tool_id, operation) {
            return Err(vec![diagnostic(
                "policy.operation.invalid",
                "Tool operation is not registered for this tool.",
            )]);
        }
        if !tool_pairs.insert((tool_id, operation)) {
            return Err(vec![diagnostic(
                "policy.tool.duplicate",
                "Policy contains a duplicate tool operation.",
            )]);
        }
        let version = semver::VersionReq::parse(&item.version).map_err(|_| {
            vec![diagnostic(
                "policy.tool.version.invalid",
                "Tool version range must be valid SemVer.",
            )]
        })?;
        tool_requirements.push(ToolRequirement {
            tool_id,
            operation,
            version,
        });
    }

    let mut requirements = Vec::with_capacity(raw.requirements.len());
    let mut requirement_ids = BTreeSet::new();
    for item in raw.requirements {
        if !valid_namespaced_id(&item.id) {
            return Err(vec![diagnostic(
                "policy.requirement_id.invalid",
                "Requirement identifier is invalid.",
            )]);
        }
        if !requirement_ids.insert(item.id.clone()) {
            return Err(vec![diagnostic(
                "policy.requirement.duplicate",
                "Policy contains a duplicate requirement.",
            )]);
        }
        validate_text(&item.description, 1024, "policy.description.invalid")?;
        let severity = parse_severity(&item.severity)?;
        let phase = parse_phase(&item.phase)?;
        let enforcement = parse_enforcement(&item.enforcement)?;
        validate_id_list(&item.depends_on, "policy.requirement_reference.invalid")?;
        if item
            .guidance_id
            .as_deref()
            .is_some_and(|id| !valid_namespaced_id(id))
        {
            return Err(vec![diagnostic(
                "policy.guidance_id.invalid",
                "Guidance identifier is invalid.",
            )]);
        }
        let applies_when = item
            .applies_when
            .map(parse_applicability_condition)
            .transpose()?;
        let check = parse_check(item.check)?;
        requirements.push(Requirement {
            id: item.id,
            description: item.description,
            severity,
            required: item.required,
            phase,
            enforcement,
            depends_on: item.depends_on,
            guidance_id: item.guidance_id,
            applies_when,
            check,
        });
    }

    validate_requirement_graph(&requirements)?;
    validate_registered_tools(&requirements, &tool_pairs)?;
    Ok(Policy {
        schema_version: 1,
        profile: raw.profile,
        scope,
        requirements,
        tool_requirements,
    })
}

fn project_scope() -> String {
    "project".to_owned()
}

fn parse_scope(value: &str) -> Result<Scope, Vec<Diagnostic>> {
    match value {
        "user" => Ok(Scope::User),
        "project" => Ok(Scope::Project),
        _ => Err(vec![diagnostic(
            "policy.scope.invalid",
            "Policy scope is invalid.",
        )]),
    }
}

fn parse_applicability_condition(
    raw: RawApplicabilityCondition,
) -> Result<ApplicabilityCondition, Vec<Diagnostic>> {
    let fact = match raw.fact.as_str() {
        "os" => ApplicabilityFact::Os,
        "architecture" => ApplicabilityFact::Architecture,
        "stack" => ApplicabilityFact::Stack,
        "host" => ApplicabilityFact::Host,
        "context" => ApplicabilityFact::Context,
        "capability" => ApplicabilityFact::Capability,
        _ => return Err(applicability_diagnostic()),
    };
    let allowed_values: &[&str] = match fact {
        ApplicabilityFact::Os => &["windows", "linux", "other"],
        ApplicabilityFact::Architecture => &["x86_64", "aarch64", "x86", "arm", "other"],
        ApplicabilityFact::Stack => &["rust", "node", "rust-node", "generic"],
        ApplicabilityFact::Host => &["github", "other"],
        ApplicabilityFact::Context => &["user", "project", "repository"],
        ApplicabilityFact::Capability => &["supported", "needs-verification", "unsupported"],
    };
    if !allowed_values.contains(&raw.equals.as_str()) {
        return Err(applicability_diagnostic());
    }
    Ok(ApplicabilityCondition {
        fact,
        equals: raw.equals,
    })
}

fn applicability_diagnostic() -> Vec<Diagnostic> {
    vec![diagnostic(
        "policy.applies_when.invalid",
        "Applicability fact or value is not registered.",
    )]
}

fn parse_severity(value: &str) -> Result<Severity, Vec<Diagnostic>> {
    match value {
        "info" => Ok(Severity::Info),
        "warning" => Ok(Severity::Warning),
        "error" => Ok(Severity::Error),
        _ => Err(vec![diagnostic(
            "policy.severity.invalid",
            "Requirement severity is invalid.",
        )]),
    }
}

fn parse_phase(value: &str) -> Result<Phase, Vec<Diagnostic>> {
    match value {
        "pre-install" => Ok(Phase::PreInstall),
        "commit" => Ok(Phase::Commit),
        "pull-request" => Ok(Phase::PullRequest),
        "ci" => Ok(Phase::Ci),
        "pre-release" => Ok(Phase::PreRelease),
        _ => Err(vec![diagnostic(
            "policy.phase.invalid",
            "Requirement phase is invalid.",
        )]),
    }
}

fn parse_enforcement(value: &str) -> Result<Enforcement, Vec<Diagnostic>> {
    match value {
        "instruction" => Ok(Enforcement::Instruction),
        "local-check" => Ok(Enforcement::LocalCheck),
        "local-hook" => Ok(Enforcement::LocalHook),
        "required-ci" => Ok(Enforcement::RequiredCi),
        "host-rule" => Ok(Enforcement::HostRule),
        _ => Err(vec![diagnostic(
            "policy.enforcement.invalid",
            "Requirement enforcement is invalid.",
        )]),
    }
}

fn parse_tool_id(value: &str) -> Result<ToolId, Vec<Diagnostic>> {
    match value {
        "git" => Ok(ToolId::Git),
        "gitleaks" => Ok(ToolId::Gitleaks),
        "commitlint" => Ok(ToolId::Commitlint),
        "gh" => Ok(ToolId::Gh),
        "cargo" => Ok(ToolId::Cargo),
        "npm" => Ok(ToolId::Npm),
        "node" => Ok(ToolId::Node),
        "rustc" => Ok(ToolId::Rustc),
        "cargo-audit" => Ok(ToolId::CargoAudit),
        "cargo-deny" => Ok(ToolId::CargoDeny),
        _ => Err(vec![diagnostic(
            "policy.tool.invalid",
            "Tool ID is not registered.",
        )]),
    }
}

fn parse_tool_operation(value: &str) -> Result<ToolOperation, Vec<Diagnostic>> {
    match value {
        "repository-root" => Ok(ToolOperation::RepositoryRoot),
        "ignore-check" => Ok(ToolOperation::IgnoreCheck),
        "scan-tracked" => Ok(ToolOperation::ScanTracked),
        "lint-message" => Ok(ToolOperation::LintMessage),
        "branch-rules" => Ok(ToolOperation::BranchRules),
        "check-runs" => Ok(ToolOperation::CheckRuns),
        "quality-suite" => Ok(ToolOperation::QualitySuite),
        "version" => Ok(ToolOperation::Version),
        "audit" => Ok(ToolOperation::Audit),
        "deny" => Ok(ToolOperation::Deny),
        _ => Err(vec![diagnostic(
            "policy.operation.invalid",
            "Tool operation is not registered.",
        )]),
    }
}

fn operation_allowed(tool: ToolId, operation: ToolOperation) -> bool {
    matches!(
        (tool, operation),
        (
            ToolId::Git,
            ToolOperation::RepositoryRoot | ToolOperation::IgnoreCheck
        ) | (ToolId::Gitleaks, ToolOperation::ScanTracked)
            | (ToolId::Commitlint, ToolOperation::LintMessage)
            | (
                ToolId::Gh,
                ToolOperation::BranchRules | ToolOperation::CheckRuns
            )
            | (ToolId::Cargo | ToolId::Npm, ToolOperation::QualitySuite)
            | (
                ToolId::Git
                    | ToolId::Gitleaks
                    | ToolId::Commitlint
                    | ToolId::Gh
                    | ToolId::Cargo
                    | ToolId::Npm
                    | ToolId::Node
                    | ToolId::Rustc
                    | ToolId::CargoAudit
                    | ToolId::CargoDeny,
                ToolOperation::Version
            )
            | (ToolId::CargoAudit, ToolOperation::Audit)
            | (ToolId::CargoDeny, ToolOperation::Deny)
    )
}

fn parse_check(raw: RawCheck) -> Result<Check, Vec<Diagnostic>> {
    match raw {
        RawCheck::GitRepository => Ok(Check::GitRepository),
        RawCheck::GitignorePatterns { path, patterns } => {
            validate_string_list(&patterns, "policy.check.invalid")?;
            Ok(Check::GitignorePatterns { path, patterns })
        }
        RawCheck::TrackedSecrets { include_history } => {
            Ok(Check::TrackedSecrets { include_history })
        }
        RawCheck::ReadmeSections { path, headings } => {
            validate_string_list(&headings, "policy.check.invalid")?;
            Ok(Check::ReadmeSections { path, headings })
        }
        RawCheck::ConventionalCommit => Ok(Check::ConventionalCommit),
        RawCheck::ProtectedMainLocal { branch } => {
            validate_text(&branch, 128, "policy.check.invalid")?;
            Ok(Check::ProtectedMainLocal { branch })
        }
        RawCheck::GithubBranchPolicy {
            branch,
            require_pull_request,
            required_checks,
            require_no_bypass,
        } => {
            validate_text(&branch, 128, "policy.check.invalid")?;
            validate_string_list(&required_checks, "policy.check.invalid")?;
            Ok(Check::GithubBranchPolicy {
                branch,
                require_pull_request,
                required_checks,
                require_no_bypass,
            })
        }
        RawCheck::CiContract {
            workflow_paths,
            required_jobs,
        } => {
            validate_bounded_len(workflow_paths.len(), "policy.check.invalid")?;
            validate_string_list(&required_jobs, "policy.check.invalid")?;
            Ok(Check::CiContract {
                workflow_paths,
                required_jobs,
            })
        }
        RawCheck::CiEvidence { required_checks } => {
            validate_string_list(&required_checks, "policy.check.invalid")?;
            Ok(Check::CiEvidence { required_checks })
        }
        RawCheck::ReleaseContract {
            require_changelog,
            require_checksums,
            require_signature,
        } => Ok(Check::ReleaseContract {
            require_changelog,
            require_checksums,
            require_signature,
        }),
        RawCheck::ToolchainVersion {
            tool_id,
            version_range,
        } => {
            let tool_id = parse_tool_id(&tool_id)?;
            let version_range = semver::VersionReq::parse(&version_range).map_err(|_| {
                vec![diagnostic(
                    "policy.tool.version.invalid",
                    "Tool version range must be valid SemVer.",
                )]
            })?;
            Ok(Check::ToolchainVersion {
                tool_id,
                version_range,
            })
        }
    }
}

fn validate_requirement_graph(requirements: &[Requirement]) -> Result<(), Vec<Diagnostic>> {
    let known: BTreeSet<&str> = requirements.iter().map(|item| item.id.as_str()).collect();
    let mut edges = BTreeMap::new();
    for item in requirements {
        let mut dependencies = BTreeSet::new();
        for dependency in &item.depends_on {
            if !known.contains(dependency.as_str()) || dependency == &item.id {
                return Err(vec![diagnostic(
                    "policy.requirement_reference.invalid",
                    "Requirement references an ID outside this policy.",
                )]);
            }
            if !dependencies.insert(dependency.as_str()) {
                return Err(vec![diagnostic(
                    "policy.requirement_reference.duplicate",
                    "Requirement contains a duplicate dependency.",
                )]);
            }
        }
        edges.insert(item.id.as_str(), dependencies);
    }
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for id in edges.keys().copied() {
        if has_cycle(id, &edges, &mut visiting, &mut visited) {
            return Err(vec![diagnostic(
                "policy.requirement_cycle",
                "Requirement dependency graph contains a cycle.",
            )]);
        }
    }
    Ok(())
}

fn has_cycle<'a>(
    id: &'a str,
    edges: &BTreeMap<&'a str, BTreeSet<&'a str>>,
    visiting: &mut BTreeSet<&'a str>,
    visited: &mut BTreeSet<&'a str>,
) -> bool {
    if visited.contains(id) {
        return false;
    }
    if !visiting.insert(id) {
        return true;
    }
    if edges.get(id).is_some_and(|dependencies| {
        dependencies
            .iter()
            .any(|dependency| has_cycle(dependency, edges, visiting, visited))
    }) {
        return true;
    }
    visiting.remove(id);
    visited.insert(id);
    false
}

fn validate_registered_tools(
    requirements: &[Requirement],
    available: &BTreeSet<(ToolId, ToolOperation)>,
) -> Result<(), Vec<Diagnostic>> {
    let required: &[ToolOperation] = &[];
    for requirement in requirements {
        let operations: &[ToolOperation] = match requirement.check {
            Check::GitRepository => &[ToolOperation::RepositoryRoot],
            Check::GitignorePatterns { .. } => &[ToolOperation::IgnoreCheck],
            Check::TrackedSecrets { .. } => &[ToolOperation::ScanTracked],
            Check::ConventionalCommit => &[ToolOperation::LintMessage],
            Check::GithubBranchPolicy { .. } => &[ToolOperation::BranchRules],
            Check::CiEvidence { .. } => &[ToolOperation::CheckRuns],
            Check::ToolchainVersion { tool_id, .. } => {
                if !available.contains(&(tool_id, ToolOperation::Version)) {
                    return Err(vec![diagnostic(
                        "policy.tool_reference.invalid",
                        "Check references a tool absent from the registry.",
                    )]);
                }
                required
            }
            _ => required,
        };
        for operation in operations {
            let tool = match operation {
                ToolOperation::RepositoryRoot | ToolOperation::IgnoreCheck => ToolId::Git,
                ToolOperation::ScanTracked => ToolId::Gitleaks,
                ToolOperation::LintMessage => ToolId::Commitlint,
                ToolOperation::BranchRules | ToolOperation::CheckRuns => ToolId::Gh,
                ToolOperation::QualitySuite => ToolId::Cargo,
                ToolOperation::Version | ToolOperation::Audit | ToolOperation::Deny => {
                    return Err(vec![diagnostic(
                        "policy.tool_reference.invalid",
                        "Check references an unsupported tool operation.",
                    )]);
                }
            };
            if !available.contains(&(tool, *operation)) {
                return Err(vec![diagnostic(
                    "policy.tool_reference.invalid",
                    "Check references a tool operation absent from the registry.",
                )]);
            }
        }
    }
    Ok(())
}

fn validate_profile(value: &str) -> Result<(), Vec<Diagnostic>> {
    if valid_slug(value) {
        Ok(())
    } else {
        Err(vec![diagnostic(
            "policy.profile.invalid",
            "Policy profile is invalid.",
        )])
    }
}

fn validate_text(value: &str, max_chars: usize, code: &'static str) -> Result<(), Vec<Diagnostic>> {
    if value.trim().is_empty()
        || value.chars().count() > max_chars
        || value.chars().any(char::is_control)
    {
        Err(vec![diagnostic(
            code,
            "Policy text is empty or exceeds its size limit.",
        )])
    } else {
        Ok(())
    }
}

fn validate_id_list(values: &[String], code: &'static str) -> Result<(), Vec<Diagnostic>> {
    validate_bounded_len(values.len(), code)?;
    let mut ids = BTreeSet::new();
    if values
        .iter()
        .any(|value| !valid_namespaced_id(value) || !ids.insert(value))
    {
        return Err(vec![diagnostic(
            code,
            "Policy contains an invalid or duplicate reference.",
        )]);
    }
    Ok(())
}

fn validate_string_list(values: &[String], code: &'static str) -> Result<(), Vec<Diagnostic>> {
    validate_bounded_len(values.len(), code)?;
    let mut seen = BTreeSet::new();
    if values.iter().any(|value| {
        value.trim().is_empty()
            || value.chars().count() > 256
            || value.chars().any(char::is_control)
            || !seen.insert(value)
    }) {
        return Err(vec![diagnostic(
            code,
            "Policy list contains an empty, oversized, or duplicate value.",
        )]);
    }
    Ok(())
}

fn validate_bounded_len(length: usize, code: &'static str) -> Result<(), Vec<Diagnostic>> {
    if length > MAX_LIST_ITEMS {
        Err(vec![diagnostic(
            code,
            "Policy list exceeds its size limit.",
        )])
    } else {
        Ok(())
    }
}

fn valid_namespaced_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= 128 && value.split('.').all(valid_slug)
}

fn valid_slug(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty()
        || bytes.len() > 64
        || !bytes[0].is_ascii_lowercase() && !bytes[0].is_ascii_digit()
    {
        return false;
    }
    let mut prior_hyphen = false;
    for (index, byte) in bytes.iter().copied().enumerate() {
        if byte == b'-' {
            if index == 0 || index + 1 == bytes.len() || prior_hyphen {
                return false;
            }
            prior_hyphen = true;
        } else if byte.is_ascii_lowercase() || byte.is_ascii_digit() {
            prior_hyphen = false;
        } else {
            return false;
        }
    }
    true
}

fn diagnostic(code: &'static str, message: &'static str) -> Diagnostic {
    Diagnostic::new(code, None, None, None, message, DiagnosticSeverity::Error)
}

fn located_diagnostic(
    code: &'static str,
    location: Option<(u32, u32)>,
    message: &'static str,
) -> Diagnostic {
    Diagnostic::new(
        code,
        None,
        location.map(|value| value.0),
        location.map(|value| value.1),
        message,
        DiagnosticSeverity::Error,
    )
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
