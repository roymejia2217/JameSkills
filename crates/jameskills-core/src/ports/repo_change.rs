use crate::{
    AppResult, ContentHash,
    domain::{ApprovedRepoChange, RepoChangePlan, RepoTemplateId, policy::RepositoryHead},
    ports::process::{
        ApprovedEnv, ApprovedExecutable, ApprovedRoot, CancellationToken, ExecutableFingerprint,
    },
};
use async_trait::async_trait;

/// User-approved Git identity for fixed, read-only head/version probes.
pub struct ApprovedRepoGit {
    executable: ApprovedExecutable,
    fingerprint: ExecutableFingerprint,
    environment: ApprovedEnv,
}

impl ApprovedRepoGit {
    pub fn after_explicit_fingerprint_confirmation(
        executable: ApprovedExecutable,
        fingerprint: ExecutableFingerprint,
        confirmed_fingerprint: &ExecutableFingerprint,
        environment: ApprovedEnv,
    ) -> Result<Self, Vec<crate::Diagnostic>> {
        if &fingerprint != confirmed_fingerprint {
            return Err(vec![crate::Diagnostic::error(
                "repo.change.git.confirmation.mismatch",
                "Git executable confirmation does not match its approved fingerprint.",
            )]);
        }
        Ok(Self {
            executable,
            fingerprint,
            environment,
        })
    }

    pub fn executable(&self) -> &ApprovedExecutable {
        &self.executable
    }

    pub fn fingerprint(&self) -> ExecutableFingerprint {
        self.fingerprint
    }

    pub fn environment(&self) -> &ApprovedEnv {
        &self.environment
    }
}

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
        git: &ApprovedRepoGit,
        cancellation: CancellationToken,
    ) -> AppResult<RepoChangePlan>;

    async fn apply(
        &self,
        root: &ApprovedRoot,
        approval: ApprovedRepoChange,
        git: &ApprovedRepoGit,
        cancellation: CancellationToken,
    ) -> AppResult<()>;
}
