use jameskills_core::{
    AppError, AppResult,
    domain::{
        BACKUP_NONCE_LEN, BACKUP_SALT_LEN, BACKUP_WRAPPED_MASTER_LEN, BackupHeaderV1,
        EncryptedSnapshot, SnapshotId, UnlockedVault, VaultId, WrappingKey,
    },
    ports::{CryptoPort, SecretInput},
};
use uuid::Uuid;
use zeroize::{ZeroizeOnDrop, Zeroizing};

fn header(ciphertext_len: u64) -> BackupHeaderV1 {
    BackupHeaderV1::new(
        [0x11; BACKUP_SALT_LEN],
        VaultId::from_uuid(Uuid::from_bytes([0x22; 16])),
        SnapshotId::from_uuid(Uuid::from_bytes([0x33; 16])),
        [0x44; BACKUP_NONCE_LEN],
        [0x55; BACKUP_NONCE_LEN],
        [0x66; BACKUP_WRAPPED_MASTER_LEN],
        ciphertext_len,
    )
    .unwrap()
}

#[test]
fn encrypted_snapshot_requires_exact_header_ciphertext_length() {
    let encrypted = EncryptedSnapshot::new(header(16), vec![0xA5; 16]).unwrap();
    assert_eq!(encrypted.header().ciphertext_len(), 16);
    assert_eq!(encrypted.ciphertext(), &[0xA5; 16]);

    assert!(EncryptedSnapshot::new(header(16), vec![0xA5; 15]).is_err());
}

#[test]
fn unlocked_vault_keeps_key_material_opaque_and_zeroized_on_drop() {
    fn assert_zeroize_on_drop<T: ZeroizeOnDrop>() {}
    fn assert_object_safe(_: &dyn CryptoPort) {}

    let vault_id = VaultId::from_uuid(Uuid::from_bytes([0x22; 16]));
    let vault = UnlockedVault::from_crypto_material(
        vault_id,
        Zeroizing::new([0xA5; 32]),
        WrappingKey::from_kdf_output([0x5A; 32]),
        [0x11; BACKUP_SALT_LEN],
    );

    assert_eq!(vault.vault_id(), vault_id);
    assert_eq!(vault.master_key(), &[0xA5; 32]);
    assert_eq!(vault.wrapping_key().expose_secret(), &[0x5A; 32]);
    assert_eq!(vault.salt(), &[0x11; BACKUP_SALT_LEN]);
    assert_zeroize_on_drop::<UnlockedVault>();
    let _object_safe_assertion = assert_object_safe;
}

struct KdfOnly;

impl CryptoPort for KdfOnly {
    fn derive_wrapping_key(
        &self,
        _password: &SecretInput,
        _salt: &[u8; BACKUP_SALT_LEN],
    ) -> AppResult<WrappingKey> {
        Err(AppError::CryptoInvalid)
    }
}

#[test]
fn snapshot_operations_without_a_provider_fail_closed() {
    let password = SecretInput::new(b"test-only".to_vec()).unwrap();
    assert!(matches!(
        KdfOnly.create_vault(&password),
        Err(AppError::CapabilityUnavailable { .. })
    ));
}
