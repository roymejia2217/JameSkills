use crate::{
    AppError, AppResult, ContentHash, Diagnostic, OperationId,
    domain::policy::{
        ApplicabilityFact, CheckEvidence, CheckObservation, CheckReport, CheckStatus, Policy,
        RepositoryHead, Requirement, TestSuiteDeclaration, TestSuiteKind, TestSuiteRunResult,
        evaluate_predicate, not_applicable_result,
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
