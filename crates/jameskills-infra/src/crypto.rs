use argon2::{Algorithm, Argon2, Block, Params, Version};
use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit, OsRng, Payload, rand_core::RngCore},
};
use jameskills_core::{
    AppError, AppResult,
    domain::{
        BACKUP_AEAD_TAG_LEN, BACKUP_ARGON2_ITERATIONS, BACKUP_ARGON2_MEMORY_KIB,
        BACKUP_ARGON2_PARALLELISM, BACKUP_NONCE_LEN, BACKUP_SALT_LEN, BACKUP_WRAPPED_MASTER_LEN,
        BackupHeaderV1, EncryptedSnapshot, MAX_ENCRYPTED_PAYLOAD_BYTES, SnapshotPayload,
        UnlockedVault, VaultId, VerifiedSnapshot, WrappingKey,
    },
    ports::{CryptoPort, SecretInput},
};
use zeroize::{Zeroize, Zeroizing};

#[derive(Clone, Copy, Debug, Default)]
pub struct CryptoProvider;

impl CryptoProvider {
    pub fn new() -> Self {
        Self
    }
}

impl CryptoPort for CryptoProvider {
    fn derive_wrapping_key(
        &self,
        password: &SecretInput,
        salt: &[u8; 16],
    ) -> AppResult<WrappingKey> {
        let params = Params::new(
            BACKUP_ARGON2_MEMORY_KIB,
            BACKUP_ARGON2_ITERATIONS,
            BACKUP_ARGON2_PARALLELISM,
            Some(32),
        )
        .map_err(|_| AppError::CryptoInvalid)?;
        let block_count = params.block_count();
        let mut memory = Zeroizing::new(Vec::<Block>::new());
        memory
            .try_reserve_exact(block_count)
            .map_err(|_| AppError::CapabilityUnavailable {
                id: "crypto.kdf.memory.unavailable".to_owned(),
                guidance_id: "sync.crypto.memory.setup".to_owned(),
            })?;
        memory.resize(block_count, Block::default());

        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
        let mut output = Zeroizing::new([0u8; 32]);
        argon2
            .hash_password_into_with_memory(
                password.expose_secret(),
                salt,
                &mut *output,
                &mut *memory,
            )
            .map_err(|_| AppError::CryptoInvalid)?;
        Ok(WrappingKey::from_kdf_output(*output))
    }

    fn create_vault(&self, password: &SecretInput) -> AppResult<UnlockedVault> {
        let mut vault_id_bytes = [0u8; 16];
        let mut salt = [0u8; BACKUP_SALT_LEN];
        let mut master_key = Zeroizing::new([0u8; 32]);
        fill_random(&mut vault_id_bytes)?;
        fill_random(&mut salt)?;
        fill_random(&mut *master_key)?;
        let vault_id = VaultId::from_random_bytes(vault_id_bytes);
        let wrapping_key = self.derive_wrapping_key(password, &salt)?;
        Ok(UnlockedVault::from_crypto_material(
            vault_id,
            master_key,
            wrapping_key,
            salt,
        ))
    }

    fn unlock_vault(
        &self,
        data: &EncryptedSnapshot,
        password: &SecretInput,
    ) -> AppResult<UnlockedVault> {
        let header = data.header();
        let wrapping_key = self.derive_wrapping_key(password, header.salt())?;
        let cipher = XChaCha20Poly1305::new_from_slice(wrapping_key.expose_secret())
            .map_err(|_| AppError::CryptoInvalid)?;
        let aad = header.wrap_aad();
        let mut master_plaintext = Zeroizing::new(
            cipher
                .decrypt(
                    XNonce::from_slice(header.wrap_nonce()),
                    Payload {
                        msg: header.wrapped_master(),
                        aad: &aad,
                    },
                )
                .map_err(|_| AppError::CryptoInvalid)?,
        );
        if master_plaintext.len() != 32 {
            return Err(AppError::CryptoInvalid);
        }
        let mut master_key = Zeroizing::new([0u8; 32]);
        master_key.copy_from_slice(&master_plaintext);
        master_plaintext.zeroize();
        Ok(UnlockedVault::from_crypto_material(
            header.vault_id(),
            master_key,
            wrapping_key,
            *header.salt(),
        ))
    }

    fn open_with_vault(
        &self,
        data: &EncryptedSnapshot,
        key: &UnlockedVault,
    ) -> AppResult<VerifiedSnapshot> {
        let header = data.header();
        if header.vault_id() != key.vault_id() || header.salt() != key.salt() {
            return Err(AppError::CryptoInvalid);
        }
        let cipher = XChaCha20Poly1305::new_from_slice(key.master_key())
            .map_err(|_| AppError::CryptoInvalid)?;
        let aad = header.payload_aad();
        let mut plaintext = Zeroizing::new(
            cipher
                .decrypt(
                    XNonce::from_slice(header.payload_nonce()),
                    Payload {
                        msg: data.ciphertext(),
                        aad: &aad,
                    },
                )
                .map_err(|_| AppError::CryptoInvalid)?,
        );
        let payload = crate::snapshot_archive::decode(&plaintext)?;
        if payload.vault_id() != header.vault_id() || payload.snapshot_id() != header.snapshot_id()
        {
            return Err(AppError::CryptoInvalid);
        }
        plaintext.zeroize();
        payload.verify()
    }

    fn seal(&self, data: &SnapshotPayload, key: &UnlockedVault) -> AppResult<EncryptedSnapshot> {
        if data.vault_id() != key.vault_id() {
            return Err(AppError::CryptoInvalid);
        }
        let mut plaintext = Zeroizing::new(crate::snapshot_archive::encode(data)?);
        let ciphertext_len = u64::try_from(plaintext.len())
            .ok()
            .and_then(|length| length.checked_add(BACKUP_AEAD_TAG_LEN))
            .filter(|length| *length <= MAX_ENCRYPTED_PAYLOAD_BYTES)
            .ok_or(AppError::CryptoInvalid)?;

        let mut wrap_nonce = [0u8; BACKUP_NONCE_LEN];
        let mut payload_nonce = [0u8; BACKUP_NONCE_LEN];
        fill_random(&mut wrap_nonce)?;
        fill_random(&mut payload_nonce)?;
        let placeholder = BackupHeaderV1::new(
            *key.salt(),
            key.vault_id(),
            data.snapshot_id(),
            wrap_nonce,
            payload_nonce,
            [0u8; BACKUP_WRAPPED_MASTER_LEN],
            ciphertext_len,
        )?;
        let wrapping_cipher = XChaCha20Poly1305::new_from_slice(key.wrapping_key().expose_secret())
            .map_err(|_| AppError::CryptoInvalid)?;
        let wrap_aad = placeholder.wrap_aad();
        let wrapped_master = wrapping_cipher
            .encrypt(
                XNonce::from_slice(placeholder.wrap_nonce()),
                Payload {
                    msg: key.master_key(),
                    aad: &wrap_aad,
                },
            )
            .map_err(|_| AppError::CryptoInvalid)?;
        let wrapped_master: [u8; BACKUP_WRAPPED_MASTER_LEN] = wrapped_master
            .try_into()
            .map_err(|_| AppError::CryptoInvalid)?;
        let header = BackupHeaderV1::new(
            *placeholder.salt(),
            placeholder.vault_id(),
            placeholder.snapshot_id(),
            *placeholder.wrap_nonce(),
            *placeholder.payload_nonce(),
            wrapped_master,
            placeholder.ciphertext_len(),
        )?;
        let payload_cipher = XChaCha20Poly1305::new_from_slice(key.master_key())
            .map_err(|_| AppError::CryptoInvalid)?;
        let payload_aad = header.payload_aad();
        let ciphertext = payload_cipher
            .encrypt(
                XNonce::from_slice(header.payload_nonce()),
                Payload {
                    msg: &plaintext,
                    aad: &payload_aad,
                },
            )
            .map_err(|_| AppError::CryptoInvalid)?;
        plaintext.zeroize();
        EncryptedSnapshot::new(header, ciphertext)
    }

    fn open(
        &self,
        data: &EncryptedSnapshot,
        password: &SecretInput,
    ) -> AppResult<VerifiedSnapshot> {
        let key = self.unlock_vault(data, password)?;
        self.open_with_vault(data, &key)
    }
}

fn fill_random(destination: &mut [u8]) -> AppResult<()> {
    let mut rng = OsRng;
    fill_random_with(destination, |output| {
        rng.try_fill_bytes(output).map_err(|_| ())
    })
}

fn fill_random_with(
    destination: &mut [u8],
    fill: impl FnOnce(&mut [u8]) -> Result<(), ()>,
) -> AppResult<()> {
    fill(destination).map_err(|_| AppError::CapabilityUnavailable {
        id: "crypto.rng.unavailable".to_owned(),
        guidance_id: "sync.crypto.randomness.setup".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::fill_random_with;

    #[test]
    fn random_source_failure_aborts_without_filling_the_output() {
        let mut destination = [0xA5; 32];
        assert!(fill_random_with(&mut destination, |_| Err(())).is_err());
        assert_eq!(destination, [0xA5; 32]);
    }
}
