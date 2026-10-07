use async_trait::async_trait;
use jameskills_core::{
    AppError, AppResult, OperationId,
    application::{
        GuidanceFactsProvider, GuidanceService, UserAnswer,
        policy::{PolicyCheckProvider, PolicyService},
    },
    domain::{
        GuidanceAction, GuidanceFacts, GuidanceProgressStatus, PortablePath,
        policy::{CheckObservation, CheckStatus},
        skill::validate_bundle,
    },
    ports::ClockPort,
};
use std::{
    collections::BTreeMap,
    future::Future,
    sync::{Arc, Mutex},
    task::{Context, Poll, Wake, Waker},
};

const MANIFEST: &str = include_str!("../../../docs/examples/repository-foundation/jameskills.toml");
const SKILL: &str = include_str!("../../../docs/examples/repository-foundation/SKILL.md");
const POLICY: &str =
    include_str!("../../../docs/examples/repository-foundation/policies/repository.toml");
const GUIDANCE: &str =
    include_str!("../../../docs/examples/repository-foundation/guidance/repository.toml");
const APPLICATION_GUIDANCE: &str =
    include_str!("../../../examples/repository-foundation/guidance/repository.toml");
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
        "2026-10-05T12:00:00Z".to_owned()
    }

    fn monotonic_ms(&self) -> u64 {
        100
    }
}

struct FakeChecks {
    passing_requirement: Mutex<Option<String>>,
    environment_fingerprint: Mutex<String>,
}

impl FakeChecks {
    fn set_pass(&self, requirement_id: &str, environment_fingerprint: &str) {
        *self.passing_requirement.lock().unwrap() = Some(requirement_id.to_owned());
        *self.environment_fingerprint.lock().unwrap() = environment_fingerprint.to_owned();
    }
}

#[async_trait]
impl PolicyCheckProvider for FakeChecks {
    async fn observe(
        &self,
        requirement: &jameskills_core::domain::Requirement,
    ) -> AppResult<CheckObservation> {
        let passing_requirement = self.passing_requirement.lock().unwrap().clone();
        if passing_requirement.as_deref() != Some(requirement.id()) {
            return Ok(CheckObservation::unknown());
        }
        let environment_fingerprint = self.environment_fingerprint.lock().unwrap().clone();
        let evidence = jameskills_core::domain::policy::CheckEvidence::new(
            "test.guidance.check",
            "2026-10-05T12:00:00Z",
            None,
            &environment_fingerprint,
            "fixture check passed",
            Some(200),
        )
        .map_err(AppError::Validation)?;
        CheckObservation::new(
            CheckStatus::Pass,
            Some(jameskills_core::domain::Enforcement::LocalCheck),
            vec![evidence],
        )
        .map_err(AppError::Validation)
    }
}

struct FixedFacts(Mutex<Option<GuidanceFacts>>);

impl FixedFacts {
    fn set(&self, facts: GuidanceFacts) {
        *self.0.lock().unwrap() = Some(facts);
    }

    fn unavailable(&self) {
        *self.0.lock().unwrap() = None;
    }
}

#[async_trait]
impl GuidanceFactsProvider for FixedFacts {
    async fn observe_facts(&self) -> AppResult<GuidanceFacts> {
        self.0
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| AppError::CapabilityUnavailable {
                id: "test.guidance.facts".to_owned(),
                guidance_id: "guidance-setup".to_owned(),
            })
    }
}

fn validated_bundle() -> Arc<jameskills_core::domain::ValidatedBundle> {
    validated_bundle_with_guidance(GUIDANCE)
}

fn validated_bundle_with_guidance(guidance: &str) -> Arc<jameskills_core::domain::ValidatedBundle> {
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
    .collect::<BTreeMap<_, _>>();
    Arc::new(validate_bundle(&files).unwrap())
}

fn service() -> (GuidanceService, Arc<FakeChecks>, Arc<FixedFacts>) {
    let checks = Arc::new(FakeChecks {
        passing_requirement: Mutex::new(None),
        environment_fingerprint: Mutex::new(ENVIRONMENT.to_owned()),
    });
    let policy = Arc::new(PolicyService::new(checks.clone(), Arc::new(TestClock)));
    let facts = Arc::new(FixedFacts(Mutex::new(Some(
        GuidanceFacts::new(ENVIRONMENT, []).unwrap(),
    ))));
    (
        GuidanceService::new(policy, facts.clone(), Arc::new(TestClock)),
        checks,
        facts,
    )
}

#[test]
fn start_guidance_selects_the_validated_step_and_keeps_unknown_check_visible() {
    let (service, _, _) = service();
    let bundle = validated_bundle();
    let progress = block_on(service.start_guidance(bundle, "git-setup")).unwrap();

    assert!(!progress.session_id().as_uuid().is_nil());
    assert_eq!(
        progress.decision().status(),
        GuidanceProgressStatus::AwaitingEvidence
    );
    assert_eq!(
        progress.decision().next_step().map(|step| step.id()),
        Some("verify-git")
    );
    assert_eq!(
        progress.decision().steps()[0].verification_status(),
        CheckStatus::Unknown
    );
}

#[test]
fn application_ci_guidance_keeps_template_review_manual_and_non_mutating() {
    let (service, _, _) = service();
    let progress = block_on(service.start_guidance(
        validated_bundle_with_guidance(APPLICATION_GUIDANCE),
        "ci-setup",
    ))
    .unwrap();
    let step = progress.decision().next_step().unwrap();

    assert_eq!(step.id(), "pipeline");
    assert!(matches!(step.action(), GuidanceAction::ManualInstruction));
    assert!(
        step.prompt_es()
            .contains("revisa manualmente el contenido y el diff")
    );
    assert!(step.prompt_es().contains("no la aplica ni activa hooks"));
    assert_eq!(
        progress.decision().status(),
        GuidanceProgressStatus::AwaitingEvidence
    );
}

#[test]
fn acknowledgement_and_missing_session_cannot_create_success() {
    let (service, _, _) = service();
    let bundle = validated_bundle();
    let progress = block_on(service.start_guidance(bundle, "git-setup")).unwrap();
    assert!(matches!(
        service.advance(
            progress.session_id(),
            UserAnswer::Acknowledge {
                step_id: "review-readme".to_owned(),
            },
        ),
        Err(AppError::Validation(_))
    ));
    assert!(matches!(
        service.advance(
            progress.session_id(),
            UserAnswer::Choose {
                step_id: "verify-git".to_owned(),
                choice: "arbitrary".to_owned(),
            },
        ),
        Err(AppError::Validation(_))
    ));
    let after_answer = service
        .advance(
            progress.session_id(),
            UserAnswer::Acknowledge {
                step_id: "verify-git".to_owned(),
            },
        )
        .unwrap();

    assert_eq!(
        after_answer.decision().status(),
        GuidanceProgressStatus::AwaitingEvidence
    );
    assert!(matches!(
        service.advance(
            OperationId::new(),
            UserAnswer::Acknowledge {
                step_id: "verify-git".to_owned(),
            },
        ),
        Err(AppError::NotFound)
    ));
}

#[test]
fn acknowledging_a_copy_command_action_does_not_replace_check_evidence() {
    let (service, _, _) = service();
    let guidance = GUIDANCE.replace("\r\n", "\n").replacen(
        "kind = \"open-official-url\"\nsource_id = \"git-install\"",
        "kind = \"copy-approved-command\"\ntool_id = \"git\"\noperation = \"repository-root\"",
        1,
    );
    assert_ne!(guidance, GUIDANCE, "fixture mutation must target Git setup");
    let progress =
        block_on(service.start_guidance(validated_bundle_with_guidance(&guidance), "git-setup"))
            .unwrap();
    let acknowledged = service
        .advance(
            progress.session_id(),
            UserAnswer::Acknowledge {
                step_id: "verify-git".to_owned(),
            },
        )
        .unwrap();

    assert_eq!(
        acknowledged.decision().status(),
        GuidanceProgressStatus::AwaitingEvidence
    );
    assert_eq!(
        acknowledged.decision().steps()[0].verification_status(),
        jameskills_core::domain::policy::CheckStatus::Unknown
    );
}

#[test]
fn recheck_replaces_unknown_reports_with_fresh_policy_results() {
    let (service, checks, _) = service();
    let progress = block_on(service.start_guidance(validated_bundle(), "git-setup")).unwrap();
    assert_eq!(
        progress.decision().status(),
        GuidanceProgressStatus::AwaitingEvidence
    );

    checks.set_pass("git-ready", ENVIRONMENT);
    let refreshed = block_on(service.recheck(progress.session_id())).unwrap();
    assert_eq!(
        refreshed.decision().status(),
        GuidanceProgressStatus::Complete
    );
    assert!(refreshed.decision().next_step().is_none());
}

#[test]
fn failed_recheck_drops_old_pass_reports_instead_of_reusing_them() {
    let (service, checks, facts) = service();
    checks.set_pass("git-ready", ENVIRONMENT);
    let progress = block_on(service.start_guidance(validated_bundle(), "git-setup")).unwrap();
    assert_eq!(
        progress.decision().status(),
        GuidanceProgressStatus::Complete
    );

    facts.unavailable();
    assert!(matches!(
        block_on(service.recheck(progress.session_id())),
        Err(AppError::CapabilityUnavailable { .. })
    ));
    let after_failed_recheck = service
        .advance(
            progress.session_id(),
            UserAnswer::Acknowledge {
                step_id: "verify-git".to_owned(),
            },
        )
        .unwrap();
    assert_eq!(
        after_failed_recheck.decision().status(),
        GuidanceProgressStatus::AwaitingEvidence
    );
}

#[test]
fn changed_environment_fingerprint_clears_saved_choice_answers() {
    let (service, checks, facts) = service();
    let mut guidance = GUIDANCE.replace("\r\n", "\n");
    guidance = guidance.replacen(
        "kind = \"manual-instruction\"",
        "kind = \"answer-choice\"\nchoices = [\"rust\", \"node\"]",
        1,
    );
    let bundle = validated_bundle_with_guidance(&guidance);
    checks.set_pass("readme-structure", ENVIRONMENT);
    let progress = block_on(service.start_guidance(bundle, "readme-setup")).unwrap();
    let selected = service
        .advance(
            progress.session_id(),
            UserAnswer::Choose {
                step_id: "review-readme".to_owned(),
                choice: "rust".to_owned(),
            },
        )
        .unwrap();
    assert_eq!(
        selected.decision().status(),
        GuidanceProgressStatus::Complete
    );

    let new_fingerprint = format!("sha256:{}", "1".repeat(64));
    facts.set(GuidanceFacts::new(&new_fingerprint, []).unwrap());
    checks.set_pass("readme-structure", &new_fingerprint);
    let rechecked = block_on(service.recheck(progress.session_id())).unwrap();
    assert_eq!(
        rechecked.decision().status(),
        GuidanceProgressStatus::AwaitingAnswer
    );
    assert_eq!(
        rechecked.decision().next_step().map(|step| step.id()),
        Some("review-readme")
    );
}

#[test]
fn live_session_limit_is_enforced_and_close_releases_capacity() {
    let (service, _, _) = service();
    let bundle = validated_bundle();
    let mut session_ids = Vec::new();
    for _ in 0..64 {
        session_ids.push(
            block_on(service.start_guidance(bundle.clone(), "git-setup"))
                .unwrap()
                .session_id(),
        );
    }
    assert!(matches!(
        block_on(service.start_guidance(bundle.clone(), "git-setup")),
        Err(AppError::CapabilityUnavailable { .. })
    ));

    service.close_session(session_ids[0]).unwrap();
    assert!(block_on(service.start_guidance(bundle, "git-setup")).is_ok());
}
