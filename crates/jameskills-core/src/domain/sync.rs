use super::{ContentHash, RevisionId, RevisionKind, RevisionRecord, SkillId};
use crate::{AppError, AppResult, Diagnostic};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt,
};
use uuid::Uuid;
use zeroize::{Zeroize, ZeroizeOnDrop};

pub const SNAPSHOT_SCHEMA_VERSION: u16 = 1;
pub const CANONICAL_HASH_VERSION: u16 = 1;
pub const MAX_SNAPSHOT_METADATA_ENTRIES: usize = 100_000;
pub const MAX_SNAPSHOT_PARENTS: usize = 100_000;
pub const MAX_SNAPSHOT_BUNDLE_BYTES: usize = 22 * 1024 * 1024;
pub const MAX_SNAPSHOT_ARCHIVES_BYTES: usize = 256 * 1024 * 1024;
pub const BACKUP_HEADER_V1_LEN: usize = 176;
pub const BACKUP_HEADER_V1_VERSION: u16 = 1;
pub const BACKUP_CIPHER_XCHACHA20_POLY1305: u8 = 1;
pub const BACKUP_KDF_ARGON2ID: u8 = 1;
pub const BACKUP_ARGON2_MEMORY_KIB: u32 = 65_536;
pub const BACKUP_ARGON2_ITERATIONS: u32 = 3;
pub const BACKUP_ARGON2_PARALLELISM: u32 = 1;
pub const BACKUP_SALT_LEN: usize = 16;
pub const BACKUP_NONCE_LEN: usize = 24;
pub const BACKUP_WRAPPED_MASTER_LEN: usize = 48;
pub const BACKUP_AEAD_TAG_LEN: u64 = 16;
pub const MAX_ENCRYPTED_PAYLOAD_BYTES: u64 = 256 * 1024 * 1024;
const MAX_SNAPSHOT_HEADS_PER_SKILL: usize = 128;
const MAX_SNAPSHOT_CREATED_AT_BYTES: usize = 64;
const MAX_SNAPSHOT_REVISION_VERSION_BYTES: usize = 128;

const BACKUP_MAGIC_V1: &[u8; 8] = b"JSKSBK01";

/// Strict fixed-layout v1 envelope header. It is not a serialized struct: the
/// byte codec below is the protocol authority and rejects noncanonical fields.
pub struct BackupHeaderV1 {
    salt: [u8; BACKUP_SALT_LEN],
    vault_id: VaultId,
    snapshot_id: SnapshotId,
    wrap_nonce: [u8; BACKUP_NONCE_LEN],
    payload_nonce: [u8; BACKUP_NONCE_LEN],
    wrapped_master: [u8; BACKUP_WRAPPED_MASTER_LEN],
    ciphertext_len: u64,
}

/// Ciphertext paired with the strict v1 header. The header length is checked
/// against the supplied ciphertext before the pair can enter remote storage.
pub struct EncryptedSnapshot {
    header: BackupHeaderV1,
    ciphertext: Vec<u8>,
}

impl EncryptedSnapshot {
    pub fn new(header: BackupHeaderV1, ciphertext: Vec<u8>) -> AppResult<Self> {
        let ciphertext_len = u64::try_from(ciphertext.len())
            .map_err(|_| crypto_header_error("crypto.snapshot.ciphertext_length.invalid"))?;
        if ciphertext_len != header.ciphertext_len() {
            return Err(crypto_header_error(
                "crypto.snapshot.ciphertext_length.mismatch",
            ));
        }
        Ok(Self { header, ciphertext })
    }

    pub fn header(&self) -> &BackupHeaderV1 {
        &self.header
    }

    pub fn ciphertext(&self) -> &[u8] {
        &self.ciphertext
    }
}

/// In-memory unlocked key material. It is deliberately not serializable,
/// clonable, or printable; both keys are erased when this value is dropped.
pub struct UnlockedVault {
    vault_id: VaultId,
    master_key: zeroize::Zeroizing<[u8; 32]>,
    wrapping_key: WrappingKey,
    salt: [u8; BACKUP_SALT_LEN],
}

impl UnlockedVault {
    /// Provider construction hook; callers must only pass freshly generated or
    /// successfully unwrapped key material.
    #[doc(hidden)]
    pub fn from_crypto_material(
        vault_id: VaultId,
        master_key: zeroize::Zeroizing<[u8; 32]>,
        wrapping_key: WrappingKey,
        salt: [u8; BACKUP_SALT_LEN],
    ) -> Self {
        Self {
            vault_id,
            master_key,
            wrapping_key,
            salt,
        }
    }

    pub fn vault_id(&self) -> VaultId {
        self.vault_id
    }

    pub fn master_key(&self) -> &[u8; 32] {
        &self.master_key
    }

    pub fn wrapping_key(&self) -> &WrappingKey {
        &self.wrapping_key
    }

    pub fn salt(&self) -> &[u8; BACKUP_SALT_LEN] {
        &self.salt
    }
}

impl ZeroizeOnDrop for UnlockedVault {}

impl BackupHeaderV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        salt: [u8; BACKUP_SALT_LEN],
        vault_id: VaultId,
        snapshot_id: SnapshotId,
        wrap_nonce: [u8; BACKUP_NONCE_LEN],
        payload_nonce: [u8; BACKUP_NONCE_LEN],
        wrapped_master: [u8; BACKUP_WRAPPED_MASTER_LEN],
        ciphertext_len: u64,
    ) -> AppResult<Self> {
        validate_ciphertext_len(ciphertext_len)?;
        Ok(Self {
            salt,
            vault_id,
            snapshot_id,
            wrap_nonce,
            payload_nonce,
            wrapped_master,
            ciphertext_len,
        })
    }

    /// Parses exactly the fixed 176-byte prefix and validates all attacker-
    /// controlled KDF parameters/lengths before a caller may derive a key.
    pub fn parse(bytes: &[u8]) -> AppResult<Self> {
        if bytes.len() != BACKUP_HEADER_V1_LEN {
            return Err(crypto_header_error("crypto.header.length.invalid"));
        }
        if bytes.get(0..8) != Some(BACKUP_MAGIC_V1.as_slice())
            || read_u16(bytes, 8)? != BACKUP_HEADER_V1_VERSION
            || bytes[10] != BACKUP_CIPHER_XCHACHA20_POLY1305
            || bytes[11] != BACKUP_KDF_ARGON2ID
            || read_u32(bytes, 12)? != BACKUP_ARGON2_MEMORY_KIB
            || read_u32(bytes, 16)? != BACKUP_ARGON2_ITERATIONS
            || read_u32(bytes, 20)? != BACKUP_ARGON2_PARALLELISM
        {
            return Err(crypto_header_error("crypto.header.parameters.unsupported"));
        }
        let salt = read_array::<BACKUP_SALT_LEN>(bytes, 24)?;
        let vault_id = VaultId::from_uuid(uuid::Uuid::from_bytes(read_array::<16>(bytes, 40)?));
        let snapshot_id =
            SnapshotId::from_uuid(uuid::Uuid::from_bytes(read_array::<16>(bytes, 56)?));
        let wrap_nonce = read_array::<BACKUP_NONCE_LEN>(bytes, 72)?;
        let payload_nonce = read_array::<BACKUP_NONCE_LEN>(bytes, 96)?;
        let wrapped_master = read_array::<BACKUP_WRAPPED_MASTER_LEN>(bytes, 120)?;
        let ciphertext_len = read_u64(bytes, 168)?;
        Self::new(
            salt,
            vault_id,
            snapshot_id,
            wrap_nonce,
            payload_nonce,
            wrapped_master,
            ciphertext_len,
        )
    }

    pub fn encode(&self) -> [u8; BACKUP_HEADER_V1_LEN] {
        let mut bytes = [0u8; BACKUP_HEADER_V1_LEN];
        bytes[0..8].copy_from_slice(BACKUP_MAGIC_V1);
        bytes[8..10].copy_from_slice(&BACKUP_HEADER_V1_VERSION.to_be_bytes());
        bytes[10] = BACKUP_CIPHER_XCHACHA20_POLY1305;
        bytes[11] = BACKUP_KDF_ARGON2ID;
        bytes[12..16].copy_from_slice(&BACKUP_ARGON2_MEMORY_KIB.to_be_bytes());
        bytes[16..20].copy_from_slice(&BACKUP_ARGON2_ITERATIONS.to_be_bytes());
        bytes[20..24].copy_from_slice(&BACKUP_ARGON2_PARALLELISM.to_be_bytes());
        bytes[24..40].copy_from_slice(&self.salt);
        bytes[40..56].copy_from_slice(self.vault_id.as_uuid().as_bytes());
        bytes[56..72].copy_from_slice(self.snapshot_id.as_uuid().as_bytes());
        bytes[72..96].copy_from_slice(&self.wrap_nonce);
        bytes[96..120].copy_from_slice(&self.payload_nonce);
        bytes[120..168].copy_from_slice(&self.wrapped_master);
        bytes[168..176].copy_from_slice(&self.ciphertext_len.to_be_bytes());
        bytes
    }

    /// Wrap AAD is exactly header bytes 0..120 followed by 168..176.
    pub fn wrap_aad(&self) -> [u8; 128] {
        let header = self.encode();
        let mut aad = [0u8; 128];
        aad[..120].copy_from_slice(&header[..120]);
        aad[120..].copy_from_slice(&header[168..176]);
        aad
    }

    /// Payload AAD is the complete canonical header.
    pub fn payload_aad(&self) -> [u8; BACKUP_HEADER_V1_LEN] {
        self.encode()
    }

    pub fn salt(&self) -> &[u8; BACKUP_SALT_LEN] {
        &self.salt
    }

    pub fn vault_id(&self) -> VaultId {
        self.vault_id
    }

    pub fn snapshot_id(&self) -> SnapshotId {
        self.snapshot_id
    }

    pub fn wrap_nonce(&self) -> &[u8; BACKUP_NONCE_LEN] {
        &self.wrap_nonce
    }

    pub fn payload_nonce(&self) -> &[u8; BACKUP_NONCE_LEN] {
        &self.payload_nonce
    }

    pub fn wrapped_master(&self) -> &[u8; BACKUP_WRAPPED_MASTER_LEN] {
        &self.wrapped_master
    }

    pub fn ciphertext_len(&self) -> u64 {
        self.ciphertext_len
    }
}

fn validate_ciphertext_len(length: u64) -> AppResult<()> {
    if !(BACKUP_AEAD_TAG_LEN..=MAX_ENCRYPTED_PAYLOAD_BYTES).contains(&length) {
        return Err(crypto_header_error(
            "crypto.header.ciphertext_length.invalid",
        ));
    }
    Ok(())
}

fn read_array<const N: usize>(bytes: &[u8], offset: usize) -> AppResult<[u8; N]> {
    let slice = bytes
        .get(offset..offset + N)
        .ok_or_else(|| crypto_header_error("crypto.header.truncated"))?;
    let mut value = [0u8; N];
    value.copy_from_slice(slice);
    Ok(value)
}

fn read_u16(bytes: &[u8], offset: usize) -> AppResult<u16> {
    Ok(u16::from_be_bytes(read_array::<2>(bytes, offset)?))
}

fn read_u32(bytes: &[u8], offset: usize) -> AppResult<u32> {
    Ok(u32::from_be_bytes(read_array::<4>(bytes, offset)?))
}

fn read_u64(bytes: &[u8], offset: usize) -> AppResult<u64> {
    Ok(u64::from_be_bytes(read_array::<8>(bytes, offset)?))
}

fn crypto_header_error(code: &'static str) -> AppError {
    AppError::Validation(vec![Diagnostic::error(
        code,
        "Encrypted snapshot header is invalid or unsupported.",
    )])
}

macro_rules! uuid_id {
    ($name:ident) => {
        #[derive(
            Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            #[allow(clippy::new_without_default)]
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            pub const fn from_uuid(value: Uuid) -> Self {
                Self(value)
            }

            pub const fn as_uuid(self) -> Uuid {
                self.0
            }
        }
    };
}

uuid_id!(VaultId);
uuid_id!(SnapshotId);
uuid_id!(DeviceId);

impl VaultId {
    /// Builds an RFC 4122 version-4 identifier from caller-provided random bytes.
    /// Randomness acquisition stays fallible and belongs to the infrastructure.
    pub fn from_random_bytes(mut bytes: [u8; 16]) -> Self {
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        Self(Uuid::from_bytes(bytes))
    }
}

/// A key returned by the configured KDF. The bytes are borrowed only by the
/// crypto provider and are erased on Drop.
pub struct WrappingKey([u8; 32]);

impl WrappingKey {
    pub fn from_kdf_output(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn expose_secret(&self) -> &[u8; 32] {
        &self.0
    }
}

impl Drop for WrappingKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl ZeroizeOnDrop for WrappingKey {}

impl fmt::Debug for WrappingKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SnapshotRevisionKind {
    Content,
    Tombstone { observed_heads: Vec<RevisionId> },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotRevision {
    revision_id: RevisionId,
    skill_id: SkillId,
    parents: Vec<RevisionId>,
    kind: SnapshotRevisionKind,
    bundle_hash: Option<ContentHash>,
    semantic_version: String,
}

impl SnapshotRevision {
    pub fn new(
        revision_id: RevisionId,
        skill_id: SkillId,
        parents: Vec<RevisionId>,
        kind: SnapshotRevisionKind,
        bundle_hash: Option<ContentHash>,
        semantic_version: String,
    ) -> Self {
        Self {
            revision_id,
            skill_id,
            parents,
            kind,
            bundle_hash,
            semantic_version,
        }
    }

    pub fn from_record(record: &RevisionRecord) -> Self {
        let (kind, bundle_hash) = match record.kind() {
            RevisionKind::Content => (
                SnapshotRevisionKind::Content,
                Some(record.bundle_hash().clone()),
            ),
            RevisionKind::Tombstone { observed_heads } => (
                SnapshotRevisionKind::Tombstone {
                    observed_heads: observed_heads.clone(),
                },
                None,
            ),
        };
        Self::new(
            record.id().clone(),
            record.skill_id(),
            record.parents().to_vec(),
            kind,
            bundle_hash,
            record.semantic_version().to_owned(),
        )
    }

    pub fn revision_id(&self) -> &RevisionId {
        &self.revision_id
    }

    pub fn skill_id(&self) -> SkillId {
        self.skill_id
    }

    pub fn parents(&self) -> &[RevisionId] {
        &self.parents
    }

    pub fn kind(&self) -> &SnapshotRevisionKind {
        &self.kind
    }

    pub fn bundle_hash(&self) -> Option<&ContentHash> {
        self.bundle_hash.as_ref()
    }

    pub fn semantic_version(&self) -> &str {
        &self.semantic_version
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotHead {
    skill_id: SkillId,
    revision_ids: Vec<RevisionId>,
}

impl SnapshotHead {
    pub fn new(skill_id: SkillId, mut revision_ids: Vec<RevisionId>) -> AppResult<Self> {
        if revision_ids.is_empty() || revision_ids.len() > MAX_SNAPSHOT_HEADS_PER_SKILL {
            return Err(snapshot_error("sync.snapshot.heads.invalid"));
        }
        revision_ids.sort();
        if revision_ids.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(snapshot_error("sync.snapshot.heads.duplicate"));
        }
        Ok(Self {
            skill_id,
            revision_ids,
        })
    }

    pub fn skill_id(&self) -> SkillId {
        self.skill_id
    }

    pub fn revision_ids(&self) -> &[RevisionId] {
        &self.revision_ids
    }
}

/// Portable bundle bytes addressed by their canonical content hash. No local
/// file path or trust/evidence metadata is part of this transport DTO.
pub struct SnapshotBundle {
    content_hash: ContentHash,
    archive_bytes: Vec<u8>,
}

impl SnapshotBundle {
    pub fn new(content_hash: ContentHash, archive_bytes: Vec<u8>) -> AppResult<Self> {
        if archive_bytes.is_empty() || archive_bytes.len() > MAX_SNAPSHOT_BUNDLE_BYTES {
            return Err(snapshot_error("sync.snapshot.bundle.bytes.invalid"));
        }
        Ok(Self {
            content_hash,
            archive_bytes,
        })
    }

    pub fn content_hash(&self) -> &ContentHash {
        &self.content_hash
    }

    pub fn archive_bytes(&self) -> &[u8] {
        &self.archive_bytes
    }
}

/// In-memory candidate for one immutable encrypted snapshot. Constructor bounds
/// resource use and canonical ordering; T058 validates graph/content relations.
pub struct SnapshotPayload {
    vault_id: VaultId,
    snapshot_id: SnapshotId,
    parents: Vec<SnapshotId>,
    device_id: DeviceId,
    library_generation: u64,
    created_at: String,
    schema_version: u16,
    canonical_hash_version: u16,
    revisions: Vec<SnapshotRevision>,
    heads: Vec<SnapshotHead>,
    bundles: Vec<SnapshotBundle>,
}

impl SnapshotPayload {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        vault_id: VaultId,
        snapshot_id: SnapshotId,
        mut parents: Vec<SnapshotId>,
        device_id: DeviceId,
        library_generation: u64,
        created_at: String,
        schema_version: u16,
        canonical_hash_version: u16,
        mut revisions: Vec<SnapshotRevision>,
        mut heads: Vec<SnapshotHead>,
        mut bundles: Vec<SnapshotBundle>,
    ) -> AppResult<Self> {
        let relation_count = revisions.iter().try_fold(parents.len(), |count, revision| {
            if revision.parents.len() > MAX_SNAPSHOT_HEADS_PER_SKILL
                || revision.semantic_version.len() > MAX_SNAPSHOT_REVISION_VERSION_BYTES
            {
                return None;
            }
            let observed_count = match &revision.kind {
                SnapshotRevisionKind::Content => 0,
                SnapshotRevisionKind::Tombstone { observed_heads } => {
                    if observed_heads.len() > MAX_SNAPSHOT_HEADS_PER_SKILL {
                        return None;
                    }
                    observed_heads.len()
                }
            };
            count
                .checked_add(revision.parents.len())
                .and_then(|count| count.checked_add(observed_count))
        });
        let relation_count = relation_count.and_then(|count| {
            heads.iter().try_fold(count, |count, head| {
                count.checked_add(head.revision_ids.len())
            })
        });
        let entry_count = relation_count
            .and_then(|count| count.checked_add(revisions.len()))
            .and_then(|count| count.checked_add(heads.len()))
            .and_then(|count| count.checked_add(bundles.len()));
        let archive_bytes = bundles.iter().try_fold(0usize, |total, bundle| {
            total.checked_add(bundle.archive_bytes.len())
        });
        if schema_version != SNAPSHOT_SCHEMA_VERSION
            || canonical_hash_version != CANONICAL_HASH_VERSION
            || created_at.is_empty()
            || created_at.len() > MAX_SNAPSHOT_CREATED_AT_BYTES
            || created_at.chars().any(char::is_control)
            || parents.len() > MAX_SNAPSHOT_PARENTS
            || entry_count.is_none_or(|count| count > MAX_SNAPSHOT_METADATA_ENTRIES)
            || archive_bytes.is_none_or(|count| count > MAX_SNAPSHOT_ARCHIVES_BYTES)
        {
            return Err(snapshot_error("sync.snapshot.limits.invalid"));
        }
        parents.sort();
        if parents.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(snapshot_error("sync.snapshot.parents.duplicate"));
        }
        revisions.sort_by(|left, right| left.revision_id.cmp(&right.revision_id));
        if revisions
            .windows(2)
            .any(|pair| pair[0].revision_id == pair[1].revision_id)
        {
            return Err(snapshot_error("sync.snapshot.revisions.duplicate"));
        }
        heads.sort_by_key(|head| head.skill_id.as_uuid());
        if heads
            .windows(2)
            .any(|pair| pair[0].skill_id == pair[1].skill_id)
        {
            return Err(snapshot_error("sync.snapshot.heads.skill.duplicate"));
        }
        bundles.sort_by(|left, right| left.content_hash.cmp(&right.content_hash));
        if bundles
            .windows(2)
            .any(|pair| pair[0].content_hash == pair[1].content_hash)
        {
            return Err(snapshot_error("sync.snapshot.bundles.duplicate"));
        }
        Ok(Self {
            vault_id,
            snapshot_id,
            parents,
            device_id,
            library_generation,
            created_at,
            schema_version,
            canonical_hash_version,
            revisions,
            heads,
            bundles,
        })
    }

    pub fn vault_id(&self) -> VaultId {
        self.vault_id
    }

    pub fn snapshot_id(&self) -> SnapshotId {
        self.snapshot_id
    }

    pub fn parents(&self) -> &[SnapshotId] {
        &self.parents
    }

    pub fn device_id(&self) -> DeviceId {
        self.device_id
    }

    pub fn library_generation(&self) -> u64 {
        self.library_generation
    }

    pub fn created_at(&self) -> &str {
        &self.created_at
    }

    pub fn schema_version(&self) -> u16 {
        self.schema_version
    }

    pub fn canonical_hash_version(&self) -> u16 {
        self.canonical_hash_version
    }

    pub fn revisions(&self) -> &[SnapshotRevision] {
        &self.revisions
    }

    pub fn heads(&self) -> &[SnapshotHead] {
        &self.heads
    }

    pub fn bundles(&self) -> &[SnapshotBundle] {
        &self.bundles
    }

    /// Validates all revision, head, and bundle references before creating the
    /// only DTO accepted by restore/merge consumers. Authentication is handled
    /// by CryptoPort before this method is called on decrypted input.
    pub fn validate(&self) -> AppResult<()> {
        let mut revisions = HashMap::with_capacity(self.revisions.len());
        for revision in &self.revisions {
            let revision_kind = match &revision.kind {
                SnapshotRevisionKind::Content => {
                    if revision.bundle_hash.is_none() {
                        return Err(snapshot_error("sync.snapshot.revision.bundle.missing"));
                    }
                    RevisionKind::Content
                }
                SnapshotRevisionKind::Tombstone { observed_heads } => {
                    if revision.bundle_hash.is_some() || observed_heads != &revision.parents {
                        return Err(snapshot_error("sync.snapshot.tombstone.parents.invalid"));
                    }
                    RevisionKind::Tombstone {
                        observed_heads: observed_heads.clone(),
                    }
                }
            };
            let record = RevisionRecord::new(
                revision.skill_id,
                revision.bundle_hash.clone(),
                revision.parents.clone(),
                revision_kind,
                revision.semantic_version.clone(),
            )?;
            if record.id() != &revision.revision_id || record.parents() != revision.parents {
                return Err(snapshot_error("sync.snapshot.revision.id.invalid"));
            }
            if revisions
                .insert(revision.revision_id.clone(), revision)
                .is_some()
            {
                return Err(snapshot_error("sync.snapshot.revision.duplicate"));
            }
        }

        let mut children = HashMap::<RevisionId, Vec<RevisionId>>::new();
        let mut referenced_parents = HashSet::<RevisionId>::new();
        let mut parent_counts = HashMap::<RevisionId, usize>::with_capacity(revisions.len());
        for revision in &self.revisions {
            parent_counts.insert(revision.revision_id.clone(), revision.parents.len());
            for parent in &revision.parents {
                let Some(parent_revision) = revisions.get(parent) else {
                    return Err(snapshot_error("sync.snapshot.parent.missing"));
                };
                if parent_revision.skill_id != revision.skill_id {
                    return Err(snapshot_error("sync.snapshot.parent.skill_mismatch"));
                }
                referenced_parents.insert(parent.clone());
                children
                    .entry(parent.clone())
                    .or_default()
                    .push(revision.revision_id.clone());
            }
        }
        let mut ready = parent_counts
            .iter()
            .filter_map(|(revision_id, count)| (*count == 0).then_some(revision_id.clone()))
            .collect::<VecDeque<_>>();
        let mut visited = 0usize;
        while let Some(parent) = ready.pop_front() {
            visited += 1;
            if let Some(descendants) = children.get(&parent) {
                for child in descendants {
                    let count = parent_counts
                        .get_mut(child)
                        .ok_or_else(|| snapshot_error("sync.snapshot.parent.missing"))?;
                    *count = count.saturating_sub(1);
                    if *count == 0 {
                        ready.push_back(child.clone());
                    }
                }
            }
        }
        if visited != revisions.len() {
            return Err(snapshot_error("sync.snapshot.revision.cycle"));
        }

        let mut bundle_metadata = HashMap::with_capacity(self.bundles.len());
        for bundle in &self.bundles {
            let inventory = crate::ports::validate_archive_entries(&bundle.archive_bytes)
                .map_err(AppError::Validation)?;
            let files = crate::ports::extract_archive_files(&bundle.archive_bytes, &inventory)
                .map_err(AppError::Validation)?;
            let validated = super::validate_bundle(&files).map_err(AppError::Validation)?;
            if validated.content_hash() != &bundle.content_hash {
                return Err(snapshot_error("sync.snapshot.bundle.hash_mismatch"));
            }
            if bundle_metadata
                .insert(
                    bundle.content_hash.clone(),
                    (
                        validated.manifest().id(),
                        validated.manifest().version().to_string(),
                    ),
                )
                .is_some()
            {
                return Err(snapshot_error("sync.snapshot.bundle.duplicate"));
            }
        }

        let mut referenced_bundles = HashSet::new();
        for revision in &self.revisions {
            if let Some(bundle_hash) = &revision.bundle_hash {
                let Some((skill_id, version)) = bundle_metadata.get(bundle_hash) else {
                    return Err(snapshot_error("sync.snapshot.bundle.missing"));
                };
                if *skill_id != revision.skill_id || version != &revision.semantic_version {
                    return Err(snapshot_error("sync.snapshot.bundle.revision_mismatch"));
                }
                referenced_bundles.insert(bundle_hash.clone());
            }
        }
        if referenced_bundles.len() != bundle_metadata.len() {
            return Err(snapshot_error("sync.snapshot.bundle.unreferenced"));
        }

        let mut actual_heads = HashMap::<SkillId, Vec<RevisionId>>::new();
        for head in &self.heads {
            for revision_id in &head.revision_ids {
                let Some(revision) = revisions.get(revision_id) else {
                    return Err(snapshot_error("sync.snapshot.head.revision.missing"));
                };
                if revision.skill_id != head.skill_id {
                    return Err(snapshot_error("sync.snapshot.head.skill_mismatch"));
                }
            }
            actual_heads.insert(head.skill_id, head.revision_ids.clone());
        }
        let mut expected_heads = HashMap::<SkillId, Vec<RevisionId>>::new();
        for revision in &self.revisions {
            if !referenced_parents.contains(&revision.revision_id) {
                expected_heads
                    .entry(revision.skill_id)
                    .or_default()
                    .push(revision.revision_id.clone());
            }
        }
        for heads in expected_heads.values_mut() {
            heads.sort();
        }
        if actual_heads != expected_heads {
            return Err(snapshot_error("sync.snapshot.heads.mismatch"));
        }
        Ok(())
    }

    pub fn verify(self) -> AppResult<VerifiedSnapshot> {
        self.validate()?;
        Ok(VerifiedSnapshot { payload: self })
    }
}

/// Authenticated and structurally validated payload. Construction is private;
/// consumers cannot turn an arbitrary decrypted/deserialized DTO into trust.
pub struct VerifiedSnapshot {
    payload: SnapshotPayload,
}

impl VerifiedSnapshot {
    pub fn payload(&self) -> &SnapshotPayload {
        &self.payload
    }

    pub fn into_payload(self) -> SnapshotPayload {
        self.payload
    }
}

fn snapshot_error(code: &'static str) -> AppError {
    AppError::Validation(vec![Diagnostic::error(
        code,
        "Snapshot DTO is invalid or exceeds bounded resource limits.",
    )])
}
