use crate::{
    Diagnostic,
    domain::{
        AssetFiles, AssetPreview, ContentHash, CreateSkill, DeleteRequest, ForkRequest,
        ImportPreview, ImportResolution, ImportResult, ImportScanStatus, ImportSourceKind,
        PortablePath, RepositoryBinding, RepositoryBindingReport, RestoreRevisionRequest,
        RevisionId, RevisionKind, RevisionTrust, SaveRevisionRequest, SaveRevisionResult,
        SkillDraft, SkillId, TrustState, ValidatedBundle,
        assets::{
            add_asset as add_asset_files, preview_asset as build_asset_preview,
            remove_asset as remove_asset_file, rename_bundle_path,
            replace_asset as replace_asset_file,
        },
        validate_bundle,
    },
    ports::{
        BundleFiles, ExportDestinationState, ImportScanPort, LibraryHistoryPage,
        LibraryHistoryQuery, LibraryPage, LibraryQuery, LibrarySkillDetail, SaveDraftRequest,
        StoragePort, filesystem::FileSystemPort,
    },
};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::sync::Arc;

const MAX_PUBLISH_EXPECTED_HEADS: usize = 128;

fn export_confirmation_digest(
    bundle: &ExportedBundle,
    destination: &Path,
    state: &ExportDestinationState,
    overwrite: bool,
) -> ContentHash {
    let mut digest = Sha256::new();
    digest.update(b"JAMESKILLS-EXPORT-CONFIRM-V1\0");
    digest.update(bundle.skill_id().as_uuid().into_bytes());
    digest.update(bundle.revision_id().as_str().as_bytes());
    digest.update(bundle.content_hash().as_str().as_bytes());
    digest.update(Sha256::digest(bundle.archive_bytes()));
    let destination_bytes = destination.as_os_str().as_encoded_bytes();
    digest.update((destination_bytes.len() as u64).to_be_bytes());
    digest.update(destination_bytes);
    match state {
        ExportDestinationState::Missing => digest.update([0]),
        ExportDestinationState::Existing(hash) => {
            digest.update([1]);
            digest.update(hash.as_str().as_bytes());
        }
    }
    digest.update([u8::from(overwrite)]);
    ContentHash::from_digest(digest.finalize().into())
}

/// Explicit publication intent tied to the saved draft generation and every
/// revision head the editor observed when it began publishing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublishDraft {
    skill_id: SkillId,
    draft_generation: u64,
    expected_heads: Vec<crate::domain::RevisionId>,
}

/// Selects a stored content revision to export. Omitting the revision is valid
/// only when the skill has exactly one active head.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportRequest {
    skill_id: SkillId,
    revision_id: Option<RevisionId>,
}

impl ExportRequest {
    pub fn new(skill_id: SkillId, revision_id: Option<RevisionId>) -> Self {
        Self {
            skill_id,
            revision_id,
        }
    }

    pub fn skill_id(&self) -> SkillId {
        self.skill_id
    }

    pub fn revision_id(&self) -> Option<&RevisionId> {
        self.revision_id.as_ref()
    }
}

/// Deterministic, portable `.jskill` bytes for one immutable content revision.
/// Local trust metadata, SQLite rows and filesystem destinations are excluded.
pub struct ExportedBundle {
    skill_id: SkillId,
    revision_id: RevisionId,
    content_hash: ContentHash,
    archive_bytes: Vec<u8>,
}

/// Read-only export preview bound to destination state, revision bytes and the
/// exact path. It is rebuilt before apply rather than deserialized from JSON.
pub struct ExportPreview {
    bundle: ExportedBundle,
    destination: std::path::PathBuf,
    destination_state: ExportDestinationState,
    create_digest: ContentHash,
    overwrite_digest: ContentHash,
}

impl ExportPreview {
    pub fn bundle(&self) -> &ExportedBundle {
        &self.bundle
    }

    pub fn destination_state(&self) -> &ExportDestinationState {
        &self.destination_state
    }

    pub fn confirmation_digest(&self, overwrite: bool) -> crate::AppResult<&ContentHash> {
        match (&self.destination_state, overwrite) {
            (ExportDestinationState::Missing, false) => Ok(&self.create_digest),
            (ExportDestinationState::Existing(_), true) => Ok(&self.overwrite_digest),
            (ExportDestinationState::Existing(_), false) => {
                Err(crate::AppError::PermissionDenied {
                    operation: "library.export.overwrite.required".to_owned(),
                })
            }
            (ExportDestinationState::Missing, true) => {
                Err(crate::AppError::Validation(vec![Diagnostic::error(
                    "library.export.overwrite.unexpected",
                    "Overwrite was selected for a destination that did not exist in preview.",
                )]))
            }
        }
    }
}

impl ExportedBundle {
    pub fn skill_id(&self) -> SkillId {
        self.skill_id
    }

    pub fn revision_id(&self) -> &RevisionId {
        &self.revision_id
    }

    pub fn content_hash(&self) -> &ContentHash {
        &self.content_hash
    }

    pub fn archive_bytes(&self) -> &[u8] {
        &self.archive_bytes
    }
}

impl PublishDraft {
    pub fn new(
        skill_id: SkillId,
        draft_generation: u64,
        mut expected_heads: Vec<crate::domain::RevisionId>,
    ) -> crate::AppResult<Self> {
        if draft_generation == 0 || expected_heads.len() > MAX_PUBLISH_EXPECTED_HEADS {
            return Err(crate::AppError::Validation(vec![Diagnostic::error(
                "library.publish.request.invalid",
                "Publish request is invalid or exceeds its head limit.",
            )]));
        }
        expected_heads.sort();
        expected_heads.dedup();
        Ok(Self {
            skill_id,
            draft_generation,
            expected_heads,
        })
    }

    pub fn skill_id(&self) -> SkillId {
        self.skill_id
    }
    pub fn draft_generation(&self) -> u64 {
        self.draft_generation
    }
    pub fn expected_heads(&self) -> &[crate::domain::RevisionId] {
        &self.expected_heads
    }
}

/// Read-only library queries plus pure import validation over injected ports.
pub struct LibraryService {
    filesystem: Arc<dyn FileSystemPort>,
    storage: Option<Arc<dyn StoragePort>>,
    import_scanner: Option<Arc<dyn ImportScanPort>>,
}

impl LibraryService {
    pub fn new(filesystem: Arc<dyn FileSystemPort>) -> Self {
        Self {
            filesystem,
            storage: None,
            import_scanner: None,
        }
    }

    pub fn with_storage(mut self, storage: Arc<dyn StoragePort>) -> Self {
        self.storage = Some(storage);
        self
    }

    fn storage(&self) -> crate::AppResult<&dyn StoragePort> {
        self.storage
            .as_deref()
            .ok_or_else(|| crate::AppError::CapabilityUnavailable {
                id: "library.storage.unavailable".to_owned(),
                guidance_id: "library.storage.setup".to_owned(),
            })
    }

    pub fn with_import_scanner(mut self, scanner: Arc<dyn ImportScanPort>) -> Self {
        self.import_scanner = Some(scanner);
        self
    }

    pub fn validate_import(&self, root: &Path) -> Result<ValidatedBundle, Vec<Diagnostic>> {
        let files = self.filesystem.read_bundle_directory(root)?;
        validate_bundle(&files)
    }

    pub async fn preview_import(
        &self,
        source: &Path,
        source_kind: ImportSourceKind,
    ) -> crate::AppResult<ImportPreview> {
        self.preview_import_with_skill_id(source, source_kind, None)
            .await
    }

    /// Recreates a plain-skill preview with the identity shown in a prior
    /// preview, allowing a separate CLI apply invocation to verify its digest.
    pub async fn preview_import_with_skill_id(
        &self,
        source: &Path,
        source_kind: ImportSourceKind,
        requested_skill_id: Option<SkillId>,
    ) -> crate::AppResult<ImportPreview> {
        let mut files = self
            .filesystem
            .read_bundle_source(source)
            .map_err(crate::AppError::Validation)?;
        let manifest_path = PortablePath::new("jameskills.toml".to_owned())
            .map_err(|_| crate::AppError::CryptoInvalid)?;
        let skill_path =
            PortablePath::new("SKILL.md".to_owned()).map_err(|_| crate::AppError::CryptoInvalid)?;
        let source_kind = if files.contains_key(&manifest_path) {
            if source_kind == ImportSourceKind::PlainSkill || requested_skill_id.is_some() {
                return Err(crate::AppError::Validation(vec![Diagnostic::error(
                    "library.import.source_kind.mismatch",
                    "Plain-skill identity overrides cannot be used with a manifested bundle.",
                )]));
            }
            source_kind
        } else if files.len() == 1 && files.contains_key(&skill_path) {
            let source_bytes = files.get(&skill_path).cloned().unwrap_or_default();
            let (_, draft) = match requested_skill_id {
                Some(skill_id) => {
                    CreateSkill::from_plain_skill_source_with_id(source_bytes, skill_id)?
                }
                None => CreateSkill::from_plain_skill_source(source_bytes)?,
            };
            files = draft.files().clone();
            ImportSourceKind::PlainSkill
        } else {
            return Err(crate::AppError::Validation(vec![Diagnostic::error(
                "library.import.manifest.missing",
                "A directory without a manifest can only import a standalone SKILL.md.",
            )]));
        };
        let validated = validate_bundle(&files).map_err(crate::AppError::Validation)?;
        let skill_id = validated.manifest().id();
        let storage = self.storage()?;
        let existing_skill = storage.skill_exists(skill_id).await?;
        let current_heads = storage.get_heads(skill_id).await?;
        let identical_revision = storage
            .find_revision_by_bundle(skill_id, validated.content_hash().clone())
            .await?;
        let preview = ImportPreview::new(
            files,
            source_kind,
            existing_skill,
            current_heads,
            identical_revision,
        )?;
        let Some(scanner) = &self.import_scanner else {
            return Ok(preview);
        };
        let scan_status = match scanner.scan(preview.files()).await {
            Ok(status) => status,
            Err(crate::AppError::Cancelled) => return Err(crate::AppError::Cancelled),
            Err(_) => ImportScanStatus::Unknown,
        };
        Ok(preview.with_scan_status(scan_status))
    }

    pub async fn apply_import(
        &self,
        preview: ImportPreview,
        resolution: ImportResolution,
    ) -> crate::AppResult<ImportResult> {
        self.storage()?.apply_import(preview, resolution).await
    }

    pub async fn export_bundle(&self, request: ExportRequest) -> crate::AppResult<ExportedBundle> {
        let storage = self.storage()?;
        let detail = storage
            .load_skill(request.skill_id())
            .await?
            .ok_or(crate::AppError::NotFound)?;
        let revision_id = match request.revision_id() {
            Some(revision_id) => revision_id.clone(),
            None if detail.heads().len() == 1 && !detail.heads()[0].summary().deleted() => {
                detail.heads()[0].summary().revision_id().clone()
            }
            None if detail.heads().is_empty() => return Err(crate::AppError::NotFound),
            None => {
                return Err(crate::AppError::Conflict {
                    current: detail
                        .heads()
                        .iter()
                        .map(|head| head.summary().revision_id().clone())
                        .collect(),
                });
            }
        };
        let files = if let Some(head) = detail
            .heads()
            .iter()
            .find(|head| head.summary().revision_id() == &revision_id)
        {
            if head.summary().deleted() {
                return Err(crate::AppError::NotFound);
            }
            head.files().cloned().ok_or(crate::AppError::NotFound)?
        } else {
            storage
                .load_revision_files(request.skill_id(), &revision_id)
                .await?
                .ok_or(crate::AppError::NotFound)?
        };
        let validated = validate_bundle(&files).map_err(crate::AppError::Validation)?;
        if validated.manifest().id() != request.skill_id() {
            return Err(crate::AppError::Storage {
                code: "library.export.skill_id.mismatch".to_owned(),
            });
        }
        let archive_bytes =
            crate::ports::write_bundle_archive(&files).map_err(crate::AppError::Validation)?;
        Ok(ExportedBundle {
            skill_id: request.skill_id(),
            revision_id,
            content_hash: validated.content_hash().clone(),
            archive_bytes,
        })
    }

    pub async fn preview_export(
        &self,
        request: ExportRequest,
        destination: std::path::PathBuf,
    ) -> crate::AppResult<ExportPreview> {
        let bundle = self.export_bundle(request).await?;
        let destination_state = self.inspect_export_destination(&destination)?;
        let create_digest =
            export_confirmation_digest(&bundle, &destination, &destination_state, false);
        let overwrite_digest =
            export_confirmation_digest(&bundle, &destination, &destination_state, true);
        Ok(ExportPreview {
            bundle,
            destination,
            destination_state,
            create_digest,
            overwrite_digest,
        })
    }

    /// Rebuilds the selected archive, checks the preview digest and destination
    /// snapshot again, then delegates an atomic/create-only filesystem commit.
    pub async fn apply_export(
        &self,
        preview: ExportPreview,
        overwrite: bool,
        confirmation_digest: &ContentHash,
    ) -> crate::AppResult<ExportedBundle> {
        let expected_digest = preview.confirmation_digest(overwrite)?;
        if expected_digest != confirmation_digest {
            return Err(crate::AppError::Conflict {
                current: Vec::new(),
            });
        }
        let current_state = self.inspect_export_destination(&preview.destination)?;
        if current_state != preview.destination_state {
            return Err(crate::AppError::Conflict {
                current: Vec::new(),
            });
        }
        let current_bundle = self
            .export_bundle(ExportRequest::new(
                preview.bundle.skill_id(),
                Some(preview.bundle.revision_id().clone()),
            ))
            .await?;
        if current_bundle.content_hash() != preview.bundle.content_hash()
            || current_bundle.archive_bytes() != preview.bundle.archive_bytes()
        {
            return Err(crate::AppError::Conflict {
                current: Vec::new(),
            });
        }
        self.write_export_archive(
            &preview.destination,
            current_bundle.archive_bytes(),
            &preview.destination_state,
            overwrite,
        )?;
        Ok(current_bundle)
    }

    pub fn inspect_export_destination(
        &self,
        destination: &Path,
    ) -> crate::AppResult<ExportDestinationState> {
        self.filesystem.inspect_export_destination(destination)
    }

    pub fn write_export_archive(
        &self,
        destination: &Path,
        archive_bytes: &[u8],
        expected_state: &ExportDestinationState,
        overwrite: bool,
    ) -> crate::AppResult<()> {
        self.filesystem
            .write_export_archive(destination, archive_bytes, expected_state, overwrite)
    }

    pub async fn list_skills(&self, query: LibraryQuery) -> crate::AppResult<LibraryPage> {
        self.storage()?.list_skills(query).await
    }

    /// Saves a local repository binding only while its exact suite revision is
    /// still a content head of the selected skill.
    pub async fn bind_repository(
        &self,
        binding: RepositoryBinding,
    ) -> crate::AppResult<RepositoryBinding> {
        let storage = self.storage()?;
        let existing = storage
            .list_repository_bindings(binding.skill_id())
            .await?
            .into_iter()
            .find(|existing| {
                existing.repository_root() == binding.repository_root()
                    && existing.profile() == binding.profile()
            });
        let binding = match existing {
            Some(existing) => RepositoryBinding::from_storage(
                existing.id().to_owned(),
                binding.skill_id(),
                binding.suite_revision().clone(),
                binding.repository_root().to_path_buf(),
                binding.profile(),
                binding.strict(),
                binding.repository_head().clone(),
                binding.environment_fingerprint().to_owned(),
            )?,
            None => binding,
        };
        let current_heads = storage.get_heads(binding.skill_id()).await?;
        if !current_heads.contains(binding.suite_revision()) {
            return Err(crate::AppError::Conflict {
                current: current_heads,
            });
        }
        let files = storage
            .load_revision_files(binding.skill_id(), binding.suite_revision())
            .await?
            .ok_or(crate::AppError::NotFound)?;
        let suite = validate_bundle(&files).map_err(crate::AppError::Validation)?;
        if suite.manifest().id() != binding.skill_id() {
            return Err(crate::AppError::Storage {
                code: "library.binding.suite.identity_mismatch".to_owned(),
            });
        }
        let latest_heads = storage.get_heads(binding.skill_id()).await?;
        if latest_heads != current_heads {
            return Err(crate::AppError::Conflict {
                current: latest_heads,
            });
        }
        storage.save_repository_binding(binding.clone()).await?;
        Ok(binding)
    }

    pub async fn list_repository_bindings(
        &self,
        skill_id: SkillId,
    ) -> crate::AppResult<Vec<RepositoryBinding>> {
        self.storage()?.list_repository_bindings(skill_id).await
    }

    pub async fn load_skill(
        &self,
        skill_id: SkillId,
    ) -> crate::AppResult<Option<LibrarySkillDetail>> {
        self.storage()?.load_skill(skill_id).await
    }

    pub async fn load_history(
        &self,
        query: LibraryHistoryQuery,
    ) -> crate::AppResult<LibraryHistoryPage> {
        self.storage()?.load_history(query).await
    }

    pub async fn load_revision_trust(
        &self,
        skill_id: SkillId,
        revision_id: &RevisionId,
    ) -> crate::AppResult<RevisionTrust> {
        self.storage()?
            .load_revision_trust(skill_id, revision_id)
            .await
    }

    pub async fn load_validated_revision(
        &self,
        skill_id: SkillId,
        revision_id: &RevisionId,
    ) -> crate::AppResult<ValidatedBundle> {
        let files = self
            .storage()?
            .load_revision_files(skill_id, revision_id)
            .await?
            .ok_or(crate::AppError::NotFound)?;
        let bundle = validate_bundle(&files).map_err(crate::AppError::Validation)?;
        if bundle.manifest().id() != skill_id {
            return Err(crate::AppError::Storage {
                code: "library.revision.skill_id.mismatch".to_owned(),
            });
        }
        Ok(bundle)
    }

    pub async fn load_repository_binding(
        &self,
        binding_id: &str,
    ) -> crate::AppResult<Option<RepositoryBinding>> {
        self.storage()?.load_repository_binding(binding_id).await
    }

    pub async fn load_repository_binding_report(
        &self,
        binding_id: &str,
    ) -> crate::AppResult<Option<RepositoryBindingReport>> {
        self.storage()?
            .load_repository_binding_report(binding_id)
            .await
    }

    pub async fn save_repository_binding_report(
        &self,
        binding: &RepositoryBinding,
        report: RepositoryBindingReport,
    ) -> crate::AppResult<()> {
        self.storage()?
            .save_repository_binding_report(binding, report)
            .await
    }

    /// Commits a causal soft-delete only if every current head is exactly the
    /// set the user reviewed. The content blobs and revision history remain.
    pub async fn delete_skill(
        &self,
        request: DeleteRequest,
    ) -> crate::AppResult<SaveRevisionResult> {
        let storage = self.storage()?;
        let detail = storage
            .load_skill(request.skill_id())
            .await?
            .ok_or(crate::AppError::NotFound)?;
        let current_heads = storage.get_heads(request.skill_id()).await?;
        if current_heads != request.expected_heads() {
            return Err(crate::AppError::Conflict {
                current: current_heads,
            });
        }

        let detail_heads = detail
            .heads()
            .iter()
            .map(|head| head.summary().revision_id().clone())
            .collect::<Vec<_>>();
        if detail_heads != current_heads {
            return Err(crate::AppError::Conflict {
                current: current_heads,
            });
        }

        let content_versions = detail
            .heads()
            .iter()
            .filter(|head| !head.summary().deleted())
            .map(|head| {
                head.summary()
                    .semantic_version()
                    .parse::<semver::Version>()
                    .map_err(|_| crate::AppError::Storage {
                        code: "library.revision.version.corrupt".to_owned(),
                    })
            })
            .collect::<crate::AppResult<Vec<_>>>()?;
        let latest_content_version = content_versions
            .into_iter()
            .max()
            .ok_or(crate::AppError::NotFound)?
            .to_string();
        let heads = request.expected_heads().to_vec();
        let tombstone = SaveRevisionRequest::new(
            request.skill_id(),
            None,
            heads.clone(),
            RevisionKind::Tombstone {
                observed_heads: heads.clone(),
            },
            latest_content_version,
            1,
            heads,
        );
        storage.commit_revision(tombstone).await
    }

    /// Restores selected historical content by writing a new validated content
    /// revision. It never moves a head backward and preserves local trust.
    pub async fn restore_revision_as_new(
        &self,
        request: RestoreRevisionRequest,
    ) -> crate::AppResult<SaveRevisionResult> {
        self.restore_revision(request, false).await
    }

    /// Reopens a deleted skill only when every current head is a tombstone.
    pub async fn restore_deleted_skill(
        &self,
        request: RestoreRevisionRequest,
    ) -> crate::AppResult<SaveRevisionResult> {
        self.restore_revision(request, true).await
    }

    async fn restore_revision(
        &self,
        request: RestoreRevisionRequest,
        deleted_only: bool,
    ) -> crate::AppResult<SaveRevisionResult> {
        let storage = self.storage()?;
        let detail = storage
            .load_skill(request.skill_id())
            .await?
            .ok_or(crate::AppError::NotFound)?;
        let current_heads = storage.get_heads(request.skill_id()).await?;
        if current_heads != request.expected_heads() {
            return Err(crate::AppError::Conflict {
                current: current_heads,
            });
        }
        let detail_heads = detail
            .heads()
            .iter()
            .map(|head| head.summary().revision_id().clone())
            .collect::<Vec<_>>();
        if detail_heads != current_heads {
            return Err(crate::AppError::Conflict {
                current: current_heads,
            });
        }
        if deleted_only && detail.heads().iter().any(|head| !head.summary().deleted()) {
            return Err(crate::AppError::Conflict {
                current: current_heads,
            });
        }
        let next_version = request
            .next_version()
            .parse::<semver::Version>()
            .map_err(|_| crate::AppError::CryptoInvalid)?;
        for head in detail.heads() {
            let previous = head
                .summary()
                .semantic_version()
                .parse::<semver::Version>()
                .map_err(|_| crate::AppError::Storage {
                    code: "library.revision.version.corrupt".to_owned(),
                })?;
            if next_version <= previous {
                return Err(crate::AppError::Validation(vec![Diagnostic::error(
                    "revision.version.bump_required",
                    "Restoring content requires a version newer than every current head.",
                )]));
            }
        }

        let source_trust = storage
            .load_revision_trust(request.skill_id(), request.source_revision_id())
            .await?;
        let mut files = storage
            .load_revision_files(request.skill_id(), request.source_revision_id())
            .await?
            .ok_or(crate::AppError::NotFound)?;
        rewrite_bundle_version(&mut files, &next_version.to_string())?;
        let validated = validate_bundle(&files).map_err(crate::AppError::Validation)?;
        if validated.manifest().id() != request.skill_id()
            || validated.manifest().version() != &next_version
        {
            return Err(crate::AppError::Storage {
                code: "library.restore.source.invalid".to_owned(),
            });
        }
        storage
            .store_validated_bundle(validated.clone(), files)
            .await?;
        let parents = current_heads.clone();
        let revision = SaveRevisionRequest::new(
            request.skill_id(),
            Some(validated.content_hash().clone()),
            parents.clone(),
            RevisionKind::Content,
            next_version.to_string(),
            validated.manifest().schema_version(),
            current_heads,
        )
        .with_validated_bundle(&validated)?
        .with_trust(source_trust)?;
        storage.commit_revision(revision).await
    }

    /// Creates a new draft identity containing the selected revision's exact
    /// skill content, while keeping imported quarantine local and unchanged.
    pub async fn fork_skill(&self, request: ForkRequest) -> crate::AppResult<SkillDraft> {
        let storage = self.storage()?;
        let source_trust = storage
            .load_revision_trust(request.source_skill_id(), request.source_revision_id())
            .await?;
        let mut files = storage
            .load_revision_files(request.source_skill_id(), request.source_revision_id())
            .await?
            .ok_or(crate::AppError::NotFound)?;
        let description = bundle_description(&files)?;
        let create = CreateSkill::new_with_description(
            request.new_slug().to_owned(),
            request.new_display_name().to_owned(),
            description,
        )?;
        rewrite_bundle_identity(
            &mut files,
            create.skill_id(),
            create.slug(),
            create.display_name(),
        )?;
        let validated = validate_bundle(&files).map_err(crate::AppError::Validation)?;
        let manifest = validated.manifest();
        if manifest.id() != create.skill_id()
            || manifest.slug() != create.slug()
            || manifest.display_name() != create.display_name()
        {
            return Err(crate::AppError::Storage {
                code: "library.fork.source.invalid".to_owned(),
            });
        }
        let draft = match source_trust.state() {
            TrustState::Reviewed => SkillDraft::new(create.skill_id(), None, 1, files)?,
            TrustState::Quarantined => {
                SkillDraft::new_quarantined(create.skill_id(), None, 1, files)?
            }
        };
        storage.create_skill(create, draft.clone()).await?;
        Ok(draft)
    }

    pub async fn load_draft(
        &self,
        skill_id: SkillId,
    ) -> crate::AppResult<Option<crate::domain::SkillDraft>> {
        self.storage()?.load_draft(skill_id).await
    }

    pub async fn save_draft(&self, request: SaveDraftRequest) -> crate::AppResult<()> {
        self.storage()?.save_draft(request).await
    }

    pub async fn create_skill(&self, create: CreateSkill) -> crate::AppResult<SkillDraft> {
        let draft = create.initial_draft()?;
        self.storage()?.create_skill(create, draft.clone()).await?;
        Ok(draft)
    }

    pub async fn publish(&self, request: PublishDraft) -> crate::AppResult<SaveRevisionResult> {
        let storage = self.storage()?;
        let draft = storage
            .load_draft(request.skill_id())
            .await?
            .ok_or(crate::AppError::NotFound)?;
        if draft.generation() != request.draft_generation() {
            return Err(crate::AppError::Conflict {
                current: request.expected_heads().to_vec(),
            });
        }
        if draft.trust_state() != TrustState::Reviewed {
            return Err(crate::AppError::UntrustedInput {
                code: "library.draft.review_required".to_owned(),
            });
        }
        let draft_base = draft.base_head().cloned().into_iter().collect::<Vec<_>>();
        if draft_base != request.expected_heads() {
            return Err(crate::AppError::Conflict {
                current: request.expected_heads().to_vec(),
            });
        }
        let validated = validate_bundle(draft.files()).map_err(crate::AppError::Validation)?;
        let manifest = validated.manifest();
        if manifest.id() != request.skill_id() {
            return Err(crate::AppError::Validation(vec![Diagnostic::error(
                "library.publish.skill_id.mismatch",
                "Validated manifest ID does not match the selected skill.",
            )]));
        }
        storage
            .store_validated_bundle(validated.clone(), draft.files().clone())
            .await?;
        let revision = SaveRevisionRequest::new(
            request.skill_id(),
            Some(validated.content_hash().clone()),
            request.expected_heads().to_vec(),
            RevisionKind::Content,
            manifest.version().to_string(),
            manifest.schema_version(),
            request.expected_heads().to_vec(),
        )
        .with_validated_bundle(&validated)?
        .with_expected_draft_generation(request.draft_generation())?;
        storage.commit_revision(revision).await
    }

    pub async fn add_asset(
        &self,
        skill_id: SkillId,
        generation: u64,
        path: &str,
        bytes: Vec<u8>,
    ) -> crate::AppResult<SkillDraft> {
        self.mutate_asset(skill_id, generation, |files| {
            add_asset_files(files, path, bytes)
        })
        .await
    }

    pub async fn replace_asset(
        &self,
        skill_id: SkillId,
        generation: u64,
        path: &str,
        expected_hash: &ContentHash,
        bytes: Vec<u8>,
    ) -> crate::AppResult<SkillDraft> {
        self.mutate_asset(skill_id, generation, |files| {
            replace_asset_file(files, path, expected_hash, bytes)
        })
        .await
    }

    pub async fn remove_asset(
        &self,
        skill_id: SkillId,
        generation: u64,
        path: &str,
        expected_hash: &ContentHash,
    ) -> crate::AppResult<SkillDraft> {
        self.mutate_asset(skill_id, generation, |files| {
            remove_asset_file(files, path, expected_hash)
        })
        .await
    }

    pub async fn rename_asset(
        &self,
        skill_id: SkillId,
        generation: u64,
        from: &str,
        to: &str,
        expected_hash: &ContentHash,
    ) -> crate::AppResult<SkillDraft> {
        self.mutate_asset(skill_id, generation, |files| {
            rename_bundle_path(files, from, to, expected_hash)
        })
        .await
    }

    pub async fn preview_asset(
        &self,
        skill_id: SkillId,
        path: &str,
    ) -> crate::AppResult<AssetPreview> {
        let path = PortablePath::new(path.to_owned()).map_err(|_| {
            crate::AppError::Validation(vec![Diagnostic::error(
                "library.asset.path.nonportable",
                "Asset preview path is not portable.",
            )])
        })?;
        let storage = self.storage()?;
        let draft = storage
            .load_draft(skill_id)
            .await?
            .ok_or(crate::AppError::NotFound)?;
        let bytes = draft.files().get(&path).ok_or(crate::AppError::NotFound)?;
        Ok(build_asset_preview(&path, bytes))
    }

    async fn mutate_asset(
        &self,
        skill_id: SkillId,
        expected_generation: u64,
        edit: impl FnOnce(&AssetFiles) -> crate::AppResult<AssetFiles> + Send,
    ) -> crate::AppResult<SkillDraft> {
        let storage = self.storage()?;
        let draft = storage
            .load_draft(skill_id)
            .await?
            .ok_or(crate::AppError::NotFound)?;
        if draft.generation() != expected_generation {
            return Err(crate::AppError::Conflict {
                current: draft.base_head().cloned().into_iter().collect(),
            });
        }
        let files = edit(draft.files())?;
        let updated = draft.replace_files(files)?;
        let request = SaveDraftRequest::new(
            updated.clone(),
            Some(expected_generation),
            draft.base_head().cloned(),
        )?;
        storage.save_draft(request).await?;
        Ok(updated)
    }
}

fn rewrite_bundle_version(files: &mut BundleFiles, version: &str) -> crate::AppResult<()> {
    let manifest_path = PortablePath::new("jameskills.toml".to_owned())
        .map_err(|_| crate::AppError::CryptoInvalid)?;
    let mut manifest = bundle_manifest(files, "library.restore.manifest.corrupt")?;
    let manifest_table = manifest
        .as_table_mut()
        .ok_or_else(|| crate::AppError::Storage {
            code: "library.restore.manifest.corrupt".to_owned(),
        })?;
    manifest_table.insert(
        "version".to_owned(),
        toml::Value::String(version.to_owned()),
    );
    let manifest_bytes = toml::to_string(&manifest)
        .map_err(|_| crate::AppError::Storage {
            code: "library.restore.manifest.corrupt".to_owned(),
        })?
        .into_bytes();
    files.insert(manifest_path, manifest_bytes);

    let skill_path =
        PortablePath::new("SKILL.md".to_owned()).map_err(|_| crate::AppError::CryptoInvalid)?;
    let skill_bytes = files.get(&skill_path).ok_or(crate::AppError::NotFound)?;
    let frontmatter = crate::domain::skill::parse_frontmatter(skill_bytes).map_err(|_| {
        crate::AppError::Storage {
            code: "library.restore.frontmatter.corrupt".to_owned(),
        }
    })?;
    let mut metadata = frontmatter.metadata().clone();
    if metadata.contains_key("jameskills-version") {
        metadata.insert("jameskills-version".to_owned(), version.to_owned());
    }
    files.insert(
        skill_path,
        serialize_frontmatter(&frontmatter, frontmatter.name(), metadata),
    );
    Ok(())
}

fn bundle_description(files: &BundleFiles) -> crate::AppResult<String> {
    bundle_manifest(files, "library.fork.manifest.corrupt")?
        .get("description")
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| crate::AppError::Storage {
            code: "library.fork.manifest.corrupt".to_owned(),
        })
}

fn rewrite_bundle_identity(
    files: &mut BundleFiles,
    skill_id: SkillId,
    slug: &str,
    display_name: &str,
) -> crate::AppResult<()> {
    let manifest_path = PortablePath::new("jameskills.toml".to_owned())
        .map_err(|_| crate::AppError::CryptoInvalid)?;
    let mut manifest = bundle_manifest(files, "library.fork.manifest.corrupt")?;
    let table = manifest
        .as_table_mut()
        .ok_or_else(|| crate::AppError::Storage {
            code: "library.fork.manifest.corrupt".to_owned(),
        })?;
    let policy_paths = table
        .get("policy_files")
        .and_then(toml::Value::as_array)
        .map(|paths| {
            paths
                .iter()
                .map(|path| {
                    path.as_str()
                        .map(str::to_owned)
                        .ok_or_else(|| crate::AppError::Storage {
                            code: "library.fork.manifest.corrupt".to_owned(),
                        })
                })
                .collect::<crate::AppResult<Vec<_>>>()
        })
        .transpose()?
        .unwrap_or_default();
    table.insert(
        "id".to_owned(),
        toml::Value::String(skill_id.as_uuid().to_string()),
    );
    table.insert("slug".to_owned(), toml::Value::String(slug.to_owned()));
    table.insert(
        "display_name".to_owned(),
        toml::Value::String(display_name.to_owned()),
    );
    let bytes = toml::to_string(&manifest)
        .map_err(|_| crate::AppError::Storage {
            code: "library.fork.manifest.corrupt".to_owned(),
        })?
        .into_bytes();
    files.insert(manifest_path, bytes);
    for policy_path in policy_paths {
        let path = PortablePath::new(policy_path).map_err(|_| crate::AppError::Storage {
            code: "library.fork.policy_path.invalid".to_owned(),
        })?;
        let bytes = files.get(&path).ok_or(crate::AppError::NotFound)?;
        let text = std::str::from_utf8(bytes).map_err(|_| crate::AppError::Storage {
            code: "library.fork.policy.corrupt".to_owned(),
        })?;
        let mut policy: toml::Value =
            toml::from_str(text).map_err(|_| crate::AppError::Storage {
                code: "library.fork.policy.corrupt".to_owned(),
            })?;
        let policy_table = policy
            .as_table_mut()
            .ok_or_else(|| crate::AppError::Storage {
                code: "library.fork.policy.corrupt".to_owned(),
            })?;
        policy_table.insert("profile".to_owned(), toml::Value::String(slug.to_owned()));
        files.insert(
            path,
            toml::to_string(&policy)
                .map_err(|_| crate::AppError::Storage {
                    code: "library.fork.policy.corrupt".to_owned(),
                })?
                .into_bytes(),
        );
    }

    let skill_path =
        PortablePath::new("SKILL.md".to_owned()).map_err(|_| crate::AppError::CryptoInvalid)?;
    let bytes = files.get(&skill_path).ok_or(crate::AppError::NotFound)?;
    let frontmatter =
        crate::domain::skill::parse_frontmatter(bytes).map_err(|_| crate::AppError::Storage {
            code: "library.fork.frontmatter.corrupt".to_owned(),
        })?;
    let mut metadata = frontmatter.metadata().clone();
    if metadata.contains_key("jameskills-id") {
        metadata.insert("jameskills-id".to_owned(), skill_id.as_uuid().to_string());
    }
    files.insert(
        skill_path,
        serialize_frontmatter(&frontmatter, slug, metadata),
    );
    Ok(())
}

fn bundle_manifest(files: &BundleFiles, error_code: &'static str) -> crate::AppResult<toml::Value> {
    let path = PortablePath::new("jameskills.toml".to_owned())
        .map_err(|_| crate::AppError::CryptoInvalid)?;
    let bytes = files.get(&path).ok_or(crate::AppError::NotFound)?;
    let text = std::str::from_utf8(bytes).map_err(|_| crate::AppError::Storage {
        code: error_code.to_owned(),
    })?;
    toml::from_str(text).map_err(|_| crate::AppError::Storage {
        code: error_code.to_owned(),
    })
}

fn serialize_frontmatter(
    frontmatter: &crate::domain::SkillFrontmatter,
    name: &str,
    metadata: std::collections::BTreeMap<String, String>,
) -> Vec<u8> {
    let mut skill = String::from("---\nname: ");
    skill.push_str(&yaml_quote(name));
    skill.push_str("\ndescription: ");
    skill.push_str(&yaml_quote(frontmatter.description()));
    if let Some(compatibility) = frontmatter.compatibility() {
        skill.push_str("\ncompatibility: ");
        skill.push_str(&yaml_quote(compatibility));
    }
    if !metadata.is_empty() {
        skill.push_str("\nmetadata:\n");
        for (key, value) in metadata {
            skill.push_str("  ");
            skill.push_str(&yaml_quote(&key));
            skill.push_str(": ");
            skill.push_str(&yaml_quote(&value));
            skill.push('\n');
        }
        skill.pop();
    }
    skill.push_str("\n---\n");
    skill.push_str(frontmatter.body());
    skill.into_bytes()
}

fn yaml_quote(value: &str) -> String {
    let mut quoted = String::with_capacity(value.len() + 2);
    quoted.push('"');
    for character in value.chars() {
        match character {
            '"' => quoted.push_str("\\\""),
            '\\' => quoted.push_str("\\\\"),
            '\n' => quoted.push_str("\\n"),
            '\r' => quoted.push_str("\\r"),
            '\t' => quoted.push_str("\\t"),
            control if control.is_control() => {
                quoted.push_str(&format!("\\u{:04X}", control as u32));
            }
            ordinary => quoted.push(ordinary),
        }
    }
    quoted.push('"');
    quoted
}

#[cfg(test)]
mod tests {
    use super::LibraryService;
    use crate::{
        AppResult, Diagnostic, PortablePath,
        domain::{
            CreateSkill, ImportPreview, ImportResolution, ImportResult, ImportScanStatus,
            ImportSourceKind, RepositoryBinding, RepositoryBindingReport, RevisionTrust,
            SaveRevisionRequest, SaveRevisionResult, SkillDraft, TrustState,
        },
        ports::filesystem::{BundleFiles, FileSystemPort},
        ports::{
            ImportScanPort, LibraryHistoryPage, LibraryHistoryQuery, LibraryPage, LibraryQuery,
            LibrarySkillDetail, SaveDraftRequest, StoragePort,
        },
    };
    use std::{
        future::Future,
        path::Path,
        sync::Arc,
        task::{Context, Poll, Wake, Waker},
    };

    struct NoopWake;

    impl Wake for NoopWake {
        fn wake(self: Arc<Self>) {}
    }

    fn block_on<F: Future>(future: F) -> F::Output {
        let waker = Waker::from(Arc::new(NoopWake));
        let mut context = Context::from_waker(&waker);
        let mut future = Box::pin(future);
        loop {
            match future.as_mut().poll(&mut context) {
                Poll::Ready(output) => return output,
                Poll::Pending => std::thread::yield_now(),
            }
        }
    }

    struct MemoryFileSystem {
        result: Result<BundleFiles, Vec<Diagnostic>>,
    }

    impl FileSystemPort for MemoryFileSystem {
        fn read_bundle_directory(&self, _root: &Path) -> Result<BundleFiles, Vec<Diagnostic>> {
            self.result.clone()
        }

        fn read_bundle_source(&self, source: &Path) -> Result<BundleFiles, Vec<Diagnostic>> {
            self.read_bundle_directory(source)
        }
    }

    struct FixedImportScanner(Result<ImportScanStatus, ScanError>);

    enum ScanError {
        Failure,
        Cancelled,
    }

    #[async_trait::async_trait]
    impl ImportScanPort for FixedImportScanner {
        async fn scan(&self, files: &BundleFiles) -> AppResult<ImportScanStatus> {
            assert_eq!(files.len(), 4);
            self.0
                .as_ref()
                .map(|status| *status)
                .map_err(|error| match error {
                    ScanError::Failure => crate::AppError::ExternalTool {
                        tool_id: "gitleaks".to_owned(),
                        exit_code: None,
                    },
                    ScanError::Cancelled => crate::AppError::Cancelled,
                })
        }
    }

    struct MemoryStorage;

    #[async_trait::async_trait]
    impl StoragePort for MemoryStorage {
        fn schema_version(&self) -> AppResult<u32> {
            Ok(crate::ports::CURRENT_SCHEMA_VERSION)
        }

        fn check_integrity(&self) -> AppResult<()> {
            Ok(())
        }

        async fn list_skills(&self, _query: LibraryQuery) -> AppResult<LibraryPage> {
            Ok(LibraryPage::new(Vec::new(), None))
        }

        async fn save_repository_binding(&self, _binding: RepositoryBinding) -> AppResult<()> {
            Err(crate::AppError::Storage {
                code: "test.storage.unexpected".to_owned(),
            })
        }

        async fn load_repository_binding(
            &self,
            _binding_id: &str,
        ) -> AppResult<Option<RepositoryBinding>> {
            Ok(None)
        }

        async fn list_repository_bindings(
            &self,
            _skill_id: crate::domain::SkillId,
        ) -> AppResult<Vec<RepositoryBinding>> {
            Ok(Vec::new())
        }

        async fn save_repository_binding_report(
            &self,
            _binding: &RepositoryBinding,
            _report: RepositoryBindingReport,
        ) -> AppResult<()> {
            Err(crate::AppError::Storage {
                code: "test.storage.unexpected".to_owned(),
            })
        }

        async fn load_repository_binding_report(
            &self,
            _binding_id: &str,
        ) -> AppResult<Option<RepositoryBindingReport>> {
            Ok(None)
        }

        async fn load_skill(
            &self,
            _skill_id: crate::domain::SkillId,
        ) -> AppResult<Option<LibrarySkillDetail>> {
            Ok(None)
        }

        async fn load_revision_files(
            &self,
            _skill_id: crate::domain::SkillId,
            _revision_id: &crate::domain::RevisionId,
        ) -> AppResult<Option<BundleFiles>> {
            Ok(None)
        }

        async fn load_revision_trust(
            &self,
            _skill_id: crate::domain::SkillId,
            _revision_id: &crate::domain::RevisionId,
        ) -> AppResult<RevisionTrust> {
            Ok(RevisionTrust::reviewed())
        }

        async fn load_history(&self, _query: LibraryHistoryQuery) -> AppResult<LibraryHistoryPage> {
            Ok(LibraryHistoryPage::new(Vec::new(), None))
        }

        async fn load_draft(
            &self,
            _skill_id: crate::domain::SkillId,
        ) -> AppResult<Option<SkillDraft>> {
            Ok(None)
        }

        async fn save_draft(&self, _request: SaveDraftRequest) -> AppResult<()> {
            Err(crate::AppError::Storage {
                code: "test.storage.unexpected".to_owned(),
            })
        }

        async fn create_skill(&self, _create: CreateSkill, _draft: SkillDraft) -> AppResult<()> {
            Err(crate::AppError::Storage {
                code: "test.storage.unexpected".to_owned(),
            })
        }

        async fn get_heads(
            &self,
            _skill_id: crate::domain::SkillId,
        ) -> AppResult<Vec<crate::domain::RevisionId>> {
            Ok(Vec::new())
        }

        async fn skill_exists(&self, _skill_id: crate::domain::SkillId) -> AppResult<bool> {
            Ok(false)
        }

        async fn find_revision_by_bundle(
            &self,
            _skill_id: crate::domain::SkillId,
            _content_hash: crate::domain::ContentHash,
        ) -> AppResult<Option<crate::domain::RevisionId>> {
            Ok(None)
        }

        async fn apply_import(
            &self,
            _preview: ImportPreview,
            _resolution: ImportResolution,
        ) -> AppResult<ImportResult> {
            Err(crate::AppError::Storage {
                code: "test.storage.unexpected".to_owned(),
            })
        }

        async fn store_validated_bundle(
            &self,
            _bundle: crate::domain::ValidatedBundle,
            _files: BundleFiles,
        ) -> AppResult<()> {
            Err(crate::AppError::Storage {
                code: "test.storage.unexpected".to_owned(),
            })
        }

        async fn commit_revision(
            &self,
            _request: SaveRevisionRequest,
        ) -> AppResult<SaveRevisionResult> {
            Err(crate::AppError::Storage {
                code: "test.storage.unexpected".to_owned(),
            })
        }
    }

    fn valid_files() -> BundleFiles {
        [
            (
                "SKILL.md",
                include_str!("../../../../docs/examples/repository-foundation/SKILL.md"),
            ),
            (
                "jameskills.toml",
                include_str!("../../../../docs/examples/repository-foundation/jameskills.toml"),
            ),
            (
                "policies/repository.toml",
                include_str!(
                    "../../../../docs/examples/repository-foundation/policies/repository.toml"
                ),
            ),
            (
                "guidance/repository.toml",
                include_str!(
                    "../../../../docs/examples/repository-foundation/guidance/repository.toml"
                ),
            ),
        ]
        .into_iter()
        .map(|(path, source)| {
            (
                PortablePath::new(path.to_owned()).unwrap(),
                source.as_bytes().to_vec(),
            )
        })
        .collect()
    }

    #[test]
    fn validation_uses_the_injected_filesystem_and_returns_domain_summary() {
        let service = LibraryService::new(Arc::new(MemoryFileSystem {
            result: Ok(valid_files()),
        }));

        let result = service.validate_import(Path::new("selected-bundle"));
        assert_eq!(result.unwrap().manifest().slug(), "repository-foundation");
    }

    #[test]
    fn validation_preserves_filesystem_diagnostics() {
        let service = LibraryService::new(Arc::new(MemoryFileSystem {
            result: Err(vec![Diagnostic::error(
                "bundle.file.unreadable",
                "Bundle file is not readable.",
            )]),
        }));

        let errors = match service.validate_import(Path::new("selected-bundle")) {
            Ok(_) => panic!("filesystem errors must not be reported as success"),
            Err(errors) => errors,
        };
        assert_eq!(errors[0].code(), "bundle.file.unreadable");
    }

    #[test]
    fn import_scan_never_promotes_preview_and_provider_failure_is_unknown() {
        let unconfigured_service = LibraryService::new(Arc::new(MemoryFileSystem {
            result: Ok(valid_files()),
        }))
        .with_storage(Arc::new(MemoryStorage));
        let unconfigured = block_on(
            unconfigured_service
                .preview_import(Path::new("candidate"), ImportSourceKind::Directory),
        )
        .unwrap();
        assert_eq!(unconfigured.scan_status(), ImportScanStatus::Unavailable);
        assert_eq!(unconfigured.trust_state(), TrustState::Quarantined);

        for (scan_result, expected) in [
            (
                Ok(ImportScanStatus::NoFindings),
                ImportScanStatus::NoFindings,
            ),
            (Err(ScanError::Failure), ImportScanStatus::Unknown),
            (Ok(ImportScanStatus::Findings), ImportScanStatus::Findings),
        ] {
            let service = LibraryService::new(Arc::new(MemoryFileSystem {
                result: Ok(valid_files()),
            }))
            .with_storage(Arc::new(MemoryStorage))
            .with_import_scanner(Arc::new(FixedImportScanner(scan_result)));
            let preview = block_on(
                service.preview_import(Path::new("candidate"), ImportSourceKind::Directory),
            )
            .unwrap();
            assert_eq!(preview.scan_status(), expected);
            assert_eq!(preview.trust_state(), TrustState::Quarantined);
        }
    }

    #[test]
    fn cancelled_import_scan_cancels_preview() {
        let service = LibraryService::new(Arc::new(MemoryFileSystem {
            result: Ok(valid_files()),
        }))
        .with_storage(Arc::new(MemoryStorage))
        .with_import_scanner(Arc::new(FixedImportScanner(Err(ScanError::Cancelled))));
        assert!(matches!(
            block_on(service.preview_import(Path::new("candidate"), ImportSourceKind::Directory,)),
            Err(crate::AppError::Cancelled)
        ));
    }

    #[test]
    fn manifest_bundle_cannot_be_mislabeled_as_plain_skill() {
        let service = LibraryService::new(Arc::new(MemoryFileSystem {
            result: Ok(valid_files()),
        }));
        let result =
            block_on(service.preview_import(Path::new("candidate"), ImportSourceKind::PlainSkill));
        assert!(matches!(result, Err(crate::AppError::Validation(_))));
    }

    #[test]
    fn plain_import_preview_reuses_the_confirmed_identity_and_digest() {
        let source_bytes = b"---\nname: plain-instructions\ndescription: Imported instructions.\n---\n# Guidance\n";
        let files = [(
            PortablePath::new("SKILL.md".to_owned()).unwrap(),
            source_bytes.to_vec(),
        )]
        .into_iter()
        .collect();
        let service = LibraryService::new(Arc::new(MemoryFileSystem { result: Ok(files) }))
            .with_storage(Arc::new(MemoryStorage));
        let skill_id = crate::domain::SkillId::new();
        let first = block_on(service.preview_import_with_skill_id(
            Path::new("candidate"),
            ImportSourceKind::Directory,
            Some(skill_id),
        ))
        .unwrap();
        let second = block_on(service.preview_import_with_skill_id(
            Path::new("candidate"),
            ImportSourceKind::Directory,
            Some(skill_id),
        ))
        .unwrap();
        assert_eq!(first.skill_id(), skill_id);
        assert_eq!(second.skill_id(), skill_id);
        assert_eq!(first.files(), second.files());
        assert_eq!(
            first
                .confirmation_digest(ImportResolution::CreateQuarantinedDraft)
                .unwrap(),
            second
                .confirmation_digest(ImportResolution::CreateQuarantinedDraft)
                .unwrap()
        );
        assert_eq!(first.trust_state(), TrustState::Quarantined);
    }
}
