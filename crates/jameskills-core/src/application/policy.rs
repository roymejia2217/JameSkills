use crate::{
    AppError, AppResult, ContentHash, Diagnostic, OperationId,
    domain::{
        RepositoryBinding, RepositoryBindingCheck, RepositoryBindingEvidence,
        RepositoryBindingReport,
        policy::{
            ApplicabilityFact, CheckEvidence, CheckObservation, CheckReport, CheckStatus, Policy,
            RepositoryHead, Requirement, TestSuiteDeclaration, TestSuiteKind, TestSuiteRunResult,
            evaluate_predicate, not_applicable_result,
        },
    },
    ports::{
        ClockPort,
        process::{ApprovedRoot, CancellationToken},
    },
};
use async_trait::async_trait;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Clone, Default)]
pub struct CheckContext {
    facts: BTreeMap<ApplicabilityFact, ApplicabilityObservation>,
}

impl CheckContext {
    pub fn with_fact(
        mut self,
        fact: ApplicabilityFact,
        value: &str,
        evidence: CheckEvidence,
    ) -> Result<Self, Vec<Diagnostic>> {
        if !fact.accepts_value(value) {
            return Err(vec![Diagnostic::error(
                "policy.context.fact.invalid",
                "Policy context fact value is not registered.",
            )]);
        }
        self.facts.insert(
            fact,
            ApplicabilityObservation {
                value: value.to_owned(),
                evidence,
            },
        );
        Ok(self)
    }
}

#[derive(Clone)]
struct ApplicabilityObservation {
    value: String,
    evidence: CheckEvidence,
}

pub struct CheckRequest {
    policy: Policy,
    context: CheckContext,
}

/// Inputs gathered by the binding coordinator before evaluating a suite. An
/// absent observation/report is explicit: stale results are retained, never
/// converted into a successful empty report.
pub struct RepositoryBindingEvaluationRequest {
    binding: RepositoryBinding,
    observed_binding: Option<RepositoryBinding>,
    observation_failure: Option<RepositoryBindingStaleReason>,
    current_suite_revision: Option<crate::domain::RevisionId>,
    policies: Option<Vec<Policy>>,
    context: CheckContext,
    previous_report: Option<RepositoryBindingReport>,
    observed_at: String,
}

impl RepositoryBindingEvaluationRequest {
    pub fn new(
        binding: RepositoryBinding,
        observed_binding: Option<RepositoryBinding>,
        current_suite_revision: Option<crate::domain::RevisionId>,
        policies: Option<Vec<Policy>>,
        context: CheckContext,
        previous_report: Option<RepositoryBindingReport>,
        observed_at: String,
    ) -> Self {
        Self {
            binding,
            observed_binding,
            observation_failure: None,
            current_suite_revision,
            policies,
            context,
            previous_report,
            observed_at,
        }
    }

    pub fn with_observation_failure(mut self, reason: RepositoryBindingStaleReason) -> Self {
        self.observation_failure = Some(reason);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepositoryBindingStaleReason {
    RepositoryUnavailable,
    RepositoryMoved,
    GitApprovalRequired,
    ProfileChanged,
    EnvironmentChanged,
    SuiteRevisionChanged,
    EvaluationUnavailable,
    BindingChangedDuringEvaluation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepositoryBindingNextStep {
    LocateRepository,
    RebindRepository,
    ApproveGit,
    ReviewProfile,
    ReconfirmEnvironment,
    SelectCurrentSuiteRevision,
    RetryChecks,
    RefreshBinding,
}

#[derive(Clone, PartialEq, Eq)]
pub struct RepositoryBindingEvaluation {
    binding: RepositoryBinding,
    report: Option<RepositoryBindingReport>,
    stale_reason: Option<RepositoryBindingStaleReason>,
}

impl RepositoryBindingEvaluation {
    pub fn stale(
        binding: RepositoryBinding,
        report: Option<RepositoryBindingReport>,
        reason: RepositoryBindingStaleReason,
    ) -> Self {
        Self {
            binding,
            report,
            stale_reason: Some(reason),
        }
    }

    pub fn is_stale(&self) -> bool {
        self.stale_reason.is_some()
    }

    pub fn stale_reason(&self) -> Option<RepositoryBindingStaleReason> {
        self.stale_reason
    }

    pub fn next_step(&self) -> Option<RepositoryBindingNextStep> {
        self.stale_reason.map(|reason| match reason {
            RepositoryBindingStaleReason::RepositoryUnavailable => {
                RepositoryBindingNextStep::LocateRepository
            }
            RepositoryBindingStaleReason::RepositoryMoved => {
                RepositoryBindingNextStep::RebindRepository
            }
            RepositoryBindingStaleReason::GitApprovalRequired => {
                RepositoryBindingNextStep::ApproveGit
            }
            RepositoryBindingStaleReason::ProfileChanged => {
                RepositoryBindingNextStep::ReviewProfile
            }
            RepositoryBindingStaleReason::EnvironmentChanged => {
                RepositoryBindingNextStep::ReconfirmEnvironment
            }
            RepositoryBindingStaleReason::SuiteRevisionChanged => {
                RepositoryBindingNextStep::SelectCurrentSuiteRevision
            }
            RepositoryBindingStaleReason::EvaluationUnavailable => {
                RepositoryBindingNextStep::RetryChecks
            }
            RepositoryBindingStaleReason::BindingChangedDuringEvaluation => {
                RepositoryBindingNextStep::RefreshBinding
            }
        })
    }

    pub fn binding(&self) -> &RepositoryBinding {
        &self.binding
    }

    pub fn report(&self) -> Option<&RepositoryBindingReport> {
        self.report.as_ref()
    }
}

/// Read-only preview of the selected suite and repository snapshot.
pub struct TestSuiteSnapshot {
    root: ApprovedRoot,
    suite: TestSuiteKind,
    expected_head: RepositoryHead,
    manifest_fingerprint: ContentHash,
    declaration: TestSuiteDeclaration,
}

impl TestSuiteSnapshot {
    pub fn new(
        root: ApprovedRoot,
        suite: TestSuiteKind,
        expected_head: RepositoryHead,
        manifest_fingerprint: ContentHash,
        declaration: TestSuiteDeclaration,
    ) -> Self {
        Self {
            root,
            suite,
            expected_head,
            manifest_fingerprint,
            declaration,
        }
    }

    pub fn root(&self) -> &ApprovedRoot {
        &self.root
    }

    pub fn suite(&self) -> TestSuiteKind {
        self.suite
    }

    pub fn expected_head(&self) -> &RepositoryHead {
        &self.expected_head
    }

    pub fn manifest_fingerprint(&self) -> &ContentHash {
        &self.manifest_fingerprint
    }

    pub fn declaration(&self) -> TestSuiteDeclaration {
        self.declaration
    }
}

/// One-time, non-deserializable approval created only after the user trusts
/// the selected repository and confirms this suite action.
pub struct TestSuiteRunApproval {
    operation_id: OperationId,
    snapshot: TestSuiteSnapshot,
}

impl TestSuiteRunApproval {
    pub fn after_explicit_trust_confirmation(snapshot: TestSuiteSnapshot) -> Self {
        Self {
            operation_id: OperationId::new(),
            snapshot,
        }
    }

    pub fn operation_id(&self) -> OperationId {
        self.operation_id
    }

    pub fn snapshot(&self) -> &TestSuiteSnapshot {
        &self.snapshot
    }
}

impl CheckRequest {
    pub fn new(policy: Policy, context: CheckContext) -> Self {
        Self { policy, context }
    }
}

/// Provider for facts and checks that require infrastructure or host access.
/// An unavailable provider must return Unknown with no asserted enforcement.
#[async_trait]
pub trait PolicyCheckProvider: Send + Sync {
    async fn observe(&self, requirement: &Requirement) -> AppResult<CheckObservation>;
}

#[async_trait]
pub trait TestSuiteRunnerPort: Send + Sync {
    async fn inspect(
        &self,
        suite: TestSuiteKind,
        cancellation: CancellationToken,
    ) -> AppResult<TestSuiteSnapshot>;
    async fn run(
        &self,
        approval: TestSuiteRunApproval,
        cancellation: CancellationToken,
    ) -> AppResult<TestSuiteRunResult>;
}

pub struct TestSuiteService {
    runner: Arc<dyn TestSuiteRunnerPort>,
}

impl TestSuiteService {
    pub fn new(runner: Arc<dyn TestSuiteRunnerPort>) -> Self {
        Self { runner }
    }

    /// Read-only suite preview; it must not build or execute project code.
    pub async fn inspect(
        &self,
        suite: TestSuiteKind,
        cancellation: CancellationToken,
    ) -> AppResult<TestSuiteSnapshot> {
        self.runner.inspect(suite, cancellation).await
    }

    /// Run is separate from preview and requires the user-confirmed approval.
    pub async fn run(
        &self,
        approval: TestSuiteRunApproval,
        cancellation: CancellationToken,
    ) -> AppResult<TestSuiteRunResult> {
        self.runner.run(approval, cancellation).await
    }
}

pub struct PolicyService {
    checks: Arc<dyn PolicyCheckProvider>,
    clock: Arc<dyn ClockPort>,
}

impl PolicyService {
    pub fn new(checks: Arc<dyn PolicyCheckProvider>, clock: Arc<dyn ClockPort>) -> Self {
        Self { checks, clock }
    }

    pub async fn check(&self, request: CheckRequest) -> AppResult<CheckReport> {
        let now = self.clock.monotonic_ms();
        let mut results = Vec::with_capacity(request.policy.requirements().len());
        let mut required_ids = BTreeSet::new();
        for requirement in request.policy.requirements() {
            if requirement.required() {
                required_ids.insert(requirement.id().to_owned());
            }
            if let Some(condition) = requirement.applies_when() {
                let Some(observed) = request.context.facts.get(&condition.fact()) else {
                    results.push(evaluate_predicate(
                        requirement,
                        &CheckObservation::unknown(),
                        now,
                    ));
                    continue;
                };
                if observed.value != condition.equals() {
                    results.push(not_applicable_result(
                        requirement,
                        observed.evidence.clone(),
                        now,
                    ));
                    continue;
                }
                if observed.evidence.is_expired_at(now) {
                    let stale_fact = CheckObservation::new(
                        CheckStatus::Unknown,
                        None,
                        vec![observed.evidence.clone()],
                    )
                    .map_err(AppError::Validation)?;
                    results.push(evaluate_predicate(requirement, &stale_fact, now));
                    continue;
                }
                let observation = self.checks.observe(requirement).await?;
                let mut evidence = observation.evidence().to_vec();
                evidence.push(observed.evidence.clone());
                let combined = CheckObservation::new(
                    observation.status(),
                    observation.enforcement(),
                    evidence,
                )
                .map_err(AppError::Validation)?;
                results.push(evaluate_predicate(requirement, &combined, now));
            } else {
                results.push(evaluate_predicate(
                    requirement,
                    &self.checks.observe(requirement).await?,
                    now,
                ));
            }
        }
        Ok(CheckReport::new(results, required_ids))
    }

    /// Evaluates every typed policy in one validated suite revision and
    /// combines results without allowing duplicate requirement identifiers.
    pub async fn check_suite(
        &self,
        policies: Vec<Policy>,
        context: CheckContext,
    ) -> AppResult<CheckReport> {
        if policies.is_empty() {
            return Err(AppError::Validation(vec![Diagnostic::error(
                "policy.suite.empty",
                "A repository binding requires at least one policy in its suite revision.",
            )]));
        }
        let mut results = Vec::new();
        let mut required_ids = BTreeSet::new();
        let mut seen = BTreeSet::new();
        for policy in policies {
            let report = self
                .check(CheckRequest::new(policy, context.clone()))
                .await?;
            for result in report.results() {
                if !seen.insert(result.requirement_id().to_owned()) {
                    return Err(AppError::Validation(vec![Diagnostic::error(
                        "policy.suite.requirement.duplicate",
                        "Requirement identifiers must be unique across suite policies.",
                    )]));
                }
                results.push(result.clone());
            }
            required_ids.extend(report.required_ids().iter().cloned());
        }
        Ok(CheckReport::new(results, required_ids))
    }

    /// Evaluates only when the selected suite revision and repository
    /// environment still match the saved binding. HEAD-only changes trigger a
    /// fresh check; unavailable or moved bindings retain their prior report as
    /// explicitly stale.
    pub async fn evaluate_binding(
        &self,
        request: RepositoryBindingEvaluationRequest,
    ) -> AppResult<RepositoryBindingEvaluation> {
        let stale = |reason, report| RepositoryBindingEvaluation {
            binding: request.binding.clone(),
            report,
            stale_reason: Some(reason),
        };
        if request.current_suite_revision.as_ref() != Some(request.binding.suite_revision()) {
            return Ok(stale(
                RepositoryBindingStaleReason::SuiteRevisionChanged,
                request.previous_report,
            ));
        }
        if let Some(reason) = request.observation_failure {
            return Ok(stale(reason, request.previous_report));
        }
        let Some(observed) = request.observed_binding.as_ref() else {
            return Ok(stale(
                RepositoryBindingStaleReason::RepositoryUnavailable,
                request.previous_report,
            ));
        };
        if observed.skill_id() != request.binding.skill_id()
            || observed.suite_revision() != request.binding.suite_revision()
        {
            return Ok(stale(
                RepositoryBindingStaleReason::SuiteRevisionChanged,
                request.previous_report,
            ));
        }
        if observed.repository_root() != request.binding.repository_root() {
            return Ok(stale(
                RepositoryBindingStaleReason::RepositoryMoved,
                request.previous_report,
            ));
        }
        if observed.profile() != request.binding.profile()
            || observed.strict() != request.binding.strict()
        {
            return Ok(stale(
                RepositoryBindingStaleReason::ProfileChanged,
                request.previous_report,
            ));
        }
        if observed.environment_fingerprint() != request.binding.environment_fingerprint() {
            return Ok(stale(
                RepositoryBindingStaleReason::EnvironmentChanged,
                request.previous_report,
            ));
        }
        let Some(policies) = request.policies else {
            return Ok(stale(
                RepositoryBindingStaleReason::EvaluationUnavailable,
                request.previous_report,
            ));
        };
        let binding = RepositoryBinding::from_storage(
            request.binding.id().to_owned(),
            request.binding.skill_id(),
            request.binding.suite_revision().clone(),
            observed.repository_root().to_path_buf(),
            observed.profile(),
            observed.strict(),
            observed.repository_head().clone(),
            observed.environment_fingerprint().to_owned(),
        )?;
        let report = match self.check_suite(policies, request.context).await {
            Ok(report) => report,
            Err(_) => {
                return Ok(stale(
                    RepositoryBindingStaleReason::EvaluationUnavailable,
                    request.previous_report,
                ));
            }
        };
        let checks = report
            .results()
            .iter()
            .map(binding_check_snapshot)
            .collect::<AppResult<Vec<_>>>()?;
        let stored_report = RepositoryBindingReport::new(
            &binding,
            request.observed_at,
            report.strict_exit() == 0,
            checks,
        )?;
        Ok(RepositoryBindingEvaluation {
            binding,
            report: Some(stored_report),
            stale_reason: None,
        })
    }
}

fn binding_check_snapshot(
    result: &crate::domain::policy::CheckResult,
) -> AppResult<RepositoryBindingCheck> {
    let evidence = result
        .evidence()
        .iter()
        .map(|item| {
            RepositoryBindingEvidence::new(
                item.source_id().to_owned(),
                item.observed_at().to_owned(),
                item.revision().cloned(),
                item.environment_fingerprint().to_owned(),
                item.summary().to_owned(),
            )
        })
        .collect::<AppResult<Vec<_>>>()?;
    RepositoryBindingCheck::new(
        result.requirement_id().to_owned(),
        check_status_name(result.status()).to_owned(),
        severity_name(result.severity()).to_owned(),
        result
            .enforcement()
            .map(enforcement_name)
            .map(str::to_owned),
        result.guidance_id().map(str::to_owned),
        evidence,
    )
}

fn check_status_name(status: CheckStatus) -> &'static str {
    match status {
        CheckStatus::Pass => "pass",
        CheckStatus::Fail => "fail",
        CheckStatus::Blocked => "blocked",
        CheckStatus::Unknown => "unknown",
        CheckStatus::Unsupported => "unsupported",
        CheckStatus::NotApplicable => "not-applicable",
    }
}

fn severity_name(severity: crate::domain::policy::Severity) -> &'static str {
    use crate::domain::policy::Severity;
    match severity {
        Severity::Info => "info",
        Severity::Warning => "warning",
        Severity::Error => "error",
    }
}

fn enforcement_name(enforcement: crate::domain::policy::Enforcement) -> &'static str {
    use crate::domain::policy::Enforcement;
    match enforcement {
        Enforcement::Instruction => "instruction",
        Enforcement::LocalCheck => "local-check",
        Enforcement::LocalHook => "local-hook",
        Enforcement::RequiredCi => "required-ci",
        Enforcement::HostRule => "host-rule",
    }
}

#[cfg(test)]
mod test_suite_tests {
    use super::{TestSuiteRunApproval, TestSuiteRunnerPort, TestSuiteService, TestSuiteSnapshot};
    use crate::{
        AppError, AppResult, ContentHash, OperationId,
        domain::policy::{
            RepositoryHead, TestSuiteDeclaration, TestSuiteExecution, TestSuiteKind,
            TestSuiteRunResult,
        },
        ports::process::{ApprovedRoot, CancellationToken},
    };
    use async_trait::async_trait;
    use std::{
        future::Future,
        path::PathBuf,
        sync::{Arc, Mutex},
        task::{Context, Poll, Wake, Waker},
    };

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

    #[derive(Default)]
    struct FakeTestSuiteRunner {
        inspected: Mutex<Option<TestSuiteKind>>,
        ran: Mutex<Option<OperationId>>,
        root: PathBuf,
    }

    #[async_trait]
    impl TestSuiteRunnerPort for FakeTestSuiteRunner {
        async fn inspect(
            &self,
            suite: TestSuiteKind,
            _cancellation: CancellationToken,
        ) -> AppResult<TestSuiteSnapshot> {
            *self.inspected.lock().unwrap() = Some(suite);
            Ok(TestSuiteSnapshot::new(
                ApprovedRoot::from_absolute_path(self.root.clone())
                    .map_err(AppError::Validation)?,
                suite,
                RepositoryHead::parse(&"a".repeat(40)).map_err(AppError::Validation)?,
                ContentHash::parse_hex(&"b".repeat(64)).unwrap(),
                TestSuiteDeclaration::Declared,
            ))
        }

        async fn run(
            &self,
            approval: TestSuiteRunApproval,
            _cancellation: CancellationToken,
        ) -> AppResult<TestSuiteRunResult> {
            *self.ran.lock().unwrap() = Some(approval.operation_id());
            TestSuiteRunResult::new(
                approval.snapshot().suite(),
                TestSuiteDeclaration::Declared,
                TestSuiteExecution::Passed,
                Some(0),
            )
            .map_err(AppError::Validation)
        }
    }

    #[test]
    fn test_suite_inspection_is_read_only_until_explicit_run_approval() {
        let root_path = std::env::current_dir().unwrap();
        let runner = Arc::new(FakeTestSuiteRunner {
            root: root_path.clone(),
            ..FakeTestSuiteRunner::default()
        });
        let service = TestSuiteService::new(runner.clone());
        let snapshot =
            block_on(service.inspect(TestSuiteKind::CargoTest, CancellationToken::new())).unwrap();

        assert!(matches!(snapshot.suite(), TestSuiteKind::CargoTest));
        assert_eq!(snapshot.expected_head().as_str(), "a".repeat(40));
        assert_eq!(snapshot.manifest_fingerprint().as_str(), "b".repeat(64));
        assert_eq!(snapshot.root().path(), root_path);
        assert!(matches!(
            *runner.inspected.lock().unwrap(),
            Some(TestSuiteKind::CargoTest)
        ));
        assert!(runner.ran.lock().unwrap().is_none());

        let approval = TestSuiteRunApproval::after_explicit_trust_confirmation(snapshot);
        let operation_id = approval.operation_id();
        assert!(!operation_id.as_uuid().is_nil());
        let result = block_on(service.run(approval, CancellationToken::new())).unwrap();

        assert!(matches!(result.execution(), TestSuiteExecution::Passed));
        assert_eq!(*runner.ran.lock().unwrap(), Some(operation_id));
    }

    #[test]
    fn repository_head_rejects_unreviewed_or_malformed_identifiers() {
        assert!(RepositoryHead::parse(&"A".repeat(40)).is_err());
        assert!(RepositoryHead::parse(&"g".repeat(40)).is_err());
        assert!(RepositoryHead::parse("short").is_err());
    }
}
