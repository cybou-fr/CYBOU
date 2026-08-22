// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Cryptographic sealing, key derivation, and key store primitives for Cybou.
//!
//! Provides byte- and algorithm-level compatibility with the predecessor's
//! XChaCha20-Poly1305 sealing, `KeyStore` storage, and sealed payload commitments.

pub mod keystore;
pub mod seal;
pub mod types;

pub use keystore::KeyStore;
pub use seal::Seal;
pub use types::{
    CryptoError, KeyDomain, KeyStoreError, SEAL_KEY_BYTES, SEAL_NONCE_BYTES, SEAL_TAG_BYTES,
    SealedPayload, decode_hex, encode_hex,
};

#[cfg(test)]
mod tests {
    use tempfile::tempdir;
    use uuid::Uuid;

    use super::*;

    #[test]
    fn a_sealed_payload_survives_the_process_that_sealed_it() {
        let dir = tempdir().expect("temp dir");
        let contribution = Uuid::from_u128(0x8f14_e45f_ceea_467a_9c9e_4d3f_2a1b_7c60);

        let (wrapped_domain, ciphertext) = {
            let store = KeyStore::open(dir.path()).expect("open store");
            let (kek, domain) = store.master().expect("establish master material");
            let data_key = store
                .create_key_for(&contribution, &kek)
                .expect("create data key");
            let sealed = Seal::seal(b"a thought worth keeping", &data_key).expect("seal");
            (domain, sealed)
        };

        let store = KeyStore::open(dir.path()).expect("reopen store");
        let (kek, domain) = store.master().expect("reload master material");
        assert_eq!(domain, wrapped_domain, "the key domain must be continuous");

        let data_key = store
            .key_for(&contribution, &kek)
            .expect("the data key must still unwrap with the persisted secret");
        let recovered = Seal::unseal(&ciphertext, &data_key).expect("unseal");
        assert_eq!(recovered, b"a thought worth keeping");
    }

    #[test]
    fn seal_and_unseal_roundtrip() {
        let key = Seal::generate_key().expect("generate key");
        let plaintext = b"sensitive cognitive state observation payload";

        let sealed1 = Seal::seal(plaintext, &key).expect("seal");
        let sealed2 = Seal::seal(plaintext, &key).expect("seal second time");

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

        store.destroy_key_for(&id).expect("destroy key");
        assert!(!store.has_key_for(&id));
        assert_eq!(store.key_for(&id, &kek), None);

        store
            .destroy_key_for(&id)
            .expect("destroy already absent key");
    }
}
