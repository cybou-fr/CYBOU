// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! File-based per-contribution `KeyStore` implementation.

use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use uuid::Uuid;

use crate::seal::Seal;
use crate::types::{
    KeyDomain, KeyStoreError, SEAL_KEY_BYTES, SEAL_NONCE_BYTES, SEAL_TAG_BYTES, SealedPayload,
    StoredMaster, encode_hex, write_secret_file,
};

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
