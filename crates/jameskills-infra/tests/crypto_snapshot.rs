use jameskills_core::{
    AppError,
    domain::{
        BackupHeaderV1, CANONICAL_HASH_VERSION, CreateSkill, DeviceId, EncryptedSnapshot,
        RevisionKind, RevisionRecord, SNAPSHOT_SCHEMA_VERSION, SnapshotBundle, SnapshotHead,
        SnapshotId, SnapshotPayload, SnapshotRevision, UnlockedVault, VaultId, WrappingKey,
        validate_bundle,
    },
    ports::{CryptoPort, SecretInput, write_bundle_archive},
};
use jameskills_infra::crypto::CryptoProvider;
use zeroize::Zeroizing;

fn payload(vault_id: VaultId) -> SnapshotPayload {
    let draft = CreateSkill::new(
        "portable-snapshot".to_owned(),
        "Portable snapshot".to_owned(),
    )
    .unwrap()
    .initial_draft()
    .unwrap();
    let validated = validate_bundle(draft.files()).unwrap();
    let archive = write_bundle_archive(draft.files()).unwrap();
    let record = RevisionRecord::new(
        draft.skill_id(),
        Some(validated.content_hash().clone()),
        Vec::new(),
        RevisionKind::Content,
        "0.1.0".to_owned(),
    )
    .unwrap();
    SnapshotPayload::new(
        vault_id,
        SnapshotId::new(),
        Vec::new(),
        DeviceId::new(),
        4,
        "2026-10-08T00:00:00Z".to_owned(),
        SNAPSHOT_SCHEMA_VERSION,
        CANONICAL_HASH_VERSION,
        vec![SnapshotRevision::from_record(&record)],
        vec![SnapshotHead::new(draft.skill_id(), vec![record.id().clone()]).unwrap()],
        vec![SnapshotBundle::new(validated.content_hash().clone(), archive).unwrap()],
    )
    .unwrap()
}

fn test_vault(provider: &CryptoProvider) -> (SecretInput, UnlockedVault) {
    let password = SecretInput::new(b"correct horse battery staple".to_vec()).unwrap();
    let salt = [0x31; 16];
    let wrapping_key = provider.derive_wrapping_key(&password, &salt).unwrap();
    let vault_id = VaultId::from_random_bytes([0x52; 16]);
    let vault = UnlockedVault::from_crypto_material(
        vault_id,
        Zeroizing::new([0xA7; 32]),
        wrapping_key,
        salt,
    );
    (password, vault)
}

#[test]
fn vault_wraps_and_authenticates_snapshot_before_decode() {
    let provider = CryptoProvider::new();
    let password = SecretInput::new(b"correct horse battery staple".to_vec()).unwrap();
    let vault = provider.create_vault(&password).unwrap();
    let payload = payload(vault.vault_id());
    let snapshot_id = payload.snapshot_id();
    assert_eq!(
        jameskills_infra::snapshot_archive::encode(&payload).unwrap(),
        jameskills_infra::snapshot_archive::encode(&payload).unwrap()
    );
    let encrypted = provider.seal(&payload, &vault).unwrap();

    assert_eq!(encrypted.header().vault_id(), vault.vault_id());
    assert_eq!(encrypted.header().snapshot_id(), snapshot_id);
    let verified = provider.open_with_vault(&encrypted, &vault).unwrap();
    assert_eq!(verified.payload().snapshot_id(), snapshot_id);

    let unlocked = provider.unlock_vault(&encrypted, &password).unwrap();
    assert_eq!(
        provider
            .open_with_vault(&encrypted, &unlocked)
            .unwrap()
            .payload()
            .snapshot_id(),
        snapshot_id
    );

    let wrong_password = SecretInput::new(b"wrong password".to_vec()).unwrap();
    assert!(matches!(
        provider.open(&encrypted, &wrong_password),
        Err(AppError::CryptoInvalid)
    ));

    let mut tampered_ciphertext = encrypted.ciphertext().to_vec();
    tampered_ciphertext[0] ^= 1;
    let tampered =
        EncryptedSnapshot::new(copy_header(encrypted.header()), tampered_ciphertext).unwrap();
    assert!(matches!(
        provider.open_with_vault(&tampered, &vault),
        Err(AppError::CryptoInvalid)
    ));
}

#[test]
fn sealing_uses_fresh_nonces_and_rejects_cross_vault_payloads() {
    let provider = CryptoProvider::new();
    let (_password, vault) = test_vault(&provider);
    let payload = payload(vault.vault_id());
    let first = provider.seal(&payload, &vault).unwrap();
    let second = provider.seal(&payload, &vault).unwrap();

    assert_ne!(first.header().wrap_nonce(), second.header().wrap_nonce());
    assert_ne!(
        first.header().payload_nonce(),
        second.header().payload_nonce()
    );
    assert_ne!(first.ciphertext(), second.ciphertext());

    let other_vault = UnlockedVault::from_crypto_material(
        VaultId::from_random_bytes([0x61; 16]),
        Zeroizing::new([0xA7; 32]),
        WrappingKey::from_kdf_output([0x31; 32]),
        [0x31; 16],
    );
    assert!(matches!(
        provider.seal(&payload, &other_vault),
        Err(AppError::CryptoInvalid)
    ));
    let wrong_master = UnlockedVault::from_crypto_material(
        vault.vault_id(),
        Zeroizing::new([0xA6; 32]),
        WrappingKey::from_kdf_output([0x32; 32]),
        *vault.salt(),
    );
    assert!(matches!(
        provider.open_with_vault(&first, &wrong_master),
        Err(AppError::CryptoInvalid)
    ));
}

fn copy_header(header: &BackupHeaderV1) -> BackupHeaderV1 {
    BackupHeaderV1::new(
        *header.salt(),
        header.vault_id(),
        header.snapshot_id(),
        *header.wrap_nonce(),
        *header.payload_nonce(),
        *header.wrapped_master(),
        header.ciphertext_len(),
    )
    .unwrap()
}

fn envelope_bytes(snapshot: &EncryptedSnapshot) -> Vec<u8> {
    let mut bytes = snapshot.header().encode().to_vec();
    bytes.extend_from_slice(snapshot.ciphertext());
    bytes
}

#[test]
fn header_field_tampering_is_rejected_and_retries_keep_the_original_bytes() {
    let provider = CryptoProvider::new();
    let (password, vault) = test_vault(&provider);
    let payload = payload(vault.vault_id());
    let encrypted = provider.seal(&payload, &vault).unwrap();

    for offset in [0usize, 8, 10, 11, 12, 16, 20] {
        let mut encoded = encrypted.header().encode();
        encoded[offset] ^= 1;
        assert!(BackupHeaderV1::parse(&encoded).is_err(), "offset {offset}");
    }

    for offset in [24usize, 40, 56, 72, 96, 120, 168] {
        let mut encoded = encrypted.header().encode();
        let mut ciphertext = encrypted.ciphertext().to_vec();
        if offset == 168 {
            let shortened_length = encrypted.header().ciphertext_len() - 1;
            encoded[168..176].copy_from_slice(&shortened_length.to_be_bytes());
            ciphertext.truncate(usize::try_from(shortened_length).unwrap());
        } else {
            encoded[offset] ^= 1;
        }
        let changed_header = BackupHeaderV1::parse(&encoded).unwrap();
        let tampered = EncryptedSnapshot::new(changed_header, ciphertext).unwrap();
        assert!(
            provider.open_with_vault(&tampered, &vault).is_err(),
            "authenticated header offset {offset}"
        );
    }

    let before_retry = envelope_bytes(&encrypted);
    let wrong_password = SecretInput::new(b"incorrect".to_vec()).unwrap();
    assert!(provider.open(&encrypted, &wrong_password).is_err());
    assert_eq!(envelope_bytes(&encrypted), before_retry);
    assert!(provider.open(&encrypted, &password).is_ok());
}
