use super::filesystem::BundleFiles;
use crate::{AppResult, domain::ImportScanStatus};

/// Optional, read-only secret scan over already validated import bytes.
/// Implementations return only a redacted status and never mutate trust.
#[async_trait::async_trait]
pub trait ImportScanPort: Send + Sync {
    async fn scan(&self, files: &BundleFiles) -> AppResult<ImportScanStatus>;
}
