use crate::{
    AppError, AppResult,
    domain::{EncryptedSnapshot, SnapshotPayload, UnlockedVault, VerifiedSnapshot, WrappingKey},
    ports::SecretInput,
};

/// Synchronous CPU-bound cryptographic derivation. Desktop/CLI callers run it
/// off their UI thread; all v1 KDF parameters are provider-owned and fixed.
pub trait CryptoPort: Send + Sync {
    fn derive_wrapping_key(
        &self,
        password: &SecretInput,
        salt: &[u8; 16],
    ) -> AppResult<WrappingKey>;

    fn create_vault(&self, _password: &SecretInput) -> AppResult<UnlockedVault> {
        Err(snapshot_crypto_unavailable())
    }

    fn unlock_vault(
        &self,
        _data: &EncryptedSnapshot,
        _password: &SecretInput,
    ) -> AppResult<UnlockedVault> {
        Err(snapshot_crypto_unavailable())
    }

    fn open_with_vault(
        &self,
        _data: &EncryptedSnapshot,
        _key: &UnlockedVault,
    ) -> AppResult<VerifiedSnapshot> {
        Err(snapshot_crypto_unavailable())
    }

    fn seal(&self, _data: &SnapshotPayload, _key: &UnlockedVault) -> AppResult<EncryptedSnapshot> {
        Err(snapshot_crypto_unavailable())
    }

    fn open(
        &self,
        _data: &EncryptedSnapshot,
        _password: &SecretInput,
    ) -> AppResult<VerifiedSnapshot> {
        Err(snapshot_crypto_unavailable())
    }
}

fn snapshot_crypto_unavailable() -> AppError {
    AppError::CapabilityUnavailable {
        id: "crypto.snapshot.unsupported".to_owned(),
        guidance_id: "sync.crypto.snapshot.setup".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::CryptoPort;
    use crate::domain::WrappingKey;
    use zeroize::ZeroizeOnDrop;

    fn assert_object_safe(_: &dyn CryptoPort) {}
    fn assert_zeroize_on_drop<T: ZeroizeOnDrop>() {}

    #[test]
    fn wrapping_key_debug_is_redacted_and_drop_zeroizes() {
        let key = WrappingKey::from_kdf_output([0xA5; 32]);
        assert_eq!(key.expose_secret(), &[0xA5; 32]);
        assert_eq!(format!("{key:?}"), "[REDACTED]");
        assert_zeroize_on_drop::<WrappingKey>();
        let _object_safe_assertion = assert_object_safe;
    }
}
