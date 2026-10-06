use super::policy::ApplicabilityFact;
use super::policy::{CheckEvidence, CheckReport, CheckResult, CheckStatus, ToolId, ToolOperation};
use crate::{Diagnostic, DiagnosticSeverity};
use semver::{Version, VersionReq};
use std::collections::{BTreeMap, BTreeSet};

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

#[derive(Clone, PartialEq, Eq)]
pub struct GuidanceFactObservation {
    fact: ApplicabilityFact,
    value: String,
    evidence: CheckEvidence,
}

impl GuidanceFactObservation {
    pub fn new(
        fact: ApplicabilityFact,
        value: &str,
        evidence: CheckEvidence,
    ) -> Result<Self, Vec<Diagnostic>> {
        if !fact.accepts_value(value) {
            return Err(guidance_diagnostic(
                "guidance.fact.invalid",
                "Guidance fact value is not registered.",
            ));
        }
        if evidence.expires_at_monotonic_ms().is_none() {
            return Err(guidance_diagnostic(
                "guidance.fact.expiration.missing",
                "Guidance facts must carry a process-local freshness deadline.",
            ));
        }
        Ok(Self {
            fact,
            value: value.to_owned(),
            evidence,
        })
    }

    pub fn fact(&self) -> ApplicabilityFact {
        self.fact
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn evidence(&self) -> &CheckEvidence {
        &self.evidence
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct GuidanceFacts {
    environment_fingerprint: String,
    observations: BTreeMap<ApplicabilityFact, GuidanceFactObservation>,
}

impl GuidanceFacts {
    pub fn new(
        environment_fingerprint: &str,
        observations: impl IntoIterator<Item = GuidanceFactObservation>,
    ) -> Result<Self, Vec<Diagnostic>> {
        if !valid_guidance_fingerprint(environment_fingerprint) {
            return Err(guidance_diagnostic(
                "guidance.fingerprint.invalid",
                "Guidance environment fingerprint is invalid.",
            ));
        }
        let mut by_fact = BTreeMap::new();
        for observation in observations {
            if by_fact.len() >= 6 {
                return Err(guidance_diagnostic(
                    "guidance.fact.limit",
                    "Guidance fact count exceeds the registered fact set.",
                ));
            }
            if observation.evidence.environment_fingerprint() != environment_fingerprint
                || by_fact.insert(observation.fact, observation).is_some()
            {
                return Err(guidance_diagnostic(
                    "guidance.fact.inconsistent",
                    "Guidance facts must be unique and share one environment fingerprint.",
                ));
            }
        }
        Ok(Self {
            environment_fingerprint: environment_fingerprint.to_owned(),
            observations: by_fact,
        })
    }

    pub fn environment_fingerprint(&self) -> &str {
        &self.environment_fingerprint
    }

    pub fn observations(
        &self,
    ) -> impl Iterator<Item = (&ApplicabilityFact, &GuidanceFactObservation)> {
        self.observations.iter()
    }

    fn fresh_value(&self, fact: ApplicabilityFact, now_monotonic_ms: u64) -> Option<&str> {
        let observation = self.observations.get(&fact)?;
        (!observation.evidence.is_expired_at(now_monotonic_ms)).then_some(observation.value())
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum GuidanceAnswer {
    Acknowledge,
    Choose(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuidanceProgressStatus {
    AwaitingFacts,
    AwaitingEvidence,
    AwaitingAnswer,
    Complete,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuidanceStepStatus {
    BlockedByDependency,
    AwaitingFacts,
    AwaitingEvidence,
    AwaitingAnswer,
    Completed,
    NotApplicable,
}

#[derive(Clone, PartialEq, Eq)]
pub struct GuidanceStepProgress {
    step_id: String,
    status: GuidanceStepStatus,
    verification_status: CheckStatus,
}

impl GuidanceStepProgress {
    pub fn step_id(&self) -> &str {
        &self.step_id
    }

    pub fn status(&self) -> GuidanceStepStatus {
        self.status
    }

    pub fn verification_status(&self) -> CheckStatus {
        self.verification_status
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct GuidanceDecision {
    status: GuidanceProgressStatus,
    next_step: Option<GuidanceStep>,
    steps: Vec<GuidanceStepProgress>,
}

impl GuidanceDecision {
    pub fn status(&self) -> GuidanceProgressStatus {
        self.status
    }

    pub fn next_step(&self) -> Option<&GuidanceStep> {
        self.next_step.as_ref()
    }

    pub fn steps(&self) -> &[GuidanceStepProgress] {
        &self.steps
    }
}

pub fn validate_guidance_graph(plan: &GuidancePlan) -> Result<(), Vec<Diagnostic>> {
    let mut plan_requirements = BTreeSet::new();
    if !valid_guidance_id(&plan.id)
        || plan.requirement_ids.is_empty()
        || plan
            .requirement_ids
            .iter()
            .any(|id| !valid_guidance_id(id) || !plan_requirements.insert(id.as_str()))
    {
        return Err(guidance_diagnostic(
            "guidance.plan.requirements.invalid",
            "Guidance plan requirements must be unique registered IDs.",
        ));
    }
    if plan.steps.is_empty() {
        return Err(guidance_diagnostic(
            "guidance.plan.steps.invalid",
            "Guidance plan must contain steps.",
        ));
    }
    let mut step_ids = BTreeSet::new();
    for step in &plan.steps {
        if !valid_guidance_id(&step.id)
            || !step_ids.insert(step.id.as_str())
            || step.prompt_es.trim().is_empty()
            || step.verification_requirement_ids.is_empty()
            || step
                .verification_requirement_ids
                .iter()
                .any(|id| !plan_requirements.contains(id.as_str()))
        {
            return Err(guidance_diagnostic(
                "guidance.step.invalid",
                "Guidance step IDs, prompt, or verification requirements are invalid.",
            ));
        }
        if let Some(condition) = &step.applies_when
            && !condition.fact.accepts_value(&condition.equals)
        {
            return Err(guidance_diagnostic(
                "guidance.condition.invalid",
                "Guidance condition is outside its registered fact values.",
            ));
        }
    }
    for step in &plan.steps {
        let mut dependencies = BTreeSet::new();
        if step
            .requires
            .iter()
            .any(|id| !step_ids.contains(id.as_str()) || !dependencies.insert(id.as_str()))
        {
            return Err(guidance_diagnostic(
                "guidance.step.dependencies.invalid",
                "Guidance step dependencies must be unique IDs in the plan.",
            ));
        }
    }
    if topological_steps(plan).is_none() {
        return Err(guidance_diagnostic(
            "guidance.graph.cycle",
            "Guidance step dependencies contain a cycle.",
        ));
    }
    Ok(())
}

pub fn next_step(
    plan: &GuidancePlan,
    facts: &GuidanceFacts,
    reports: &[CheckReport],
    answers: &BTreeMap<String, GuidanceAnswer>,
    now_monotonic_ms: u64,
) -> GuidanceDecision {
    if validate_guidance_graph(plan).is_err() {
        return GuidanceDecision {
            status: GuidanceProgressStatus::Unknown,
            next_step: None,
            steps: Vec::new(),
        };
    }
    let ordered_steps = topological_steps(plan).unwrap_or_default();
    let mut statuses = BTreeMap::new();
    let mut progress = Vec::with_capacity(ordered_steps.len());
    for step in &ordered_steps {
        let prerequisites_ready = step.requires.iter().all(|dependency| {
            matches!(
                statuses.get(dependency),
                Some(GuidanceStepStatus::Completed | GuidanceStepStatus::NotApplicable)
            )
        });
        let verification_status = verification_status(
            &step.verification_requirement_ids,
            facts.environment_fingerprint(),
            reports,
            now_monotonic_ms,
        );
        let status = if let Some(condition) = &step.applies_when {
            match facts.fresh_value(condition.fact, now_monotonic_ms) {
                None if prerequisites_ready => GuidanceStepStatus::AwaitingFacts,
                None => GuidanceStepStatus::BlockedByDependency,
                Some(value) if value != condition.equals => GuidanceStepStatus::NotApplicable,
                Some(_) => step_progress_status(step, &statuses, answers, verification_status),
            }
        } else {
            step_progress_status(step, &statuses, answers, verification_status)
        };
        statuses.insert(step.id.clone(), status);
        progress.push(GuidanceStepProgress {
            step_id: step.id.clone(),
            status,
            verification_status,
        });
    }
    let complete = progress.iter().all(|step| {
        matches!(
            step.status,
            GuidanceStepStatus::Completed | GuidanceStepStatus::NotApplicable
        )
    });
    let next = progress
        .iter()
        .find(|step| {
            matches!(
                step.status,
                GuidanceStepStatus::AwaitingFacts
                    | GuidanceStepStatus::AwaitingEvidence
                    | GuidanceStepStatus::AwaitingAnswer
            )
        })
        .and_then(|progress| {
            ordered_steps
                .iter()
                .find(|step| step.id == progress.step_id)
        })
        .cloned();
    let status = if complete {
        GuidanceProgressStatus::Complete
    } else if let Some(next) = &next {
        match statuses.get(&next.id) {
            Some(GuidanceStepStatus::AwaitingFacts) => GuidanceProgressStatus::AwaitingFacts,
            Some(GuidanceStepStatus::AwaitingAnswer) => GuidanceProgressStatus::AwaitingAnswer,
            Some(GuidanceStepStatus::AwaitingEvidence) => GuidanceProgressStatus::AwaitingEvidence,
            _ => GuidanceProgressStatus::Unknown,
        }
    } else {
        GuidanceProgressStatus::Unknown
    };
    GuidanceDecision {
        status,
        next_step: next.cloned(),
        steps: progress,
    }
}

fn step_progress_status(
    step: &GuidanceStep,
    statuses: &BTreeMap<String, GuidanceStepStatus>,
    answers: &BTreeMap<String, GuidanceAnswer>,
    verification_status: CheckStatus,
) -> GuidanceStepStatus {
    if step.requires.iter().any(|dependency| {
        !matches!(
            statuses.get(dependency),
            Some(GuidanceStepStatus::Completed | GuidanceStepStatus::NotApplicable)
        )
    }) {
        return GuidanceStepStatus::BlockedByDependency;
    }
    if let GuidanceAction::AnswerChoice { choices } = &step.action {
        let chosen = answers.get(&step.id).is_some_and(|answer| match answer {
            GuidanceAnswer::Choose(choice) => choices.contains(choice),
            GuidanceAnswer::Acknowledge => false,
        });
        if !chosen {
            return GuidanceStepStatus::AwaitingAnswer;
        }
    }
    if verification_status == CheckStatus::Pass {
        GuidanceStepStatus::Completed
    } else {
        GuidanceStepStatus::AwaitingEvidence
    }
}

fn verification_status(
    requirement_ids: &[String],
    environment_fingerprint: &str,
    reports: &[CheckReport],
    now_monotonic_ms: u64,
) -> CheckStatus {
    let mut statuses = Vec::with_capacity(requirement_ids.len());
    for requirement_id in requirement_ids {
        let mut matches = reports
            .iter()
            .flat_map(|report| report.results())
            .filter(|result| result.requirement_id() == requirement_id);
        let Some(result) = matches.next() else {
            statuses.push(CheckStatus::Unknown);
            continue;
        };
        if matches.next().is_some() {
            statuses.push(CheckStatus::Unknown);
            continue;
        }
        statuses.push(check_result_status(
            result,
            environment_fingerprint,
            now_monotonic_ms,
        ));
    }
    if statuses.contains(&CheckStatus::Fail) {
        CheckStatus::Fail
    } else if statuses.contains(&CheckStatus::Blocked) {
        CheckStatus::Blocked
    } else if statuses.iter().all(|status| *status == CheckStatus::Pass) {
        CheckStatus::Pass
    } else {
        CheckStatus::Unknown
    }
}

fn check_result_status(
    result: &CheckResult,
    environment_fingerprint: &str,
    now_monotonic_ms: u64,
) -> CheckStatus {
    if result.evidence().is_empty()
        || result.evidence().iter().any(|evidence| {
            evidence.environment_fingerprint() != environment_fingerprint
                || evidence.is_expired_at(now_monotonic_ms)
        })
    {
        CheckStatus::Unknown
    } else {
        match result.status() {
            CheckStatus::Pass => CheckStatus::Pass,
            CheckStatus::Fail => CheckStatus::Fail,
            CheckStatus::Blocked => CheckStatus::Blocked,
            CheckStatus::Unknown | CheckStatus::Unsupported | CheckStatus::NotApplicable => {
                CheckStatus::Unknown
            }
        }
    }
}

fn topological_steps(plan: &GuidancePlan) -> Option<Vec<&GuidanceStep>> {
    let all_steps = plan
        .steps
        .iter()
        .map(|step| step.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut remaining = all_steps.clone();
    if remaining.len() != plan.steps.len() {
        return None;
    }
    if plan
        .steps
        .iter()
        .flat_map(|step| &step.requires)
        .any(|dependency| !all_steps.contains(dependency.as_str()))
    {
        return None;
    }
    let mut ordered = Vec::with_capacity(plan.steps.len());
    while !remaining.is_empty() {
        let ready = plan
            .steps
            .iter()
            .filter(|step| remaining.contains(step.id.as_str()))
            .filter(|step| {
                step.requires
                    .iter()
                    .all(|dependency| !remaining.contains(dependency.as_str()))
            })
            .collect::<Vec<_>>();
        if ready.is_empty() {
            return None;
        }
        for step in ready {
            remaining.remove(step.id.as_str());
            ordered.push(step);
        }
    }
    Some(ordered)
}

fn valid_guidance_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.split('.').all(|component| {
            !component.is_empty()
                && component
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
                && !component.starts_with('-')
                && !component.ends_with('-')
        })
}

fn valid_guidance_fingerprint(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn guidance_diagnostic(code: &'static str, message: &'static str) -> Vec<Diagnostic> {
    vec![Diagnostic::new(
        code,
        None,
        None,
        None,
        message,
        DiagnosticSeverity::Error,
    )]
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
