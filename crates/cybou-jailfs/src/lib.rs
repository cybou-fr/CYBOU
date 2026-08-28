// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Bounded filesystem sandbox and path traversal protection for CYBOU Body capabilities.
//!
//! Enforces strict directory confinement so that Shell commands or ephemeral tool capabilities
//! can never escape the designated sandbox boundary (`JailFs`).

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use thiserror::Error;

static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Error returned by sandboxed filesystem operations.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum JailError {
    /// An attempt to escape the sandbox boundary was detected.
    #[error("sandbox path traversal violation: {0}")]
    TraversalAttempt(String),
    /// The specified path does not exist.
    #[error("file or directory not found: {0}")]
    NotFound(String),
    /// An illegal or malformed path string was provided.
    #[error("invalid path syntax: {0}")]
    InvalidPath(String),
    /// Operation exceeds the allowed size budget.
    #[error("file size limit exceeded: {actual_bytes} bytes (limit: {max_bytes} bytes)")]
    SizeLimitExceeded {
        /// Configured limit in bytes.
        max_bytes: usize,
        /// Actual detected bytes.
        actual_bytes: usize,
    },
    /// An underlying I/O error occurred.
    #[error("I/O error: {0}")]
    Io(String),
}

/// Directory entry description within the sandbox.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct DirEntry {
    /// File or directory name.
    pub name: String,
    /// Whether this entry is a directory.
    pub is_dir: bool,
    /// Size in bytes for files, 0 for directories.
    pub size_bytes: u64,
}

/// Bounded sandboxed filesystem root.
#[derive(Clone, Debug)]
pub struct JailFs {
    canonical_root: PathBuf,
}

impl JailFs {
    /// Initialize a new sandbox rooted at the specified filesystem path.
    ///
    /// The directory is created if it does not already exist.
    ///
    /// # Errors
    ///
    /// Returns [`JailError::Io`] if the root directory cannot be created or canonicalized.
    pub fn new(root: impl AsRef<Path>) -> Result<Self, JailError> {
        let root = root.as_ref();
        if !root.exists() {
            fs::create_dir_all(root).map_err(|e| {
                JailError::Io(format!(
                    "failed to create jail root {}: {e}",
                    root.display()
                ))
            })?;
        }
        let canonical_root = root.canonicalize().map_err(|e| {
            JailError::Io(format!(
                "failed to canonicalize jail root {}: {e}",
                root.display()
            ))
        })?;
        Ok(Self { canonical_root })
    }

    /// Return the canonical host path of this sandbox root.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.canonical_root
    }

    /// Resolve and validate a virtual path against the sandbox boundary.
    ///
    /// # Security Invariants
    ///
    /// 1. Reject null bytes and path components containing `..` (`ParentDir`).
    /// 2. Normalize leading slashes so `/foo/bar` is treated relative to the sandbox root.
    /// 3. Verify that the resolved path is strictly a descendant of `canonical_root`.
    ///
    /// # Errors
    ///
    /// Returns [`JailError::TraversalAttempt`] or [`JailError::InvalidPath`] on violation.
    pub fn resolve(&self, virtual_path: &str) -> Result<PathBuf, JailError> {
        if virtual_path.contains('\0') {
            return Err(JailError::InvalidPath("null bytes not permitted".into()));
        }

        let raw_path = Path::new(virtual_path);
        let mut clean_components = Vec::new();

        for component in raw_path.components() {
            match component {
                Component::Prefix(_) | Component::RootDir | Component::CurDir => {}
                Component::ParentDir => {
                    return Err(JailError::TraversalAttempt(format!(
                        "parent directory navigation '..' is forbidden: {virtual_path}"
                    )));
                }
                Component::Normal(c) => {
                    let s = c.to_string_lossy();
                    if s == ".." {
                        return Err(JailError::TraversalAttempt(format!(
                            "parent directory navigation '..' is forbidden: {virtual_path}"
                        )));
                    }
                    clean_components.push(s.to_string());
                }
            }
        }

        let mut resolved = self.canonical_root.clone();
        for seg in clean_components {
            resolved.push(seg);
        }

        // If the path exists on disk, canonicalize and verify containment to prevent symlink bypasses
        if resolved.exists() {
            let canon = resolved
                .canonicalize()
                .map_err(|e| JailError::Io(format!("failed to canonicalize path: {e}")))?;
            if !canon.starts_with(&self.canonical_root) {
                return Err(JailError::TraversalAttempt(format!(
                    "symlink target escapes sandbox boundary: {virtual_path}"
                )));
            }
            return Ok(canon);
        }

        // Path does not exist yet (e.g. for write operations). Verify parent if it exists.
        if let Some(parent) = resolved.parent()
            && parent.exists()
        {
            let canon_parent = parent
                .canonicalize()
                .map_err(|e| JailError::Io(format!("failed to canonicalize parent: {e}")))?;
            if !canon_parent.starts_with(&self.canonical_root) {
                return Err(JailError::TraversalAttempt(format!(
                    "parent directory escapes sandbox boundary: {virtual_path}"
                )));
            }
        }

        Ok(resolved)
    }

    /// Read an entire file as a UTF-8 string, subject to a maximum byte limit.
    ///
    /// # Errors
    ///
    /// Returns [`JailError`] if the file cannot be found, read, or exceeds `max_bytes`.
    pub fn read_to_string(
        &self,
        virtual_path: &str,
        max_bytes: usize,
    ) -> Result<String, JailError> {
        let bytes = self.read_bytes(virtual_path, max_bytes)?;
        String::from_utf8(bytes).map_err(|e| JailError::Io(format!("invalid UTF-8 content: {e}")))
    }

    /// Read raw bytes of a file up to `max_bytes`.
    ///
    /// # Errors
    ///
    /// Returns [`JailError`] if the file cannot be opened, read, or exceeds `max_bytes`.
    pub fn read_bytes(&self, virtual_path: &str, max_bytes: usize) -> Result<Vec<u8>, JailError> {
        let path = self.resolve(virtual_path)?;
        if !path.exists() {
            return Err(JailError::NotFound(virtual_path.to_owned()));
        }
        if path.is_dir() {
            return Err(JailError::Io(format!("{virtual_path} is a directory")));
        }

        let metadata = fs::metadata(&path).map_err(|e| JailError::Io(e.to_string()))?;
        let len = usize::try_from(metadata.len()).unwrap_or(usize::MAX);
        if len > max_bytes {
            return Err(JailError::SizeLimitExceeded {
                max_bytes,
                actual_bytes: len,
            });
        }

        let mut file = File::open(&path).map_err(|e| JailError::Io(e.to_string()))?;
        let mut buffer = Vec::with_capacity(len);
        file.read_to_end(&mut buffer)
            .map_err(|e| JailError::Io(e.to_string()))?;
        Ok(buffer)
    }

    /// Write bytes to a sandboxed file, enforcing size limits.
    ///
    /// # Errors
    ///
    /// Returns [`JailError`] on permission, limit, or I/O failure.
    pub fn write_bytes(
        &self,
        virtual_path: &str,
        data: &[u8],
        max_bytes: usize,
    ) -> Result<(), JailError> {
        if data.len() > max_bytes {
            return Err(JailError::SizeLimitExceeded {
                max_bytes,
                actual_bytes: data.len(),
            });
        }
        let path = self.resolve(virtual_path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| JailError::Io(e.to_string()))?;
        }
        let mut file = File::create(&path).map_err(|e| JailError::Io(e.to_string()))?;
        file.write_all(data)
            .map_err(|e| JailError::Io(e.to_string()))?;
        file.flush().map_err(|e| JailError::Io(e.to_string()))?;
        Ok(())
    }

    /// Replace a sandboxed file through a same-directory temporary file and atomic rename.
    ///
    /// The target is never truncated before every replacement byte has been written and synced.
    /// This operation is intended for the Linux gateway deployment, where same-filesystem rename
    /// replaces the destination atomically.
    ///
    /// # Errors
    ///
    /// Returns [`JailError`] on boundary, size, temporary-file, sync, or rename failure.
    pub fn replace_bytes_atomic(
        &self,
        virtual_path: &str,
        data: &[u8],
        max_bytes: usize,
    ) -> Result<(), JailError> {
        if data.len() > max_bytes {
            return Err(JailError::SizeLimitExceeded {
                max_bytes,
                actual_bytes: data.len(),
            });
        }
        let target = self.resolve(virtual_path)?;
        let parent = target
            .parent()
            .ok_or_else(|| JailError::InvalidPath(virtual_path.to_string()))?;
        fs::create_dir_all(parent).map_err(|error| JailError::Io(error.to_string()))?;
        let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temporary = parent.join(format!(
            ".cybou-write-{}-{sequence}.tmp",
            std::process::id()
        ));

        let existing_permissions = fs::symlink_metadata(&target)
            .ok()
            .map(|meta| meta.permissions());

        let result = (|| {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
                .map_err(|error| JailError::Io(error.to_string()))?;
            file.write_all(data)
                .map_err(|error| JailError::Io(error.to_string()))?;

            #[cfg(unix)]
            if let Some(perms) = existing_permissions {
                use std::os::unix::fs::PermissionsExt;
                let mode = perms.mode();
                let mut temp_perms = file
                    .metadata()
                    .map_err(|error| JailError::Io(error.to_string()))?
                    .permissions();
                temp_perms.set_mode(mode);
                file.set_permissions(temp_perms)
                    .map_err(|error| JailError::Io(error.to_string()))?;
            }

            #[cfg(not(unix))]
            if let Some(perms) = existing_permissions {
                let _ = file.set_permissions(perms);
            }

            file.sync_all()
                .map_err(|error| JailError::Io(error.to_string()))?;
            fs::rename(&temporary, &target).map_err(|error| JailError::Io(error.to_string()))
        })();
        if result.is_err() {
            drop(fs::remove_file(&temporary));
        }
        result
    }

    /// Create a new file exclusively within the sandbox (`O_CREAT | O_EXCL`).
    ///
    /// # Errors
    ///
    /// Returns [`JailError`] if the file already exists, exceeds limits, or if I/O fails.
    pub fn create_file_exclusive(
        &self,
        virtual_path: &str,
        data: &[u8],
        max_bytes: usize,
    ) -> Result<(), JailError> {
        if data.len() > max_bytes {
            return Err(JailError::SizeLimitExceeded {
                max_bytes,
                actual_bytes: data.len(),
            });
        }
        let target = self.resolve(virtual_path)?;
        if target.exists() {
            return Err(JailError::Io(format!(
                "file already exists: {virtual_path}"
            )));
        }
        let parent = target
            .parent()
            .ok_or_else(|| JailError::InvalidPath(virtual_path.to_string()))?;
        fs::create_dir_all(parent).map_err(|error| JailError::Io(error.to_string()))?;

        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
            .map_err(|error| JailError::Io(error.to_string()))?;
        file.write_all(data)
            .map_err(|error| JailError::Io(error.to_string()))?;
        file.sync_all()
            .map_err(|error| JailError::Io(error.to_string()))?;
        Ok(())
    }

    /// List directory contents within the sandbox.
    ///
    /// # Errors
    ///
    /// Returns [`JailError`] if the directory does not exist or cannot be read.
    pub fn list_dir(&self, virtual_path: &str) -> Result<Vec<DirEntry>, JailError> {
        let path = self.resolve(virtual_path)?;
        if !path.exists() {
            return Err(JailError::NotFound(virtual_path.to_owned()));
        }
        if !path.is_dir() {
            return Err(JailError::Io(format!("{virtual_path} is not a directory")));
        }

        let mut entries = Vec::new();
        let read_dir = fs::read_dir(&path).map_err(|e| JailError::Io(e.to_string()))?;

        for entry_res in read_dir {
            let entry = entry_res.map_err(|e| JailError::Io(e.to_string()))?;
            let name = entry.file_name().to_string_lossy().to_string();
            let metadata = entry.metadata().map_err(|e| JailError::Io(e.to_string()))?;
            let is_dir = metadata.is_dir();
            let size_bytes = if is_dir { 0 } else { metadata.len() };
            entries.push(DirEntry {
                name,
                is_dir,
                size_bytes,
            });
        }

        entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        });

        Ok(entries)
    }

    /// Create directories recursively inside the sandbox.
    ///
    /// # Errors
    ///
    /// Returns [`JailError`] on traversal violation or creation failure.
    pub fn create_dir_all(&self, virtual_path: &str) -> Result<(), JailError> {
        let path = self.resolve(virtual_path)?;
        fs::create_dir_all(&path).map_err(|e| JailError::Io(e.to_string()))
    }

    /// Check if a virtual path exists within the sandbox.
    #[must_use]
    pub fn exists(&self, virtual_path: &str) -> bool {
        self.resolve(virtual_path).is_ok_and(|p| p.exists())
    }

    /// Check if a virtual path is a regular file.
    #[must_use]
    pub fn is_file(&self, virtual_path: &str) -> bool {
        self.resolve(virtual_path).is_ok_and(|p| p.is_file())
    }

    /// Check if a virtual path is a directory.
    #[must_use]
    pub fn is_dir(&self, virtual_path: &str) -> bool {
        self.resolve(virtual_path).is_ok_and(|p| p.is_dir())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_jail() -> (JailFs, PathBuf) {
        let unique = format!(
            "cybou_jail_test_{}_{}",
            std::process::id(),
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        );
        let path = std::env::temp_dir().join(unique);
        let jail = JailFs::new(&path).expect("create test jail");
        (jail, path)
    }

    #[test]
    fn jail_resolves_root_and_nested_paths() {
        let (jail, dir) = test_jail();
        let root_res = jail.resolve("/").expect("resolve root");
        assert_eq!(root_res, jail.root());

        let nested = jail
            .resolve("/workspace/notes.txt")
            .expect("resolve nested");
        assert!(nested.starts_with(jail.root()));
        assert_eq!(nested.file_name().unwrap(), "notes.txt");

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn jail_rejects_parent_traversal_attempts() {
        let (jail, dir) = test_jail();

        let err1 = jail.resolve("../etc/passwd").unwrap_err();
        assert!(matches!(err1, JailError::TraversalAttempt(_)));

        let err2 = jail.resolve("/foo/../../bar").unwrap_err();
        assert!(matches!(err2, JailError::TraversalAttempt(_)));

        let err3 = jail.resolve("a/b/../../../c").unwrap_err();
        assert!(matches!(err3, JailError::TraversalAttempt(_)));

        let err4 = jail.resolve("/foo\0bar").unwrap_err();
        assert!(matches!(err4, JailError::InvalidPath(_)));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn write_and_read_file_within_limits() {
        let (jail, dir) = test_jail();

        let content = "Hello CYBOU Body sandbox!";
        jail.write_bytes("hello.txt", content.as_bytes(), 1024)
            .expect("write file");

        assert!(jail.exists("hello.txt"));
        assert!(jail.is_file("hello.txt"));

        let read_back = jail.read_to_string("hello.txt", 1024).expect("read back");
        assert_eq!(read_back, content);

        let limit_err = jail
            .read_to_string("hello.txt", 5)
            .expect_err("should exceed limit");
        assert!(matches!(limit_err, JailError::SizeLimitExceeded { .. }));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn list_directory_contents() {
        let (jail, dir) = test_jail();

        jail.create_dir_all("docs").expect("create docs dir");
        jail.write_bytes("docs/readme.md", b"# Readme", 100)
            .expect("write readme");
        jail.write_bytes("file1.txt", b"1", 100)
            .expect("write file1");

        let list = jail.list_dir("/").expect("list root");
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "docs");
        assert!(list[0].is_dir);
        assert_eq!(list[1].name, "file1.txt");
        assert!(!list[1].is_dir);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn atomic_replace_preserves_file_permissions() {
        let (jail, dir) = test_jail();

        let target_name = "script.sh";
        jail.write_bytes(target_name, b"#!/bin/sh\necho initial", 1024)
            .expect("initial write");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let resolved = jail.resolve(target_name).expect("resolve path");
            let mut perms = fs::metadata(&resolved).expect("metadata").permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&resolved, perms).expect("set 0755");
        }

        // Atomically replace the contents
        let new_content = b"#!/bin/sh\necho updated";
        jail.replace_bytes_atomic(target_name, new_content, 1024)
            .expect("atomic replace");

        // Verify content updated
        let read_back = jail.read_to_string(target_name, 1024).expect("read back");
        assert_eq!(read_back, "#!/bin/sh\necho updated");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let resolved = jail.resolve(target_name).expect("resolve path");
            let mode = fs::metadata(&resolved).expect("metadata").permissions().mode();
            assert_eq!(mode & 0o777, 0o755, "mode should remain 0755 after atomic replace");
        }

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn create_file_exclusive_fails_if_exists_and_succeeds_for_new() {
        let (jail, dir) = test_jail();

        let filename = "brand_new.txt";
        let content = b"created exclusively";

        jail.create_file_exclusive(filename, content, 1024)
            .expect("create new file");

        let read_back = jail.read_to_string(filename, 1024).expect("read new file");
        assert_eq!(read_back, "created exclusively");

        // Second creation must fail (O_EXCL semantics)
        let err = jail
            .create_file_exclusive(filename, b"duplicate", 1024)
            .expect_err("must fail because file exists");
        assert!(matches!(err, JailError::Io(_)));

        let _ = fs::remove_dir_all(dir);
    }
}
