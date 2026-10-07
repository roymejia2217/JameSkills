use crate::{
    AppResult, Diagnostic,
    domain::{ContentHash, RevisionId, SkillId},
    ports::filesystem::BundleFiles,
};
pub const MAX_LIBRARY_PAGE_SIZE: usize = 50;
const MAX_LIBRARY_SEARCH_BYTES: usize = 256;
const MAX_LIBRARY_FILTERS: usize = 64;
const MAX_LIBRARY_FILTER_BYTES: usize = 128;
const MAX_LIBRARY_HISTORY_PAGE_SIZE: usize = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibraryItemState {
    Any,
    Active,
    Deleted,
    Conflicted,
}

/// Bounded catalog query. Ordering is always by normalized display name and
/// then skill UUID; cursor values are typed so callers cannot inject SQL.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryQuery {
    search: Option<String>,
    tags: Vec<String>,
    capabilities: Vec<String>,
    state: LibraryItemState,
    after: Option<LibraryCursor>,
    page_size: usize,
}

impl LibraryQuery {
    pub fn new(
        search: Option<&str>,
        tags: Vec<String>,
        capabilities: Vec<String>,
        state: LibraryItemState,
        after: Option<LibraryCursor>,
        page_size: usize,
    ) -> AppResult<Self> {
        if page_size == 0 || page_size > MAX_LIBRARY_PAGE_SIZE {
            return Err(query_error("library.query.page_size.invalid"));
        }
        if tags.len() > MAX_LIBRARY_FILTERS || capabilities.len() > MAX_LIBRARY_FILTERS {
            return Err(query_error("library.query.filters.limit"));
        }
        let search = search.map(str::trim).filter(|value| !value.is_empty());
        if search.is_some_and(|value| value.len() > MAX_LIBRARY_SEARCH_BYTES) {
            return Err(query_error("library.query.search.limit"));
        }
        let search = search.map(normalize_library_text);
        let tags = normalize_filters(tags)?;
        let capabilities = normalize_filters(capabilities)?;
        Ok(Self {
            search,
            tags,
            capabilities,
            state,
            after,
            page_size,
        })
    }

    pub fn search(&self) -> Option<&str> {
        self.search.as_deref()
    }
    pub fn tags(&self) -> &[String] {
        &self.tags
    }
    pub fn capabilities(&self) -> &[String] {
        &self.capabilities
    }
    pub fn state(&self) -> LibraryItemState {
        self.state
    }
    pub fn after(&self) -> Option<&LibraryCursor> {
        self.after.as_ref()
    }
    pub fn page_size(&self) -> usize {
        self.page_size
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryCursor {
    normalized_display_name: String,
    skill_id: SkillId,
}

impl LibraryCursor {
    pub fn new(display_name: &str, skill_id: SkillId) -> AppResult<Self> {
        let display_name = display_name.trim();
        if display_name.is_empty() || display_name.len() > MAX_LIBRARY_SEARCH_BYTES {
            return Err(query_error("library.query.cursor.invalid"));
        }
        Ok(Self {
            normalized_display_name: normalize_library_text(display_name),
            skill_id,
        })
    }

    pub fn normalized_display_name(&self) -> &str {
        &self.normalized_display_name
    }
    pub fn skill_id(&self) -> SkillId {
        self.skill_id
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryHeadSummary {
    revision_id: RevisionId,
    semantic_version: String,
    deleted: bool,
}

impl LibraryHeadSummary {
    pub fn new(revision_id: RevisionId, semantic_version: String, deleted: bool) -> Self {
        Self {
            revision_id,
            semantic_version,
            deleted,
        }
    }
    pub fn revision_id(&self) -> &RevisionId {
        &self.revision_id
    }
    pub fn semantic_version(&self) -> &str {
        &self.semantic_version
    }
    pub fn deleted(&self) -> bool {
        self.deleted
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibrarySkillSummary {
    skill_id: SkillId,
    slug: String,
    display_name: String,
    tags: Vec<String>,
    capabilities: Vec<String>,
    heads: Vec<LibraryHeadSummary>,
}

impl LibrarySkillSummary {
    pub fn new(
        skill_id: SkillId,
        slug: String,
        display_name: String,
        tags: Vec<String>,
        capabilities: Vec<String>,
        heads: Vec<LibraryHeadSummary>,
    ) -> Self {
        Self {
            skill_id,
            slug,
            display_name,
            tags,
            capabilities,
            heads,
        }
    }
    pub fn skill_id(&self) -> SkillId {
        self.skill_id
    }
    pub fn slug(&self) -> &str {
        &self.slug
    }
    pub fn display_name(&self) -> &str {
        &self.display_name
    }
    pub fn tags(&self) -> &[String] {
        &self.tags
    }
    pub fn capabilities(&self) -> &[String] {
        &self.capabilities
    }
    pub fn heads(&self) -> &[LibraryHeadSummary] {
        &self.heads
    }
    pub fn conflicted(&self) -> bool {
        self.heads.len() > 1
    }
    pub fn deleted(&self) -> bool {
        !self.heads.is_empty() && self.heads.iter().all(LibraryHeadSummary::deleted)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryPage {
    items: Vec<LibrarySkillSummary>,
    next: Option<LibraryCursor>,
}

impl LibraryPage {
    pub fn new(items: Vec<LibrarySkillSummary>, next: Option<LibraryCursor>) -> Self {
        Self { items, next }
    }
    pub fn items(&self) -> &[LibrarySkillSummary] {
        &self.items
    }
    pub fn next(&self) -> Option<&LibraryCursor> {
        self.next.as_ref()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryLoadedHead {
    summary: LibraryHeadSummary,
    files: Option<BundleFiles>,
}

impl LibraryLoadedHead {
    pub fn new(summary: LibraryHeadSummary, files: Option<BundleFiles>) -> Self {
        Self { summary, files }
    }
    pub fn summary(&self) -> &LibraryHeadSummary {
        &self.summary
    }
    pub fn files(&self) -> Option<&BundleFiles> {
        self.files.as_ref()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibrarySkillDetail {
    summary: LibrarySkillSummary,
    heads: Vec<LibraryLoadedHead>,
}

impl LibrarySkillDetail {
    pub fn new(summary: LibrarySkillSummary, heads: Vec<LibraryLoadedHead>) -> Self {
        Self { summary, heads }
    }
    pub fn summary(&self) -> &LibrarySkillSummary {
        &self.summary
    }
    pub fn heads(&self) -> &[LibraryLoadedHead] {
        &self.heads
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryHistoryQuery {
    skill_id: SkillId,
    after: Option<RevisionId>,
    page_size: usize,
}

impl LibraryHistoryQuery {
    pub fn new(skill_id: SkillId, after: Option<RevisionId>, page_size: usize) -> AppResult<Self> {
        if page_size == 0 || page_size > MAX_LIBRARY_HISTORY_PAGE_SIZE {
            return Err(query_error("library.history.page_size.invalid"));
        }
        Ok(Self {
            skill_id,
            after,
            page_size,
        })
    }
    pub fn skill_id(&self) -> SkillId {
        self.skill_id
    }
    pub fn after(&self) -> Option<&RevisionId> {
        self.after.as_ref()
    }
    pub fn page_size(&self) -> usize {
        self.page_size
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryHistoryEntry {
    revision_id: RevisionId,
    parents: Vec<RevisionId>,
    bundle_hash: Option<ContentHash>,
    semantic_version: String,
    deleted: bool,
    observed_heads: Vec<RevisionId>,
}

impl LibraryHistoryEntry {
    pub fn new(
        revision_id: RevisionId,
        parents: Vec<RevisionId>,
        bundle_hash: Option<ContentHash>,
        semantic_version: String,
        deleted: bool,
        observed_heads: Vec<RevisionId>,
    ) -> Self {
        Self {
            revision_id,
            parents,
            bundle_hash,
            semantic_version,
            deleted,
            observed_heads,
        }
    }
    pub fn revision_id(&self) -> &RevisionId {
        &self.revision_id
    }
    pub fn parents(&self) -> &[RevisionId] {
        &self.parents
    }
    pub fn bundle_hash(&self) -> Option<&ContentHash> {
        self.bundle_hash.as_ref()
    }
    pub fn semantic_version(&self) -> &str {
        &self.semantic_version
    }
    pub fn deleted(&self) -> bool {
        self.deleted
    }
    pub fn observed_heads(&self) -> &[RevisionId] {
        &self.observed_heads
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryHistoryPage {
    entries: Vec<LibraryHistoryEntry>,
    next: Option<RevisionId>,
}

impl LibraryHistoryPage {
    pub fn new(entries: Vec<LibraryHistoryEntry>, next: Option<RevisionId>) -> Self {
        Self { entries, next }
    }
    pub fn entries(&self) -> &[LibraryHistoryEntry] {
        &self.entries
    }
    pub fn next(&self) -> Option<&RevisionId> {
        self.next.as_ref()
    }
}

fn normalize_filters(filters: Vec<String>) -> AppResult<Vec<String>> {
    let mut normalized = Vec::with_capacity(filters.len());
    for filter in filters {
        let filter = filter.trim();
        if filter.is_empty() || filter.len() > MAX_LIBRARY_FILTER_BYTES {
            return Err(query_error("library.query.filter.invalid"));
        }
        normalized.push(normalize_library_text(filter));
    }
    normalized.sort();
    normalized.dedup();
    Ok(normalized)
}

fn normalize_library_text(value: &str) -> String {
    value.chars().flat_map(char::to_lowercase).collect()
}

fn query_error(code: &'static str) -> crate::AppError {
    crate::AppError::Validation(vec![Diagnostic::error(
        code,
        "Library query is invalid or exceeds its limits.",
    )])
}


/// Schema version applied by the storage actor. Migration files map one to
/// one onto versions: 001_library.sql is version 1, and so on.
pub const CURRENT_SCHEMA_VERSION: u32 = 4;

/// Persistent library storage seam. Only the operations the storage actor
/// implements today are exposed; snapshot merge and the remaining DTOs
/// arrive with their providers instead of as placeholders.
pub trait StoragePort: Send + Sync {
    /// Schema version recorded in the backing store.
    fn schema_version(&self) -> AppResult<u32>;
    /// Verifies the backing store is readable and internally consistent.
    fn check_integrity(&self) -> AppResult<()>;
}
