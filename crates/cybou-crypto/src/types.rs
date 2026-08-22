// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Cryptographic types, errors, domains, and persistent master representations.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
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
pub struct StoredMaster {
    /// Storage format version.
    pub version: u8,
    /// Hex-encoded master secret.
    pub secret: String,
    /// Key domain UUID.
    pub key_domain_id: Uuid,
    /// Monotonic key epoch.
    pub key_epoch: u32,
}

impl StoredMaster {
    /// Convert stored master into secret key bytes and key domain.
    ///
    /// # Errors
    ///
    /// Returns [`KeyStoreError::MasterUnreadable`] if format version or decoding fails.
    pub fn into_material(self) -> Result<([u8; SEAL_KEY_BYTES], KeyDomain), KeyStoreError> {
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

/// Encode byte slice as lowercase hexadecimal string.
#[must_use]
pub fn encode_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    bytes.iter().fold(String::new(), |mut text, byte| {
        let _ = write!(text, "{byte:02x}");
        text
    })
}

/// Decode hexadecimal string into byte vector.
#[must_use]
pub fn decode_hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(text.get(index..index + 2)?, 16).ok())
        .collect()
}

/// Write a secret to disk readable only by its owner.
///
/// # Errors
///
/// Returns [`KeyStoreError::Io`] if file creation, writing, or synchronization fails.
pub fn write_secret_file(path: &Path, bytes: &[u8]) -> Result<(), KeyStoreError> {
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
    #[error("stored master secret cannot be read and will not be replaced")]
    MasterUnreadable,
}
