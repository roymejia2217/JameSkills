use crate::{
    AppError, AppResult, Diagnostic, OperationId,
    application::policy::{CheckContext, CheckRequest, PolicyService},
    domain::policy::CheckReport,
    domain::{
        GuidanceAnswer, GuidanceDecision, GuidanceFacts, GuidancePlan, ValidatedBundle, next_step,
    },
    ports::ClockPort,
};
use async_trait::async_trait;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, MutexGuard};

const MAX_LIVE_GUIDANCE_SESSIONS: usize = 64;
const MAX_GUIDANCE_CHOICE_BYTES: usize = 256;

#[async_trait]
pub trait GuidanceFactsProvider: Send + Sync {
    async fn observe_facts(&self) -> AppResult<GuidanceFacts>;
}

#[derive(Clone, PartialEq, Eq)]
pub enum UserAnswer {
    Acknowledge { step_id: String },
    Choose { step_id: String, choice: String },
}

#[derive(Clone, PartialEq, Eq)]
pub struct GuidanceProgress {
    session_id: OperationId,
    decision: GuidanceDecision,
}

impl GuidanceProgress {
    pub fn session_id(&self) -> OperationId {
        self.session_id
    }

    pub fn decision(&self) -> &GuidanceDecision {
        &self.decision
    }
}

#[derive(Clone)]
struct SessionState {
    bundle: Arc<ValidatedBundle>,
    plan_id: String,
    facts: GuidanceFacts,
    reports: Vec<CheckReport>,
    answers: BTreeMap<String, GuidanceAnswer>,
    generation: u64,
}

pub struct GuidanceService {
    policy: Arc<PolicyService>,
    facts: Arc<dyn GuidanceFactsProvider>,
    clock: Arc<dyn ClockPort>,
    sessions: Mutex<HashMap<OperationId, SessionState>>,
}

impl GuidanceService {
    pub fn new(
        policy: Arc<PolicyService>,
        facts: Arc<dyn GuidanceFactsProvider>,
        clock: Arc<dyn ClockPort>,
    ) -> Self {
        Self {
            policy,
            facts,
            clock,
            sessions: Mutex::new(HashMap::new()),
        }
    }

    pub async fn start_guidance(
        &self,
        bundle: Arc<ValidatedBundle>,
        plan_id: &str,
    ) -> AppResult<GuidanceProgress> {
        let plan = guidance_plan(&bundle, plan_id)?;
        crate::domain::guidance::validate_guidance_graph(plan).map_err(AppError::Validation)?;
        let facts = self.facts.observe_facts().await?;
        let reports = self.check_policies(&bundle, plan, &facts).await?;
        let session_id = OperationId::new();
        let session = SessionState {
            bundle,
            plan_id: plan_id.to_owned(),
            facts,
            reports,
            answers: BTreeMap::new(),
            generation: 0,
        };
        let progress = self.progress(session_id, &session)?;
        let mut sessions = self.lock_sessions()?;
        if sessions.len() >= MAX_LIVE_GUIDANCE_SESSIONS {
            return Err(AppError::CapabilityUnavailable {
                id: "guidance.session.capacity".to_owned(),
                guidance_id: "guidance-session-capacity".to_owned(),
            });
        }
        sessions.insert(session_id, session);
        Ok(progress)
    }

    pub fn advance(
        &self,
        session_id: OperationId,
        answer: UserAnswer,
    ) -> AppResult<GuidanceProgress> {
        let mut sessions = self.lock_sessions()?;
        let session = sessions.get_mut(&session_id).ok_or(AppError::NotFound)?;
        let plan = guidance_plan(&session.bundle, &session.plan_id)?;
        let (step_id, answer) = validate_user_answer(plan, answer).map_err(AppError::Validation)?;
        if self
            .progress(session_id, session)?
            .decision()
            .next_step()
            .is_none_or(|current| current.id() != step_id)
        {
            return Err(AppError::Validation(vec![Diagnostic::error(
                "guidance.answer.step.not_current",
                "Guidance answers must refer to the current actionable step.",
            )]));
        }
        session.answers.insert(step_id, answer);
        session.generation = session.generation.saturating_add(1);
        self.progress(session_id, session)
    }

    pub async fn recheck(&self, session_id: OperationId) -> AppResult<GuidanceProgress> {
        let (generation, bundle, plan_id) = {
            let mut sessions = self.lock_sessions()?;
            let session = sessions.get_mut(&session_id).ok_or(AppError::NotFound)?;
            session.generation = session.generation.saturating_add(1);
            session.reports.clear();
            let generation = session.generation;
            let bundle = session.bundle.clone();
            (generation, bundle, session.plan_id.clone())
        };
        let facts = self.facts.observe_facts().await?;
        let plan = guidance_plan(&bundle, &plan_id)?;
        let reports = self.check_policies(&bundle, plan, &facts).await?;
        let mut sessions = self.lock_sessions()?;
        let session = sessions.get_mut(&session_id).ok_or(AppError::NotFound)?;
        if session.generation != generation {
            return self.progress(session_id, session);
        }
        if session.facts.environment_fingerprint() != facts.environment_fingerprint() {
            session.answers.clear();
        }
        session.facts = facts;
        session.reports = reports;
        session.generation = session.generation.saturating_add(1);
        self.progress(session_id, session)
    }

    pub fn close_session(&self, session_id: OperationId) -> AppResult<()> {
        self.lock_sessions()?
            .remove(&session_id)
            .map(|_| ())
            .ok_or(AppError::NotFound)
    }

    async fn check_policies(
        &self,
        bundle: &ValidatedBundle,
        plan: &GuidancePlan,
        facts: &GuidanceFacts,
    ) -> AppResult<Vec<CheckReport>> {
        let mut context = CheckContext::default();
        for (fact, observation) in facts.observations() {
            context = context
                .with_fact(*fact, observation.value(), observation.evidence().clone())
                .map_err(AppError::Validation)?;
        }
        let mut reports = Vec::with_capacity(bundle.policies().len());
        for policy in bundle.policies() {
            if !policy.requirements().iter().any(|requirement| {
                plan.requirement_ids()
                    .iter()
                    .any(|id| id == requirement.id())
            }) {
                continue;
            }
            reports.push(
                self.policy
                    .check(CheckRequest::new(policy.clone(), context.clone()))
                    .await?,
            );
        }
        Ok(reports)
    }

    fn progress(
        &self,
        session_id: OperationId,
        session: &SessionState,
    ) -> AppResult<GuidanceProgress> {
        let plan = guidance_plan(&session.bundle, &session.plan_id)?;
        let decision = next_step(
            plan,
            &session.facts,
            &session.reports,
            &session.answers,
            self.clock.monotonic_ms(),
        );
        Ok(GuidanceProgress {
            session_id,
            decision,
        })
    }

    fn lock_sessions(&self) -> AppResult<MutexGuard<'_, HashMap<OperationId, SessionState>>> {
        self.sessions.lock().map_err(|_| AppError::Storage {
            code: "guidance.sessions.poisoned".to_owned(),
        })
    }
}

fn guidance_plan<'a>(bundle: &'a ValidatedBundle, id: &str) -> AppResult<&'a GuidancePlan> {
    bundle
        .guidance_plans()
        .iter()
        .find(|plan| plan.id() == id)
        .ok_or(AppError::NotFound)
}

fn validate_user_answer(
    plan: &GuidancePlan,
    answer: UserAnswer,
) -> Result<(String, GuidanceAnswer), Vec<Diagnostic>> {
    let (step_id, answer) = match answer {
        UserAnswer::Acknowledge { step_id } => (step_id, GuidanceAnswer::Acknowledge),
        UserAnswer::Choose { step_id, choice } => {
            if choice.len() > MAX_GUIDANCE_CHOICE_BYTES {
                return Err(vec![Diagnostic::error(
                    "guidance.answer.invalid",
                    "Guidance choice is outside its registered limit.",
                )]);
            }
            (step_id, GuidanceAnswer::Choose(choice))
        }
    };
    let Some(step) = plan.steps().iter().find(|step| step.id() == step_id) else {
        return Err(vec![Diagnostic::error(
            "guidance.answer.invalid",
            "Guidance answer does not refer to a step in the active plan.",
        )]);
    };
    match (&answer, step.action()) {
        (
            GuidanceAnswer::Choose(choice),
            crate::domain::GuidanceAction::AnswerChoice { choices },
        ) if choices.contains(choice) => {}
        (GuidanceAnswer::Acknowledge, crate::domain::GuidanceAction::AnswerChoice { .. })
        | (GuidanceAnswer::Choose(_), _) => {
            return Err(vec![Diagnostic::error(
                "guidance.answer.invalid",
                "Guidance answer does not match the registered step action.",
            )]);
        }
        _ => {}
    }
    Ok((step_id, answer))
}
