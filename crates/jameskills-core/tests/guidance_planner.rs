use async_trait::async_trait;
use jameskills_core::{
    AppError, AppResult,
    application::policy::{CheckRequest, PolicyCheckProvider, PolicyService},
    domain::{
        GuidanceAnswer, GuidanceFactObservation, GuidanceFacts, GuidanceProgressStatus,
        GuidanceStep, GuidanceStepStatus, PortablePath, next_step,
        policy::{ApplicabilityFact, CheckEvidence, CheckObservation, CheckStatus, Enforcement},
        skill::validate_bundle,
    },
    ports::ClockPort,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    future::Future,
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
};

const MANIFEST: &str = include_str!("../../../docs/examples/repository-foundation/jameskills.toml");
const SKILL: &str = include_str!("../../../docs/examples/repository-foundation/SKILL.md");
const POLICY: &str =
    include_str!("../../../docs/examples/repository-foundation/policies/repository.toml");
const GUIDANCE: &str =
    include_str!("../../../docs/examples/repository-foundation/guidance/repository.toml");
const NOW: &str = "2026-10-05T12:00:00Z";
const ENVIRONMENT: &str = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn block_on<F: Future>(future: F) -> F::Output {
    struct Noop;
    impl Wake for Noop {
        fn wake(self: Arc<Self>) {}
    }
    let waker = Waker::from(Arc::new(Noop));
    let mut context = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

struct TestClock;

impl ClockPort for TestClock {
    fn now_utc(&self) -> String {
        NOW.to_owned()
    }

    fn monotonic_ms(&self) -> u64 {
        100
    }
}

struct FakeChecks {
    passing: BTreeSet<String>,
    failing: BTreeSet<String>,
    blocked: BTreeSet<String>,
    expires_at: Option<u64>,
}

#[async_trait]
impl PolicyCheckProvider for FakeChecks {
    async fn observe(
        &self,
        requirement: &jameskills_core::domain::Requirement,
    ) -> AppResult<CheckObservation> {
        let status = if self.passing.contains(requirement.id()) {
            CheckStatus::Pass
        } else if self.failing.contains(requirement.id()) {
            CheckStatus::Fail
        } else if self.blocked.contains(requirement.id()) {
            CheckStatus::Blocked
        } else {
            return Ok(CheckObservation::unknown());
        };
        let evidence = CheckEvidence::new(
            "test.guidance.check",
            NOW,
            None,
            ENVIRONMENT,
            format!("{} observed", requirement.id()),
            self.expires_at,
        )
        .map_err(AppError::Validation)?;
        CheckObservation::new(status, Some(Enforcement::LocalCheck), vec![evidence])
            .map_err(AppError::Validation)
    }
}

fn validated_bundle(guidance: &str) -> jameskills_core::domain::ValidatedBundle {
    let files = [
        ("jameskills.toml", MANIFEST),
        ("SKILL.md", SKILL),
        ("policies/repository.toml", POLICY),
        ("guidance/repository.toml", guidance),
    ]
    .into_iter()
    .map(|(path, source)| {
        (
            PortablePath::new(path.to_owned()).unwrap(),
            source.as_bytes().to_vec(),
        )
    })
    .collect();
    validate_bundle(&files).unwrap()
}

fn check_report(
    bundle: &jameskills_core::domain::ValidatedBundle,
    passing: &[&str],
    expires_at: Option<u64>,
) -> jameskills_core::domain::policy::CheckReport {
    check_report_with_failures(bundle, passing, &[], expires_at)
}

fn check_report_with_failures(
    bundle: &jameskills_core::domain::ValidatedBundle,
    passing: &[&str],
    failing: &[&str],
    expires_at: Option<u64>,
) -> jameskills_core::domain::policy::CheckReport {
    check_report_with_statuses(bundle, passing, failing, &[], expires_at)
}

fn check_report_with_statuses(
    bundle: &jameskills_core::domain::ValidatedBundle,
    passing: &[&str],
    failing: &[&str],
    blocked: &[&str],
    expires_at: Option<u64>,
) -> jameskills_core::domain::policy::CheckReport {
    let service = PolicyService::new(
        Arc::new(FakeChecks {
            passing: passing.iter().map(|id| (*id).to_owned()).collect(),
            failing: failing.iter().map(|id| (*id).to_owned()).collect(),
            blocked: blocked.iter().map(|id| (*id).to_owned()).collect(),
            expires_at,
        }),
        Arc::new(TestClock),
    );
    block_on(service.check(CheckRequest::new(
        bundle.policies()[0].clone(),
        Default::default(),
    )))
    .unwrap()
}

fn facts(os: Option<&str>) -> GuidanceFacts {
    facts_with_expiry(os, Some(200))
}

fn facts_with_expiry(os: Option<&str>, expires_at: Option<u64>) -> GuidanceFacts {
    let observations = os
        .map(|os| {
            vec![
                GuidanceFactObservation::new(
                    ApplicabilityFact::Os,
                    os,
                    CheckEvidence::new(
                        "test.guidance.facts",
                        NOW,
                        None,
                        ENVIRONMENT,
                        "OS observation",
                        expires_at,
                    )
                    .unwrap(),
                )
                .unwrap(),
            ]
        })
        .unwrap_or_default();
    GuidanceFacts::new(ENVIRONMENT, observations).unwrap()
}

fn one_fact(fact: ApplicabilityFact, value: &str, expires_at: Option<u64>) -> GuidanceFacts {
    let observation = GuidanceFactObservation::new(
        fact,
        value,
        CheckEvidence::new(
            "test.guidance.facts",
            NOW,
            None,
            ENVIRONMENT,
            "Typed applicability fact",
            expires_at,
        )
        .unwrap(),
    )
    .unwrap();
    GuidanceFacts::new(ENVIRONMENT, [observation]).unwrap()
}

fn guide<'a>(
    bundle: &'a jameskills_core::domain::ValidatedBundle,
    plan_id: &str,
) -> &'a jameskills_core::domain::GuidancePlan {
    bundle
        .guidance_plans()
        .iter()
        .find(|plan| plan.id() == plan_id)
        .unwrap()
}

#[test]
fn fresh_pass_completes_a_step_with_passing_evidence() {
    let bundle = validated_bundle(GUIDANCE);
    let report = check_report(&bundle, &["git-ready"], Some(200));
    let decision = next_step(
        guide(&bundle, "git-setup"),
        &facts(Some("linux")),
        &[report],
        &BTreeMap::new(),
        100,
    );

    assert_eq!(decision.status(), GuidanceProgressStatus::Complete);
    assert!(decision.next_step().is_none());
    assert_eq!(decision.steps()[0].status(), GuidanceStepStatus::Completed);
}

#[test]
fn expired_verification_stays_pending_after_manual_acknowledgement() {
    let conditional_guidance = GUIDANCE.replacen(
        "verification_requirement_ids = [\"git-ready\"]",
        "verification_requirement_ids = [\"git-ready\"]\napplies_when = { fact = \"os\", equals = \"windows\" }",
        1,
    );
    let bundle = validated_bundle(&conditional_guidance);
    let report = check_report(&bundle, &["git-ready"], Some(50));
    let answers = BTreeMap::from([("verify-git".to_owned(), GuidanceAnswer::Acknowledge)]);
    let decision = next_step(
        guide(&bundle, "git-setup"),
        &facts(Some("windows")),
        &[report],
        &answers,
        100,
    );

    assert_eq!(decision.status(), GuidanceProgressStatus::AwaitingEvidence);
    assert_eq!(
        decision.next_step().map(GuidanceStep::id),
        Some("verify-git")
    );
    assert_eq!(
        decision.steps()[0].verification_status(),
        CheckStatus::Unknown
    );
}

#[test]
fn fresh_mismatched_fact_skips_only_its_conditional_step() {
    let conditional = GUIDANCE.replacen(
        "verification_requirement_ids = [\"git-ready\"]",
        "verification_requirement_ids = [\"git-ready\"]\napplies_when = { fact = \"os\", equals = \"windows\" }",
        1,
    );
    let bundle = validated_bundle(&conditional);
    let report = check_report(&bundle, &["git-ready"], Some(200));
    let decision = next_step(
        guide(&bundle, "git-setup"),
        &facts(Some("linux")),
        &[report],
        &BTreeMap::new(),
        100,
    );

    assert_eq!(decision.status(), GuidanceProgressStatus::Complete);
    assert!(decision.next_step().is_none());
    assert_eq!(
        decision.steps()[0].status(),
        GuidanceStepStatus::NotApplicable
    );
}

#[test]
fn expired_applicability_fact_never_selects_its_branch() {
    let conditional = GUIDANCE.replacen(
        "verification_requirement_ids = [\"git-ready\"]",
        "verification_requirement_ids = [\"git-ready\"]\napplies_when = { fact = \"os\", equals = \"windows\" }",
        1,
    );
    let bundle = validated_bundle(&conditional);
    let report = check_report(&bundle, &["git-ready"], Some(200));
    let decision = next_step(
        guide(&bundle, "git-setup"),
        &facts_with_expiry(Some("windows"), Some(50)),
        &[report],
        &BTreeMap::new(),
        100,
    );

    assert_eq!(decision.status(), GuidanceProgressStatus::AwaitingFacts);
    assert_eq!(
        decision.next_step().map(GuidanceStep::id),
        Some("verify-git")
    );
}

#[test]
fn failed_check_remains_failed_and_cannot_complete_a_step() {
    let bundle = validated_bundle(GUIDANCE);
    let report = check_report_with_failures(&bundle, &[], &["git-ready"], Some(200));
    let decision = next_step(
        guide(&bundle, "git-setup"),
        &facts(Some("linux")),
        &[report],
        &BTreeMap::new(),
        100,
    );

    assert_eq!(decision.status(), GuidanceProgressStatus::AwaitingEvidence);
    assert_eq!(decision.steps()[0].verification_status(), CheckStatus::Fail);
    assert_eq!(
        decision.steps()[0].status(),
        GuidanceStepStatus::AwaitingEvidence
    );
}

#[test]
fn blocked_check_for_missing_auth_never_completes_from_manual_answer() {
    let bundle = validated_bundle(GUIDANCE);
    let report = check_report_with_statuses(&bundle, &[], &[], &["git-ready"], Some(200));
    let answers = BTreeMap::from([("verify-git".to_owned(), GuidanceAnswer::Acknowledge)]);
    let decision = next_step(
        guide(&bundle, "git-setup"),
        &facts(Some("windows")),
        &[report],
        &answers,
        100,
    );

    assert_eq!(decision.status(), GuidanceProgressStatus::AwaitingEvidence);
    assert_eq!(
        decision.steps()[0].verification_status(),
        CheckStatus::Blocked
    );
    assert_eq!(
        decision.steps()[0].status(),
        GuidanceStepStatus::AwaitingEvidence
    );
}

#[test]
fn stack_fact_selects_only_the_matching_registered_toolchain_branch() {
    let conditional = GUIDANCE.replacen(
        "verification_requirement_ids = [\"git-ready\"]",
        "verification_requirement_ids = [\"git-ready\"]\napplies_when = { fact = \"stack\", equals = \"node\" }",
        1,
    );
    let bundle = validated_bundle(&conditional);
    let report = check_report(&bundle, &["git-ready"], Some(200));
    let decision = next_step(
        guide(&bundle, "git-setup"),
        &one_fact(ApplicabilityFact::Stack, "rust", Some(200)),
        &[report],
        &BTreeMap::new(),
        100,
    );

    assert_eq!(decision.status(), GuidanceProgressStatus::Complete);
    assert_eq!(
        decision.steps()[0].status(),
        GuidanceStepStatus::NotApplicable
    );
}

#[test]
fn check_evidence_fingerprint_is_not_compared_to_platform_fact_fingerprint() {
    let bundle = validated_bundle(GUIDANCE);
    let report = check_report(&bundle, &["git-ready"], Some(200));
    let platform_fingerprint = format!("sha256:{}", "1".repeat(64));
    let platform_fact = GuidanceFactObservation::new(
        ApplicabilityFact::Os,
        "linux",
        CheckEvidence::new(
            "test.guidance.facts",
            NOW,
            None,
            &platform_fingerprint,
            "Platform fact from a different observation source.",
            Some(200),
        )
        .unwrap(),
    )
    .unwrap();
    let facts = GuidanceFacts::new(&platform_fingerprint, [platform_fact]).unwrap();
    let decision = next_step(
        guide(&bundle, "git-setup"),
        &facts,
        &[report],
        &BTreeMap::new(),
        100,
    );

    assert_eq!(decision.status(), GuidanceProgressStatus::Complete);
}

#[test]
fn answer_choice_must_be_registered_and_never_substitutes_for_verification() {
    let guidance = GUIDANCE.replacen(
        "kind = \"manual-instruction\"",
        "kind = \"answer-choice\"\nchoices = [\"rust\", \"node\"]",
        1,
    );
    let bundle = validated_bundle(&guidance);
    let report = check_report(&bundle, &["readme-structure"], Some(200));
    let plan = guide(&bundle, "readme-setup");

    let acknowledge = next_step(
        plan,
        &facts(Some("linux")),
        std::slice::from_ref(&report),
        &BTreeMap::from([("review-readme".to_owned(), GuidanceAnswer::Acknowledge)]),
        100,
    );
    assert_eq!(acknowledge.status(), GuidanceProgressStatus::AwaitingAnswer);

    let valid_choice = next_step(
        plan,
        &facts(Some("linux")),
        std::slice::from_ref(&report),
        &BTreeMap::from([(
            "review-readme".to_owned(),
            GuidanceAnswer::Choose("node".to_owned()),
        )]),
        100,
    );
    assert_eq!(valid_choice.status(), GuidanceProgressStatus::Complete);

    let invalid_choice = next_step(
        plan,
        &facts(Some("linux")),
        &[report],
        &BTreeMap::from([(
            "review-readme".to_owned(),
            GuidanceAnswer::Choose("arbitrary input".to_owned()),
        )]),
        100,
    );
    assert_eq!(
        invalid_choice.status(),
        GuidanceProgressStatus::AwaitingAnswer
    );
}

#[test]
fn unknown_branch_blocks_its_descendant_but_not_an_independent_root_step() {
    let conditional = GUIDANCE.replace("\r\n", "\n").replacen(
        "verification_requirement_ids = [\"git-ready\"]",
        "verification_requirement_ids = [\"git-ready\"]\napplies_when = { fact = \"os\", equals = \"windows\" }",
        1,
    );
    let inserted_steps = r#"
[[plans.steps]]
id = "windows-follow-up"
prompt_es = "Continue Windows setup."
requires = ["verify-git"]
verification_requirement_ids = ["git-ready"]
[plans.steps.action]
kind = "manual-instruction"
[[plans.steps]]
id = "independent"
prompt_es = "Check this independent requirement."
requires = []
verification_requirement_ids = ["git-ready"]
[plans.steps.action]
kind = "manual-instruction"
"#;
    let guidance = conditional.replacen(
        "\n[[plans]]\nid = \"readme-setup\"",
        &format!("{inserted_steps}\n[[plans]]\nid = \"readme-setup\""),
        1,
    );
    assert_ne!(guidance, conditional);
    let bundle = validated_bundle(&guidance);
    let report = check_report(&bundle, &["git-ready"], Some(200));
    let decision = next_step(
        guide(&bundle, "git-setup"),
        &facts(None),
        &[report],
        &BTreeMap::new(),
        100,
    );

    assert_eq!(decision.status(), GuidanceProgressStatus::AwaitingFacts);
    assert_eq!(
        decision.next_step().map(GuidanceStep::id),
        Some("verify-git")
    );
    assert!(decision.steps().iter().any(|step| {
        step.step_id() == "windows-follow-up"
            && step.status() == GuidanceStepStatus::BlockedByDependency
    }));
    assert!(decision.steps().iter().any(|step| {
        step.step_id() == "independent" && step.status() == GuidanceStepStatus::Completed
    }));
}
