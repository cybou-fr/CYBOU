// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! XChaCha20-Poly1305 sealing, key generation, and commitment calculation.

use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce, aead::Aead};
use sha2::{Digest, Sha256};

use crate::types::{CryptoError, KeyDomain, SEAL_KEY_BYTES, SEAL_NONCE_BYTES, SealedPayload};

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
