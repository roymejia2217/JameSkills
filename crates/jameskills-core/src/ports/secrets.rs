use crate::{AppError, AppResult, Diagnostic};
use std::fmt;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Maximum transient passphrase/keyring secret accepted by the core boundary.
pub const MAX_SECRET_INPUT_BYTES: usize = 1024;

/// Opaque secret bytes. It is intentionally non-cloneable, non-serializable,
/// and redacts its contents from all formatting.
pub struct SecretInput(Vec<u8>);

impl SecretInput {
    pub fn new(mut bytes: Vec<u8>) -> AppResult<Self> {
        if bytes.is_empty() || bytes.len() > MAX_SECRET_INPUT_BYTES {
            bytes.zeroize();
            return Err(AppError::Validation(vec![Diagnostic::error(
                "sync.secret_input.length.invalid",
                "Secret input must be between 1 and 1024 bytes.",
            )]));
        }
        Ok(Self(bytes))
    }

    /// Borrow the secret only for the duration of the cryptographic operation.
    pub fn expose_secret(&self) -> &[u8] {
        &self.0
    }
}

impl Drop for SecretInput {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl ZeroizeOnDrop for SecretInput {}

impl fmt::Debug for SecretInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}
