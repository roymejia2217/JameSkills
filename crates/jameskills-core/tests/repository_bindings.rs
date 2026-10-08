use async_trait::async_trait;
use jameskills_core::domain::{
    RepositoryBinding, RepositoryBindingCheck, RepositoryBindingEvidence, RepositoryBindingReport,
    RepositoryProfile, RevisionId, SkillId,
    policy::{CheckObservation, CheckStatus, RepositoryHead, Requirement, parse_policy},
};
use jameskills_core::{
    AppResult,
    application::policy::{
        CheckContext, PolicyCheckProvider, PolicyService, RepositoryBindingEvaluationRequest,
        RepositoryBindingNextStep, RepositoryBindingStaleReason,
    },
    ports::ClockPort,
};
use std::{
    future::Future,
    path::PathBuf,
    sync::Arc,
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

struct FixedClock;

impl ClockPort for FixedClock {
    fn now_utc(&self) -> String {
        "2026-10-07T00:00:00.000Z".to_owned()
    }

    fn monotonic_ms(&self) -> u64 {
        1
    }
}

struct UnknownChecks;

#[async_trait]
impl PolicyCheckProvider for UnknownChecks {
    async fn observe(&self, _requirement: &Requirement) -> AppResult<CheckObservation> {
        Ok(CheckObservation::unknown())
    }
}

fn binding(root: PathBuf, fingerprint: String) -> jameskills_core::AppResult<RepositoryBinding> {
    RepositoryBinding::new(
        SkillId::new(),
        RevisionId::from_digest([1; 32]),
        root,
        RepositoryProfile::Rust,
        true,
        RepositoryHead::parse(&"a".repeat(40)).unwrap(),
        fingerprint,
    )
}

#[test]
fn binding_is_local_profiled_and_roundtrips_only_its_typed_fields() {
    let root = std::env::temp_dir().join("jameskills-repository-binding");
    let expected_fingerprint = format!("sha256:{}", "b".repeat(64));
    let original = binding(root, expected_fingerprint).unwrap();
    assert!(!original.id().is_empty());
    assert_eq!(original.profile(), RepositoryProfile::Rust);
    assert!(original.strict());
    assert_eq!(original.repository_head().as_str(), "a".repeat(40));

    let restored = RepositoryBinding::from_storage(
        original.id().to_owned(),
        original.skill_id(),
        original.suite_revision().clone(),
        original.repository_root().to_path_buf(),
        original.profile(),
        original.strict(),
        original.repository_head().clone(),
        original.environment_fingerprint().to_owned(),
    )
    .unwrap();
    assert!(restored == original);
}

#[test]
fn binding_rejects_noncanonical_roots_and_unregistered_facts() {
    let root = std::env::temp_dir().join("..").join("outside");
    assert!(binding(root, format!("sha256:{}", "b".repeat(64))).is_err());
    assert!(binding(std::env::temp_dir(), "sha256:ABCDEF".to_owned()).is_err());
    assert!(RepositoryProfile::parse("python").is_err());
    assert_eq!(
        RepositoryProfile::parse("node").unwrap(),
        RepositoryProfile::Node
    );
}

#[test]
fn prior_binding_report_is_stale_when_its_repository_basis_changes() {
    let binding = binding(
        std::env::temp_dir().join("jameskills-repository-binding-report"),
        format!("sha256:{}", "b".repeat(64)),
    )
    .unwrap();
    let evidence = RepositoryBindingEvidence::new(
        "policy.readme".to_owned(),
        "2026-10-07T00:00:00.000Z".to_owned(),
        None,
        binding.environment_fingerprint().to_owned(),
        "README sections were observed.".to_owned(),
    )
    .unwrap();
    let check = RepositoryBindingCheck::new(
        "readme-structure".to_owned(),
        "pass".to_owned(),
        "error".to_owned(),
        Some("local-check".to_owned()),
        Some("readme.setup".to_owned()),
        vec![evidence],
    )
    .unwrap();
    let report = RepositoryBindingReport::new(
        &binding,
        "2026-10-07T00:00:00.000Z".to_owned(),
        true,
        vec![check],
    )
    .unwrap();
    assert!(!report.is_stale_for(&binding));
    assert!(report.strict_passed());
    assert_eq!(report.checks().len(), 1);

    let moved = RepositoryBinding::from_storage(
        binding.id().to_owned(),
        binding.skill_id(),
        binding.suite_revision().clone(),
        std::env::temp_dir().join("jameskills-repository-binding-moved"),
        binding.profile(),
        binding.strict(),
        binding.repository_head().clone(),
        format!("sha256:{}", "c".repeat(64)),
    )
    .unwrap();
    assert!(report.is_stale_for(&moved));
}

#[test]
fn policy_binding_evaluation_rechecks_new_head_and_retains_previous_report_as_stale_offline() {
    let binding = binding(
        std::env::temp_dir().join("jameskills-repository-binding-evaluation"),
        format!("sha256:{}", "b".repeat(64)),
    )
    .unwrap();
    let observed_head = RepositoryBinding::from_storage(
        binding.id().to_owned(),
        binding.skill_id(),
        binding.suite_revision().clone(),
        binding.repository_root().to_path_buf(),
        binding.profile(),
        binding.strict(),
        RepositoryHead::parse(&"c".repeat(40)).unwrap(),
        binding.environment_fingerprint().to_owned(),
    )
    .unwrap();
    let policy = parse_policy(include_bytes!(
        "../../../tests/fixtures/valid-suite/policies/repository.toml"
    ))
    .unwrap();
    let service = PolicyService::new(Arc::new(UnknownChecks), Arc::new(FixedClock));
    let fresh = block_on(
        service.evaluate_binding(RepositoryBindingEvaluationRequest::new(
            binding.clone(),
            Some(observed_head.clone()),
            Some(binding.suite_revision().clone()),
            Some(vec![policy.clone()]),
            CheckContext::default(),
            None,
            "2026-10-07T00:00:00.000Z".to_owned(),
        )),
    )
    .unwrap();
    assert!(!fresh.is_stale());
    let prior_report = fresh.report().unwrap().clone();
    assert_eq!(prior_report.repository_head().as_str(), "c".repeat(40));
    assert!(!prior_report.strict_passed());

    let offline = block_on(
        service.evaluate_binding(RepositoryBindingEvaluationRequest::new(
            observed_head.clone(),
            None,
            Some(binding.suite_revision().clone()),
            None,
            CheckContext::default(),
            Some(prior_report.clone()),
            "2026-10-07T00:01:00.000Z".to_owned(),
        )),
    )
    .unwrap();
    assert_eq!(
        offline.stale_reason(),
        Some(RepositoryBindingStaleReason::RepositoryUnavailable)
    );
    assert_eq!(
        offline.next_step(),
        Some(RepositoryBindingNextStep::LocateRepository)
    );
    assert!(offline.report().unwrap() == &prior_report);

    let stale_suite = block_on(
        service.evaluate_binding(RepositoryBindingEvaluationRequest::new(
            observed_head,
            None,
            Some(RevisionId::from_digest([9; 32])),
            None,
            CheckContext::default(),
            Some(prior_report.clone()),
            "2026-10-07T00:02:00.000Z".to_owned(),
        )),
    )
    .unwrap();
    assert_eq!(
        stale_suite.stale_reason(),
        Some(RepositoryBindingStaleReason::SuiteRevisionChanged)
    );
    assert!(stale_suite.report().unwrap() == &prior_report);
}

#[test]
fn policy_service_combines_every_policy_in_a_suite_and_rejects_duplicate_requirement_ids() {
    let first = parse_policy(
        br#"
schema_version = 1
profile = "sample"
scope = "project"
[[requirements]]
id = "first"
description = "First check."
severity = "error"
required = true
phase = "pre-install"
enforcement = "instruction"
depends_on = []
[requirements.check]
kind = "readme-sections"
path = "README.md"
headings = ["Intro"]
"#,
    )
    .unwrap();
    let second = parse_policy(
        br#"
schema_version = 1
profile = "sample"
scope = "project"
[[requirements]]
id = "second"
description = "Second check."
severity = "warning"
required = false
phase = "pre-install"
enforcement = "instruction"
depends_on = []
[requirements.check]
kind = "readme-sections"
path = "CONTRIBUTING.md"
headings = ["Contributing"]
"#,
    )
    .unwrap();
    let service = PolicyService::new(Arc::new(UnknownChecks), Arc::new(FixedClock));
    let report =
        block_on(service.check_suite(vec![first.clone(), second], CheckContext::default()))
            .unwrap();
    assert_eq!(report.results().len(), 2);
    assert_eq!(report.required_ids().len(), 1);
    assert_eq!(report.results()[0].status(), CheckStatus::Unknown);
    assert_eq!(report.results()[1].status(), CheckStatus::Unknown);

    let duplicate =
        block_on(service.check_suite(vec![first.clone(), first], CheckContext::default()));
    assert!(matches!(
        duplicate,
        Err(jameskills_core::AppError::Validation(_))
    ));
}
