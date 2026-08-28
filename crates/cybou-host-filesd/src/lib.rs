// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Bounded request protocol and filesystem core for one user's `HostUserPath` owner.

use std::path::{Component, Path, PathBuf};

use cybou_jailfs::JailFs;
use cybou_protocol::LocationRef;
use cybou_web_contracts::{
    DirectoryEntryProjection, FILE_LISTING_MAX_ENTRIES, FILE_READ_MAX_BYTES, FileContentProjection,
    HostDirectoryListingProjection, WEB_SCHEMA_V1,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

/// Maximum encoded request accepted on one connection.
pub const MAX_REQUEST_BYTES: u64 = 16 * 1024;
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
}

/// A bounded owner response.
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "outcome")]
pub enum Response {
    /// Directory listing established by the owner.
    Directory(HostDirectoryListingProjection),
    /// File content established by the owner.
    File(FileContentProjection),
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
        let content_sha256 =
            Sha256::digest(&bytes)
                .iter()
                .fold(String::with_capacity(64), |mut output, byte| {
                    use std::fmt::Write as _;
                    let _ = write!(output, "{byte:02x}");
                    output
                });
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
}
