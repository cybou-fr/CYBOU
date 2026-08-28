// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Bounded filesystem sandbox and path traversal protection for CYBOU Body capabilities.
//!
//! Enforces strict directory confinement so that Shell commands or ephemeral tool capabilities
//! can never escape the designated sandbox boundary (`JailFs`).

#[cfg(not(target_os = "linux"))]
use std::fs::OpenOptions;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
#[cfg(target_os = "linux")]
use std::sync::Arc;
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
    /// Exclusive creation was requested for a path that already exists.
    #[error("file already exists: {0}")]
    AlreadyExists(String),
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
    #[cfg(target_os = "linux")]
    root_dir: Arc<File>,
}

impl JailFs {
    /// Create and validate a parent directory without trusting only the immediate parent.
    #[cfg(not(target_os = "linux"))]
    fn create_confined_parent(&self, parent: &Path, virtual_path: &str) -> Result<(), JailError> {
        let mut existing = parent;
        while !existing.exists() {
            existing = existing
                .parent()
                .ok_or_else(|| JailError::InvalidPath(virtual_path.to_string()))?;
        }
        let canonical_existing = existing
            .canonicalize()
            .map_err(|error| JailError::Io(error.to_string()))?;
        if !canonical_existing.starts_with(&self.canonical_root) {
            return Err(JailError::TraversalAttempt(format!(
                "ancestor directory escapes sandbox boundary: {virtual_path}"
            )));
        }

        fs::create_dir_all(parent).map_err(|error| JailError::Io(error.to_string()))?;
        let canonical_parent = parent
            .canonicalize()
            .map_err(|error| JailError::Io(error.to_string()))?;
        if !canonical_parent.starts_with(&self.canonical_root) {
            return Err(JailError::TraversalAttempt(format!(
                "created parent directory escapes sandbox boundary: {virtual_path}"
            )));
        }
        Ok(())
    }

    #[cfg(all(unix, not(target_os = "linux")))]
    fn sync_directory(path: &Path) -> Result<(), JailError> {
        File::open(path)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| JailError::Io(format!("failed to sync directory: {error}")))?;
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn create_file_exclusive_beneath(
        &self,
        virtual_path: &str,
        data: &[u8],
    ) -> Result<(), JailError> {
        use rustix::fs::{Mode, OFlags, ResolveFlags, fsync, mkdirat, openat2};
        use std::ffi::OsString;
        use std::os::fd::AsFd;

        let components = Path::new(virtual_path)
            .components()
            .filter_map(|component| match component {
                Component::Normal(value) => Some(value.to_os_string()),
                _ => None,
            })
            .collect::<Vec<OsString>>();
        let (file_name, parents) = components
            .split_last()
            .ok_or_else(|| JailError::InvalidPath(virtual_path.to_string()))?;
        let resolve =
            ResolveFlags::BENEATH | ResolveFlags::NO_MAGICLINKS | ResolveFlags::NO_SYMLINKS;
        let directory_flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC;
        let mut directory = openat2(
            self.root_dir.as_fd(),
            ".",
            directory_flags,
            Mode::empty(),
            resolve,
        )
        .map_err(|error| JailError::Io(format!("failed to open jail root dirfd: {error}")))?;

        for component in parents {
            match openat2(
                directory.as_fd(),
                component.as_os_str(),
                directory_flags,
                Mode::empty(),
                resolve,
            ) {
                Ok(next) => directory = next,
                Err(rustix::io::Errno::NOENT) => {
                    match mkdirat(
                        directory.as_fd(),
                        component.as_os_str(),
                        Mode::from_raw_mode(0o755),
                    ) {
                        Ok(()) | Err(rustix::io::Errno::EXIST) => {}
                        Err(error) => return Err(JailError::Io(error.to_string())),
                    }
                    directory = openat2(
                        directory.as_fd(),
                        component.as_os_str(),
                        directory_flags,
                        Mode::empty(),
                        resolve,
                    )
                    .map_err(|error| {
                        JailError::TraversalAttempt(format!(
                            "parent component is not confined for {virtual_path}: {error}"
                        ))
                    })?;
                }
                Err(error) => {
                    return Err(JailError::TraversalAttempt(format!(
                        "parent component is not confined for {virtual_path}: {error}"
                    )));
                }
            }
        }

        let descriptor = openat2(
            directory.as_fd(),
            file_name.as_os_str(),
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::CLOEXEC,
            Mode::from_raw_mode(0o600),
            resolve,
        )
        .map_err(|error| {
            if error == rustix::io::Errno::EXIST {
                JailError::AlreadyExists(virtual_path.to_string())
            } else {
                JailError::Io(error.to_string())
            }
        })?;
        let mut file = File::from(descriptor);
        file.write_all(data)
            .map_err(|error| JailError::Io(error.to_string()))?;
        file.sync_all()
            .map_err(|error| JailError::Io(error.to_string()))?;
        fsync(directory.as_fd())
            .map_err(|error| JailError::Io(format!("failed to sync directory: {error}")))?;
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn preserve_linux_metadata(existing: &File, replacement: &File) -> Result<(), JailError> {
        use rustix::fs::{
            Gid, Mode, Uid, XattrFlags, fchmod, fchown, fgetxattr, flistxattr, fsetxattr, fstat,
        };
        use std::ffi::OsString;
        use std::os::fd::AsFd;
        use std::os::unix::ffi::{OsStrExt, OsStringExt};

        let stat = fstat(existing.as_fd()).map_err(|error| JailError::Io(error.to_string()))?;
        fchown(
            replacement.as_fd(),
            Some(Uid::from_raw(stat.st_uid)),
            Some(Gid::from_raw(stat.st_gid)),
        )
        .map_err(|error| JailError::Io(format!("failed to preserve owner/group: {error}")))?;
        let mut names = vec![0_u8; 64 * 1024];
        let names_len = flistxattr(existing.as_fd(), &mut names)
            .map_err(|error| JailError::Io(format!("failed to list file xattrs: {error}")))?;
        names.truncate(names_len);
        for name in names
            .split(|byte| *byte == 0)
            .filter(|name| !name.is_empty())
        {
            let name = OsString::from_vec(name.to_vec());
            let mut value = vec![0_u8; 64 * 1024];
            let value_len =
                fgetxattr(existing.as_fd(), name.as_os_str(), &mut value).map_err(|error| {
                    JailError::Io(format!(
                        "failed to read xattr {}: {error}",
                        name.as_os_str().as_bytes().escape_ascii()
                    ))
                })?;
            value.truncate(value_len);
            fsetxattr(
                replacement.as_fd(),
                name.as_os_str(),
                &value,
                XattrFlags::empty(),
            )
            .map_err(|error| {
                JailError::Io(format!(
                    "failed to preserve xattr {}: {error}",
                    name.as_os_str().as_bytes().escape_ascii()
                ))
            })?;
        }
        fchmod(
            replacement.as_fd(),
            Mode::from_raw_mode(stat.st_mode & 0o7777),
        )
        .map_err(|error| JailError::Io(format!("failed to preserve file mode: {error}")))
    }

    #[cfg(target_os = "linux")]
    fn replace_bytes_atomic_beneath(
        &self,
        virtual_path: &str,
        data: &[u8],
    ) -> Result<(), JailError> {
        use rustix::fs::{AtFlags, Mode, OFlags, ResolveFlags, fsync, openat2, renameat, unlinkat};
        use std::ffi::OsString;
        use std::os::fd::AsFd;

        let components = Path::new(virtual_path)
            .components()
            .filter_map(|component| match component {
                Component::Normal(value) => Some(value.to_os_string()),
                _ => None,
            })
            .collect::<Vec<OsString>>();
        let (file_name, parents) = components
            .split_last()
            .ok_or_else(|| JailError::InvalidPath(virtual_path.to_string()))?;
        let resolve =
            ResolveFlags::BENEATH | ResolveFlags::NO_MAGICLINKS | ResolveFlags::NO_SYMLINKS;
        let directory_flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC;
        let mut directory = openat2(
            self.root_dir.as_fd(),
            ".",
            directory_flags,
            Mode::empty(),
            resolve,
        )
        .map_err(|error| JailError::Io(format!("failed to open jail root dirfd: {error}")))?;
        for component in parents {
            directory = openat2(
                directory.as_fd(),
                component.as_os_str(),
                directory_flags,
                Mode::empty(),
                resolve,
            )
            .map_err(|error| {
                JailError::TraversalAttempt(format!(
                    "parent component is not confined for {virtual_path}: {error}"
                ))
            })?;
        }

        let existing = openat2(
            directory.as_fd(),
            file_name.as_os_str(),
            OFlags::RDONLY | OFlags::CLOEXEC,
            Mode::empty(),
            resolve,
        )
        .map_err(|error| {
            if error == rustix::io::Errno::NOENT {
                JailError::NotFound(virtual_path.to_string())
            } else {
                JailError::TraversalAttempt(format!(
                    "replacement target is not confined for {virtual_path}: {error}"
                ))
            }
        })?;
        let existing = File::from(existing);

        let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temporary = format!(".cybou-write-{}-{sequence}.tmp", std::process::id());
        let result = (|| {
            let descriptor = openat2(
                directory.as_fd(),
                temporary.as_str(),
                OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::CLOEXEC,
                Mode::from_raw_mode(0o600),
                resolve,
            )
            .map_err(|error| JailError::Io(error.to_string()))?;
            let mut file = File::from(descriptor);
            file.write_all(data)
                .map_err(|error| JailError::Io(error.to_string()))?;
            Self::preserve_linux_metadata(&existing, &file)?;
            file.sync_all()
                .map_err(|error| JailError::Io(error.to_string()))?;
            renameat(
                directory.as_fd(),
                temporary.as_str(),
                directory.as_fd(),
                file_name.as_os_str(),
            )
            .map_err(|error| JailError::Io(error.to_string()))?;
            fsync(directory.as_fd())
                .map_err(|error| JailError::Io(format!("failed to sync directory: {error}")))?;
            Ok(())
        })();
        if result.is_err() {
            let _ = unlinkat(directory.as_fd(), temporary.as_str(), AtFlags::empty());
        }
        result
    }

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
        #[cfg(target_os = "linux")]
        let root_dir = Arc::new(File::open(&canonical_root).map_err(|error| {
            JailError::Io(format!("failed to open jail root directory: {error}"))
        })?);
        Ok(Self {
            canonical_root,
            #[cfg(target_os = "linux")]
            root_dir,
        })
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
        #[cfg(target_os = "linux")]
        {
            let _ = self.resolve(virtual_path)?;
            self.replace_bytes_atomic_beneath(virtual_path, data)
        }

        #[cfg(not(target_os = "linux"))]
        {
            let target = self.resolve(virtual_path)?;
            let parent = target
                .parent()
                .ok_or_else(|| JailError::InvalidPath(virtual_path.to_string()))?;
            self.create_confined_parent(parent, virtual_path)?;
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
                fs::rename(&temporary, &target)
                    .map_err(|error| JailError::Io(error.to_string()))?;
                #[cfg(unix)]
                Self::sync_directory(parent)?;
                Ok(())
            })();
            if result.is_err() {
                drop(fs::remove_file(&temporary));
            }
            result
        }
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
        #[cfg(target_os = "linux")]
        {
            // Validate syntax before passing the relative path to the kernel-enforced boundary.
            let _ = self.resolve(virtual_path)?;
            self.create_file_exclusive_beneath(virtual_path, data)
        }

        #[cfg(not(target_os = "linux"))]
        {
            let target = self.resolve(virtual_path)?;
            if target.exists() {
                return Err(JailError::AlreadyExists(virtual_path.to_string()));
            }
            let parent = target
                .parent()
                .ok_or_else(|| JailError::InvalidPath(virtual_path.to_string()))?;
            self.create_confined_parent(parent, virtual_path)?;

            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&target)
                .map_err(|error| {
                    if error.kind() == std::io::ErrorKind::AlreadyExists {
                        JailError::AlreadyExists(virtual_path.to_string())
                    } else {
                        JailError::Io(error.to_string())
                    }
                })?;
            file.write_all(data)
                .map_err(|error| JailError::Io(error.to_string()))?;
            file.sync_all()
                .map_err(|error| JailError::Io(error.to_string()))?;
            #[cfg(unix)]
            Self::sync_directory(parent)?;
            Ok(())
        }
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

    /// Create a single directory inside the sandbox.
    ///
    /// # Errors
    ///
    /// Returns [`JailError`] on traversal violation, existing target, or creation failure.
    pub fn create_dir(&self, virtual_path: &str) -> Result<(), JailError> {
        let path = self.resolve(virtual_path)?;
        if path.exists() {
            return Err(JailError::AlreadyExists(virtual_path.to_string()));
        }
        fs::create_dir(&path).map_err(|e| JailError::Io(e.to_string()))
    }

    /// Remove a file or directory within the sandbox.
    ///
    /// Refuses to remove the sandbox root.
    ///
    /// # Errors
    ///
    /// Returns [`JailError`] if the path does not exist, attempts to delete root, or I/O fails.
    pub fn remove_path(&self, virtual_path: &str, recursive: bool) -> Result<(), JailError> {
        let path = self.resolve(virtual_path)?;
        if path == self.canonical_root {
            return Err(JailError::TraversalAttempt(
                "cannot delete jail root".to_string(),
            ));
        }
        if !path.exists() {
            return Err(JailError::NotFound(virtual_path.to_string()));
        }
        if path.is_dir() {
            if recursive {
                fs::remove_dir_all(&path).map_err(|e| JailError::Io(e.to_string()))?;
            } else {
                fs::remove_dir(&path).map_err(|e| JailError::Io(e.to_string()))?;
            }
        } else {
            fs::remove_file(&path).map_err(|e| JailError::Io(e.to_string()))?;
        }
        Ok(())
    }

    /// Rename or move a path within the sandbox.
    ///
    /// Refuses to move the sandbox root.
    ///
    /// # Errors
    ///
    /// Returns [`JailError`] on traversal attempt, missing source, or I/O failure.
    pub fn rename_path(&self, from_virtual: &str, to_virtual: &str) -> Result<(), JailError> {
        let from = self.resolve(from_virtual)?;
        let to = self.resolve(to_virtual)?;
        if from == self.canonical_root || to == self.canonical_root {
            return Err(JailError::TraversalAttempt(
                "cannot move jail root".to_string(),
            ));
        }
        if !from.exists() {
            return Err(JailError::NotFound(from_virtual.to_string()));
        }
        if let Some(parent) = to.parent()
            && !parent.exists()
        {
            fs::create_dir_all(parent).map_err(|e| JailError::Io(e.to_string()))?;
        }
        fs::rename(&from, &to).map_err(|e| JailError::Io(e.to_string()))
    }

    /// Copy a file or directory tree within the sandbox.
    ///
    /// # Errors
    ///
    /// Returns [`JailError`] on traversal attempt, missing source, or I/O failure.
    pub fn copy_path(&self, from_virtual: &str, to_virtual: &str) -> Result<(), JailError> {
        let from = self.resolve(from_virtual)?;
        let to = self.resolve(to_virtual)?;
        if !from.exists() {
            return Err(JailError::NotFound(from_virtual.to_string()));
        }
        if let Some(parent) = to.parent()
            && !parent.exists()
        {
            fs::create_dir_all(parent).map_err(|e| JailError::Io(e.to_string()))?;
        }
        if from.is_dir() {
            fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
                fs::create_dir_all(dst)?;
                for entry in fs::read_dir(src)? {
                    let entry = entry?;
                    let ty = entry.file_type()?;
                    let target = dst.join(entry.file_name());
                    if ty.is_dir() {
                        copy_dir_recursive(&entry.path(), &target)?;
                    } else {
                        fs::copy(entry.path(), target)?;
                    }
                }
                Ok(())
            }
            copy_dir_recursive(&from, &to).map_err(|e| JailError::Io(e.to_string()))?;
        } else {
            fs::copy(&from, &to).map_err(|e| JailError::Io(e.to_string()))?;
        }
        Ok(())
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
            let mode = fs::metadata(&resolved)
                .expect("metadata")
                .permissions()
                .mode();
            assert_eq!(
                mode & 0o777,
                0o755,
                "mode should remain 0755 after atomic replace"
            );
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
        assert!(matches!(err, JailError::AlreadyExists(_)));

        let _ = fs::remove_dir_all(dir);
    }

    #[cfg(unix)]
    #[test]
    fn exclusive_create_rejects_symlink_in_missing_parent_chain() {
        use std::os::unix::fs::symlink;

        let (jail, dir) = test_jail();
        let (_outside_jail, outside) = test_jail();
        symlink(&outside, dir.join("link")).expect("create escaping symlink");

        let error = jail
            .create_file_exclusive("link/new-dir/file.txt", b"must stay jailed", 1024)
            .expect_err("escaping nested create must fail");

        assert!(matches!(error, JailError::TraversalAttempt(_)));
        assert!(!outside.join("new-dir").exists());

        let _ = fs::remove_dir_all(dir);
        let _ = fs::remove_dir_all(outside);
    }

    #[cfg(unix)]
    #[test]
    fn atomic_replace_rejects_symlink_target() {
        use std::os::unix::fs::symlink;

        let (jail, dir) = test_jail();
        jail.write_bytes("actual.txt", b"original", 1024)
            .expect("create actual file");
        symlink(dir.join("actual.txt"), dir.join("alias.txt")).expect("create symlink alias");

        let error = jail
            .replace_bytes_atomic("alias.txt", b"replacement", 1024)
            .expect_err("replacement through a symlink must fail");

        assert!(matches!(error, JailError::TraversalAttempt(_)));
        assert_eq!(
            jail.read_to_string("actual.txt", 1024)
                .expect("read actual file"),
            "original"
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn atomic_replace_preserves_owner_group_and_xattrs() {
        use rustix::fs::{XattrFlags, fgetxattr, fsetxattr};
        use std::os::fd::AsFd;
        use std::os::unix::fs::MetadataExt;

        let (jail, dir) = test_jail();
        jail.write_bytes("metadata.txt", b"original", 1024)
            .expect("create metadata test file");
        let resolved = jail.resolve("metadata.txt").expect("resolve file");
        let original = fs::metadata(&resolved).expect("original metadata");
        let file = File::open(&resolved).expect("open original file");
        fsetxattr(
            file.as_fd(),
            "user.cybou-test",
            b"preserved",
            XattrFlags::empty(),
        )
        .expect("set test xattr");

        jail.replace_bytes_atomic("metadata.txt", b"replacement", 1024)
            .expect("replace file with metadata");

        let replaced = fs::metadata(&resolved).expect("replacement metadata");
        assert_eq!(replaced.uid(), original.uid());
        assert_eq!(replaced.gid(), original.gid());
        let replaced_file = File::open(&resolved).expect("open replacement file");
        let mut value = vec![0_u8; 64];
        let length = fgetxattr(replaced_file.as_fd(), "user.cybou-test", &mut value)
            .expect("read preserved xattr");
        value.truncate(length);
        assert_eq!(value, b"preserved");

        let _ = fs::remove_dir_all(dir);
    }
}
