use async_trait::async_trait;
use jameskills_core::{
    AppError, AppResult,
    application::policy::{CheckContext, CheckRequest, PolicyCheckProvider, PolicyService},
    domain::policy::{
        ApplicabilityFact, CheckEvidence, CheckObservation, CheckResult, CheckStatus, Enforcement,
        Policy, Requirement, TestSuiteDeclaration, TestSuiteExecution, TestSuiteKind,
        TestSuiteRunResult, parse_policy, strict_exit,
    },
    ports::ClockPort,
};
use std::{
    collections::BTreeMap,
    future::Future,
    sync::{Arc, Mutex},
    task::{Context, Poll, Wake, Waker},
};

const POLICY: &str = r#"
schema_version = 1
profile = "evaluation-fixture"

[[tool_requirements]]
tool_id = "commitlint"
operation = "lint-message"
version = ">=19.0.0"

[[requirements]]
id = "policy.commit"
description = "Commit messages meet the declared convention."
severity = "error"
required = true
phase = "commit"
enforcement = "local-hook"
depends_on = []
[requirements.check]
kind = "conventional-commit"

[[requirements]]
id = "policy.ci"
description = "The declared workflow is required by the host."
severity = "error"
required = true
phase = "pull-request"
enforcement = "required-ci"
depends_on = []
[requirements.check]
kind = "ci-contract"
workflow_paths = [".github/workflows/ci.yml"]
required_jobs = ["quality"]

[[requirements]]
id = "policy.advisory"
description = "An optional advisory condition."
severity = "warning"
required = false
phase = "ci"
enforcement = "local-check"
depends_on = []
[requirements.check]
kind = "protected-main-local"
branch = "main"
"#;

const NOW: &str = "2026-10-04T12:00:00Z";
const ENVIRONMENT: &str = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

struct FakeClock {
    monotonic_ms: u64,
}

impl ClockPort for FakeClock {
    fn now_utc(&self) -> String {
        NOW.to_owned()
    }

    fn monotonic_ms(&self) -> u64 {
        self.monotonic_ms
    }
}

#[derive(Default)]
struct FakeChecks {
    observations: BTreeMap<String, CheckObservation>,
    calls: Mutex<Vec<String>>,
}

impl FakeChecks {
    fn with(mut self, id: &str, observation: CheckObservation) -> Self {
        self.observations.insert(id.to_owned(), observation);
        self
    }
}

#[async_trait]
impl PolicyCheckProvider for FakeChecks {
    async fn observe(&self, requirement: &Requirement) -> AppResult<CheckObservation> {
        self.calls.lock().unwrap().push(requirement.id().to_owned());
        Ok(self
            .observations
            .get(requirement.id())
            .cloned()
            .unwrap_or_else(CheckObservation::unknown))
    }
}

struct CancelledChecks;

#[async_trait]
impl PolicyCheckProvider for CancelledChecks {
    async fn observe(&self, _requirement: &Requirement) -> AppResult<CheckObservation> {
        Err(AppError::Cancelled)
    }
}

fn policy() -> Policy {
    parse_policy(POLICY.as_bytes()).unwrap()
}

fn evidence(source_id: &str, expires_at_ms: Option<u64>) -> CheckEvidence {
    CheckEvidence::new(
        source_id,
        NOW,
        None,
        ENVIRONMENT,
        "Registered check completed.",
        expires_at_ms,
    )
    .unwrap()
}

fn observation(status: CheckStatus, enforcement: Enforcement) -> CheckObservation {
    CheckObservation::new(
        status,
        Some(enforcement),
        vec![evidence("policy.check.observation", Some(100))],
    )
    .unwrap()
}

fn service(checks: Arc<FakeChecks>, monotonic_ms: u64) -> PolicyService {
    PolicyService::new(checks, Arc::new(FakeClock { monotonic_ms }))
}

struct NoopWake;

impl Wake for NoopWake {
    fn wake(self: Arc<Self>) {}
}

fn block_on<F: Future>(future: F) -> F::Output {
    let waker = Waker::from(Arc::new(NoopWake));
    let mut context = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

fn result<'a>(results: &'a [CheckResult], id: &str) -> &'a CheckResult {
    results
        .iter()
        .find(|result| result.requirement_id() == id)
        .unwrap()
}

#[test]
fn evidence_accepts_the_rfc3339_millisecond_clock_format() {
    assert!(
        CheckEvidence::new(
            "policy.check.clock",
            "2026-10-04T12:00:00.123Z",
            None,
            ENVIRONMENT,
            "Clock evidence.",
            None,
        )
        .is_ok()
    );
}

#[test]
fn weaker_observed_authority_blocks_pass_and_optional_fail_does_not_block_strict() {
    let checks = Arc::new(
        FakeChecks::default()
            .with(
                "policy.commit",
                observation(CheckStatus::Pass, Enforcement::LocalCheck),
            )
            .with(
                "policy.ci",
                observation(CheckStatus::Pass, Enforcement::LocalCheck),
            )
            .with(
                "policy.advisory",
                observation(CheckStatus::Fail, Enforcement::LocalCheck),
            ),
    );
    let report =
        block_on(service(checks, 50).check(CheckRequest::new(policy(), CheckContext::default())))
            .unwrap();

    assert_eq!(
        result(report.results(), "policy.commit").status(),
        CheckStatus::Blocked
    );
    assert_eq!(
        result(report.results(), "policy.ci").status(),
        CheckStatus::Blocked
    );
    assert!(
        result(report.results(), "policy.commit").enforcement() == Some(Enforcement::LocalCheck)
    );
    assert_eq!(
        result(report.results(), "policy.advisory").status(),
        CheckStatus::Fail
    );
    assert_eq!(strict_exit(&report), 1);
}

#[test]
fn only_required_passes_allow_strict_exit_and_observed_authority_is_not_copied() {
    let checks = Arc::new(
        FakeChecks::default()
            .with(
                "policy.commit",
                observation(CheckStatus::Pass, Enforcement::LocalHook),
            )
            .with(
                "policy.ci",
                observation(CheckStatus::Pass, Enforcement::RequiredCi),
            )
            .with(
                "policy.advisory",
                observation(CheckStatus::Fail, Enforcement::LocalCheck),
            ),
    );
    let report =
        block_on(service(checks, 50).check(CheckRequest::new(policy(), CheckContext::default())))
            .unwrap();

    assert_eq!(
        result(report.results(), "policy.commit").status(),
        CheckStatus::Pass
    );
    assert_eq!(
        result(report.results(), "policy.ci").status(),
        CheckStatus::Pass
    );
    assert_eq!(
        result(report.results(), "policy.advisory").status(),
        CheckStatus::Fail
    );
    assert!(
        result(report.results(), "policy.commit").enforcement() == Some(Enforcement::LocalHook)
    );
    assert_eq!(strict_exit(&report), 0);
}

#[test]
fn expired_evidence_turns_a_passing_predicate_into_unknown() {
    let expired = CheckEvidence::new(
        "policy.check.expired",
        NOW,
        None,
        ENVIRONMENT,
        "Previously observed check.",
        Some(40),
    )
    .unwrap();
    let checks = Arc::new(
        FakeChecks::default().with(
            "policy.commit",
            CheckObservation::new(
                CheckStatus::Pass,
                Some(Enforcement::LocalHook),
                vec![expired],
            )
            .unwrap(),
        ),
    );
    let report =
        block_on(service(checks, 50).check(CheckRequest::new(policy(), CheckContext::default())))
            .unwrap();

    assert_eq!(
        result(report.results(), "policy.commit").status(),
        CheckStatus::Unknown
    );
    assert_eq!(strict_exit(&report), 1);
}

#[test]
fn not_applicable_requires_a_matching_profile_condition_and_fresh_fact() {
    let source = POLICY.replace(
        "id = \"policy.ci\"",
        "id = \"policy.ci\"\napplies_when = { fact = \"stack\", equals = \"node\" }",
    );
    let policy = parse_policy(source.as_bytes()).unwrap();
    let fact_evidence = evidence("environment.stack", Some(100));
    let context = CheckContext::default()
        .with_fact(ApplicabilityFact::Stack, "rust", fact_evidence)
        .unwrap();
    let checks = Arc::new(
        FakeChecks::default()
            .with(
                "policy.commit",
                observation(CheckStatus::Pass, Enforcement::LocalHook),
            )
            .with(
                "policy.advisory",
                observation(CheckStatus::Fail, Enforcement::LocalCheck),
            ),
    );
    let report =
        block_on(service(Arc::clone(&checks), 50).check(CheckRequest::new(policy, context)))
            .unwrap();

    assert_eq!(
        result(report.results(), "policy.ci").status(),
        CheckStatus::NotApplicable
    );
    assert!(
        !checks
            .calls
            .lock()
            .unwrap()
            .contains(&"policy.ci".to_owned())
    );
    assert_eq!(strict_exit(&report), 0);
}

#[test]
fn missing_applicability_fact_remains_unknown_not_not_applicable() {
    let source = POLICY.replace(
        "id = \"policy.ci\"",
        "id = \"policy.ci\"\napplies_when = { fact = \"stack\", equals = \"node\" }",
    );
    let checks = Arc::new(FakeChecks::default());
    let report = block_on(service(Arc::clone(&checks), 50).check(CheckRequest::new(
        parse_policy(source.as_bytes()).unwrap(),
        CheckContext::default(),
    )))
    .unwrap();

    assert_eq!(
        result(report.results(), "policy.ci").status(),
        CheckStatus::Unknown
    );
    assert!(
        !checks
            .calls
            .lock()
            .unwrap()
            .contains(&"policy.ci".to_owned())
    );
    assert_eq!(strict_exit(&report), 1);
}

#[test]
fn unavailable_observation_is_not_reported_as_success() {
    let checks = Arc::new(FakeChecks::default().with("policy.commit", CheckObservation::unknown()));
    let report =
        block_on(service(checks, 50).check(CheckRequest::new(policy(), CheckContext::default())))
            .unwrap();
    assert_eq!(
        result(report.results(), "policy.commit").status(),
        CheckStatus::Unknown
    );
}

#[test]
fn explicit_provider_blocked_status_remains_blocked() {
    let blocked = CheckObservation::new(
        CheckStatus::Blocked,
        Some(Enforcement::LocalHook),
        vec![evidence("policy.check.permission", Some(100))],
    )
    .unwrap();
    let checks = Arc::new(FakeChecks::default().with("policy.commit", blocked));
    let report =
        block_on(service(checks, 50).check(CheckRequest::new(policy(), CheckContext::default())))
            .unwrap();

    assert_eq!(
        result(report.results(), "policy.commit").status(),
        CheckStatus::Blocked
    );
    assert_eq!(strict_exit(&report), 1);
}

#[test]
fn provider_cancellation_is_propagated_not_rewritten_as_unknown() {
    let service = PolicyService::new(
        Arc::new(CancelledChecks),
        Arc::new(FakeClock { monotonic_ms: 50 }),
    );
    let result = block_on(service.check(CheckRequest::new(policy(), CheckContext::default())));

    assert!(matches!(result, Err(AppError::Cancelled)));
}

#[test]
fn passing_predicate_without_evidence_is_unknown_and_has_no_observed_authority() {
    let observation =
        CheckObservation::new(CheckStatus::Pass, Some(Enforcement::LocalHook), Vec::new()).unwrap();
    let checks = Arc::new(FakeChecks::default().with("policy.commit", observation));
    let report =
        block_on(service(checks, 50).check(CheckRequest::new(policy(), CheckContext::default())))
            .unwrap();

    assert_eq!(
        result(report.results(), "policy.commit").status(),
        CheckStatus::Unknown
    );
    assert!(
        result(report.results(), "policy.commit")
            .enforcement()
            .is_none()
    );
}

#[test]
fn suite_run_result_keeps_declaration_separate_from_exit_status() {
    let passed = TestSuiteRunResult::new(
        TestSuiteKind::CargoTest,
        TestSuiteDeclaration::Declared,
        TestSuiteExecution::Passed,
        Some(0),
    )
    .unwrap();
    assert!(matches!(
        passed.declaration(),
        TestSuiteDeclaration::Declared
    ));
    assert!(matches!(passed.execution(), TestSuiteExecution::Passed));
    assert_eq!(passed.exit_code(), Some(0));

    let missing = TestSuiteRunResult::new(
        TestSuiteKind::NodeTest,
        TestSuiteDeclaration::Missing,
        TestSuiteExecution::NotRun,
        None,
    )
    .unwrap();
    assert!(matches!(
        missing.declaration(),
        TestSuiteDeclaration::Missing
    ));
    assert!(matches!(missing.execution(), TestSuiteExecution::NotRun));
    assert_eq!(missing.exit_code(), None);

    let failed = TestSuiteRunResult::new(
        TestSuiteKind::CargoTest,
        TestSuiteDeclaration::Declared,
        TestSuiteExecution::Failed,
        Some(1),
    )
    .unwrap();
    assert!(matches!(failed.execution(), TestSuiteExecution::Failed));
    assert_eq!(failed.exit_code(), Some(1));

    let unknown_declaration = TestSuiteRunResult::new(
        TestSuiteKind::NodeTest,
        TestSuiteDeclaration::Unknown,
        TestSuiteExecution::Passed,
        Some(0),
    )
    .unwrap();
    assert!(matches!(
        unknown_declaration.declaration(),
        TestSuiteDeclaration::Unknown
    ));
    assert!(matches!(
        unknown_declaration.execution(),
        TestSuiteExecution::Passed
    ));
}

#[test]
fn suite_run_result_rejects_exit_and_execution_status_mismatches() {
    assert!(
        TestSuiteRunResult::new(
            TestSuiteKind::CargoTest,
            TestSuiteDeclaration::Declared,
            TestSuiteExecution::Passed,
            Some(1),
        )
        .is_err()
    );
    assert!(
        TestSuiteRunResult::new(
            TestSuiteKind::NodeTest,
            TestSuiteDeclaration::Missing,
            TestSuiteExecution::Passed,
            Some(0),
        )
        .is_err()
    );
    assert!(
        TestSuiteRunResult::new(
            TestSuiteKind::CargoTest,
            TestSuiteDeclaration::Declared,
            TestSuiteExecution::NotRun,
            Some(0),
        )
        .is_err()
    );
}
