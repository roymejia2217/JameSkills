use crate::fs::{list_blob_hashes, verify_blob_bytes};
use chrono::{SecondsFormat, Utc};
use jameskills_core::{
    AppError, AppResult, Diagnostic, OperationId, PortablePath,
    domain::{
        ContentHash, CreateSkill, ImportClassification, ImportPreview, ImportResolution,
        ImportResult, ImportSourceKind, RevisionId, RevisionKind, RevisionRecord,
        SaveRevisionRequest, SaveRevisionResult, SkillDraft, SkillId, TrustState, ValidatedBundle,
        policy::RepositoryHead, validate_bundle,
    },
    ports::{
        BundleFiles, CURRENT_SCHEMA_VERSION, LibraryCursor, LibraryHeadSummary,
        LibraryHistoryEntry, LibraryHistoryPage, LibraryHistoryQuery, LibraryItemState,
        LibraryLoadedHead, LibraryPage, LibraryQuery, LibrarySkillDetail, LibrarySkillSummary,
        OperationJournalPort, RepoChangeJournal, RepoChangeJournalState, SaveDraftRequest,
        StoragePort, process::ApprovedRoot,
    },
};
use rusqlite::{Connection, ErrorCode, OpenFlags, OptionalExtension, Transaction};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

const MIGRATIONS: [(u32, &str); 6] = [
    (1, include_str!("../migrations/001_library.sql")),
    (2, include_str!("../migrations/002_operations.sql")),
    (3, include_str!("../migrations/003_sync.sql")),
    (4, include_str!("../migrations/004_library_catalog.sql")),
    (5, include_str!("../migrations/005_import_lookup.sql")),
    (6, include_str!("../migrations/006_revision_trust.sql")),
];
const MAX_DRAFT_PAYLOAD_BYTES: usize = 32 * 1024 * 1024;

fn storage_error(code: &'static str) -> AppError {
    AppError::Storage {
        code: code.to_owned(),
    }
}

/// Single-writer SQLite actor: one mutex guards the only connection, so all
/// writes serialize without an external lock order and never block UI
/// rendering on a mutex. Migrations apply from embedded SQL with a file
/// backup before any destructive upgrade; a future schema opens read-only
/// and locked instead of downgrading.
pub struct SqliteStore {
    connection: Arc<Mutex<Connection>>,
    path: PathBuf,
    blob_root: PathBuf,
    read_only: bool,
}

impl SqliteStore {
    /// Opens or creates the library database, applies pending migrations and
    /// enforces WAL, foreign keys and a 5s busy timeout. Corrupt files fail
    /// here, before any caller observes a store.
    pub fn open(path: &Path) -> AppResult<Self> {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).map_err(|_| storage_error("storage.open.failed"))?;
        }
        let connection =
            Connection::open(path).map_err(|_| storage_error("storage.open.failed"))?;
        connection
            .execute_batch("PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 5000;")
            .map_err(|_| storage_error("storage.open.failed"))?;
        let version = read_user_version(&connection)?;
        if version > CURRENT_SCHEMA_VERSION {
            return Self::open_locked(path);
        }
        let mode: String = connection
            .query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))
            .map_err(|_| storage_error("storage.open.failed"))?;
        if !mode.eq_ignore_ascii_case("wal") {
            return Err(storage_error("storage.open.failed"));
        }
        if version > 0 && version < CURRENT_SCHEMA_VERSION {
            backup_before_upgrade(path, version)?;
        }
        for (number, sql) in MIGRATIONS
            .iter()
            .skip_while(|(number, _)| *number <= version)
        {
            connection
                .execute_batch(sql)
                .map_err(|_| storage_error("storage.migrate.failed"))?;
            set_user_version(&connection, *number)?;
        }
        let blob_root = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
            .join("blobs");
        let store = Self {
            connection: Arc::new(Mutex::new(connection)),
            path: path.to_path_buf(),
            blob_root,
            read_only: false,
        };
        if version < 4 {
            store.refresh_normalized_catalog_names()?;
        }
        store.verify_referenced_blobs()?;
        Ok(store)
    }

    /// Opens a future schema without writing: migrations are skipped, every
    /// write fails, reads and integrity checks keep working.
    fn open_locked(path: &Path) -> AppResult<Self> {
        let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|_| storage_error("storage.open.failed"))?;
        connection
            .execute_batch("PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 5000;")
            .map_err(|_| storage_error("storage.open.failed"))?;
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
            path: path.to_path_buf(),
            blob_root: path
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."))
                .join("blobs"),
            read_only: true,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    fn refresh_normalized_catalog_names(&self) -> AppResult<()> {
        let mut guard = self
            .connection
            .lock()
            .map_err(|_| storage_error("storage.lock.poisoned"))?;
        let rows = {
            let mut statement = guard
                .prepare("SELECT id, display_name FROM skills ORDER BY id")
                .map_err(|_| storage_error("storage.data.corrupt"))?;
            let rows = statement
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(|_| storage_error("storage.data.corrupt"))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|_| storage_error("storage.data.corrupt"))?
        };
        let transaction = guard
            .transaction()
            .map_err(|_| storage_error("storage.transaction.failed"))?;
        for (skill_id, display_name) in rows {
            let normalized: String = display_name.chars().flat_map(char::to_lowercase).collect();
            transaction
                .execute(
                    "INSERT INTO library_catalog(skill_id, normalized_display_name) VALUES (?1, ?2) ON CONFLICT(skill_id) DO UPDATE SET normalized_display_name = excluded.normalized_display_name WHERE library_catalog.normalized_display_name <> excluded.normalized_display_name",
                    (skill_id, normalized),
                )
                .map_err(|_| storage_error("storage.data.corrupt"))?;
        }
        transaction
            .commit()
            .map_err(|_| storage_error("storage.transaction.failed"))
    }

    pub fn schema_version(&self) -> AppResult<u32> {
        let guard = self
            .connection
            .lock()
            .map_err(|_| storage_error("storage.lock.poisoned"))?;
        read_user_version(&guard)
    }

    pub fn check_integrity(&self) -> AppResult<()> {
        let guard = self
            .connection
            .lock()
            .map_err(|_| storage_error("storage.lock.poisoned"))?;
        let report: String = guard
            .query_row("PRAGMA quick_check", [], |row| row.get(0))
            .map_err(|_| storage_error("storage.integrity.failed"))?;
        if report.eq_ignore_ascii_case("ok") {
            Ok(())
        } else {
            Err(storage_error("storage.integrity.failed"))
        }
    }

    /// Returns a stable, bounded metadata page. Query values are always SQL
    /// parameters; only fixed predicates and the bounded count of filter
    /// clauses are interpolated into the statement.
    pub fn list_skills(&self, query: &LibraryQuery) -> AppResult<LibraryPage> {
        list_skills_from_connection(&self.connection, query)
    }

    async fn list_skills_async(&self, query: LibraryQuery) -> AppResult<LibraryPage> {
        let connection = self.connection.clone();
        tokio::task::spawn_blocking(move || list_skills_from_connection(&connection, &query))
            .await
            .map_err(|_| storage_error("storage.query.worker.failed"))?
    }

    pub fn get_heads(&self, skill_id: SkillId) -> AppResult<Vec<RevisionId>> {
        let guard = self
            .connection
            .lock()
            .map_err(|_| storage_error("storage.lock.poisoned"))?;
        let mut statement = guard
            .prepare("SELECT revision_id FROM skill_heads WHERE skill_id = ?1 ORDER BY revision_id")
            .map_err(|_| storage_error("storage.query.failed"))?;
        let rows = statement
            .query_map([skill_id.as_uuid().to_string()], |row| {
                row.get::<_, String>(0)
            })
            .map_err(|_| storage_error("storage.query.failed"))?;
        rows.map(|row| {
            RevisionId::parse_hex(&row.map_err(|_| storage_error("storage.data.corrupt"))?)
                .map_err(|_| storage_error("storage.data.corrupt"))
        })
        .collect()
    }

    pub fn skill_exists(&self, skill_id: SkillId) -> AppResult<bool> {
        let guard = self
            .connection
            .lock()
            .map_err(|_| storage_error("storage.lock.poisoned"))?;
        guard
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM skills WHERE id = ?1)",
                [skill_id.as_uuid().to_string()],
                |row| row.get(0),
            )
            .map_err(|_| storage_error("storage.query.failed"))
    }

    async fn skill_exists_async(&self, skill_id: SkillId) -> AppResult<bool> {
        let store = Self {
            connection: self.connection.clone(),
            path: self.path.clone(),
            blob_root: self.blob_root.clone(),
            read_only: self.read_only,
        };
        tokio::task::spawn_blocking(move || store.skill_exists(skill_id))
            .await
            .map_err(|_| storage_error("storage.query.worker.failed"))?
    }

    async fn get_heads_async(&self, skill_id: SkillId) -> AppResult<Vec<RevisionId>> {
        let store = Self {
            connection: self.connection.clone(),
            path: self.path.clone(),
            blob_root: self.blob_root.clone(),
            read_only: self.read_only,
        };
        tokio::task::spawn_blocking(move || store.get_heads(skill_id))
            .await
            .map_err(|_| storage_error("storage.query.worker.failed"))?
    }

    pub fn find_revision_by_bundle(
        &self,
        skill_id: SkillId,
        content_hash: &ContentHash,
    ) -> AppResult<Option<RevisionId>> {
        let guard = self
            .connection
            .lock()
            .map_err(|_| storage_error("storage.lock.poisoned"))?;
        let revision = guard
            .query_row(
                "SELECT id FROM revisions WHERE skill_id = ?1 AND bundle_hash = ?2 AND state = 'content' ORDER BY id LIMIT 1",
                rusqlite::params![skill_id.as_uuid().to_string(), content_hash.as_str()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|_| storage_error("storage.query.failed"))?;
        revision
            .map(|value| {
                RevisionId::parse_hex(&value).map_err(|_| storage_error("storage.data.corrupt"))
            })
            .transpose()
    }

    async fn find_revision_by_bundle_async(
        &self,
        skill_id: SkillId,
        content_hash: ContentHash,
    ) -> AppResult<Option<RevisionId>> {
        let store = Self {
            connection: self.connection.clone(),
            path: self.path.clone(),
            blob_root: self.blob_root.clone(),
            read_only: self.read_only,
        };
        tokio::task::spawn_blocking(move || store.find_revision_by_bundle(skill_id, &content_hash))
            .await
            .map_err(|_| storage_error("storage.query.worker.failed"))?
    }

    pub fn load_skill(&self, skill_id: SkillId) -> AppResult<Option<LibrarySkillDetail>> {
        load_skill_from_connection(&self.connection, &self.blob_root, skill_id)
    }

    async fn load_skill_async(&self, skill_id: SkillId) -> AppResult<Option<LibrarySkillDetail>> {
        let connection = self.connection.clone();
        let blob_root = self.blob_root.clone();
        tokio::task::spawn_blocking(move || {
            load_skill_from_connection(&connection, &blob_root, skill_id)
        })
        .await
        .map_err(|_| storage_error("storage.query.worker.failed"))?
    }

    pub fn load_history(&self, query: &LibraryHistoryQuery) -> AppResult<LibraryHistoryPage> {
        load_history_from_connection(&self.connection, query)
    }

    async fn load_history_async(
        &self,
        query: LibraryHistoryQuery,
    ) -> AppResult<LibraryHistoryPage> {
        let connection = self.connection.clone();
        tokio::task::spawn_blocking(move || load_history_from_connection(&connection, &query))
            .await
            .map_err(|_| storage_error("storage.query.worker.failed"))?
    }

    pub fn load_draft(&self, skill_id: SkillId) -> AppResult<Option<SkillDraft>> {
        let guard = self
            .connection
            .lock()
            .map_err(|_| storage_error("storage.lock.poisoned"))?;
        let stored = guard
            .query_row(
                "SELECT base_head, CASE WHEN length(draft_json) <= ?1 THEN draft_json ELSE NULL END, generation FROM drafts WHERE skill_id = ?2",
                rusqlite::params![MAX_DRAFT_PAYLOAD_BYTES as i64, skill_id.as_uuid().to_string()],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<Vec<u8>>>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(|_| storage_error("storage.query.failed"))?;
        let Some((base_head, bytes, generation)) = stored else {
            return Ok(None);
        };
        let bytes = bytes.ok_or_else(|| storage_error("storage.draft.corrupt"))?;
        if generation <= 0 {
            return Err(storage_error("storage.draft.corrupt"));
        }
        let draft = decode_skill_draft(&bytes)?;
        let stored_base = base_head
            .map(|value| RevisionId::parse_hex(&value))
            .transpose()
            .map_err(|_| storage_error("storage.draft.corrupt"))?;
        if draft.skill_id() != skill_id
            || draft.generation() != generation as u64
            || draft.base_head() != stored_base.as_ref()
        {
            return Err(storage_error("storage.draft.corrupt"));
        }
        Ok(Some(draft))
    }

    pub fn save_draft(&self, request: &SaveDraftRequest) -> AppResult<()> {
        if self.read_only {
            return Err(storage_error("storage.store.read_only"));
        }
        let draft = request.draft();
        let generation = i64::try_from(draft.generation())
            .map_err(|_| storage_error("storage.draft.generation.invalid"))?;
        let payload = encode_skill_draft(draft)?;
        let skill_id = draft.skill_id();
        let skill_id_text = skill_id.as_uuid().to_string();
        let base_head = draft.base_head().map(|head| head.as_str());
        let expected_base = request.expected_base_head().map(|head| head.as_str());
        let mut guard = self
            .connection
            .lock()
            .map_err(|_| storage_error("storage.lock.poisoned"))?;
        let transaction = guard
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| storage_error("storage.transaction.failed"))?;
        let exists: bool = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM skills WHERE id = ?1)",
                [&skill_id_text],
                |row| row.get(0),
            )
            .map_err(|_| storage_error("storage.query.failed"))?;
        if !exists {
            return Err(AppError::NotFound);
        }
        let stored = transaction
            .query_row(
                "SELECT generation, base_head, CASE WHEN length(draft_json) <= ?1 THEN draft_json ELSE NULL END FROM drafts WHERE skill_id = ?2",
                rusqlite::params![MAX_DRAFT_PAYLOAD_BYTES as i64, skill_id_text],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<Vec<u8>>>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(|_| storage_error("storage.query.failed"))?;
        let matches = match (request.expected_generation(), stored.as_ref()) {
            (None, None) => true,
            (Some(expected), Some((stored_generation, current_base, _))) => {
                u64::try_from(*stored_generation).ok() == Some(expected)
                    && current_base.as_deref() == expected_base
            }
            _ => false,
        };
        if !matches {
            return Err(draft_conflict(&transaction, skill_id)?);
        }
        let current_trust = if let Some((_, _, stored_payload)) = stored {
            let stored_payload =
                stored_payload.ok_or_else(|| storage_error("storage.draft.corrupt"))?;
            decode_skill_draft(&stored_payload)?.trust_state()
        } else {
            let unreviewed_heads: i64 = transaction
                .query_row(
                    "SELECT COUNT(*) FROM skill_heads h LEFT JOIN revision_trust t ON t.revision_id = h.revision_id WHERE h.skill_id = ?1 AND COALESCE(t.trust_state, 'reviewed') <> 'reviewed'",
                    [&skill_id_text],
                    |row| row.get(0),
                )
                .map_err(|_| storage_error("storage.query.failed"))?;
            if unreviewed_heads > 0 {
                TrustState::Quarantined
            } else {
                TrustState::Reviewed
            }
        };
        if current_trust == TrustState::Quarantined && draft.trust_state() == TrustState::Reviewed {
            return Err(AppError::UntrustedInput {
                code: "library.draft.review_required".to_owned(),
            });
        }
        let changed = if request.expected_generation().is_none() {
            transaction.execute(
                "INSERT INTO drafts(skill_id, base_head, draft_json, generation) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![skill_id_text, base_head, payload, generation],
            )
        } else {
            transaction.execute(
                "UPDATE drafts SET base_head = ?1, draft_json = ?2, generation = ?3 WHERE skill_id = ?4 AND generation = ?5 AND base_head IS ?6",
                rusqlite::params![
                    base_head,
                    payload,
                    generation,
                    skill_id_text,
                    request.expected_generation().unwrap_or_default() as i64,
                    expected_base,
                ],
            )
        }
        .map_err(|_| storage_error("storage.draft.write.failed"))?;
        if changed != 1 {
            return Err(draft_conflict(&transaction, skill_id)?);
        }
        transaction
            .commit()
            .map_err(|_| storage_error("storage.transaction.failed"))
    }

    pub fn create_skill(&self, create: &CreateSkill, draft: &SkillDraft) -> AppResult<()> {
        if self.read_only {
            return Err(storage_error("storage.store.read_only"));
        }
        let validated = validate_bundle(draft.files()).map_err(AppError::Validation)?;
        let manifest = validated.manifest();
        if draft.skill_id() != create.skill_id()
            || draft.generation() != 1
            || draft.base_head().is_some()
            || manifest.id() != create.skill_id()
            || manifest.slug() != create.slug()
            || manifest.display_name() != create.display_name()
            || manifest.description() != create.description()
        {
            return Err(AppError::Validation(vec![Diagnostic::error(
                "library.create.initial_draft.mismatch",
                "Initial draft does not match its new skill metadata.",
            )]));
        }
        let payload = encode_skill_draft(draft)?;
        let skill_id = create.skill_id().as_uuid().to_string();
        let generation = i64::try_from(draft.generation())
            .map_err(|_| storage_error("storage.draft.generation.invalid"))?;
        let normalized_name: String = create
            .display_name()
            .chars()
            .flat_map(char::to_lowercase)
            .collect();
        let created_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let mut guard = self
            .connection
            .lock()
            .map_err(|_| storage_error("storage.lock.poisoned"))?;
        let transaction = guard
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| storage_error("storage.transaction.failed"))?;
        transaction
            .execute(
                "INSERT INTO skills(id, slug, display_name, created_at) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![skill_id, create.slug(), create.display_name(), created_at],
            )
            .map_err(|_| storage_error("storage.skill.create.failed"))?;
        transaction
            .execute(
                "INSERT INTO library_catalog(skill_id, normalized_display_name) VALUES (?1, ?2)",
                rusqlite::params![skill_id, normalized_name],
            )
            .map_err(|_| storage_error("storage.skill.create.failed"))?;
        transaction
            .execute(
                "INSERT INTO drafts(skill_id, base_head, draft_json, generation) VALUES (?1, NULL, ?2, ?3)",
                rusqlite::params![skill_id, payload, generation],
            )
            .map_err(|_| storage_error("storage.skill.create.failed"))?;
        transaction
            .commit()
            .map_err(|_| storage_error("storage.transaction.failed"))
    }

    async fn load_draft_async(&self, skill_id: SkillId) -> AppResult<Option<SkillDraft>> {
        let store = Self {
            connection: self.connection.clone(),
            path: self.path.clone(),
            blob_root: self.blob_root.clone(),
            read_only: self.read_only,
        };
        tokio::task::spawn_blocking(move || store.load_draft(skill_id))
            .await
            .map_err(|_| storage_error("storage.draft.worker.failed"))?
    }

    async fn save_draft_async(&self, request: SaveDraftRequest) -> AppResult<()> {
        let store = Self {
            connection: self.connection.clone(),
            path: self.path.clone(),
            blob_root: self.blob_root.clone(),
            read_only: self.read_only,
        };
        tokio::task::spawn_blocking(move || store.save_draft(&request))
            .await
            .map_err(|_| storage_error("storage.draft.worker.failed"))?
    }

    async fn create_skill_async(&self, create: CreateSkill, draft: SkillDraft) -> AppResult<()> {
        let store = Self {
            connection: self.connection.clone(),
            path: self.path.clone(),
            blob_root: self.blob_root.clone(),
            read_only: self.read_only,
        };
        tokio::task::spawn_blocking(move || store.create_skill(&create, &draft))
            .await
            .map_err(|_| storage_error("storage.skill.create.worker.failed"))?
    }

    pub fn store_validated_bundle(
        &self,
        expected: &ValidatedBundle,
        files: &BundleFiles,
    ) -> AppResult<()> {
        if self.read_only {
            return Err(storage_error("storage.store.read_only"));
        }
        let validated = validate_bundle(files).map_err(AppError::Validation)?;
        if &validated != expected {
            return Err(AppError::Validation(vec![Diagnostic::error(
                "library.bundle.changed_after_validation",
                "Bundle bytes differ from the validated publication request.",
            )]));
        }
        let archive =
            jameskills_core::ports::write_bundle_archive(files).map_err(AppError::Validation)?;
        crate::fs::store_blob_bytes(&self.blob_root, validated.content_hash(), &archive)
            .map_err(AppError::Validation)?;
        Ok(())
    }

    pub fn apply_import(
        &self,
        preview: &ImportPreview,
        resolution: ImportResolution,
    ) -> AppResult<ImportResult> {
        if self.read_only {
            return Err(storage_error("storage.store.read_only"));
        }
        let validated = validate_bundle(preview.files()).map_err(AppError::Validation)?;
        if &validated != preview.bundle() || validated.manifest().id() != preview.skill_id() {
            return Err(AppError::Validation(vec![Diagnostic::error(
                "library.import.preview.changed",
                "Import bytes do not match the immutable reviewed preview.",
            )]));
        }
        preview.confirmation_digest(resolution)?;
        if resolution == ImportResolution::AddConcurrentRoot
            && preview.source_kind() != ImportSourceKind::PlainSkill
        {
            self.store_validated_bundle(&validated, preview.files())?;
        }

        let mut guard = self
            .connection
            .lock()
            .map_err(|_| storage_error("storage.lock.poisoned"))?;
        let transaction = guard
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| storage_error("storage.transaction.failed"))?;
        let skill_id = preview.skill_id();
        let skill_id_text = skill_id.as_uuid().to_string();
        let matching_revision = transaction
            .query_row(
                "SELECT id FROM revisions WHERE skill_id = ?1 AND bundle_hash = ?2 AND state = 'content' ORDER BY id LIMIT 1",
                rusqlite::params![skill_id_text, preview.content_hash().as_str()],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|_| storage_error("storage.query.failed"))?;
        if let Some(revision) = matching_revision {
            let revision_id = RevisionId::parse_hex(&revision)
                .map_err(|_| storage_error("storage.data.corrupt"))?;
            if let ImportClassification::Identical {
                revision_id: expected,
            } = preview.classification()
                && expected != &revision_id
            {
                return Err(storage_error("storage.import.preview.stale"));
            }
            verify_blob_bytes(&self.blob_root, preview.content_hash())
                .map_err(|_| storage_error("storage.blob.invalid"))?;
            let heads = read_heads(&transaction, skill_id)?
                .into_iter()
                .map(|head| RevisionId::parse_hex(&head))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| storage_error("storage.data.corrupt"))?;
            let trust_state = read_revision_trust(&transaction, &revision_id)?;
            transaction
                .commit()
                .map_err(|_| storage_error("storage.transaction.failed"))?;
            return Ok(ImportResult::Duplicate {
                revision_id,
                heads,
                trust_state,
            });
        }

        let exists: bool = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM skills WHERE id = ?1)",
                [&skill_id_text],
                |row| row.get(0),
            )
            .map_err(|_| storage_error("storage.query.failed"))?;
        let current = read_heads(&transaction, skill_id)?;
        let expected_heads = preview
            .current_heads()
            .iter()
            .map(|head| head.as_str().to_owned())
            .collect::<Vec<_>>();
        if exists != preview.existing_skill() || current != expected_heads {
            return Err(draft_conflict(&transaction, skill_id)?);
        }
        let heads = current
            .iter()
            .map(|head| RevisionId::parse_hex(head))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| storage_error("storage.data.corrupt"))?;
        if resolution == ImportResolution::KeepExisting {
            if !preview.existing_skill() {
                return Err(AppError::Validation(vec![Diagnostic::error(
                    "library.import.resolution.invalid",
                    "Keeping an existing skill is only valid for a duplicate identity.",
                )]));
            }
            transaction
                .commit()
                .map_err(|_| storage_error("storage.transaction.failed"))?;
            return Ok(ImportResult::KeptExisting { heads });
        }

        if preview.source_kind() == ImportSourceKind::PlainSkill {
            if resolution != ImportResolution::CreateQuarantinedDraft
                || exists
                || !current.is_empty()
            {
                return Err(storage_error("storage.import.preview.stale"));
            }
            let manifest = validated.manifest();
            let draft = SkillDraft::new_quarantined(skill_id, None, 1, preview.files().clone())?;
            let payload = encode_skill_draft(&draft)?;
            let generation = i64::try_from(draft.generation())
                .map_err(|_| storage_error("storage.draft.generation.invalid"))?;
            let normalized_name: String = manifest
                .display_name()
                .chars()
                .flat_map(char::to_lowercase)
                .collect();
            let created_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
            transaction
                .execute(
                    "INSERT INTO skills(id, slug, display_name, created_at) VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![skill_id_text, manifest.slug(), manifest.display_name(), created_at],
                )
                .map_err(|_| storage_error("storage.import.write.failed"))?;
            transaction
                .execute(
                    "INSERT INTO library_catalog(skill_id, normalized_display_name) VALUES (?1, ?2)",
                    rusqlite::params![skill_id_text, normalized_name],
                )
                .map_err(|_| storage_error("storage.import.write.failed"))?;
            transaction
                .execute(
                    "INSERT INTO drafts(skill_id, base_head, draft_json, generation) VALUES (?1, NULL, ?2, ?3)",
                    rusqlite::params![skill_id_text, payload, generation],
                )
                .map_err(|_| storage_error("storage.import.write.failed"))?;
            transaction
                .commit()
                .map_err(|_| storage_error("storage.transaction.failed"))?;
            return Ok(ImportResult::DraftCreated {
                skill_id,
                generation: draft.generation(),
                trust_state: draft.trust_state(),
            });
        }

        if !preview.existing_skill() {
            let manifest = validated.manifest();
            let normalized_name: String = manifest
                .display_name()
                .chars()
                .flat_map(char::to_lowercase)
                .collect();
            let created_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
            transaction
                .execute(
                    "INSERT INTO skills(id, slug, display_name, created_at) VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![skill_id_text, manifest.slug(), manifest.display_name(), created_at],
                )
                .map_err(|_| storage_error("storage.import.write.failed"))?;
            transaction
                .execute(
                    "INSERT INTO library_catalog(skill_id, normalized_display_name) VALUES (?1, ?2)",
                    rusqlite::params![skill_id_text, normalized_name],
                )
                .map_err(|_| storage_error("storage.import.write.failed"))?;
        }
        let manifest = validated.manifest();
        let revision = RevisionRecord::new(
            skill_id,
            Some(validated.content_hash().clone()),
            Vec::new(),
            RevisionKind::Content,
            manifest.version().to_string(),
        )?;
        let created_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        transaction
            .execute(
                "INSERT INTO revisions(id, skill_id, bundle_hash, semantic_version, schema_version, state, created_at) VALUES (?1, ?2, ?3, ?4, ?5, 'content', ?6)",
                rusqlite::params![
                    revision.id().as_str(),
                    skill_id_text,
                    revision.bundle_hash().as_str(),
                    revision.semantic_version(),
                    manifest.schema_version(),
                    created_at,
                ],
            )
            .map_err(|_| storage_error("storage.import.write.failed"))?;
        for tag in manifest.tags() {
            let normalized: String = tag.chars().flat_map(char::to_lowercase).collect();
            transaction
                .execute(
                    "INSERT INTO revision_tags(revision_id, tag) VALUES (?1, ?2)",
                    (revision.id().as_str(), normalized),
                )
                .map_err(|_| storage_error("storage.import.write.failed"))?;
        }
        for capability in manifest.capabilities() {
            let normalized: String = capability
                .id()
                .chars()
                .flat_map(char::to_lowercase)
                .collect();
            transaction
                .execute(
                    "INSERT INTO revision_capabilities(revision_id, capability) VALUES (?1, ?2)",
                    (revision.id().as_str(), normalized),
                )
                .map_err(|_| storage_error("storage.import.write.failed"))?;
        }
        transaction
            .execute(
                "INSERT INTO revision_trust(revision_id, trust_state, source_kind) VALUES (?1, 'quarantined', ?2)",
                (revision.id().as_str(), import_source_name(preview.source_kind())),
            )
            .map_err(|_| storage_error("storage.import.write.failed"))?;
        transaction
            .execute(
                "INSERT INTO skill_heads(skill_id, revision_id) VALUES (?1, ?2)",
                (skill_id_text, revision.id().as_str()),
            )
            .map_err(|_| storage_error("storage.import.write.failed"))?;
        let mut new_heads = heads;
        new_heads.push(revision.id().clone());
        new_heads.sort();
        transaction
            .commit()
            .map_err(|_| storage_error("storage.transaction.failed"))?;
        Ok(ImportResult::Imported {
            revision,
            heads: new_heads,
            trust_state: TrustState::Quarantined,
        })
    }

    async fn apply_import_async(
        &self,
        preview: ImportPreview,
        resolution: ImportResolution,
    ) -> AppResult<ImportResult> {
        let store = Self {
            connection: self.connection.clone(),
            path: self.path.clone(),
            blob_root: self.blob_root.clone(),
            read_only: self.read_only,
        };
        tokio::task::spawn_blocking(move || store.apply_import(&preview, resolution))
            .await
            .map_err(|_| storage_error("storage.import.worker.failed"))?
    }

    async fn store_validated_bundle_async(
        &self,
        bundle: ValidatedBundle,
        files: BundleFiles,
    ) -> AppResult<()> {
        let store = Self {
            connection: self.connection.clone(),
            path: self.path.clone(),
            blob_root: self.blob_root.clone(),
            read_only: self.read_only,
        };
        tokio::task::spawn_blocking(move || store.store_validated_bundle(&bundle, &files))
            .await
            .map_err(|_| storage_error("storage.blob.worker.failed"))?
    }

    async fn commit_revision_async(
        &self,
        request: SaveRevisionRequest,
    ) -> AppResult<SaveRevisionResult> {
        let store = Self {
            connection: self.connection.clone(),
            path: self.path.clone(),
            blob_root: self.blob_root.clone(),
            read_only: self.read_only,
        };
        tokio::task::spawn_blocking(move || store.commit_revision(&request))
            .await
            .map_err(|_| storage_error("storage.revision.worker.failed"))?
    }

    /// Lists valid content-addressed blobs with no committed content revision.
    /// Results are informational and never removed automatically: a writer
    /// may have staged a blob before a transaction failed or the process
    /// stopped.
    pub fn orphan_blob_hashes(&self) -> AppResult<Vec<ContentHash>> {
        let referenced = self.referenced_blob_hashes()?;
        let present = list_blob_hashes(&self.blob_root)
            .map_err(|_| storage_error("storage.blob.inventory.failed"))?;
        Ok(present
            .into_iter()
            .filter(|hash| !referenced.contains(hash))
            .collect())
    }

    fn referenced_blob_hashes(&self) -> AppResult<BTreeSet<ContentHash>> {
        let guard = self
            .connection
            .lock()
            .map_err(|_| storage_error("storage.lock.poisoned"))?;
        let mut statement = guard
            .prepare(
                "SELECT DISTINCT bundle_hash, state FROM revisions ORDER BY bundle_hash, state",
            )
            .map_err(|_| storage_error("storage.data.corrupt"))?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|_| storage_error("storage.data.corrupt"))?;
        let mut referenced = BTreeSet::new();
        for row in rows {
            let (value, state) = row.map_err(|_| storage_error("storage.data.corrupt"))?;
            let hash = ContentHash::parse_hex(&value)
                .map_err(|_| storage_error("storage.data.corrupt"))?;
            match state.as_str() {
                "content" => {
                    referenced.insert(hash);
                }
                "tombstone" if hash == ContentHash::from_digest([0; 32]) => {}
                "tombstone" => return Err(storage_error("storage.data.corrupt")),
                _ => return Err(storage_error("storage.data.corrupt")),
            }
        }
        Ok(referenced)
    }

    fn verify_referenced_blobs(&self) -> AppResult<()> {
        for hash in self.referenced_blob_hashes()? {
            verify_blob_bytes(&self.blob_root, &hash)
                .map_err(|_| storage_error("storage.blob.invalid"))?;
        }
        Ok(())
    }

    /// Runs work inside one transaction on the single writer. A returned
    /// error rolls back on drop; only a returned `Ok` commits.
    pub fn with_transaction<T>(
        &self,
        work: impl FnOnce(&Transaction<'_>) -> AppResult<T>,
    ) -> AppResult<T> {
        if self.read_only {
            return Err(storage_error("storage.store.read_only"));
        }
        let mut guard = self
            .connection
            .lock()
            .map_err(|_| storage_error("storage.lock.poisoned"))?;
        let transaction = guard
            .transaction()
            .map_err(|_| storage_error("storage.transaction.failed"))?;
        let output = work(&transaction)?;
        transaction
            .commit()
            .map_err(|_| storage_error("storage.transaction.failed"))?;
        Ok(output)
    }

    /// Commits one revision: the expected heads must match the stored heads
    /// exactly, otherwise the commit fails with the current heads and nothing
    /// is written. Revision, parents and the new head land in ONE
    /// transaction; tombstones additionally record their observed heads.
    /// Blobs are staged before this call, so a failed commit leaves an
    /// unreferenced blob but never an invalid head.
    pub fn commit_revision(&self, request: &SaveRevisionRequest) -> AppResult<SaveRevisionResult> {
        if self.read_only {
            return Err(storage_error("storage.store.read_only"));
        }
        let record = RevisionRecord::new(
            request.skill_id(),
            request.bundle_hash().cloned(),
            request.parents().to_vec(),
            request.kind().clone(),
            request.semantic_version().to_owned(),
        )?;
        if matches!(request.kind(), RevisionKind::Content) {
            verify_blob_bytes(&self.blob_root, record.bundle_hash())
                .map_err(|_| storage_error("storage.blob.invalid"))?;
        }
        let mut guard = self
            .connection
            .lock()
            .map_err(|_| storage_error("storage.lock.poisoned"))?;
        let transaction = guard
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| storage_error("storage.transaction.failed"))?;
        let current = read_heads(&transaction, request.skill_id())?;
        if let Some(expected_generation) = request.expected_draft_generation() {
            let stored_generation = transaction
                .query_row(
                    "SELECT generation FROM drafts WHERE skill_id = ?1",
                    [request.skill_id().as_uuid().to_string()],
                    |row| row.get::<_, i64>(0),
                )
                .optional()
                .map_err(|_| storage_error("storage.query.failed"))?;
            if stored_generation != Some(expected_generation as i64) {
                return Err(draft_conflict(&transaction, request.skill_id())?);
            }
        }

        if matches!(request.kind(), RevisionKind::Content) && current.len() == 1 {
            let current_id = RevisionId::parse_hex(&current[0])
                .map_err(|_| storage_error("storage.data.corrupt"))?;
            let current_content = transaction
                .query_row(
                    "SELECT bundle_hash, semantic_version, state, schema_version FROM revisions WHERE id = ?1 AND skill_id = ?2",
                    rusqlite::params![current_id.as_str(), request.skill_id().as_uuid().to_string()],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, i64>(3)?,
                        ))
                    },
                )
                .optional()
                .map_err(|_| storage_error("storage.data.corrupt"))?;
            if current_content.is_none() {
                return Err(storage_error("storage.data.corrupt"));
            }
            if let Some((bundle_hash, version, state, schema_version)) = current_content
                && state == "content"
                && schema_version == request.schema_version() as i64
                && request
                    .bundle_hash()
                    .is_some_and(|hash| hash.as_str() == bundle_hash)
            {
                if version != request.semantic_version() {
                    return Err(AppError::Validation(vec![Diagnostic::error(
                        "revision.bundle.metadata_mismatch",
                        "Identical bundle bytes cannot be published with another version.",
                    )]));
                }
                let existing = load_revision_record(&transaction, request.skill_id(), &current_id)?;
                if let Some(generation) = request.expected_draft_generation() {
                    let removed = transaction
                        .execute(
                            "DELETE FROM drafts WHERE skill_id = ?1 AND generation = ?2",
                            rusqlite::params![
                                request.skill_id().as_uuid().to_string(),
                                generation as i64
                            ],
                        )
                        .map_err(|_| storage_error("storage.draft.write.failed"))?;
                    if removed != 1 {
                        return Err(draft_conflict(&transaction, request.skill_id())?);
                    }
                }
                transaction
                    .commit()
                    .map_err(|_| storage_error("storage.transaction.failed"))?;
                return Ok(SaveRevisionResult::no_op(existing, vec![current_id]));
            }
        }

        let mut expected: Vec<String> = request
            .expected_heads()
            .iter()
            .map(|head| head.as_str().to_owned())
            .collect();
        expected.sort();
        expected.dedup();
        if current != expected {
            let conflicting = current
                .iter()
                .map(|hex| RevisionId::parse_hex(hex))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| storage_error("storage.data.corrupt"))?;
            return Err(AppError::Conflict {
                current: conflicting,
            });
        }
        let parents: Vec<String> = record
            .parents()
            .iter()
            .map(|parent| parent.as_str().to_owned())
            .collect();
        if parents != current {
            let conflicting = current
                .iter()
                .map(|hex| RevisionId::parse_hex(hex))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| storage_error("storage.data.corrupt"))?;
            return Err(AppError::Conflict {
                current: conflicting,
            });
        }
        if let RevisionKind::Tombstone { observed_heads } = request.kind() {
            let mut observed: Vec<String> = observed_heads
                .iter()
                .map(|head| head.as_str().to_owned())
                .collect();
            observed.sort();
            observed.dedup();
            if observed != current {
                let conflicting = current
                    .iter()
                    .map(|hex| RevisionId::parse_hex(hex))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|_| storage_error("storage.data.corrupt"))?;
                return Err(AppError::Conflict {
                    current: conflicting,
                });
            }
        }
        if matches!(request.kind(), RevisionKind::Content) && !current.is_empty() {
            let next_version = request
                .semantic_version()
                .parse::<semver::Version>()
                .map_err(|_| AppError::CryptoInvalid)?;
            for head in &current {
                let previous: String = transaction
                    .query_row(
                        "SELECT semantic_version FROM revisions WHERE id = ?1 AND skill_id = ?2",
                        rusqlite::params![head, request.skill_id().as_uuid().to_string()],
                        |row| row.get(0),
                    )
                    .map_err(|_| storage_error("storage.data.corrupt"))?;
                let previous_version = previous
                    .parse::<semver::Version>()
                    .map_err(|_| storage_error("storage.data.corrupt"))?;
                if next_version <= previous_version {
                    return Err(AppError::Validation(vec![Diagnostic::error(
                        "revision.version.bump_required",
                        "Changed content must increase the semantic version of every current head.",
                    )]));
                }
            }
        }
        let state = match request.kind() {
            RevisionKind::Content => "content",
            RevisionKind::Tombstone { .. } => "tombstone",
        };
        let created_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        transaction
            .execute(
                "INSERT INTO revisions(id, skill_id, bundle_hash, semantic_version, schema_version, state, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                (
                    record.id().as_str(),
                    request.skill_id().as_uuid().to_string(),
                    record.bundle_hash().as_str(),
                    request.semantic_version(),
                    request.schema_version(),
                    state,
                    created_at,
                ),
            )
            .map_err(|_| storage_error("storage.revision.rejected"))?;
        if matches!(request.kind(), RevisionKind::Content) {
            let display_name: String = transaction
                .query_row(
                    "SELECT display_name FROM skills WHERE id = ?1",
                    [request.skill_id().as_uuid().to_string()],
                    |row| row.get(0),
                )
                .map_err(|_| storage_error("storage.revision.rejected"))?;
            let normalized_name: String =
                display_name.chars().flat_map(char::to_lowercase).collect();
            transaction
                .execute(
                    "INSERT INTO library_catalog(skill_id, normalized_display_name) VALUES (?1, ?2) ON CONFLICT(skill_id) DO UPDATE SET normalized_display_name = excluded.normalized_display_name",
                    (request.skill_id().as_uuid().to_string(), normalized_name),
                )
                .map_err(|_| storage_error("storage.revision.rejected"))?;
            for tag in request.catalog_tags() {
                transaction
                    .execute(
                        "INSERT INTO revision_tags(revision_id, tag) VALUES (?1, ?2)",
                        (record.id().as_str(), tag),
                    )
                    .map_err(|_| storage_error("storage.revision.rejected"))?;
            }
            for capability in request.catalog_capabilities() {
                transaction
                    .execute(
                        "INSERT INTO revision_capabilities(revision_id, capability) VALUES (?1, ?2)",
                        (record.id().as_str(), capability),
                    )
                    .map_err(|_| storage_error("storage.revision.rejected"))?;
            }
        }
        for parent in record.parents() {
            transaction
                .execute(
                    "INSERT INTO revision_parents(revision_id, parent_revision_id) VALUES (?1, ?2)",
                    (record.id().as_str(), parent.as_str()),
                )
                .map_err(|_| storage_error("storage.revision.rejected"))?;
        }
        if let RevisionKind::Tombstone { observed_heads } = request.kind() {
            transaction
                .execute(
                    "INSERT INTO deletions(skill_id, deletion_revision_id, observed_heads_json) VALUES (?1, ?2, ?3)",
                    (
                        request.skill_id().as_uuid().to_string(),
                        record.id().as_str(),
                        observed_heads_json(observed_heads),
                    ),
                )
                .map_err(|_| storage_error("storage.revision.rejected"))?;
        }
        transaction
            .execute(
                "DELETE FROM skill_heads WHERE skill_id = ?1",
                [request.skill_id().as_uuid().to_string()],
            )
            .map_err(|_| storage_error("storage.revision.rejected"))?;
        transaction
            .execute(
                "INSERT INTO skill_heads(skill_id, revision_id) VALUES (?1, ?2)",
                (
                    request.skill_id().as_uuid().to_string(),
                    record.id().as_str(),
                ),
            )
            .map_err(|_| storage_error("storage.revision.rejected"))?;
        if let Some(generation) = request.expected_draft_generation() {
            let removed = transaction
                .execute(
                    "DELETE FROM drafts WHERE skill_id = ?1 AND generation = ?2",
                    rusqlite::params![request.skill_id().as_uuid().to_string(), generation as i64],
                )
                .map_err(|_| storage_error("storage.draft.write.failed"))?;
            if removed != 1 {
                return Err(draft_conflict(&transaction, request.skill_id())?);
            }
        }
        transaction
            .commit()
            .map_err(|_| storage_error("storage.transaction.failed"))?;
        let new_heads = vec![record.id().clone()];
        Ok(SaveRevisionResult::new(record, new_heads))
    }
}

fn list_skills_from_connection(
    connection: &Mutex<Connection>,
    query: &LibraryQuery,
) -> AppResult<LibraryPage> {
    use rusqlite::types::Value;

    let guard = connection
        .lock()
        .map_err(|_| storage_error("storage.lock.poisoned"))?;
    let mut predicates = Vec::new();
    let mut values = Vec::<Value>::new();
    if let Some(search) = query.search() {
        predicates.push("c.normalized_display_name LIKE ? ESCAPE '\\'".to_owned());
        values.push(Value::Text(format!("%{}%", escape_like(search))));
    }
    for tag in query.tags() {
        predicates.push("EXISTS (SELECT 1 FROM skill_heads h JOIN revision_tags t ON t.revision_id = h.revision_id WHERE h.skill_id = s.id AND t.tag = ?)".to_owned());
        values.push(Value::Text(tag.clone()));
    }
    for capability in query.capabilities() {
        predicates.push("EXISTS (SELECT 1 FROM skill_heads h JOIN revision_capabilities c2 ON c2.revision_id = h.revision_id WHERE h.skill_id = s.id AND c2.capability = ?)".to_owned());
        values.push(Value::Text(capability.clone()));
    }
    match query.state() {
        LibraryItemState::Any => {}
        LibraryItemState::Active => predicates.push(
            "(SELECT COUNT(*) FROM skill_heads h JOIN revisions r ON r.id = h.revision_id WHERE h.skill_id = s.id) = 1 AND EXISTS (SELECT 1 FROM skill_heads h JOIN revisions r ON r.id = h.revision_id WHERE h.skill_id = s.id AND r.state = 'content')".to_owned(),
        ),
        LibraryItemState::Deleted => predicates.push(
            "EXISTS (SELECT 1 FROM skill_heads h WHERE h.skill_id = s.id) AND NOT EXISTS (SELECT 1 FROM skill_heads h JOIN revisions r ON r.id = h.revision_id WHERE h.skill_id = s.id AND r.state <> 'tombstone')".to_owned(),
        ),
        LibraryItemState::Conflicted => predicates.push(
            "(SELECT COUNT(*) FROM skill_heads h WHERE h.skill_id = s.id) > 1".to_owned(),
        ),
    }
    if let Some(cursor) = query.after() {
        predicates.push(
            "(c.normalized_display_name > ? OR (c.normalized_display_name = ? AND s.id > ?))"
                .to_owned(),
        );
        values.push(Value::Text(cursor.normalized_display_name().to_owned()));
        values.push(Value::Text(cursor.normalized_display_name().to_owned()));
        values.push(Value::Text(cursor.skill_id().as_uuid().to_string()));
    }
    let mut sql = String::from(
        "SELECT s.id, s.slug, s.display_name, c.normalized_display_name FROM skills s JOIN library_catalog c ON c.skill_id = s.id",
    );
    if !predicates.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&predicates.join(" AND "));
    }
    sql.push_str(" ORDER BY c.normalized_display_name, s.id LIMIT ?");
    values.push(Value::Integer((query.page_size() + 1) as i64));

    let mut statement = guard
        .prepare(&sql)
        .map_err(|_| storage_error("storage.query.failed"))?;
    let rows = statement
        .query_map(rusqlite::params_from_iter(values), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|_| storage_error("storage.query.failed"))?;
    let mut records = Vec::new();
    for row in rows {
        records.push(row.map_err(|_| storage_error("storage.data.corrupt"))?);
    }
    drop(statement);

    let has_more = records.len() > query.page_size();
    records.truncate(query.page_size());
    let mut items = Vec::with_capacity(records.len());
    let mut last_cursor = None;
    for (id, slug, display_name, normalized_name) in records {
        let skill_id = SkillId::parse(&id).map_err(|_| storage_error("storage.data.corrupt"))?;
        let heads = load_catalog_heads(&guard, skill_id)?;
        let tags = load_catalog_metadata(&guard, skill_id, "revision_tags", "tag")?;
        let capabilities =
            load_catalog_metadata(&guard, skill_id, "revision_capabilities", "capability")?;
        items.push(LibrarySkillSummary::new(
            skill_id,
            slug,
            display_name,
            tags,
            capabilities,
            heads,
        ));
        last_cursor = Some(
            LibraryCursor::new(&normalized_name, skill_id)
                .map_err(|_| storage_error("storage.data.corrupt"))?,
        );
    }
    let next = has_more.then_some(last_cursor).flatten();
    Ok(LibraryPage::new(items, next))
}

fn load_catalog_heads(
    connection: &Connection,
    skill_id: SkillId,
) -> AppResult<Vec<LibraryHeadSummary>> {
    let mut statement = connection
        .prepare(
            "SELECT r.id, r.semantic_version, r.state FROM skill_heads h JOIN revisions r ON r.id = h.revision_id WHERE h.skill_id = ?1 ORDER BY r.id",
        )
        .map_err(|_| storage_error("storage.query.failed"))?;
    let rows = statement
        .query_map([skill_id.as_uuid().to_string()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|_| storage_error("storage.query.failed"))?;
    rows.map(|row| {
        let (revision, version, state) = row.map_err(|_| storage_error("storage.data.corrupt"))?;
        let deleted = match state.as_str() {
            "content" => false,
            "tombstone" => true,
            _ => return Err(storage_error("storage.data.corrupt")),
        };
        Ok(LibraryHeadSummary::new(
            RevisionId::parse_hex(&revision).map_err(|_| storage_error("storage.data.corrupt"))?,
            version,
            deleted,
        ))
    })
    .collect()
}

fn load_catalog_metadata(
    connection: &Connection,
    skill_id: SkillId,
    table: &'static str,
    column: &'static str,
) -> AppResult<Vec<String>> {
    // These identifiers are fixed catalog call-site values, not query input.
    let sql = format!(
        "SELECT DISTINCT m.{column} FROM {table} m JOIN skill_heads h ON h.revision_id = m.revision_id WHERE h.skill_id = ?1 ORDER BY m.{column}"
    );
    let mut statement = connection
        .prepare(&sql)
        .map_err(|_| storage_error("storage.query.failed"))?;
    statement
        .query_map([skill_id.as_uuid().to_string()], |row| row.get(0))
        .map_err(|_| storage_error("storage.query.failed"))?
        .map(|value| value.map_err(|_| storage_error("storage.data.corrupt")))
        .collect()
}

fn escape_like(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        if matches!(character, '%' | '_' | '\\') {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

fn read_heads(transaction: &Transaction<'_>, skill_id: SkillId) -> AppResult<Vec<String>> {
    let mut statement = transaction
        .prepare("SELECT revision_id FROM skill_heads WHERE skill_id = ?1 ORDER BY revision_id")
        .map_err(|_| storage_error("storage.revision.rejected"))?;
    statement
        .query_map([skill_id.as_uuid().to_string()], |row| row.get(0))
        .map_err(|_| storage_error("storage.revision.rejected"))?
        .map(|id| id.map_err(|_| storage_error("storage.data.corrupt")))
        .collect()
}

fn load_skill_from_connection(
    connection: &Mutex<Connection>,
    blob_root: &Path,
    skill_id: SkillId,
) -> AppResult<Option<LibrarySkillDetail>> {
    let guard = connection
        .lock()
        .map_err(|_| storage_error("storage.lock.poisoned"))?;
    let metadata = guard
        .query_row(
            "SELECT slug, display_name FROM skills WHERE id = ?1",
            [skill_id.as_uuid().to_string()],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(|_| storage_error("storage.query.failed"))?;
    let Some((slug, display_name)) = metadata else {
        return Ok(None);
    };
    let heads = load_library_heads(&guard, skill_id)?;
    let tags = load_head_metadata(&guard, skill_id, "revision_tags", "tag")?;
    let capabilities = load_head_metadata(&guard, skill_id, "revision_capabilities", "capability")?;
    let summary = LibrarySkillSummary::new(
        skill_id,
        slug,
        display_name,
        tags,
        capabilities,
        heads.clone(),
    );
    let mut loaded_heads = Vec::with_capacity(heads.len());
    for head in heads {
        let files = if head.deleted() {
            None
        } else {
            let bundle_hash: String = guard
                .query_row(
                    "SELECT bundle_hash FROM revisions WHERE id = ?1 AND skill_id = ?2 AND state = 'content'",
                    rusqlite::params![head.revision_id().as_str(), skill_id.as_uuid().to_string()],
                    |row| row.get(0),
                )
                .map_err(|_| storage_error("storage.data.corrupt"))?;
            let bundle_hash = ContentHash::parse_hex(&bundle_hash)
                .map_err(|_| storage_error("storage.data.corrupt"))?;
            Some(
                verify_blob_bytes(blob_root, &bundle_hash)
                    .map_err(|_| storage_error("storage.blob.invalid"))?,
            )
        };
        loaded_heads.push(LibraryLoadedHead::new(head, files));
    }
    Ok(Some(LibrarySkillDetail::new(summary, loaded_heads)))
}

fn load_history_from_connection(
    connection: &Mutex<Connection>,
    query: &LibraryHistoryQuery,
) -> AppResult<LibraryHistoryPage> {
    const MAX_HISTORY_RELATIONS: usize = 4096;
    const MAX_OBSERVED_HEADS_JSON_BYTES: usize = 256 * 1024;

    let guard = connection
        .lock()
        .map_err(|_| storage_error("storage.lock.poisoned"))?;
    let mut sql = String::from(
        "SELECT id, bundle_hash, semantic_version, state FROM revisions WHERE skill_id = ?",
    );
    let mut parameters = vec![rusqlite::types::Value::Text(
        query.skill_id().as_uuid().to_string(),
    )];
    if let Some(after) = query.after() {
        sql.push_str(" AND id > ?");
        parameters.push(rusqlite::types::Value::Text(after.as_str().to_owned()));
    }
    sql.push_str(" ORDER BY id LIMIT ?");
    parameters.push(rusqlite::types::Value::Integer(
        (query.page_size() + 1) as i64,
    ));
    let mut statement = guard
        .prepare(&sql)
        .map_err(|_| storage_error("storage.query.failed"))?;
    let rows = statement
        .query_map(rusqlite::params_from_iter(parameters), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|_| storage_error("storage.query.failed"))?;
    let mut records = Vec::new();
    for row in rows {
        records.push(row.map_err(|_| storage_error("storage.data.corrupt"))?);
    }
    drop(statement);
    let has_more = records.len() > query.page_size();
    records.truncate(query.page_size());
    let mut entries = Vec::with_capacity(records.len());
    let mut next = None;
    for (id, bundle, version, state) in records {
        let revision_id =
            RevisionId::parse_hex(&id).map_err(|_| storage_error("storage.data.corrupt"))?;
        let (deleted, bundle_hash) = match state.as_str() {
            "content" => (
                false,
                Some(
                    ContentHash::parse_hex(&bundle)
                        .map_err(|_| storage_error("storage.data.corrupt"))?,
                ),
            ),
            "tombstone" => {
                if ContentHash::parse_hex(&bundle)
                    .map_err(|_| storage_error("storage.data.corrupt"))?
                    != ContentHash::from_digest([0; 32])
                {
                    return Err(storage_error("storage.data.corrupt"));
                }
                (true, None)
            }
            _ => return Err(storage_error("storage.data.corrupt")),
        };
        let parents = load_revision_relations(
            &guard,
            "SELECT parent_revision_id FROM revision_parents WHERE revision_id = ?1 ORDER BY parent_revision_id",
            &id,
            MAX_HISTORY_RELATIONS,
        )?;
        let observed_json = guard
            .query_row(
                "SELECT observed_heads_json FROM deletions WHERE deletion_revision_id = ?1",
                [&id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|_| storage_error("storage.data.corrupt"))?;
        let observed_heads = if let Some(json) = observed_json {
            if !deleted || json.len() > MAX_OBSERVED_HEADS_JSON_BYTES {
                return Err(storage_error("storage.data.corrupt"));
            }
            let raw: Vec<String> =
                serde_json::from_str(&json).map_err(|_| storage_error("storage.data.corrupt"))?;
            if raw.len() > MAX_HISTORY_RELATIONS {
                return Err(storage_error("storage.data.corrupt"));
            }
            raw.into_iter()
                .map(|value| {
                    RevisionId::parse_hex(&value).map_err(|_| storage_error("storage.data.corrupt"))
                })
                .collect::<AppResult<Vec<_>>>()?
        } else if deleted {
            return Err(storage_error("storage.data.corrupt"));
        } else {
            Vec::new()
        };
        entries.push(LibraryHistoryEntry::new(
            revision_id.clone(),
            parents,
            bundle_hash,
            version,
            deleted,
            observed_heads,
        ));
        next = Some(revision_id);
    }
    Ok(LibraryHistoryPage::new(
        entries,
        has_more.then_some(next).flatten(),
    ))
}

fn load_revision_relations(
    connection: &Connection,
    sql: &'static str,
    revision_id: &str,
    limit: usize,
) -> AppResult<Vec<RevisionId>> {
    let mut statement = connection
        .prepare(sql)
        .map_err(|_| storage_error("storage.query.failed"))?;
    let rows = statement
        .query_map([revision_id], |row| row.get::<_, String>(0))
        .map_err(|_| storage_error("storage.query.failed"))?;
    let mut values = Vec::new();
    for row in rows {
        values.push(row.map_err(|_| storage_error("storage.data.corrupt"))?);
        if values.len() > limit {
            return Err(storage_error("storage.data.corrupt"));
        }
    }
    values
        .into_iter()
        .map(|value| {
            RevisionId::parse_hex(&value).map_err(|_| storage_error("storage.data.corrupt"))
        })
        .collect()
}

fn load_revision_record(
    transaction: &Transaction<'_>,
    skill_id: SkillId,
    revision_id: &RevisionId,
) -> AppResult<RevisionRecord> {
    const MAX_OBSERVED_HEADS_JSON_BYTES: usize = 256 * 1024;
    let (bundle, version, state) = transaction
        .query_row(
            "SELECT bundle_hash, semantic_version, state FROM revisions WHERE id = ?1 AND skill_id = ?2",
            rusqlite::params![revision_id.as_str(), skill_id.as_uuid().to_string()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .map_err(|_| storage_error("storage.data.corrupt"))?;
    let parents = load_revision_relations(
        transaction,
        "SELECT parent_revision_id FROM revision_parents WHERE revision_id = ?1 ORDER BY parent_revision_id",
        revision_id.as_str(),
        4096,
    )?;
    let kind = match state.as_str() {
        "content" => RevisionKind::Content,
        "tombstone" => {
            if ContentHash::parse_hex(&bundle).map_err(|_| storage_error("storage.data.corrupt"))?
                != ContentHash::from_digest([0; 32])
            {
                return Err(storage_error("storage.data.corrupt"));
            }
            let observed_json = transaction
                .query_row(
                    "SELECT observed_heads_json FROM deletions WHERE deletion_revision_id = ?1",
                    [revision_id.as_str()],
                    |row| row.get::<_, String>(0),
                )
                .map_err(|_| storage_error("storage.data.corrupt"))?;
            if observed_json.len() > MAX_OBSERVED_HEADS_JSON_BYTES {
                return Err(storage_error("storage.data.corrupt"));
            }
            let raw: Vec<String> = serde_json::from_str(&observed_json)
                .map_err(|_| storage_error("storage.data.corrupt"))?;
            if raw.len() > 4096 {
                return Err(storage_error("storage.data.corrupt"));
            }
            let observed_heads = raw
                .into_iter()
                .map(|value| {
                    RevisionId::parse_hex(&value).map_err(|_| storage_error("storage.data.corrupt"))
                })
                .collect::<AppResult<Vec<_>>>()?;
            RevisionKind::Tombstone { observed_heads }
        }
        _ => return Err(storage_error("storage.data.corrupt")),
    };
    let bundle_hash = match kind {
        RevisionKind::Content => Some(
            ContentHash::parse_hex(&bundle).map_err(|_| storage_error("storage.data.corrupt"))?,
        ),
        RevisionKind::Tombstone { .. } => None,
    };
    let record = RevisionRecord::new(skill_id, bundle_hash, parents, kind, version)?;
    if record.id() != revision_id {
        return Err(storage_error("storage.data.corrupt"));
    }
    Ok(record)
}

fn draft_conflict(transaction: &Transaction<'_>, skill_id: SkillId) -> AppResult<AppError> {
    let current = read_heads(transaction, skill_id)?
        .into_iter()
        .map(|value| RevisionId::parse_hex(&value))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| storage_error("storage.data.corrupt"))?;
    Ok(AppError::Conflict { current })
}

fn read_revision_trust(
    transaction: &Transaction<'_>,
    revision_id: &RevisionId,
) -> AppResult<TrustState> {
    let state = transaction
        .query_row(
            "SELECT trust_state FROM revision_trust WHERE revision_id = ?1",
            [revision_id.as_str()],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|_| storage_error("storage.data.corrupt"))?;
    match state.as_deref() {
        None | Some("quarantined") => Ok(TrustState::Quarantined),
        Some("reviewed") => Ok(TrustState::Reviewed),
        Some(_) => Err(storage_error("storage.data.corrupt")),
    }
}

fn import_source_name(source: ImportSourceKind) -> &'static str {
    match source {
        ImportSourceKind::Directory => "directory",
        ImportSourceKind::Archive => "archive",
        ImportSourceKind::PlainSkill => "plain-skill",
    }
}

fn encode_skill_draft(draft: &SkillDraft) -> AppResult<Vec<u8>> {
    let mut payload = Vec::new();
    payload.extend_from_slice(b"JSD2");
    payload.extend_from_slice(draft.skill_id().as_uuid().to_string().as_bytes());
    payload.extend_from_slice(&draft.generation().to_be_bytes());
    match draft.base_head() {
        Some(head) => {
            payload.push(1);
            payload.extend_from_slice(head.as_str().as_bytes());
        }
        None => payload.push(0),
    }
    payload.push(match draft.trust_state() {
        TrustState::Quarantined => 0,
        TrustState::Reviewed => 1,
    });
    let count =
        u32::try_from(draft.files().len()).map_err(|_| storage_error("storage.draft.limit"))?;
    payload.extend_from_slice(&count.to_be_bytes());
    for (path, bytes) in draft.files() {
        let path = path.as_str().as_bytes();
        let path_len =
            u16::try_from(path.len()).map_err(|_| storage_error("storage.draft.path.invalid"))?;
        let data_len =
            u32::try_from(bytes.len()).map_err(|_| storage_error("storage.draft.limit"))?;
        payload.extend_from_slice(&path_len.to_be_bytes());
        payload.extend_from_slice(path);
        payload.extend_from_slice(&data_len.to_be_bytes());
        payload.extend_from_slice(bytes);
    }
    if payload.len() > MAX_DRAFT_PAYLOAD_BYTES {
        return Err(storage_error("storage.draft.limit"));
    }
    Ok(payload)
}

fn decode_skill_draft(payload: &[u8]) -> AppResult<SkillDraft> {
    if payload.len() > MAX_DRAFT_PAYLOAD_BYTES {
        return Err(storage_error("storage.draft.corrupt"));
    }
    let mut cursor = 0usize;
    let version = take_draft_bytes(payload, &mut cursor, 4)?;
    let legacy = version == b"JSD1";
    if !legacy && version != b"JSD2" {
        return Err(storage_error("storage.draft.corrupt"));
    }
    let skill_id_text = std::str::from_utf8(take_draft_bytes(payload, &mut cursor, 36)?)
        .map_err(|_| storage_error("storage.draft.corrupt"))?;
    let skill_id =
        SkillId::parse(skill_id_text).map_err(|_| storage_error("storage.draft.corrupt"))?;
    let generation = u64::from_be_bytes(
        take_draft_bytes(payload, &mut cursor, 8)?
            .try_into()
            .map_err(|_| storage_error("storage.draft.corrupt"))?,
    );
    let base_head = match take_draft_bytes(payload, &mut cursor, 1)?[0] {
        0 => None,
        1 => {
            let value = std::str::from_utf8(take_draft_bytes(payload, &mut cursor, 64)?)
                .map_err(|_| storage_error("storage.draft.corrupt"))?;
            Some(RevisionId::parse_hex(value).map_err(|_| storage_error("storage.draft.corrupt"))?)
        }
        _ => return Err(storage_error("storage.draft.corrupt")),
    };
    let trust_state = if legacy {
        // Older drafts have no independent review evidence: retain them quarantined.
        TrustState::Quarantined
    } else {
        match take_draft_bytes(payload, &mut cursor, 1)?[0] {
            0 => TrustState::Quarantined,
            1 => TrustState::Reviewed,
            _ => return Err(storage_error("storage.draft.corrupt")),
        }
    };
    let count = u32::from_be_bytes(
        take_draft_bytes(payload, &mut cursor, 4)?
            .try_into()
            .map_err(|_| storage_error("storage.draft.corrupt"))?,
    ) as usize;
    if count > 2_000 {
        return Err(storage_error("storage.draft.corrupt"));
    }
    let mut files = std::collections::BTreeMap::new();
    let mut total_bytes = 0usize;
    for _ in 0..count {
        let path_len = u16::from_be_bytes(
            take_draft_bytes(payload, &mut cursor, 2)?
                .try_into()
                .map_err(|_| storage_error("storage.draft.corrupt"))?,
        ) as usize;
        let path_text = std::str::from_utf8(take_draft_bytes(payload, &mut cursor, path_len)?)
            .map_err(|_| storage_error("storage.draft.corrupt"))?;
        let path = PortablePath::new(path_text.to_owned())
            .map_err(|_| storage_error("storage.draft.corrupt"))?;
        let content_len = u32::from_be_bytes(
            take_draft_bytes(payload, &mut cursor, 4)?
                .try_into()
                .map_err(|_| storage_error("storage.draft.corrupt"))?,
        ) as usize;
        total_bytes = total_bytes
            .checked_add(content_len)
            .ok_or_else(|| storage_error("storage.draft.corrupt"))?;
        if total_bytes > 20 * 1024 * 1024 {
            return Err(storage_error("storage.draft.corrupt"));
        }
        let content = take_draft_bytes(payload, &mut cursor, content_len)?.to_vec();
        if files.insert(path, content).is_some() {
            return Err(storage_error("storage.draft.corrupt"));
        }
    }
    if cursor != payload.len() {
        return Err(storage_error("storage.draft.corrupt"));
    }
    match trust_state {
        TrustState::Quarantined => {
            SkillDraft::new_quarantined(skill_id, base_head, generation, files)
        }
        TrustState::Reviewed => SkillDraft::new(skill_id, base_head, generation, files),
    }
    .map_err(|_| storage_error("storage.draft.corrupt"))
}

fn take_draft_bytes<'a>(
    payload: &'a [u8],
    cursor: &mut usize,
    length: usize,
) -> AppResult<&'a [u8]> {
    let end = cursor
        .checked_add(length)
        .ok_or_else(|| storage_error("storage.draft.corrupt"))?;
    let bytes = payload
        .get(*cursor..end)
        .ok_or_else(|| storage_error("storage.draft.corrupt"))?;
    *cursor = end;
    Ok(bytes)
}

fn load_library_heads(
    connection: &Connection,
    skill_id: SkillId,
) -> AppResult<Vec<LibraryHeadSummary>> {
    let mut statement = connection
        .prepare(
            "SELECT r.id, r.semantic_version, r.state FROM skill_heads h JOIN revisions r ON r.id = h.revision_id WHERE h.skill_id = ?1 ORDER BY r.id",
        )
        .map_err(|_| storage_error("storage.query.failed"))?;
    let rows = statement
        .query_map([skill_id.as_uuid().to_string()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|_| storage_error("storage.query.failed"))?;
    let mut heads = Vec::new();
    for row in rows {
        let (revision, version, state) = row.map_err(|_| storage_error("storage.data.corrupt"))?;
        let deleted = match state.as_str() {
            "content" => false,
            "tombstone" => true,
            _ => return Err(storage_error("storage.data.corrupt")),
        };
        heads.push(LibraryHeadSummary::new(
            RevisionId::parse_hex(&revision).map_err(|_| storage_error("storage.data.corrupt"))?,
            version,
            deleted,
        ));
    }
    Ok(heads)
}

fn load_head_metadata(
    connection: &Connection,
    skill_id: SkillId,
    table: &'static str,
    column: &'static str,
) -> AppResult<Vec<String>> {
    // Table and column are private fixed call-site constants, never query input.
    let sql = format!(
        "SELECT DISTINCT m.{column} FROM {table} m JOIN skill_heads h ON h.revision_id = m.revision_id WHERE h.skill_id = ?1 ORDER BY m.{column}"
    );
    let mut statement = connection
        .prepare(&sql)
        .map_err(|_| storage_error("storage.query.failed"))?;
    statement
        .query_map([skill_id.as_uuid().to_string()], |row| row.get(0))
        .map_err(|_| storage_error("storage.query.failed"))?
        .map(|value| value.map_err(|_| storage_error("storage.data.corrupt")))
        .collect()
}

/// Serializes observed heads as a JSON array by hand. Revision ids are
/// validated 64-char lowercase hex at construction, so no quoting or escape
/// sequence can appear; anything else fails the debug assertion in tests.
fn observed_heads_json(heads: &[RevisionId]) -> String {
    let mut out = String::from("[");
    for (index, head) in heads.iter().enumerate() {
        debug_assert!(head.as_str().bytes().all(|byte| byte.is_ascii_hexdigit()));
        if index > 0 {
            out.push(',');
        }
        out.push('"');
        out.push_str(head.as_str());
        out.push('"');
    }
    out.push(']');
    out
}

#[async_trait::async_trait]
impl StoragePort for SqliteStore {
    fn schema_version(&self) -> AppResult<u32> {
        SqliteStore::schema_version(self)
    }

    fn check_integrity(&self) -> AppResult<()> {
        SqliteStore::check_integrity(self)
    }

    async fn list_skills(&self, query: LibraryQuery) -> AppResult<LibraryPage> {
        self.list_skills_async(query).await
    }

    async fn load_skill(&self, skill_id: SkillId) -> AppResult<Option<LibrarySkillDetail>> {
        self.load_skill_async(skill_id).await
    }

    async fn load_history(&self, query: LibraryHistoryQuery) -> AppResult<LibraryHistoryPage> {
        self.load_history_async(query).await
    }

    async fn load_draft(&self, skill_id: SkillId) -> AppResult<Option<SkillDraft>> {
        self.load_draft_async(skill_id).await
    }

    async fn save_draft(&self, request: SaveDraftRequest) -> AppResult<()> {
        self.save_draft_async(request).await
    }

    async fn create_skill(&self, create: CreateSkill, draft: SkillDraft) -> AppResult<()> {
        self.create_skill_async(create, draft).await
    }

    async fn store_validated_bundle(
        &self,
        bundle: ValidatedBundle,
        files: BundleFiles,
    ) -> AppResult<()> {
        self.store_validated_bundle_async(bundle, files).await
    }

    async fn commit_revision(&self, request: SaveRevisionRequest) -> AppResult<SaveRevisionResult> {
        self.commit_revision_async(request).await
    }

    async fn get_heads(&self, skill_id: SkillId) -> AppResult<Vec<RevisionId>> {
        self.get_heads_async(skill_id).await
    }

    async fn skill_exists(&self, skill_id: SkillId) -> AppResult<bool> {
        self.skill_exists_async(skill_id).await
    }

    async fn find_revision_by_bundle(
        &self,
        skill_id: SkillId,
        content_hash: ContentHash,
    ) -> AppResult<Option<RevisionId>> {
        self.find_revision_by_bundle_async(skill_id, content_hash)
            .await
    }

    async fn apply_import(
        &self,
        preview: ImportPreview,
        resolution: ImportResolution,
    ) -> AppResult<ImportResult> {
        self.apply_import_async(preview, resolution).await
    }
}

const MAX_PENDING_REPO_CHANGE_JOURNALS: usize = 256;
const MAX_REPO_CHANGE_JOURNAL_BYTES: usize = 16 * 1024;

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredRepoChangeJournal {
    schema_version: u8,
    operation_id: String,
    root_path: String,
    root_fingerprint: String,
    expected_head: String,
    target: String,
    staging_target: String,
    previous_hash: Option<String>,
    proposed_hash: String,
}

fn encode_repo_change_journal(journal: &RepoChangeJournal) -> AppResult<Vec<u8>> {
    let root_path = journal
        .root()
        .path()
        .to_str()
        .ok_or_else(|| storage_error("storage.operation_journal.root.invalid"))?;
    let stored = StoredRepoChangeJournal {
        schema_version: 1,
        operation_id: journal.operation_id().as_uuid().to_string(),
        root_path: root_path.to_owned(),
        root_fingerprint: journal.root_fingerprint().as_str().to_owned(),
        expected_head: journal.expected_head().as_str().to_owned(),
        target: journal.target().as_str().to_owned(),
        staging_target: journal.staging_target().as_str().to_owned(),
        previous_hash: journal.previous_hash().map(|hash| hash.as_str().to_owned()),
        proposed_hash: journal.proposed_hash().as_str().to_owned(),
    };
    let payload = serde_json::to_vec(&stored)
        .map_err(|_| storage_error("storage.operation_journal.encode.failed"))?;
    if payload.len() > MAX_REPO_CHANGE_JOURNAL_BYTES {
        return Err(storage_error("storage.operation_journal.limit"));
    }
    Ok(payload)
}

impl OperationJournalPort for SqliteStore {
    fn record_operation(&self, journal: &RepoChangeJournal) -> AppResult<()> {
        if journal.state() != RepoChangeJournalState::Planned {
            return Err(AppError::Validation(vec![
                jameskills_core::Diagnostic::error(
                    "repo.change.journal.state.invalid",
                    "A new repository change journal must begin in Planned state.",
                ),
            ]));
        }
        let payload = encode_repo_change_journal(journal)?;
        let operation_id = journal.operation_id().as_uuid().to_string();
        let state = journal.state().as_str();
        let updated_at = journal.updated_at();
        self.with_transaction(|transaction| {
            transaction
                .execute(
                    "INSERT INTO operations(id,kind,state,journal_json,updated_at) VALUES(?1,'repo-change',?2,?3,?4)",
                    rusqlite::params![operation_id, state, payload, updated_at],
                )
                .map_err(map_journal_insert_error)?;
            Ok(())
        })
    }

    fn load_operation(&self, operation_id: OperationId) -> AppResult<Option<RepoChangeJournal>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| storage_error("storage.lock.poisoned"))?;
        let row = connection
            .query_row(
                "SELECT state,updated_at,journal_json FROM operations WHERE id=?1 AND kind='repo-change'",
                [operation_id.as_uuid().to_string()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Vec<u8>>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(|_| storage_error("storage.operation_journal.read.failed"))?;
        row.map(|(state, updated_at, payload)| {
            decode_repo_change_journal(operation_id, &state, &updated_at, &payload)
        })
        .transpose()
    }

    fn transition_operation(
        &self,
        operation_id: OperationId,
        expected: RepoChangeJournalState,
        next: RepoChangeJournalState,
        updated_at: &str,
    ) -> AppResult<()> {
        if !expected.allows_transition(next)
            || updated_at.is_empty()
            || updated_at.len() > 64
            || updated_at.chars().any(char::is_control)
        {
            return Err(AppError::Validation(vec![
                jameskills_core::Diagnostic::error(
                    "repo.change.journal.transition.invalid",
                    "Repository change journal transition is invalid.",
                ),
            ]));
        }
        self.with_transaction(|transaction| {
            let changed = transaction
                .execute(
                    "UPDATE operations SET state=?1,updated_at=?2 WHERE id=?3 AND kind='repo-change' AND state=?4",
                    rusqlite::params![
                        next.as_str(),
                        updated_at,
                        operation_id.as_uuid().to_string(),
                        expected.as_str(),
                    ],
                )
                .map_err(|_| storage_error("storage.operation_journal.write.failed"))?;
            if changed != 1 {
                return Err(AppError::Conflict { current: vec![] });
            }
            Ok(())
        })
    }

    fn pending_operations(&self) -> AppResult<Vec<RepoChangeJournal>> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| storage_error("storage.lock.poisoned"))?;
        let mut statement = connection
            .prepare(
                "SELECT id,state,updated_at,journal_json FROM operations WHERE kind='repo-change' AND state NOT IN ('committed','failed','recovered') ORDER BY updated_at,id LIMIT ?1",
            )
            .map_err(|_| storage_error("storage.operation_journal.read.failed"))?;
        let rows = statement
            .query_map([MAX_PENDING_REPO_CHANGE_JOURNALS as i64 + 1], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                ))
            })
            .map_err(|_| storage_error("storage.operation_journal.read.failed"))?;
        let rows = rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| storage_error("storage.operation_journal.read.failed"))?;
        if rows.len() > MAX_PENDING_REPO_CHANGE_JOURNALS {
            return Err(storage_error("storage.operation_journal.limit"));
        }
        rows.into_iter()
            .map(|(id, state, updated_at, payload)| {
                let operation_id =
                    OperationId::parse(&id).map_err(|_| storage_error("storage.data.corrupt"))?;
                decode_repo_change_journal(operation_id, &state, &updated_at, &payload)
            })
            .collect()
    }
}

fn decode_repo_change_journal(
    operation_id: OperationId,
    state: &str,
    updated_at: &str,
    payload: &[u8],
) -> AppResult<RepoChangeJournal> {
    if payload.len() > MAX_REPO_CHANGE_JOURNAL_BYTES {
        return Err(storage_error("storage.data.corrupt"));
    }
    let state = RepoChangeJournalState::parse(state)
        .ok_or_else(|| storage_error("storage.data.corrupt"))?;
    let stored = serde_json::from_slice::<StoredRepoChangeJournal>(payload)
        .map_err(|_| storage_error("storage.data.corrupt"))?;
    if stored.schema_version != 1
        || OperationId::parse(&stored.operation_id).ok() != Some(operation_id)
    {
        return Err(storage_error("storage.data.corrupt"));
    }
    let root = ApprovedRoot::from_absolute_path(PathBuf::from(stored.root_path))
        .map_err(|_| storage_error("storage.data.corrupt"))?;
    let root_fingerprint = ContentHash::parse_hex(&stored.root_fingerprint)
        .map_err(|_| storage_error("storage.data.corrupt"))?;
    let expected_head = RepositoryHead::parse(&stored.expected_head)
        .map_err(|_| storage_error("storage.data.corrupt"))?;
    let target =
        PortablePath::new(stored.target).map_err(|_| storage_error("storage.data.corrupt"))?;
    let staging_target = PortablePath::new(stored.staging_target)
        .map_err(|_| storage_error("storage.data.corrupt"))?;
    let previous_hash = stored
        .previous_hash
        .map(|value| ContentHash::parse_hex(&value))
        .transpose()
        .map_err(|_| storage_error("storage.data.corrupt"))?;
    let proposed_hash = ContentHash::parse_hex(&stored.proposed_hash)
        .map_err(|_| storage_error("storage.data.corrupt"))?;
    RepoChangeJournal::from_storage_parts(
        operation_id,
        state,
        root,
        root_fingerprint,
        expected_head,
        target,
        staging_target,
        previous_hash,
        proposed_hash,
        updated_at,
    )
    .map_err(|_| storage_error("storage.data.corrupt"))
}

fn map_journal_insert_error(error: rusqlite::Error) -> AppError {
    match error {
        rusqlite::Error::SqliteFailure(code, _) if code.code == ErrorCode::ConstraintViolation => {
            AppError::Conflict { current: vec![] }
        }
        _ => storage_error("storage.operation_journal.write.failed"),
    }
}

fn read_user_version(connection: &Connection) -> AppResult<u32> {
    connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|_| storage_error("storage.open.failed"))
}

fn set_user_version(connection: &Connection, version: u32) -> AppResult<()> {
    connection
        .execute_batch(&format!("PRAGMA user_version = {version}"))
        .map_err(|_| storage_error("storage.migrate.failed"))
}

/// Copies the database file next to itself before a destructive upgrade.
/// Same directory means same filesystem; the source is never renamed away.
fn backup_before_upgrade(path: &Path, from: u32) -> AppResult<PathBuf> {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .map_err(|_| storage_error("storage.backup.failed"))?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| storage_error("storage.backup.failed"))?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let backup = parent.join(format!("{file_name}.backup-v{from}-{seconds}.sqlite3"));
    std::fs::copy(path, &backup).map_err(|_| storage_error("storage.backup.failed"))?;
    Ok(backup)
}
