// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Bounded request protocol and filesystem core for one user's `HostUserPath` owner.

use std::path::{Component, Path, PathBuf};

use cybou_jailfs::JailFs;
use cybou_protocol::LocationRef;
use cybou_web_contracts::{
    DirectoryEntryProjection, FILE_LISTING_MAX_ENTRIES, FILE_READ_MAX_BYTES, FILE_WRITE_MAX_BYTES,
    FileContentProjection, FileWriteProjection, HostDirectoryListingProjection, WEB_SCHEMA_V1,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

/// Maximum encoded request accepted on one connection.
pub const MAX_REQUEST_BYTES: u64 = 1024 * 1024;
/// Maximum time allowed to receive one request.
pub const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// One operation requested by the gateway.
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "operation")]
pub enum Request {
    /// List a directory in this owner's home.
    ListDirectory {
        /// Absolute path inside the owner's established home.
        path: String,
    },
    /// Read a bounded UTF-8 file in this owner's home.
    ReadFile {
        /// Absolute path inside the owner's established home.
        path: String,
    },
    /// Conditionally write an existing file with atomic replacement.
    WriteFile {
        /// Absolute path inside the owner's established home.
        path: String,
        /// Expected SHA-256 hash before replacement, if known.
        expected_sha256: Option<String>,
        /// Replacement UTF-8 text content.
        text: String,
    },
    /// Create a new file inside the owner's home.
    CreateFile {
        /// Absolute path inside the owner's established home.
        path: String,
        /// Initial UTF-8 text content.
        text: String,
        /// Whether creation must be exclusive (`O_CREAT | O_EXCL`).
        exclusive: bool,
    },
    /// Create a directory inside the owner's home.
    CreateDirectory {
        /// Absolute path inside the owner's established home.
        path: String,
        /// Whether parent directories should be created if missing.
        recursive: bool,
    },
    /// Rename or move a path inside the owner's home.
    RenamePath {
        /// Source absolute path.
        from_path: String,
        /// Destination absolute path.
        to_path: String,
    },
    /// Delete a file or directory inside the owner's home.
    DeletePath {
        /// Absolute path to remove.
        path: String,
        /// Whether directories should be removed recursively.
        recursive: bool,
    },
    /// Copy a file or directory tree inside the owner's home.
    CopyPath {
        /// Source absolute path.
        from_path: String,
        /// Destination absolute path.
        to_path: String,
    },
}

/// A bounded owner response.
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "outcome")]
pub enum Response {
    /// Directory listing established by the owner.
    Directory(HostDirectoryListingProjection),
    /// File content established by the owner.
    File(FileContentProjection),
    /// Verified result of a file write or create.
    Written(FileWriteProjection),
    /// Successful completion of a mutation (mkdir, rename, delete, copy).
    Success,
    /// Indistinguishable filesystem refusal.
    Refused,
}

/// Filesystem authority held by one unprivileged user process.
pub struct Owner {
    home: PathBuf,
    jail: JailFs,
}

impl Owner {
    /// Bind an owner core to its home directory.
    ///
    /// # Errors
    ///
    /// Returns an error when the home cannot be created, opened, or canonicalized as a jail root.
    pub fn new(home: impl AsRef<Path>) -> Result<Self, cybou_jailfs::JailError> {
        let jail = JailFs::new(home)?;
        Ok(Self {
            home: jail.root().to_owned(),
            jail,
        })
    }

    /// Answer one request without disclosing filesystem error details.
    #[must_use]
    pub fn answer(&self, request: Request) -> Response {
        match request {
            Request::ListDirectory { path } => self
                .list_directory(&path)
                .map_or(Response::Refused, Response::Directory),
            Request::ReadFile { path } => self
                .read_file(&path)
                .map_or(Response::Refused, Response::File),
            Request::WriteFile {
                path,
                expected_sha256,
                text,
            } => self
                .write_file(&path, &text, expected_sha256.as_deref())
                .map_or(Response::Refused, Response::Written),
            Request::CreateFile {
                path,
                text,
                exclusive,
            } => self
                .create_file(&path, &text, exclusive)
                .map_or(Response::Refused, Response::Written),
            Request::CreateDirectory { path, recursive } => self
                .create_directory(&path, recursive)
                .map_or(Response::Refused, |_| Response::Success),
            Request::RenamePath { from_path, to_path } => self
                .rename_path(&from_path, &to_path)
                .map_or(Response::Refused, |_| Response::Success),
            Request::DeletePath { path, recursive } => self
                .delete_path(&path, recursive)
                .map_or(Response::Refused, |_| Response::Success),
            Request::CopyPath { from_path, to_path } => self
                .copy_path(&from_path, &to_path)
                .map_or(Response::Refused, |_| Response::Success),
        }
    }

    fn relative_path<'a>(&self, requested: &'a str) -> Option<&'a Path> {
        let path = Path::new(requested);
        if !path.is_absolute()
            || path
                .components()
                .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
        {
            return None;
        }
        path.strip_prefix(&self.home).ok()
    }

    fn compute_sha256(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .fold(String::with_capacity(64), |mut output, byte| {
                use std::fmt::Write as _;
                let _ = write!(output, "{byte:02x}");
                output
            })
    }

    fn list_directory(&self, requested: &str) -> Option<HostDirectoryListingProjection> {
        let relative = self.relative_path(requested)?;
        let relative = relative.to_str()?;
        let all = self.jail.list_dir(relative).ok()?;
        let total_entries = u32::try_from(all.len()).unwrap_or(u32::MAX);
        let entries = all
            .into_iter()
            .take(FILE_LISTING_MAX_ENTRIES)
            .map(|entry| DirectoryEntryProjection {
                name: entry.name,
                is_dir: entry.is_dir,
                size_bytes: entry.size_bytes,
            })
            .collect::<Vec<_>>();
        Some(HostDirectoryListingProjection {
            schema_version: WEB_SCHEMA_V1,
            location: LocationRef::HostUserPath(requested.to_owned()),
            truncated: usize::try_from(total_entries).unwrap_or(usize::MAX) > entries.len(),
            total_entries,
            entries,
        })
    }

    fn read_file(&self, requested: &str) -> Option<FileContentProjection> {
        let relative = self.relative_path(requested)?;
        let bytes = self
            .jail
            .read_bytes(relative.to_str()?, FILE_READ_MAX_BYTES)
            .ok()?;
        let size_bytes = u64::try_from(bytes.len()).ok()?;
        let content_sha256 = Self::compute_sha256(&bytes);
        let text = String::from_utf8(bytes).ok()?;
        Some(FileContentProjection {
            schema_version: WEB_SCHEMA_V1,
            path: requested.to_owned(),
            location: LocationRef::HostUserPath(requested.to_owned()),
            text,
            size_bytes,
            content_sha256,
        })
    }

    fn write_file(
        &self,
        requested: &str,
        text: &str,
        expected_sha256: Option<&str>,
    ) -> Option<FileWriteProjection> {
        let relative = self.relative_path(requested)?;
        let relative_str = relative.to_str()?;

        if let Some(expected) = expected_sha256 {
            let current_bytes = self
                .jail
                .read_bytes(relative_str, FILE_READ_MAX_BYTES)
                .ok()?;
            let current_sha = Self::compute_sha256(&current_bytes);
            if current_sha != expected {
                return None;
            }
        }

        self.jail
            .replace_bytes_atomic(relative_str, text.as_bytes(), FILE_WRITE_MAX_BYTES)
            .ok()?;

        let re_read = self
            .jail
            .read_bytes(relative_str, FILE_READ_MAX_BYTES)
            .ok()?;
        let content_sha256 = Self::compute_sha256(&re_read);
        let size_bytes = u64::try_from(re_read.len()).ok()?;

        Some(FileWriteProjection {
            schema_version: WEB_SCHEMA_V1,
            location: LocationRef::HostUserPath(requested.to_owned()),
            content_sha256,
            size_bytes,
        })
    }

    fn create_file(
        &self,
        requested: &str,
        text: &str,
        exclusive: bool,
    ) -> Option<FileWriteProjection> {
        let relative = self.relative_path(requested)?;
        let relative_str = relative.to_str()?;

        if exclusive {
            self.jail
                .create_file_exclusive(relative_str, text.as_bytes(), FILE_WRITE_MAX_BYTES)
                .ok()?;
        } else {
            self.jail
                .write_bytes(relative_str, text.as_bytes(), FILE_WRITE_MAX_BYTES)
                .ok()?;
        }

        let re_read = self
            .jail
            .read_bytes(relative_str, FILE_READ_MAX_BYTES)
            .ok()?;
        let content_sha256 = Self::compute_sha256(&re_read);
        let size_bytes = u64::try_from(re_read.len()).ok()?;

        Some(FileWriteProjection {
            schema_version: WEB_SCHEMA_V1,
            location: LocationRef::HostUserPath(requested.to_owned()),
            content_sha256,
            size_bytes,
        })
    }

    fn create_directory(&self, requested: &str, recursive: bool) -> Option<()> {
        let relative = self.relative_path(requested)?;
        let relative_str = relative.to_str()?;
        if recursive {
            self.jail.create_dir_all(relative_str).ok()
        } else {
            self.jail.create_dir(relative_str).ok()
        }
    }

    fn rename_path(&self, from_req: &str, to_req: &str) -> Option<()> {
        let from_rel = self.relative_path(from_req)?;
        let to_rel = self.relative_path(to_req)?;
        self.jail
            .rename_path(from_rel.to_str()?, to_rel.to_str()?)
            .ok()
    }

    fn delete_path(&self, requested: &str, recursive: bool) -> Option<()> {
        let relative = self.relative_path(requested)?;
        self.jail.remove_path(relative.to_str()?, recursive).ok()
    }

    fn copy_path(&self, from_req: &str, to_req: &str) -> Option<()> {
        let from_rel = self.relative_path(from_req)?;
        let to_rel = self.relative_path(to_req)?;
        self.jail
            .copy_path(from_rel.to_str()?, to_rel.to_str()?)
            .ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_reads_inside_home_and_refuses_escape_and_symlink() {
        let root = tempfile::tempdir().expect("home");
        std::fs::write(root.path().join("note.txt"), "hello").expect("file");
        let owner = Owner::new(root.path()).expect("owner");
        let inside = root.path().join("note.txt").display().to_string();

        assert!(matches!(
            owner.answer(Request::ReadFile { path: inside }),
            Response::File(file) if file.text == "hello"
        ));
        assert_eq!(
            owner.answer(Request::ReadFile {
                path: root.path().join("../outside").display().to_string(),
            }),
            Response::Refused
        );
    }

    #[test]
    fn listing_is_owner_issued_and_sorted_by_the_jail() {
        let root = tempfile::tempdir().expect("home");
        std::fs::write(root.path().join("b.txt"), "b").expect("file");
        std::fs::create_dir(root.path().join("a")).expect("directory");
        let owner = Owner::new(root.path()).expect("owner");
        let path = root.path().display().to_string();

        let Response::Directory(listing) =
            owner.answer(Request::ListDirectory { path: path.clone() })
        else {
            panic!("listing refused");
        };
        assert_eq!(listing.location, LocationRef::HostUserPath(path));
        assert_eq!(listing.total_entries, 2);
        assert_eq!(listing.entries[0].name, "a");
    }

    #[test]
    fn owner_writes_creates_and_deletes_files_safely() {
        let root = tempfile::tempdir().expect("home");
        let owner = Owner::new(root.path()).expect("owner");
        let file_path = root.path().join("doc.txt").display().to_string();

        // Create
        let res = owner.answer(Request::CreateFile {
            path: file_path.clone(),
            text: "initial".to_string(),
            exclusive: true,
        });
        let Response::Written(proj) = res else {
            panic!("create failed");
        };
        assert_eq!(proj.size_bytes, 7);

        // Write with matching expected sha
        let res = owner.answer(Request::WriteFile {
            path: file_path.clone(),
            expected_sha256: Some(proj.content_sha256),
            text: "updated".to_string(),
        });
        assert!(matches!(res, Response::Written(_)));

        // Write with mismatched sha -> Refused
        let res = owner.answer(Request::WriteFile {
            path: file_path.clone(),
            expected_sha256: Some("bad_sha".to_string()),
            text: "bad".to_string(),
        });
        assert_eq!(res, Response::Refused);

        // Delete
        let res = owner.answer(Request::DeletePath {
            path: file_path.clone(),
            recursive: false,
        });
        assert_eq!(res, Response::Success);
    }
}
