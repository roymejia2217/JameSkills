use crate::{
    AppError, AppResult, ContentHash, Diagnostic, PortablePath,
    domain::{ApprovedRepoChange, RepoChangePlan, RepoTemplateId, policy::RepositoryHead},
    ports::{
        RepoChangePort,
        process::{ApprovedRoot, CancellationToken},
    },
};
use std::sync::Arc;

pub struct RepoPolicyRequest {
    root: ApprovedRoot,
    root_fingerprint: ContentHash,
    expected_head: RepositoryHead,
    template_id: RepoTemplateId,
}

impl RepoPolicyRequest {
    pub fn new(
        root: ApprovedRoot,
        root_fingerprint: ContentHash,
        expected_head: RepositoryHead,
        template_id: RepoTemplateId,
    ) -> Self {
        Self {
            root,
            root_fingerprint,
            expected_head,
            template_id,
        }
    }

    pub fn root(&self) -> &ApprovedRoot {
        &self.root
    }

    pub fn root_fingerprint(&self) -> &ContentHash {
        &self.root_fingerprint
    }

    pub fn expected_head(&self) -> &RepositoryHead {
        &self.expected_head
    }

    pub fn template_id(&self) -> RepoTemplateId {
        self.template_id
    }
}

pub struct RepoChangeReceipt {
    operation_id: crate::OperationId,
    target: PortablePath,
    previous_hash: Option<ContentHash>,
    applied_hash: ContentHash,
}

impl RepoChangeReceipt {
    pub fn operation_id(&self) -> crate::OperationId {
        self.operation_id
    }

    pub fn target(&self) -> &PortablePath {
        &self.target
    }

    pub fn previous_hash(&self) -> Option<&ContentHash> {
        self.previous_hash.as_ref()
    }

    pub fn applied_hash(&self) -> &ContentHash {
        &self.applied_hash
    }
}

pub struct RepositoryChangeService {
    port: Arc<dyn RepoChangePort>,
}

impl RepositoryChangeService {
    pub fn new(port: Arc<dyn RepoChangePort>) -> Self {
        Self { port }
    }

    /// Reads registered inputs and returns a bounded preview without applying it.
    pub async fn plan_repo_changes(
        &self,
        request: &RepoPolicyRequest,
        cancellation: CancellationToken,
    ) -> AppResult<RepoChangePlan> {
        let plan = self
            .port
            .preview(
                request.root(),
                request.root_fingerprint(),
                request.expected_head(),
                request.template_id(),
                cancellation,
            )
            .await?;
        if plan.template_id() != request.template_id()
            || plan.root_fingerprint() != request.root_fingerprint()
            || plan.expected_head() != request.expected_head()
            || plan.target().as_str() != request.template_id().target()
        {
            return Err(plan_mismatch());
        }
        Ok(plan)
    }

    /// Applies only the exact digest explicitly confirmed from the preview.
    pub async fn apply_repo_changes(
        &self,
        request: RepoPolicyRequest,
        plan: RepoChangePlan,
        confirmed_digest: &ContentHash,
        cancellation: CancellationToken,
    ) -> AppResult<RepoChangeReceipt> {
        if plan.template_id() != request.template_id()
            || plan.root_fingerprint() != request.root_fingerprint()
            || plan.expected_head() != &request.expected_head
            || plan.target().as_str() != request.template_id().target()
        {
            return Err(plan_mismatch());
        }
        let approval =
            ApprovedRepoChange::after_explicit_digest_confirmation(plan.clone(), confirmed_digest)
                .map_err(AppError::Validation)?;
        self.port
            .apply(request.root(), approval, cancellation)
            .await?;
        Ok(RepoChangeReceipt {
            operation_id: plan.operation_id(),
            target: plan.target().clone(),
            previous_hash: plan.previous_hash().cloned(),
            applied_hash: plan.proposed_hash().clone(),
        })
    }
}

fn plan_mismatch() -> AppError {
    AppError::Validation(vec![Diagnostic::error(
        "repo.change.plan.mismatch",
        "Repository change plan does not match the approved repository request.",
    )])
}
