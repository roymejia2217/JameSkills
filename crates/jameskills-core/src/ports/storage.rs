use crate::AppResult;

/// Schema version applied by the storage actor. Migration files map one to
/// one onto versions: 001_library.sql is version 1, and so on.
pub const CURRENT_SCHEMA_VERSION: u32 = 3;

/// Persistent library storage seam. Only the operations the storage actor
/// implements today are exposed; snapshot merge and the remaining DTOs
/// arrive with their providers instead of as placeholders.
pub trait StoragePort: Send + Sync {
    /// Schema version recorded in the backing store.
    fn schema_version(&self) -> AppResult<u32>;
    /// Verifies the backing store is readable and internally consistent.
    fn check_integrity(&self) -> AppResult<()>;
}
