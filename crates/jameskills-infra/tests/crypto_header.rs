use argon2::{Algorithm, Argon2, AssociatedData, Block, Params, ParamsBuilder, Version};
use jameskills_core::{
    domain::{BACKUP_ARGON2_ITERATIONS, BACKUP_ARGON2_MEMORY_KIB, BACKUP_ARGON2_PARALLELISM},
    ports::{CryptoPort, SecretInput},
};
use jameskills_infra::crypto::CryptoProvider;
use zeroize::Zeroizing;

#[test]
fn rustcrypto_argon2id_matches_rfc9106_v19_vector() {
    let password = [0x01; 32];
    let salt = [0x02; 16];
    let secret = [0x03; 8];
    let associated_data = [0x04; 12];
    let associated_data = AssociatedData::new(&associated_data).unwrap();
    let mut params = ParamsBuilder::new();
    params
        .m_cost(32)
        .t_cost(3)
        .p_cost(4)
        .output_len(32)
        .data(associated_data);
    let params = params.build().unwrap();
    let block_count = params.block_count();
    let argon2 =
        Argon2::new_with_secret(&secret, Algorithm::Argon2id, Version::V0x13, params).unwrap();
    let mut memory = Zeroizing::new(vec![Block::default(); block_count]);
    let mut output = Zeroizing::new([0u8; 32]);
    argon2
        .hash_password_into_with_memory(&password, &salt, &mut *output, &mut *memory)
        .unwrap();
    let expected = [
        0x0d, 0x64, 0x0d, 0xf5, 0x8d, 0x78, 0x76, 0x6c, 0x08, 0xc0, 0x37, 0xa3, 0x4a, 0x8b, 0x53,
        0xc9, 0xd0, 0x1e, 0xf0, 0x45, 0x2d, 0x75, 0xb6, 0x5e, 0xb5, 0x25, 0x20, 0xe9, 0x6b, 0x01,
        0xe6, 0x59,
    ];
    assert_eq!(*output, expected);
}

#[test]
fn crypto_provider_uses_the_fixed_v1_kdf_parameters_deterministically() {
    let provider = CryptoProvider::new();
    let password = SecretInput::new(b"fixed test passphrase".to_vec()).unwrap();
    let salt = [0xA6; 16];
    let first = provider.derive_wrapping_key(&password, &salt).unwrap();
    let second = provider.derive_wrapping_key(&password, &salt).unwrap();
    let other_salt = provider
        .derive_wrapping_key(&password, &[0xA7; 16])
        .unwrap();

    assert_eq!(first.expose_secret(), second.expose_secret());
    assert_ne!(first.expose_secret(), other_salt.expose_secret());
    assert_ne!(first.expose_secret(), &[0; 32]);

    let params = Params::new(
        BACKUP_ARGON2_MEMORY_KIB,
        BACKUP_ARGON2_ITERATIONS,
        BACKUP_ARGON2_PARALLELISM,
        Some(32),
    )
    .unwrap();
    let block_count = params.block_count();
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut memory = Zeroizing::new(vec![Block::default(); block_count]);
    let mut reference = Zeroizing::new([0u8; 32]);
    argon2
        .hash_password_into_with_memory(
            password.expose_secret(),
            &salt,
            &mut *reference,
            &mut *memory,
        )
        .unwrap();
    assert_eq!(first.expose_secret(), &*reference);
}
