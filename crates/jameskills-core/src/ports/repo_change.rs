use crate::{
    AppResult, ContentHash,
    domain::{ApprovedRepoChange, RepoChangePlan, RepoTemplateId, policy::RepositoryHead},
    ports::process::{ApprovedRoot, CancellationToken},
};
use async_trait::async_trait;

/// Read-only preview and explicitly approved repository mutation boundary.
/// Implementations must revalidate root/head/file hashes and journal writes.
#[async_trait]
pub trait RepoChangePort: Send + Sync {
    async fn preview(
        &self,
        root: &ApprovedRoot,
        root_fingerprint: &ContentHash,
        expected_head: &RepositoryHead,
        template_id: RepoTemplateId,
        cancellation: CancellationToken,
    ) -> AppResult<RepoChangePlan>;

    async fn apply(
        &self,
        root: &ApprovedRoot,
        approval: ApprovedRepoChange,
        cancellation: CancellationToken,
    ) -> AppResult<()>;
}
