use jameskills_core::{
    AppError, AppResult,
    application::library::ExportPreview,
    domain::{
        ContentHash, DeleteRequest, ImportPreview, ImportResolution, RestoreRevisionRequest,
        SkillId,
    },
    ports::{
        LibraryCursor, LibraryHistoryPage, LibraryHistoryQuery, LibraryItemState, LibraryPage,
        LibraryQuery, LibrarySkillSummary,
    },
};

pub const LIBRARY_PAGE_SIZE: usize = 50;
const MAX_PAGE_HISTORY: usize = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibraryLoadState {
    Idle,
    Loading,
    Empty,
    Ready,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibraryPageDisposition {
    Applied,
    IgnoredStale,
}

pub struct LibraryQueryRequest {
    generation: u64,
    query: LibraryQuery,
}

impl LibraryQueryRequest {
    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn query(&self) -> &LibraryQuery {
        &self.query
    }
}

/// UI-owned bounded catalog page. Its reducer ignores responses from older
/// query generations and keeps UUID selection independent of row order.
pub struct LibraryCatalogState {
    generation: u64,
    search_query: String,
    items: Vec<LibrarySkillSummary>,
    next: Option<LibraryCursor>,
    page_starts: Vec<Option<LibraryCursor>>,
    page_index: usize,
    page_offset: usize,
    selected_skill: Option<SkillId>,
    clear_selection_on_page: bool,
    load_state: LibraryLoadState,
}

impl LibraryCatalogState {
    pub fn new() -> Self {
        Self {
            generation: 0,
            search_query: String::new(),
            items: Vec::new(),
            next: None,
            page_starts: vec![None],
            page_index: 0,
            page_offset: 0,
            selected_skill: None,
            clear_selection_on_page: false,
            load_state: LibraryLoadState::Idle,
        }
    }

    pub fn begin_search(&mut self, search: String) -> AppResult<LibraryQueryRequest> {
        self.begin_query(search, None, 0, true)
    }

    pub fn begin_next_page(&mut self) -> AppResult<Option<LibraryQueryRequest>> {
        if self.load_state != LibraryLoadState::Ready {
            return Ok(None);
        }
        let Some(cursor) = self.next.clone() else {
            return Ok(None);
        };
        let mut page_starts = self.page_starts.clone();
        page_starts.truncate(self.page_index + 1);
        let dropped_oldest = page_starts.len() == MAX_PAGE_HISTORY;
        if dropped_oldest {
            page_starts.remove(0);
        }
        page_starts.push(Some(cursor.clone()));
        let page_index = page_starts.len() - 1;
        let request =
            self.begin_query(self.search_query.clone(), Some(cursor), page_index, false)?;
        self.page_starts = page_starts;
        if dropped_oldest {
            self.page_offset = self.page_offset.saturating_add(1);
        }
        Ok(Some(request))
    }

    pub fn begin_previous_page(&mut self) -> AppResult<Option<LibraryQueryRequest>> {
        if self.load_state != LibraryLoadState::Ready || self.page_index == 0 {
            return Ok(None);
        }
        let page_index = self.page_index - 1;
        let cursor = self.page_starts[page_index].clone();
        self.begin_query(self.search_query.clone(), cursor, page_index, false)
            .map(Some)
    }

    fn begin_query(
        &mut self,
        search: String,
        after: Option<LibraryCursor>,
        page_index: usize,
        reset_pages: bool,
    ) -> AppResult<LibraryQueryRequest> {
        let query = LibraryQuery::new(
            Some(&search),
            Vec::new(),
            Vec::new(),
            LibraryItemState::Any,
            after,
            LIBRARY_PAGE_SIZE,
        )?;
        let generation = self.generation.checked_add(1).ok_or_else(|| {
            AppError::Validation(vec![jameskills_core::Diagnostic::error(
                "desktop.library.query_generation.exhausted",
                "Library query generation is exhausted.",
            )])
        })?;
        self.generation = generation;
        self.search_query = search.trim().to_owned();
        if reset_pages {
            self.page_starts.clear();
            self.page_starts.push(None);
            self.page_offset = 0;
        }
        self.page_index = page_index;
        self.clear_selection_on_page = reset_pages;
        self.load_state = LibraryLoadState::Loading;
        Ok(LibraryQueryRequest { generation, query })
    }

    pub fn apply_page(&mut self, generation: u64, page: LibraryPage) -> LibraryPageDisposition {
        if generation != self.generation {
            return LibraryPageDisposition::IgnoredStale;
        }
        self.items = page.items().to_vec();
        if self.clear_selection_on_page
            && self
                .selected_skill
                .is_some_and(|selected| !self.items.iter().any(|item| item.skill_id() == selected))
        {
            self.selected_skill = None;
        }
        self.clear_selection_on_page = false;
        self.next = page.next().cloned();
        self.load_state = if self.items.is_empty() {
            LibraryLoadState::Empty
        } else {
            LibraryLoadState::Ready
        };
        LibraryPageDisposition::Applied
    }

    pub fn apply_failure(&mut self, generation: u64) -> LibraryPageDisposition {
        if generation != self.generation {
            return LibraryPageDisposition::IgnoredStale;
        }
        self.load_state = LibraryLoadState::Error;
        LibraryPageDisposition::Applied
    }

    pub fn select_skill(&mut self, skill_id: SkillId) -> bool {
        if !self.items.iter().any(|item| item.skill_id() == skill_id) {
            return false;
        }
        self.selected_skill = Some(skill_id);
        true
    }

    pub fn clear_selection(&mut self) {
        self.selected_skill = None;
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn search_query(&self) -> &str {
        &self.search_query
    }

    pub fn items(&self) -> &[LibrarySkillSummary] {
        &self.items
    }

    pub fn next(&self) -> Option<&LibraryCursor> {
        self.next.as_ref()
    }

    pub fn can_previous(&self) -> bool {
        self.page_index > 0 && self.load_state == LibraryLoadState::Ready
    }

    pub fn can_next(&self) -> bool {
        self.next.is_some() && self.load_state == LibraryLoadState::Ready
    }

    pub fn page_number(&self) -> usize {
        self.page_offset
            .saturating_add(self.page_index)
            .saturating_add(1)
    }

    pub fn selected_skill(&self) -> Option<SkillId> {
        self.selected_skill
    }

    pub fn load_state(&self) -> LibraryLoadState {
        self.load_state
    }
}

impl Default for LibraryCatalogState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibraryOperationPhase {
    Idle,
    EditingCreate,
    ChoosingImport,
    PreviewingImport,
    ImportPreview,
    ApplyingCreate,
    ApplyingImport,
    ChoosingExport,
    PreviewingExport,
    ExportPreview,
    ApplyingExport,
    LoadingHistory,
    HistoryReady,
    ConfirmDelete,
    ApplyingDelete,
    ConfirmRestore,
    ApplyingRestore,
    Complete,
    Error,
}

/// In-memory workflow for user-confirmed library mutations. Preview DTOs are
/// deliberately not Debug/Serialize: they contain private imported bytes or
/// archive data and must remain in process memory only.
pub struct LibraryOperationState {
    generation: u64,
    phase: LibraryOperationPhase,
    import_preview: Option<ImportPreview>,
    import_resolution: Option<ImportResolution>,
    import_confirmation: Option<ContentHash>,
    export_preview: Option<ExportPreview>,
    export_overwrite: bool,
    export_confirmation: Option<ContentHash>,
    history_skill: Option<SkillId>,
    history_page: Option<LibraryHistoryPage>,
    history_next: Option<jameskills_core::domain::RevisionId>,
    history_selected_revision: Option<jameskills_core::domain::RevisionId>,
    delete_request: Option<DeleteRequest>,
    restore_request: Option<RestoreRevisionRequest>,
    restore_deleted_only: bool,
}

impl LibraryOperationState {
    pub fn new() -> Self {
        Self {
            generation: 0,
            phase: LibraryOperationPhase::Idle,
            import_preview: None,
            import_resolution: None,
            import_confirmation: None,
            export_preview: None,
            export_overwrite: false,
            export_confirmation: None,
            history_skill: None,
            history_page: None,
            history_next: None,
            history_selected_revision: None,
            delete_request: None,
            restore_request: None,
            restore_deleted_only: false,
        }
    }

    fn begin(&mut self, phase: LibraryOperationPhase) -> AppResult<u64> {
        if self.is_applying() {
            return Err(operation_transition_error());
        }
        let generation = self.generation.checked_add(1).ok_or_else(|| {
            AppError::Validation(vec![jameskills_core::Diagnostic::error(
                "desktop.library.operation_generation.exhausted",
                "Library operation generation is exhausted.",
            )])
        })?;
        self.generation = generation;
        self.phase = phase;
        self.import_preview = None;
        self.import_resolution = None;
        self.import_confirmation = None;
        self.export_preview = None;
        self.export_overwrite = false;
        self.export_confirmation = None;
        self.history_skill = None;
        self.history_page = None;
        self.history_next = None;
        self.history_selected_revision = None;
        self.delete_request = None;
        self.restore_request = None;
        self.restore_deleted_only = false;
        Ok(generation)
    }

    pub fn begin_history(&mut self, skill_id: SkillId) -> AppResult<(u64, LibraryHistoryQuery)> {
        let generation = self.begin(LibraryOperationPhase::LoadingHistory)?;
        self.history_skill = Some(skill_id);
        Ok((
            generation,
            LibraryHistoryQuery::new(skill_id, None, LIBRARY_PAGE_SIZE)?,
        ))
    }

    pub fn begin_next_history_page(&mut self) -> AppResult<Option<(u64, LibraryHistoryQuery)>> {
        if self.phase != LibraryOperationPhase::HistoryReady {
            return Ok(None);
        }
        let (Some(skill_id), Some(after)) = (self.history_skill, self.history_next.clone()) else {
            return Ok(None);
        };
        let query = LibraryHistoryQuery::new(skill_id, Some(after), LIBRARY_PAGE_SIZE)?;
        let generation = self.next_generation()?;
        self.phase = LibraryOperationPhase::LoadingHistory;
        Ok(Some((generation, query)))
    }

    pub fn set_history_page(&mut self, generation: u64, page: LibraryHistoryPage) -> bool {
        if self.generation != generation || self.phase != LibraryOperationPhase::LoadingHistory {
            return false;
        }
        self.history_next = page.next().cloned();
        self.history_page = Some(page);
        self.phase = LibraryOperationPhase::HistoryReady;
        true
    }

    pub fn history_page(&self) -> Option<&LibraryHistoryPage> {
        self.history_page.as_ref()
    }

    pub fn history_skill(&self) -> Option<SkillId> {
        self.history_skill
    }

    pub fn select_history_revision(
        &mut self,
        generation: u64,
        revision_id: &jameskills_core::domain::RevisionId,
    ) -> bool {
        if self.generation != generation || self.phase != LibraryOperationPhase::HistoryReady {
            return false;
        }
        let found = self.history_page.as_ref().is_some_and(|page| {
            page.entries().iter().any(|entry| {
                entry.revision_id() == revision_id
                    && !entry.deleted()
                    && entry.bundle_hash().is_some()
            })
        });
        if found {
            self.history_selected_revision = Some(revision_id.clone());
        }
        found
    }

    pub fn history_selected_revision(&self) -> Option<&jameskills_core::domain::RevisionId> {
        self.history_selected_revision.as_ref()
    }

    pub fn history_next(&self) -> Option<&jameskills_core::domain::RevisionId> {
        self.history_next.as_ref()
    }

    pub fn begin_delete_confirmation(
        &mut self,
        skill_id: SkillId,
        expected_heads: Vec<jameskills_core::domain::RevisionId>,
    ) -> AppResult<u64> {
        let request = DeleteRequest::new(skill_id, expected_heads)?;
        let generation = self.begin(LibraryOperationPhase::ConfirmDelete)?;
        self.delete_request = Some(request);
        Ok(generation)
    }

    pub fn begin_delete_apply(&mut self, generation: u64) -> Option<DeleteRequest> {
        if self.generation != generation || self.phase != LibraryOperationPhase::ConfirmDelete {
            return None;
        }
        let request = self.delete_request.take()?;
        self.phase = LibraryOperationPhase::ApplyingDelete;
        Some(request)
    }

    pub fn delete_request(&self) -> Option<&DeleteRequest> {
        self.delete_request.as_ref()
    }

    pub fn begin_restore_confirmation(
        &mut self,
        request: RestoreRevisionRequest,
        deleted_only: bool,
    ) -> AppResult<u64> {
        if self.phase != LibraryOperationPhase::HistoryReady
            || self.history_skill != Some(request.skill_id())
            || self.history_selected_revision.as_ref() != Some(request.source_revision_id())
        {
            return Err(operation_transition_error());
        }
        let generation = self.begin(LibraryOperationPhase::ConfirmRestore)?;
        self.restore_request = Some(request);
        self.restore_deleted_only = deleted_only;
        Ok(generation)
    }

    pub fn begin_restore_apply(
        &mut self,
        generation: u64,
    ) -> Option<(RestoreRevisionRequest, bool)> {
        if self.generation != generation || self.phase != LibraryOperationPhase::ConfirmRestore {
            return None;
        }
        let request = self.restore_request.take()?;
        self.phase = LibraryOperationPhase::ApplyingRestore;
        Some((request, self.restore_deleted_only))
    }

    pub fn restore_request(&self) -> Option<&RestoreRevisionRequest> {
        self.restore_request.as_ref()
    }

    pub fn restore_deleted_only(&self) -> bool {
        self.restore_deleted_only
    }

    pub fn begin_create(&mut self) -> AppResult<u64> {
        self.begin(LibraryOperationPhase::EditingCreate)
    }

    pub fn begin_create_apply(&mut self, generation: u64) -> bool {
        self.advance(
            generation,
            LibraryOperationPhase::EditingCreate,
            LibraryOperationPhase::ApplyingCreate,
        )
    }

    pub fn begin_import_selection(&mut self) -> AppResult<u64> {
        self.begin(LibraryOperationPhase::ChoosingImport)
    }

    pub fn begin_import_preview(&mut self, generation: u64) -> bool {
        self.advance(
            generation,
            LibraryOperationPhase::ChoosingImport,
            LibraryOperationPhase::PreviewingImport,
        )
    }

    pub fn set_import_preview(&mut self, generation: u64, preview: ImportPreview) -> bool {
        if self.generation != generation || self.phase != LibraryOperationPhase::PreviewingImport {
            return false;
        }
        self.import_preview = Some(preview);
        self.phase = LibraryOperationPhase::ImportPreview;
        true
    }

    pub fn import_preview(&self) -> Option<&ImportPreview> {
        self.import_preview.as_ref()
    }

    pub fn select_import_resolution(
        &mut self,
        generation: u64,
        resolution: ImportResolution,
    ) -> AppResult<ContentHash> {
        let Some(preview) = self.import_preview.as_ref().filter(|_| {
            self.generation == generation && self.phase == LibraryOperationPhase::ImportPreview
        }) else {
            return Err(operation_transition_error());
        };
        let confirmation = preview.confirmation_digest(resolution)?;
        self.import_resolution = Some(resolution);
        self.import_confirmation = Some(confirmation.clone());
        Ok(confirmation)
    }

    pub fn begin_import_apply(
        &mut self,
        generation: u64,
    ) -> Option<(ImportPreview, ImportResolution, ContentHash)> {
        if self.generation != generation || self.phase != LibraryOperationPhase::ImportPreview {
            return None;
        }
        let resolution = self.import_resolution?;
        let confirmation = self.import_confirmation.clone()?;
        let preview = self.import_preview.take()?;
        self.import_resolution = None;
        self.import_confirmation = None;
        self.phase = LibraryOperationPhase::ApplyingImport;
        Some((preview, resolution, confirmation))
    }

    pub fn begin_export_selection(&mut self) -> AppResult<u64> {
        self.begin(LibraryOperationPhase::ChoosingExport)
    }

    pub fn begin_export_preview(&mut self, generation: u64) -> bool {
        self.advance(
            generation,
            LibraryOperationPhase::ChoosingExport,
            LibraryOperationPhase::PreviewingExport,
        )
    }

    pub fn set_export_preview(&mut self, generation: u64, preview: ExportPreview) -> bool {
        if self.generation != generation || self.phase != LibraryOperationPhase::PreviewingExport {
            return false;
        }
        self.export_preview = Some(preview);
        self.phase = LibraryOperationPhase::ExportPreview;
        true
    }

    pub fn export_preview(&self) -> Option<&ExportPreview> {
        self.export_preview.as_ref()
    }

    pub fn select_export_overwrite(
        &mut self,
        generation: u64,
        overwrite: bool,
    ) -> AppResult<ContentHash> {
        let Some(preview) = self.export_preview.as_ref().filter(|_| {
            self.generation == generation && self.phase == LibraryOperationPhase::ExportPreview
        }) else {
            return Err(operation_transition_error());
        };
        let confirmation = preview.confirmation_digest(overwrite)?.clone();
        self.export_overwrite = overwrite;
        self.export_confirmation = Some(confirmation.clone());
        Ok(confirmation)
    }

    pub fn begin_export_apply(
        &mut self,
        generation: u64,
    ) -> Option<(ExportPreview, bool, ContentHash)> {
        if self.generation != generation || self.phase != LibraryOperationPhase::ExportPreview {
            return None;
        }
        let confirmation = self.export_confirmation.clone()?;
        let preview = self.export_preview.take()?;
        self.export_confirmation = None;
        let overwrite = self.export_overwrite;
        self.phase = LibraryOperationPhase::ApplyingExport;
        Some((preview, overwrite, confirmation))
    }

    pub fn complete(&mut self, generation: u64) -> bool {
        if self.generation != generation
            || !matches!(
                self.phase,
                LibraryOperationPhase::ApplyingCreate
                    | LibraryOperationPhase::ApplyingImport
                    | LibraryOperationPhase::ApplyingExport
                    | LibraryOperationPhase::ApplyingDelete
                    | LibraryOperationPhase::ApplyingRestore
            )
        {
            return false;
        }
        self.phase = LibraryOperationPhase::Complete;
        true
    }

    pub fn fail(&mut self, generation: u64) -> bool {
        if self.generation != generation
            || matches!(
                self.phase,
                LibraryOperationPhase::Idle
                    | LibraryOperationPhase::Complete
                    | LibraryOperationPhase::Error
            )
        {
            return false;
        }
        self.phase = LibraryOperationPhase::Error;
        self.import_preview = None;
        self.export_preview = None;
        self.import_resolution = None;
        self.import_confirmation = None;
        self.export_confirmation = None;
        self.history_page = None;
        self.history_next = None;
        self.history_selected_revision = None;
        self.delete_request = None;
        self.restore_request = None;
        true
    }

    pub fn cancel(&mut self) -> AppResult<()> {
        if self.is_applying() {
            return Err(operation_transition_error());
        }
        self.begin(LibraryOperationPhase::Idle).map(|_| ())
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn phase(&self) -> LibraryOperationPhase {
        self.phase
    }

    fn advance(
        &mut self,
        generation: u64,
        expected: LibraryOperationPhase,
        next: LibraryOperationPhase,
    ) -> bool {
        if self.generation != generation || self.phase != expected {
            return false;
        }
        self.phase = next;
        true
    }

    fn is_applying(&self) -> bool {
        matches!(
            self.phase,
            LibraryOperationPhase::ApplyingCreate
                | LibraryOperationPhase::ApplyingImport
                | LibraryOperationPhase::ApplyingExport
                | LibraryOperationPhase::ApplyingDelete
                | LibraryOperationPhase::ApplyingRestore
        )
    }

    fn next_generation(&mut self) -> AppResult<u64> {
        let generation = self.generation.checked_add(1).ok_or_else(|| {
            AppError::Validation(vec![jameskills_core::Diagnostic::error(
                "desktop.library.operation_generation.exhausted",
                "Library operation generation is exhausted.",
            )])
        })?;
        self.generation = generation;
        Ok(generation)
    }
}

impl Default for LibraryOperationState {
    fn default() -> Self {
        Self::new()
    }
}

fn operation_transition_error() -> AppError {
    AppError::Validation(vec![jameskills_core::Diagnostic::error(
        "desktop.library.operation.transition.invalid",
        "Library operation is stale or in an invalid state.",
    )])
}
