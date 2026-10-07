use crate::{
    Diagnostic,
    domain::{
        AssetFiles, AssetPreview, ContentHash, CreateSkill, ImportPreview, ImportResolution,
        ImportResult, ImportScanStatus, ImportSourceKind, PortablePath, RevisionKind,
        SaveRevisionRequest, SaveRevisionResult, SkillDraft, SkillId, TrustState, ValidatedBundle,
        assets::{
            add_asset as add_asset_files, preview_asset as build_asset_preview,
            remove_asset as remove_asset_file, rename_bundle_path,
            replace_asset as replace_asset_file,
        },
        validate_bundle,
    },
    ports::{
        ImportScanPort, LibraryHistoryPage, LibraryHistoryQuery, LibraryPage, LibraryQuery,
        LibrarySkillDetail, SaveDraftRequest, StoragePort, filesystem::FileSystemPort,
    },
};
use std::path::Path;
use std::sync::Arc;

const MAX_PUBLISH_EXPECTED_HEADS: usize = 128;

/// Explicit publication intent tied to the saved draft generation and every
/// revision head the editor observed when it began publishing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublishDraft {
    skill_id: SkillId,
    draft_generation: u64,
    expected_heads: Vec<crate::domain::RevisionId>,
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

    pub async fn list_skills(&self, query: LibraryQuery) -> crate::AppResult<LibraryPage> {
        self.storage()?.list_skills(query).await
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

#[cfg(test)]
mod tests {
    use super::LibraryService;
    use crate::{
        AppResult, Diagnostic, PortablePath,
        domain::{
            CreateSkill, ImportPreview, ImportResolution, ImportResult, ImportScanStatus,
            ImportSourceKind, SaveRevisionRequest, SaveRevisionResult, SkillDraft, TrustState,
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

        async fn load_skill(
            &self,
            _skill_id: crate::domain::SkillId,
        ) -> AppResult<Option<LibrarySkillDetail>> {
            Ok(None)
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
