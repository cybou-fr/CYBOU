// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Cryptographic sealing, key derivation, and key store primitives for Cybou.
//!
//! Provides byte- and algorithm-level compatibility with the predecessor's
//! XChaCha20-Poly1305 sealing, `KeyStore` storage, and sealed payload commitments.

use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce, aead::Aead};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

/// Length in bytes of an XChaCha20-Poly1305 symmetric key.
pub const SEAL_KEY_BYTES: usize = 32;
/// Length in bytes of an `XChaCha20` extended nonce.
pub const SEAL_NONCE_BYTES: usize = 24;
/// Length in bytes of a Poly1305 authentication tag.
pub const SEAL_TAG_BYTES: usize = 16;

/// Errors arising from cryptographic operations.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CryptoError {
    /// Random byte generation failed.
    #[error("failed to generate random bytes")]
    RandomGenerationFailed,
    /// Encryption/sealing failed.
    #[error("failed to seal plaintext")]
    SealingFailed,
    /// Decryption/unsealing failed (wrong key or altered ciphertext/tag).
    #[error("failed to unseal ciphertext: authentication failed or invalid key")]
    UnsealingFailed,
    /// Invalid nonce or ciphertext size.
    #[error("invalid sealed payload shape: expected 24-byte nonce and >16-byte ciphertext")]
    InvalidPayloadShape,
    /// Invalid key size.
    #[error("invalid key size: expected 32 bytes")]
    InvalidKeySize,
}

/// A sealed payload containing the unencrypted 24-byte nonce and the combined ciphertext + tag.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SealedPayload {
    /// Extended 24-byte nonce.
    pub nonce: Vec<u8>,
    /// Ciphertext with appended 16-byte Poly1305 authentication tag.
    pub ciphertext: Vec<u8>,
}

impl SealedPayload {
    /// Check whether the sealed payload has valid structural dimensions.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.nonce.len() == SEAL_NONCE_BYTES && self.ciphertext.len() > SEAL_TAG_BYTES
    }
}

/// A key domain identity and its associated rotation epoch.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KeyDomain {
    /// Unique domain identifier.
    pub key_domain_id: Uuid,
    /// Monotonic epoch counter for key rotation.
    pub key_epoch: u32,
}

impl KeyDomain {
    /// Create a new key domain with the given epoch.
    #[must_use]
    pub fn new(key_domain_id: Uuid, key_epoch: u32) -> Self {
        Self {
            key_domain_id,
            key_epoch,
        }
    }

    /// Generate a fresh random key domain with default epoch 1.
    #[must_use]
    pub fn generate(epoch: u32) -> Self {
        Self {
            key_domain_id: Uuid::new_v4(),
            key_epoch: epoch,
        }
    }

    /// Check whether the key domain ID is non-nil.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.key_domain_id.is_nil()
    }
}

/// The master secret and key domain as persisted between runs.
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredMaster {
    version: u8,
    secret: String,
    key_domain_id: Uuid,
    key_epoch: u32,
}

impl StoredMaster {
    fn into_material(self) -> Result<([u8; SEAL_KEY_BYTES], KeyDomain), KeyStoreError> {
        if self.version != 1 {
            return Err(KeyStoreError::MasterUnreadable);
        }
        let bytes = decode_hex(&self.secret).ok_or(KeyStoreError::MasterUnreadable)?;
        let secret: [u8; SEAL_KEY_BYTES] = bytes
            .try_into()
            .map_err(|_| KeyStoreError::MasterUnreadable)?;
        let domain = KeyDomain::new(self.key_domain_id, self.key_epoch);
        if !domain.is_valid() {
            return Err(KeyStoreError::MasterUnreadable);
        }
        Ok((secret, domain))
    }
}

fn encode_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    bytes.iter().fold(String::new(), |mut text, byte| {
        let _ = write!(text, "{byte:02x}");
        text
    })
}

fn decode_hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(text.get(index..index + 2)?, 16).ok())
        .collect()
}

/// Write a secret to disk readable only by its owner.
fn write_secret_file(path: &Path, bytes: &[u8]) -> Result<(), KeyStoreError> {
    use std::io::Write as _;

    let mut file = fs::File::create(path).map_err(|source| KeyStoreError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = file.set_permissions(fs::Permissions::from_mode(0o600));
    }
    file.write_all(bytes).map_err(|source| KeyStoreError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    file.sync_all().map_err(|source| KeyStoreError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(())
}

/// Sealing and key wrapping primitives using XChaCha20-Poly1305.
pub struct Seal;

impl Seal {
    /// Generate 32 cryptographically secure random bytes for a data encryption key (DEK).
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::RandomGenerationFailed`] if system entropy is unavailable.
    pub fn generate_key() -> Result<[u8; SEAL_KEY_BYTES], CryptoError> {
        let mut key = [0u8; SEAL_KEY_BYTES];
        getrandom::getrandom(&mut key).map_err(|_| CryptoError::RandomGenerationFailed)?;
        Ok(key)
    }

    /// Generate a fresh key domain with random UUID.
    #[must_use]
    pub fn generate_domain(epoch: u32) -> KeyDomain {
        KeyDomain::generate(epoch)
    }

    /// Encrypt plaintext using a 32-byte key and a fresh random 24-byte nonce.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError`] on random generation failure or encryption error.
    pub fn seal(
        plaintext: &[u8],
        key: &[u8; SEAL_KEY_BYTES],
    ) -> Result<SealedPayload, CryptoError> {
        let mut nonce_bytes = [0u8; SEAL_NONCE_BYTES];
        getrandom::getrandom(&mut nonce_bytes).map_err(|_| CryptoError::RandomGenerationFailed)?;

        let cipher = XChaCha20Poly1305::new(key.into());
        let nonce = XNonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|_| CryptoError::SealingFailed)?;

        Ok(SealedPayload {
            nonce: nonce_bytes.to_vec(),
            ciphertext,
        })
    }

    /// Decrypt a sealed payload using a 32-byte key.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError`] if the payload shape is invalid or authentication fails.
    pub fn unseal(
        sealed: &SealedPayload,
        key: &[u8; SEAL_KEY_BYTES],
    ) -> Result<Vec<u8>, CryptoError> {
        if !sealed.is_valid() {
            return Err(CryptoError::InvalidPayloadShape);
        }

        let cipher = XChaCha20Poly1305::new(key.into());
        let nonce = XNonce::from_slice(&sealed.nonce);

        cipher
            .decrypt(nonce, sealed.ciphertext.as_ref())
            .map_err(|_| CryptoError::UnsealingFailed)
    }

    /// Wrap a 32-byte data key (DEK) under a 32-byte key-encryption key (KEK).
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError`] if wrapping fails.
    pub fn wrap_key(
        data_key: &[u8; SEAL_KEY_BYTES],
        kek: &[u8; SEAL_KEY_BYTES],
    ) -> Result<SealedPayload, CryptoError> {
        Self::seal(data_key, kek)
    }

    /// Unwrap a wrapped data key (DEK) using the 32-byte key-encryption key (KEK).
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError`] if unwrapping fails or the recovered key size is incorrect.
    pub fn unwrap_key(
        wrapped: &SealedPayload,
        kek: &[u8; SEAL_KEY_BYTES],
    ) -> Result<[u8; SEAL_KEY_BYTES], CryptoError> {
        let plaintext = Self::unseal(wrapped, kek)?;
        if plaintext.len() != SEAL_KEY_BYTES {
            return Err(CryptoError::InvalidKeySize);
        }
        let mut key = [0u8; SEAL_KEY_BYTES];
        key.copy_from_slice(&plaintext);
        Ok(key)
    }

    /// Calculate the canonical SHA-256 commitment over `nonce || ciphertext`.
    #[must_use]
    pub fn sealed_payload_commitment(sealed: &SealedPayload) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(&sealed.nonce);
        hasher.update(&sealed.ciphertext);
        hasher.finalize().into()
    }
}

/// Errors occurring during `KeyStore` operations.
#[derive(Debug, Error)]
pub enum KeyStoreError {
    /// Underlying cryptographic failure.
    #[error("cryptographic operation failed: {0}")]
    Crypto(#[from] CryptoError),
    /// File system or I/O failure.
    #[error("I/O error at {path}: {source}")]
    Io {
        /// File or directory path.
        path: PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },
    /// Invalid contribution ID (nil UUID).
    #[error("invalid nil contribution UUID")]
    InvalidContributionId,
    /// The stored master secret is absent, malformed, or of an unknown version.
    ///
    /// Refused rather than replaced: generating a fresh secret over an unreadable one would
    /// destroy every sealed payload it had wrapped, without anything recording that a biography
    /// had been erased.
    #[error("stored master secret cannot be read and will not be replaced")]
    MasterUnreadable,
}

/// File-based `KeyStore` for per-contribution data keys.
///
/// Implements idempotent key destruction and atomic file writes.
#[derive(Clone, Debug)]
pub struct KeyStore {
    root: PathBuf,
}

impl KeyStore {
    /// Open or create a key store directory with owner-only access.
    ///
    /// # Errors
    ///
    /// Returns [`KeyStoreError::Io`] if the directory cannot be created or secured.
    pub fn open(root: impl AsRef<Path>) -> Result<Self, KeyStoreError> {
        let root = root.as_ref().to_path_buf();
        if !root.exists() {
            fs::create_dir_all(&root).map_err(|source| KeyStoreError::Io {
                path: root.clone(),
                source,
            })?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(0o700);
            let _ = fs::set_permissions(&root, permissions);
        }

        Ok(Self { root })
    }

    /// Load the master secret and key domain this store was opened with, creating them once.
    ///
    /// The key-encryption key must outlive the process. Every data key in this store is wrapped
    /// with it, so a KEK generated at startup can unwrap only what the same run wrote: restarting
    /// would silently make every earlier sealed payload unreadable, with no `ErasureRequested` and
    /// no `ErasureApplied` to say a biography had been destroyed. Erasure has to be a decision,
    /// never a side effect of a process dying.
    ///
    /// The key domain is durable for the same reason and the epoch is monotonic, so a payload can
    /// always name which domain and epoch produced it.
    ///
    /// This is continuity, not protection from someone who can read the directory: the secret sits
    /// beside the keys it wraps, in a `0700` directory with a `0600` file. It keeps the master
    /// secret out of the Journal and out of backups of the Journal, and it is where an OS keyring,
    /// a TPM or a passphrase-derived key belongs when one is available.
    ///
    /// # Errors
    ///
    /// Returns [`KeyStoreError`] when the material cannot be created, read, or written.
    pub fn master(&self) -> Result<([u8; SEAL_KEY_BYTES], KeyDomain), KeyStoreError> {
        let path = self.root.join("master.json");
        if path.exists() {
            let raw = fs::read_to_string(&path).map_err(|source| KeyStoreError::Io {
                path: path.clone(),
                source,
            })?;
            let stored: StoredMaster =
                serde_json::from_str(&raw).map_err(|_| KeyStoreError::MasterUnreadable)?;
            return stored.into_material();
        }

        let secret = Seal::generate_key()?;
        let domain = KeyDomain::generate(1);
        let stored = StoredMaster {
            version: 1,
            secret: encode_hex(&secret),
            key_domain_id: domain.key_domain_id,
            key_epoch: domain.key_epoch,
        };
        let encoded =
            serde_json::to_string(&stored).map_err(|_| KeyStoreError::MasterUnreadable)?;

        let temp = path.with_extension("tmp");
        write_secret_file(&temp, encoded.as_bytes())?;
        fs::rename(&temp, &path).map_err(|source| KeyStoreError::Io {
            path: path.clone(),
            source,
        })?;

        Ok((secret, domain))
    }

    /// Path to the key file for a contribution ID.
    #[must_use]
    pub fn path_for(&self, contribution_id: &Uuid) -> PathBuf {
        let filename = format!("{}.key", contribution_id.simple());
        self.root.join(filename)
    }

    /// Create, wrap, and atomically persist a fresh data key for the contribution.
    /// Returns the raw unwrapped 32-byte data key.
    ///
    /// # Errors
    ///
    /// Returns [`KeyStoreError`] if the ID is nil, key generation/wrapping fails, or writing fails.
    pub fn create_key_for(
        &self,
        contribution_id: &Uuid,
        kek: &[u8; SEAL_KEY_BYTES],
    ) -> Result<[u8; SEAL_KEY_BYTES], KeyStoreError> {
        if contribution_id.is_nil() {
            return Err(KeyStoreError::InvalidContributionId);
        }

        let data_key = Seal::generate_key()?;
        let wrapped = Seal::wrap_key(&data_key, kek)?;

        let path = self.path_for(contribution_id);
        let mut encoded = Vec::with_capacity(SEAL_NONCE_BYTES + wrapped.ciphertext.len());
        encoded.extend_from_slice(&wrapped.nonce);
        encoded.extend_from_slice(&wrapped.ciphertext);

        // Atomic write via temp file in the same directory
        let temp_path = path.with_extension("tmp");
        {
            let mut file = File::create(&temp_path).map_err(|source| KeyStoreError::Io {
                path: temp_path.clone(),
                source,
            })?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = file.set_permissions(fs::Permissions::from_mode(0o600));
            }

            file.write_all(&encoded)
                .map_err(|source| KeyStoreError::Io {
                    path: temp_path.clone(),
                    source,
                })?;
            file.flush().map_err(|source| KeyStoreError::Io {
                path: temp_path.clone(),
                source,
            })?;
        }

        fs::rename(&temp_path, &path).map_err(|source| KeyStoreError::Io { path, source })?;

        Ok(data_key)
    }

    /// Retrieve and unwrap the data key for a contribution ID, or `None` if destroyed/absent.
    #[must_use]
    pub fn key_for(
        &self,
        contribution_id: &Uuid,
        kek: &[u8; SEAL_KEY_BYTES],
    ) -> Option<[u8; SEAL_KEY_BYTES]> {
        let path = self.path_for(contribution_id);
        let mut file = File::open(&path).ok()?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents).ok()?;

        if contents.len() <= SEAL_NONCE_BYTES + SEAL_TAG_BYTES {
            return None;
        }

        let wrapped = SealedPayload {
            nonce: contents[..SEAL_NONCE_BYTES].to_vec(),
            ciphertext: contents[SEAL_NONCE_BYTES..].to_vec(),
        };

        Seal::unwrap_key(&wrapped, kek).ok()
    }

    /// Destroy a contribution's data key.
    ///
    /// Idempotent: returns `Ok(())` even if the key was already absent.
    ///
    /// # Errors
    ///
    /// Returns [`KeyStoreError::Io`] only if removal of an existing file failed.
    pub fn destroy_key_for(&self, contribution_id: &Uuid) -> Result<(), KeyStoreError> {
        let path = self.path_for(contribution_id);
        if !path.exists() {
            return Ok(());
        }
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(source) => Err(KeyStoreError::Io { path, source }),
        }
    }

    /// Check whether a key file exists for the given contribution ID.
    #[must_use]
    pub fn has_key_for(&self, contribution_id: &Uuid) -> bool {
        self.path_for(contribution_id).exists()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_sealed_payload_survives_the_process_that_sealed_it() {
        let dir = tempfile::tempdir().expect("temp dir");
        let contribution = uuid::Uuid::from_u128(0x8f14_e45f_ceea_467a_9c9e_4d3f_2a1b_7c60);

        // One run seals something.
        let (wrapped_domain, ciphertext) = {
            let store = super::KeyStore::open(dir.path()).expect("open store");
            let (kek, domain) = store.master().expect("establish master material");
            let data_key = store
                .create_key_for(&contribution, &kek)
                .expect("create data key");
            let sealed = super::Seal::seal(b"a thought worth keeping", &data_key).expect("seal");
            (domain, sealed)
        };

        // The next run must be able to open it. A fresh key-encryption key here would make the
        // payload unreadable for ever, with nothing recording that anything had been erased.
        let store = super::KeyStore::open(dir.path()).expect("reopen store");
        let (kek, domain) = store.master().expect("reload master material");
        assert_eq!(domain, wrapped_domain, "the key domain must be continuous");

        let data_key = store
            .key_for(&contribution, &kek)
            .expect("the data key must still unwrap with the persisted secret");
        let recovered = super::Seal::unseal(&ciphertext, &data_key).expect("unseal");
        assert_eq!(recovered, b"a thought worth keeping");
    }

    use tempfile::tempdir;
    use uuid::Uuid;

    use super::*;

    #[test]
    fn seal_and_unseal_roundtrip() {
        let key = Seal::generate_key().expect("generate key");
        let plaintext = b"sensitive cognitive state observation payload";

        let sealed1 = Seal::seal(plaintext, &key).expect("seal");
        let sealed2 = Seal::seal(plaintext, &key).expect("seal second time");

        // Random nonce ensures distinct ciphertexts
        assert_ne!(sealed1.nonce, sealed2.nonce);
        assert_ne!(sealed1.ciphertext, sealed2.ciphertext);

        let unsealed1 = Seal::unseal(&sealed1, &key).expect("unseal");
        assert_eq!(unsealed1, plaintext);

        let unsealed2 = Seal::unseal(&sealed2, &key).expect("unseal");
        assert_eq!(unsealed2, plaintext);
    }

    #[test]
    fn unseal_with_wrong_key_fails() {
        let key1 = Seal::generate_key().expect("key1");
        let key2 = Seal::generate_key().expect("key2");
        let plaintext = b"secret data";

        let sealed = Seal::seal(plaintext, &key1).expect("seal");
        let err = Seal::unseal(&sealed, &key2).expect_err("should fail");
        assert_eq!(err, CryptoError::UnsealingFailed);
    }

    #[test]
    fn key_wrapping_roundtrip() {
        let kek = Seal::generate_key().expect("kek");
        let data_key = Seal::generate_key().expect("data key");

        let wrapped = Seal::wrap_key(&data_key, &kek).expect("wrap");
        let unwrapped = Seal::unwrap_key(&wrapped, &kek).expect("unwrap");

        assert_eq!(unwrapped, data_key);
    }

    #[test]
    fn sealed_payload_commitment_is_deterministic() {
        let sealed = SealedPayload {
            nonce: vec![1u8; SEAL_NONCE_BYTES],
            ciphertext: vec![2u8; 32],
        };
        let commitment1 = Seal::sealed_payload_commitment(&sealed);
        let commitment2 = Seal::sealed_payload_commitment(&sealed);
        assert_eq!(commitment1, commitment2);
        assert_eq!(commitment1.len(), 32);
    }

    #[test]
    fn keystore_lifecycle_create_read_destroy_idempotent() {
        let dir = tempdir().expect("tempdir");
        let store = KeyStore::open(dir.path()).expect("open keystore");
        let kek = Seal::generate_key().expect("kek");
        let id = Uuid::new_v4();

        assert!(!store.has_key_for(&id));
        assert_eq!(store.key_for(&id, &kek), None);

        let data_key = store.create_key_for(&id, &kek).expect("create key");
        assert!(store.has_key_for(&id));

        let retrieved = store.key_for(&id, &kek).expect("retrieve key");
        assert_eq!(retrieved, data_key);

        // Destroy key
        store.destroy_key_for(&id).expect("destroy key");
        assert!(!store.has_key_for(&id));
        assert_eq!(store.key_for(&id, &kek), None);

        // Idempotent destroy
        store
            .destroy_key_for(&id)
            .expect("destroy already absent key");
    }
}
