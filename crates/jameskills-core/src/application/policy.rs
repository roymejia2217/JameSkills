use crate::{
    AppError, AppResult, Diagnostic,
    domain::policy::{
        ApplicabilityFact, CheckEvidence, CheckObservation, CheckReport, CheckStatus, Policy,
        Requirement, evaluate_predicate, not_applicable_result,
    },
    ports::ClockPort,
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
