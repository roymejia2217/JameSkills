use jameskills_core::domain::{
    BACKUP_AEAD_TAG_LEN, BACKUP_ARGON2_ITERATIONS, BACKUP_ARGON2_MEMORY_KIB,
    BACKUP_ARGON2_PARALLELISM, BACKUP_CIPHER_XCHACHA20_POLY1305, BACKUP_HEADER_V1_LEN,
    BACKUP_HEADER_V1_VERSION, BACKUP_KDF_ARGON2ID, BACKUP_NONCE_LEN, BACKUP_SALT_LEN,
    BACKUP_WRAPPED_MASTER_LEN, BackupHeaderV1, MAX_ENCRYPTED_PAYLOAD_BYTES, SnapshotId, VaultId,
};
use uuid::Uuid;

fn header() -> BackupHeaderV1 {
    BackupHeaderV1::new(
        [0x11; BACKUP_SALT_LEN],
        VaultId::from_uuid(Uuid::from_bytes([0x22; 16])),
        SnapshotId::from_uuid(Uuid::from_bytes([0x33; 16])),
        [0x44; BACKUP_NONCE_LEN],
        [0x55; BACKUP_NONCE_LEN],
        [0x66; BACKUP_WRAPPED_MASTER_LEN],
        0x0000_0000_0001_0000,
    )
    .unwrap()
}

#[test]
fn header_176_bytes_match_the_fixed_big_endian_layout_and_aad_slices() {
    let header = header();
    let encoded = header.encode();
    let mut expected = [0u8; BACKUP_HEADER_V1_LEN];
    expected[0..8].copy_from_slice(b"JSKSBK01");
    expected[8..10].copy_from_slice(&BACKUP_HEADER_V1_VERSION.to_be_bytes());
    expected[10] = BACKUP_CIPHER_XCHACHA20_POLY1305;
    expected[11] = BACKUP_KDF_ARGON2ID;
    expected[12..16].copy_from_slice(&BACKUP_ARGON2_MEMORY_KIB.to_be_bytes());
    expected[16..20].copy_from_slice(&BACKUP_ARGON2_ITERATIONS.to_be_bytes());
    expected[20..24].copy_from_slice(&BACKUP_ARGON2_PARALLELISM.to_be_bytes());
    expected[24..40].fill(0x11);
    expected[40..56].fill(0x22);
    expected[56..72].fill(0x33);
    expected[72..96].fill(0x44);
    expected[96..120].fill(0x55);
    expected[120..168].fill(0x66);
    expected[168..176].copy_from_slice(&0x0000_0000_0001_0000u64.to_be_bytes());

    assert_eq!(encoded.len(), 176);
    assert_eq!(encoded, expected);
    let parsed = BackupHeaderV1::parse(&encoded).unwrap();
    assert_eq!(parsed.encode(), encoded);
    assert_eq!(&parsed.wrap_aad()[..120], &encoded[..120]);
    assert_eq!(&parsed.wrap_aad()[120..], &encoded[168..176]);
    assert_eq!(parsed.payload_aad(), encoded);
}

#[test]
fn wrapped_master_is_excluded_from_wrap_aad_but_included_in_payload_aad() {
    let header = header();
    let encoded = header.encode();
    let original_wrap_aad = header.wrap_aad();
    let mut changed = encoded;
    changed[120] ^= 1;
    let changed_header = BackupHeaderV1::parse(&changed).unwrap();

    assert_eq!(changed_header.wrap_aad(), original_wrap_aad);
    assert_ne!(changed_header.payload_aad(), header.payload_aad());
}

#[test]
fn header_rejects_truncation_trailing_data_unknown_algorithms_and_hostile_kdf_limits() {
    let encoded = header().encode();
    assert!(BackupHeaderV1::parse(&encoded[..BACKUP_HEADER_V1_LEN - 1]).is_err());
    let mut trailing = encoded.to_vec();
    trailing.push(0);
    assert!(BackupHeaderV1::parse(&trailing).is_err());

    for (offset, value) in [(9, 2), (10, 99), (11, 99), (13, 2), (19, 4), (23, 2)] {
        let mut hostile = encoded;
        hostile[offset] = value;
        assert!(BackupHeaderV1::parse(&hostile).is_err(), "offset {offset}");
    }

    let mut too_short = encoded;
    too_short[168..176].copy_from_slice(&(BACKUP_AEAD_TAG_LEN - 1).to_be_bytes());
    assert!(BackupHeaderV1::parse(&too_short).is_err());
    let mut too_large = encoded;
    too_large[168..176].copy_from_slice(&(MAX_ENCRYPTED_PAYLOAD_BYTES + 1).to_be_bytes());
    assert!(BackupHeaderV1::parse(&too_large).is_err());
}
