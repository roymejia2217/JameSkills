use super::{ContentHash, OperationId, PortablePath, policy::RepositoryHead};
use crate::Diagnostic;
use sha2::{Digest, Sha256};

const MAX_REPO_CHANGE_BYTES: usize = 1024 * 1024;
const MAX_REPO_CHANGE_DIFF_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RepoTemplateId {
    RustCi,
}

impl RepoTemplateId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RustCi => "rust-ci",
        }
    }

    pub fn target(self) -> &'static str {
        match self {
            Self::RustCi => ".github/workflows/jameskills-ci.yml",
        }
    }
}

/// Immutable preview metadata. Template bytes are kept in the app-owned
/// registry; this DTO retains only hashes and a bounded presentation diff.
#[derive(Clone, PartialEq, Eq)]
pub struct RepoChangePlan {
    operation_id: OperationId,
    template_id: RepoTemplateId,
    root_fingerprint: ContentHash,
    expected_head: RepositoryHead,
    target: PortablePath,
    previous_hash: Option<ContentHash>,
    proposed_hash: ContentHash,
    diff: String,
    confirmation_digest: ContentHash,
}

impl RepoChangePlan {
    pub fn new(
        template_id: RepoTemplateId,
        root_fingerprint: ContentHash,
        expected_head: RepositoryHead,
        previous_bytes: Option<&[u8]>,
        proposed_bytes: &[u8],
        diff: String,
    ) -> Result<Self, Vec<Diagnostic>> {
        if proposed_bytes.is_empty() || diff.is_empty() {
            return Err(vec![Diagnostic::error(
                "repo.change.preview.invalid",
                "Repository change preview must include content and a non-empty diff.",
            )]);
        }
        if proposed_bytes.len() > MAX_REPO_CHANGE_BYTES
            || previous_bytes.is_some_and(|bytes| bytes.len() > MAX_REPO_CHANGE_BYTES)
            || diff.len() > MAX_REPO_CHANGE_DIFF_BYTES
            || diff
                .chars()
                .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
        {
            return Err(vec![Diagnostic::error(
                "repo.change.preview.limit",
                "Repository change preview exceeds its registered limits.",
            )]);
        }
        let target = PortablePath::new(template_id.target().to_owned()).map_err(|_| {
            vec![Diagnostic::error(
                "repo.change.template.invalid",
                "Registered repository template target is invalid.",
            )]
        })?;
        let previous_hash = previous_bytes.map(content_hash);
        let proposed_hash = content_hash(proposed_bytes);
        let confirmation_digest = confirmation_digest(
            template_id,
            &root_fingerprint,
            &expected_head,
            &target,
            previous_hash.as_ref(),
            &proposed_hash,
            &diff,
        );
        Ok(Self {
            operation_id: OperationId::new(),
            template_id,
            root_fingerprint,
            expected_head,
            target,
            previous_hash,
            proposed_hash,
            diff,
            confirmation_digest,
        })
    }

    pub fn operation_id(&self) -> OperationId {
        self.operation_id
    }

    pub fn template_id(&self) -> RepoTemplateId {
        self.template_id
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

    pub fn previous_hash(&self) -> Option<&ContentHash> {
        self.previous_hash.as_ref()
    }

    pub fn proposed_hash(&self) -> &ContentHash {
        &self.proposed_hash
    }

    pub fn diff(&self) -> &str {
        &self.diff
    }

    pub fn confirmation_digest(&self) -> &ContentHash {
        &self.confirmation_digest
    }
}

/// A non-deserializable approval bound to the exact displayed plan. Apply
/// rejects replayed operation IDs using its durable journal.
pub struct ApprovedRepoChange {
    plan: RepoChangePlan,
}

impl ApprovedRepoChange {
    pub fn after_explicit_digest_confirmation(
        plan: RepoChangePlan,
        confirmed_digest: &ContentHash,
    ) -> Result<Self, Vec<Diagnostic>> {
        if &plan.confirmation_digest != confirmed_digest {
            return Err(vec![Diagnostic::error(
                "repo.change.confirmation.mismatch",
                "Repository change confirmation does not match the displayed preview.",
            )]);
        }
        Ok(Self { plan })
    }

    pub fn plan(&self) -> &RepoChangePlan {
        &self.plan
    }
}

fn content_hash(bytes: &[u8]) -> ContentHash {
    ContentHash::from_digest(Sha256::digest(bytes).into())
}

fn confirmation_digest(
    template_id: RepoTemplateId,
    root_fingerprint: &ContentHash,
    expected_head: &RepositoryHead,
    target: &PortablePath,
    previous_hash: Option<&ContentHash>,
    proposed_hash: &ContentHash,
    diff: &str,
) -> ContentHash {
    let mut hasher = Sha256::new();
    hasher.update(b"jameskills-repo-change-v1\0");
    hash_field(&mut hasher, template_id.as_str().as_bytes());
    hash_field(&mut hasher, root_fingerprint.as_str().as_bytes());
    hash_field(&mut hasher, expected_head.as_str().as_bytes());
    hash_field(&mut hasher, target.as_str().as_bytes());
    match previous_hash {
        Some(hash) => {
            hasher.update([1]);
            hash_field(&mut hasher, hash.as_str().as_bytes());
        }
        None => hasher.update([0]),
    }
    hash_field(&mut hasher, proposed_hash.as_str().as_bytes());
    hash_field(&mut hasher, diff.as_bytes());
    ContentHash::from_digest(hasher.finalize().into())
}

fn hash_field(hasher: &mut Sha256, value: &[u8]) {
    hasher.update((value.len() as u64).to_be_bytes());
    hasher.update(value);
}
