use super::{
    ContentHash, PortablePath, RevisionId, RevisionRecord, SkillId, ValidatedBundle,
    skill::validate_bundle,
};
use crate::{AppError, AppResult, Diagnostic};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub type ImportFiles = BTreeMap<PortablePath, Vec<u8>>;
const MAX_IMPORT_HEADS: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportSourceKind {
    Directory,
    Archive,
    PlainSkill,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrustState {
    Quarantined,
    Reviewed,
}

/// Redacted result of the optional source scan. No scan outcome changes trust;
/// review is an independent, explicit action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportScanStatus {
    Unavailable,
    NoFindings,
    Findings,
    Unknown,
    Blocked,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImportClassification {
    NewSkill,
    Identical { revision_id: RevisionId },
    Conflict { current_heads: Vec<RevisionId> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportResolution {
    KeepExisting,
    AddConcurrentRoot,
    CreateQuarantinedDraft,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImportResult {
    DraftCreated {
        skill_id: SkillId,
        generation: u64,
        trust_state: TrustState,
    },
    Imported {
        revision: RevisionRecord,
        heads: Vec<RevisionId>,
        trust_state: TrustState,
    },
    Duplicate {
        revision_id: RevisionId,
        heads: Vec<RevisionId>,
        trust_state: TrustState,
    },
    KeptExisting {
        heads: Vec<RevisionId>,
    },
}

/// Immutable preview. Every import begins Quarantined; only an explicit local
/// review operation can construct a future Reviewed state.
#[derive(Clone, PartialEq, Eq)]
pub struct ImportPreview {
    bundle: ValidatedBundle,
    files: ImportFiles,
    source_kind: ImportSourceKind,
    existing_skill: bool,
    current_heads: Vec<RevisionId>,
    classification: ImportClassification,
    trust_state: TrustState,
    scan_status: ImportScanStatus,
}

impl ImportPreview {
    pub fn new(
        files: ImportFiles,
        source_kind: ImportSourceKind,
        existing_skill: bool,
        mut current_heads: Vec<RevisionId>,
        identical_revision: Option<RevisionId>,
    ) -> AppResult<Self> {
        if current_heads.len() > MAX_IMPORT_HEADS {
            return Err(AppError::Validation(vec![Diagnostic::error(
                "library.import.heads.limit",
                "Import conflict has too many current heads to preview safely.",
            )]));
        }
        current_heads.sort();
        current_heads.dedup();
        let bundle = validate_bundle(&files).map_err(AppError::Validation)?;
        let classification = if let Some(revision_id) = identical_revision {
            ImportClassification::Identical { revision_id }
        } else if !existing_skill {
            if !current_heads.is_empty() {
                return Err(AppError::Validation(vec![Diagnostic::error(
                    "library.import.heads.without_skill",
                    "Current heads exist without a matching skill identity.",
                )]));
            }
            ImportClassification::NewSkill
        } else {
            ImportClassification::Conflict {
                current_heads: current_heads.clone(),
            }
        };
        Ok(Self {
            bundle,
            files,
            source_kind,
            existing_skill,
            current_heads,
            classification,
            trust_state: TrustState::Quarantined,
            scan_status: ImportScanStatus::Unavailable,
        })
    }

    pub fn bundle(&self) -> &ValidatedBundle {
        &self.bundle
    }
    pub fn skill_id(&self) -> SkillId {
        self.bundle.manifest().id()
    }
    pub fn slug(&self) -> &str {
        self.bundle.manifest().slug()
    }
    pub fn display_name(&self) -> &str {
        self.bundle.manifest().display_name()
    }
    pub fn semantic_version(&self) -> String {
        self.bundle.manifest().version().to_string()
    }
    pub fn content_hash(&self) -> &ContentHash {
        self.bundle.content_hash()
    }
    pub fn files(&self) -> &ImportFiles {
        &self.files
    }
    pub fn source_kind(&self) -> ImportSourceKind {
        self.source_kind
    }
    pub fn existing_skill(&self) -> bool {
        self.existing_skill
    }
    pub fn current_heads(&self) -> &[RevisionId] {
        &self.current_heads
    }
    pub fn classification(&self) -> &ImportClassification {
        &self.classification
    }
    pub fn trust_state(&self) -> TrustState {
        self.trust_state
    }
    pub fn scan_status(&self) -> ImportScanStatus {
        self.scan_status
    }

    pub(crate) fn with_scan_status(mut self, scan_status: ImportScanStatus) -> Self {
        self.scan_status = scan_status;
        self
    }

    /// Digest shown for a selected import resolution. It binds the validated
    /// bytes, existing head set, source kind, identity class and decision.
    pub fn confirmation_digest(&self, resolution: ImportResolution) -> AppResult<ContentHash> {
        let valid_resolution = match (&self.classification, resolution) {
            (ImportClassification::NewSkill, ImportResolution::AddConcurrentRoot) => {
                self.source_kind != ImportSourceKind::PlainSkill
            }
            (ImportClassification::NewSkill, ImportResolution::CreateQuarantinedDraft) => {
                self.source_kind == ImportSourceKind::PlainSkill
            }
            (ImportClassification::Identical { .. }, ImportResolution::KeepExisting) => {
                self.source_kind != ImportSourceKind::PlainSkill
            }
            (ImportClassification::Conflict { .. }, ImportResolution::KeepExisting) => {
                self.source_kind != ImportSourceKind::PlainSkill
            }
            (ImportClassification::Conflict { .. }, ImportResolution::AddConcurrentRoot) => {
                self.source_kind != ImportSourceKind::PlainSkill
            }
            _ => false,
        };
        if !valid_resolution {
            return Err(AppError::Validation(vec![Diagnostic::error(
                "library.import.resolution.invalid",
                "Import resolution is incompatible with the previewed identity state.",
            )]));
        }
        let mut digest = Sha256::new();
        digest.update(b"JAMESKILLS-IMPORT-CONFIRM-V1\0");
        digest.update(self.skill_id().as_uuid().into_bytes());
        digest.update(self.content_hash().as_str().as_bytes());
        digest.update([match self.source_kind {
            ImportSourceKind::Directory => 1,
            ImportSourceKind::Archive => 2,
            ImportSourceKind::PlainSkill => 3,
        }]);
        digest.update([u8::from(self.existing_skill)]);
        digest.update([match self.scan_status {
            ImportScanStatus::Unavailable => 0,
            ImportScanStatus::NoFindings => 1,
            ImportScanStatus::Findings => 2,
            ImportScanStatus::Unknown => 3,
            ImportScanStatus::Blocked => 4,
        }]);
        match &self.classification {
            ImportClassification::NewSkill => digest.update([1]),
            ImportClassification::Identical { revision_id } => {
                digest.update([2]);
                digest.update(revision_id.as_str().as_bytes());
            }
            ImportClassification::Conflict { .. } => digest.update([3]),
        }
        digest.update([match resolution {
            ImportResolution::KeepExisting => 1,
            ImportResolution::AddConcurrentRoot => 2,
            ImportResolution::CreateQuarantinedDraft => 3,
        }]);
        digest.update((self.current_heads.len() as u32).to_be_bytes());
        for head in &self.current_heads {
            digest.update(head.as_str().as_bytes());
        }
        Ok(ContentHash::from_digest(digest.finalize().into()))
    }
}

#[cfg(test)]
mod tests {
    use super::{ImportPreview, ImportScanStatus, ImportSourceKind, TrustState};
    use crate::domain::PortablePath;
    use std::collections::BTreeMap;

    #[test]
    fn scanner_status_never_changes_import_trust() {
        let files: BTreeMap<PortablePath, Vec<u8>> = [
            (
                "SKILL.md",
                include_bytes!("../../../../docs/examples/repository-foundation/SKILL.md")
                    .as_slice(),
            ),
            (
                "jameskills.toml",
                include_bytes!("../../../../docs/examples/repository-foundation/jameskills.toml")
                    .as_slice(),
            ),
            (
                "policies/repository.toml",
                include_bytes!(
                    "../../../../docs/examples/repository-foundation/policies/repository.toml"
                )
                .as_slice(),
            ),
            (
                "guidance/repository.toml",
                include_bytes!(
                    "../../../../docs/examples/repository-foundation/guidance/repository.toml"
                )
                .as_slice(),
            ),
        ]
        .into_iter()
        .map(|(path, bytes)| (PortablePath::new(path.to_owned()).unwrap(), bytes.to_vec()))
        .collect();

        let unconfigured = ImportPreview::new(
            files.clone(),
            ImportSourceKind::Directory,
            false,
            vec![],
            None,
        )
        .unwrap();
        assert_eq!(unconfigured.scan_status(), ImportScanStatus::Unavailable);
        assert_eq!(unconfigured.trust_state(), TrustState::Quarantined);

        for status in [
            ImportScanStatus::Unavailable,
            ImportScanStatus::NoFindings,
            ImportScanStatus::Findings,
            ImportScanStatus::Unknown,
            ImportScanStatus::Blocked,
        ] {
            let preview = ImportPreview::new(
                files.clone(),
                ImportSourceKind::Directory,
                false,
                vec![],
                None,
            )
            .unwrap()
            .with_scan_status(status);
            assert_eq!(preview.scan_status(), status);
            assert_eq!(preview.trust_state(), TrustState::Quarantined);
        }
    }

    #[test]
    fn import_confirmation_digest_binds_the_redacted_scan_status() {
        let files: BTreeMap<PortablePath, Vec<u8>> = [
            (
                "SKILL.md",
                include_bytes!("../../../../docs/examples/repository-foundation/SKILL.md")
                    .as_slice(),
            ),
            (
                "jameskills.toml",
                include_bytes!("../../../../docs/examples/repository-foundation/jameskills.toml")
                    .as_slice(),
            ),
            (
                "policies/repository.toml",
                include_bytes!(
                    "../../../../docs/examples/repository-foundation/policies/repository.toml"
                )
                .as_slice(),
            ),
            (
                "guidance/repository.toml",
                include_bytes!(
                    "../../../../docs/examples/repository-foundation/guidance/repository.toml"
                )
                .as_slice(),
            ),
        ]
        .into_iter()
        .map(|(path, bytes)| (PortablePath::new(path.to_owned()).unwrap(), bytes.to_vec()))
        .collect();
        let preview =
            ImportPreview::new(files, ImportSourceKind::Directory, false, vec![], None).unwrap();
        let no_findings = preview
            .clone()
            .with_scan_status(ImportScanStatus::NoFindings)
            .confirmation_digest(super::ImportResolution::AddConcurrentRoot)
            .unwrap();
        let blocked = preview
            .with_scan_status(ImportScanStatus::Blocked)
            .confirmation_digest(super::ImportResolution::AddConcurrentRoot)
            .unwrap();
        assert_ne!(no_findings, blocked);
    }
}
