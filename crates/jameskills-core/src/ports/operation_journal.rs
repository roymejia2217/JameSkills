use crate::{
    Diagnostic, OperationId, PortablePath,
    domain::{ContentHash, RepoChangePlan, policy::RepositoryHead},
    ports::process::ApprovedRoot,
};
const MAX_JOURNAL_TIMESTAMP_BYTES: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepoChangeJournalState {
    Planned,
    Approved,
    Staged,
    NewMoved,
    Verified,
    Committed,
    Failed,
    RollbackPending,
    Recovered,
}

impl RepoChangeJournalState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Approved => "approved",
            Self::Staged => "staged",
            Self::NewMoved => "new-moved",
            Self::Verified => "verified",
            Self::Committed => "committed",
            Self::Failed => "failed",
            Self::RollbackPending => "rollback-pending",
            Self::Recovered => "recovered",
        }
    }

    pub fn allows_transition(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Planned, Self::Approved | Self::Failed)
                | (Self::Approved, Self::Staged | Self::Failed)
                | (Self::Staged, Self::NewMoved | Self::RollbackPending)
                | (Self::NewMoved, Self::Verified | Self::RollbackPending)
                | (Self::Verified, Self::Committed | Self::RollbackPending)
                | (Self::RollbackPending, Self::Recovered)
        )
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Committed | Self::Failed | Self::Recovered)
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "planned" => Some(Self::Planned),
            "approved" => Some(Self::Approved),
            "staged" => Some(Self::Staged),
            "new-moved" => Some(Self::NewMoved),
            "verified" => Some(Self::Verified),
            "committed" => Some(Self::Committed),
            "failed" => Some(Self::Failed),
            "rollback-pending" => Some(Self::RollbackPending),
            "recovered" => Some(Self::Recovered),
            _ => None,
        }
    }
}

/// Journal metadata is bounded and contains hashes/portable paths, never file
/// contents or process output. The absolute root path is retained locally only
/// so startup recovery can find the selected repository again.
pub struct RepoChangeJournal {
    operation_id: OperationId,
    state: RepoChangeJournalState,
    root: ApprovedRoot,
    root_fingerprint: ContentHash,
    expected_head: RepositoryHead,
    target: PortablePath,
    staging_target: PortablePath,
    previous_hash: Option<ContentHash>,
    proposed_hash: ContentHash,
    updated_at: String,
}

impl RepoChangeJournal {
    pub fn planned(
        root: ApprovedRoot,
        plan: &RepoChangePlan,
        updated_at: &str,
    ) -> Result<Self, Vec<Diagnostic>> {
        let Some(root_path) = root.path().to_str() else {
            return Err(journal_diagnostic(
                "repo.change.journal.root.invalid",
                "Repository root cannot be stored for recovery.",
            ));
        };
        if root_path.is_empty() {
            return Err(journal_diagnostic(
                "repo.change.journal.invalid",
                "Repository change journal metadata is invalid.",
            ));
        }
        Self::from_storage_parts(
            plan.operation_id(),
            RepoChangeJournalState::Planned,
            root,
            plan.root_fingerprint().clone(),
            plan.expected_head().clone(),
            plan.target().clone(),
            stage_path_for(plan.operation_id(), plan.target())?,
            plan.previous_hash().cloned(),
            plan.proposed_hash().clone(),
            updated_at,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_storage_parts(
        operation_id: OperationId,
        state: RepoChangeJournalState,
        root: ApprovedRoot,
        root_fingerprint: ContentHash,
        expected_head: RepositoryHead,
        target: PortablePath,
        staging_target: PortablePath,
        previous_hash: Option<ContentHash>,
        proposed_hash: ContentHash,
        updated_at: &str,
    ) -> Result<Self, Vec<Diagnostic>> {
        if root.path().to_str().is_none()
            || updated_at.is_empty()
            || updated_at.len() > MAX_JOURNAL_TIMESTAMP_BYTES
            || updated_at.chars().any(char::is_control)
            || previous_hash.is_some()
            || staging_target != stage_path_for(operation_id, &target)?
        {
            return Err(journal_diagnostic(
                "repo.change.journal.invalid",
                "Repository change journal metadata is invalid.",
            ));
        }
        Ok(Self {
            operation_id,
            state,
            root,
            root_fingerprint,
            expected_head,
            target,
            staging_target,
            previous_hash,
            proposed_hash,
            updated_at: updated_at.to_owned(),
        })
    }

    pub fn operation_id(&self) -> OperationId {
        self.operation_id
    }

    pub fn state(&self) -> RepoChangeJournalState {
        self.state
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

    pub fn target(&self) -> &PortablePath {
        &self.target
    }

    pub fn staging_target(&self) -> &PortablePath {
        &self.staging_target
    }

    pub fn previous_hash(&self) -> Option<&ContentHash> {
        self.previous_hash.as_ref()
    }

    pub fn proposed_hash(&self) -> &ContentHash {
        &self.proposed_hash
    }

    pub fn updated_at(&self) -> &str {
        &self.updated_at
    }

    pub fn can_transition_to(&self, next: RepoChangeJournalState) -> bool {
        self.state.allows_transition(next)
    }
}

fn stage_path_for(
    operation_id: OperationId,
    target: &PortablePath,
) -> Result<PortablePath, Vec<Diagnostic>> {
    let target = target.as_str();
    let (parent, _) = target.rsplit_once('/').ok_or_else(|| {
        journal_diagnostic(
            "repo.change.journal.target.invalid",
            "Repository change staging path is invalid.",
        )
    })?;
    PortablePath::new(format!(
        "{parent}/.jameskills-{}.stage",
        operation_id.as_uuid()
    ))
    .map_err(|_| {
        journal_diagnostic(
            "repo.change.journal.target.invalid",
            "Repository change staging path is invalid.",
        )
    })
}

fn journal_diagnostic(code: &'static str, message: &'static str) -> Vec<Diagnostic> {
    vec![Diagnostic::error(code, message)]
}

pub trait OperationJournalPort: Send + Sync {
    fn record_operation(&self, journal: &RepoChangeJournal) -> crate::AppResult<()>;
    fn load_operation(
        &self,
        operation_id: OperationId,
    ) -> crate::AppResult<Option<RepoChangeJournal>>;
    fn transition_operation(
        &self,
        operation_id: OperationId,
        expected: RepoChangeJournalState,
        next: RepoChangeJournalState,
        updated_at: &str,
    ) -> crate::AppResult<()>;
    fn pending_operations(&self) -> crate::AppResult<Vec<RepoChangeJournal>>;
}
