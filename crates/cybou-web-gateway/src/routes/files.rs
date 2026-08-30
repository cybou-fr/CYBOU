// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Reading the sandbox as structure rather than as terminal output.
//!
//! The File Manager used to ask the Shell for `ls -la` and parse the columns back into names,
//! kinds and sizes. That is a typed filesystem turned into text and then guessed at, and it failed
//! the way such loops always fail: the parser expected nine whitespace-separated fields, the
//! engine's own long format produced six, so every entry fell through both branches and the panel
//! showed an empty directory. Nothing reported an error, because from the parser's point of view
//! there was simply nothing there.
//!
//! These routes hand back what `cybou-jailfs` already established. They share the sandbox and the
//! entitlement boundary with the Shell, and they do not go through it.

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use base64::Engine as _;
use cybou_jailfs::JailError;
use cybou_protocol::LocationRef;
use cybou_web_contracts::{
    DirectoryEntryProjection, DirectoryListingProjection, FILE_LISTING_MAX_ENTRIES,
    FILE_READ_MAX_BYTES, FILE_TRANSFER_MAX_BYTES, FILE_WRITE_MAX_BYTES, FileContentProjection,
    FileCreateRequest, FilePathRequest, FileUploadProjection, FileUploadRequest,
    FileWriteProjection, FileWriteRequest, WEB_SCHEMA_V1,
};
use sha2::{Digest as _, Sha256};
use std::sync::{Mutex, PoisonError};

use crate::state::{ErrorBody, GatewayState};

/// One typed refusal.
type Refusal = (StatusCode, Json<ErrorBody>);

/// Serializes compare-and-replace writes so two gateway requests cannot both win one version.
static FILE_WRITES: Mutex<()> = Mutex::new(());

/// Mint a non-authorizing reference to a path established inside this request's jail.
///
/// The scope distinguishes the local desktop from authenticated browser seats without exposing a
/// bearer token. Route authorization is still performed from the request session on every call;
/// this reference is identity and authority-domain evidence, not a transferable capability.
fn jail_location(owner: &crate::shells::ShellOwner, path: String) -> LocationRef {
    let session_id = match owner {
        crate::shells::ShellOwner::LocalDesktop { .. } => "local-desktop".to_string(),
        crate::shells::ShellOwner::Session { session, .. } => {
            let mut id = String::from("session-");
            for byte in &session[..8] {
                use std::fmt::Write as _;
                let _ = write!(id, "{byte:02x}");
            }
            id
        }
    };
    LocationRef::SafeShellJail { session_id, path }
}

/// The refusal a caller who holds no seat receives.
///
/// The same boundary as Shell execution, and the same answer: reading the sandbox is a Body
/// capability, and a public preview holds none.
fn no_seat() -> Refusal {
    (
        StatusCode::FORBIDDEN,
        Json(ErrorBody {
            schema_version: WEB_SCHEMA_V1,
            error: "shellExecutionForbiddenInPublicPreview",
            retryable: false,
        }),
    )
}

/// What a sandbox failure looks like at the boundary.
///
/// A path that left the sandbox is answered exactly as a path that does not exist. Telling a caller
/// which of its guesses escaped is telling it where the sandbox ends, and a caller entitled to read
/// inside it is not thereby entitled to map its edge.
fn boundary(error: &JailError) -> Refusal {
    let (status, code, retryable) = match error {
        JailError::NotFound(_) | JailError::TraversalAttempt(_) | JailError::InvalidPath(_) => {
            (StatusCode::NOT_FOUND, "pathNotReadable", false)
        }
        JailError::SizeLimitExceeded { .. } => {
            (StatusCode::PAYLOAD_TOO_LARGE, "fileTooLarge", false)
        }
        JailError::AlreadyExists(_) => (StatusCode::CONFLICT, "fileAlreadyExists", false),
        JailError::Io(_) => (StatusCode::BAD_GATEWAY, "sandboxUnreadable", true),
    };
    (
        status,
        Json(ErrorBody {
            schema_version: WEB_SCHEMA_V1,
            error: code,
            retryable,
        }),
    )
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut output, byte| {
            use std::fmt::Write as _;
            let _ = write!(output, "{byte:02x}");
            output
        })
}

fn conflict() -> Refusal {
    (
        StatusCode::CONFLICT,
        Json(ErrorBody {
            schema_version: WEB_SCHEMA_V1,
            error: "fileChangedSinceRead",
            retryable: false,
        }),
    )
}

/// List one directory inside the sandbox.
///
/// # Errors
///
/// Refuses if the caller holds no seat, or the path names nothing readable.
pub async fn list_directory_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<FilePathRequest>,
) -> Result<Json<DirectoryListingProjection>, Refusal> {
    state.shell_seat(&headers).ok_or_else(no_seat)?;

    let entries = state
        .files
        .list_dir(&payload.path)
        .map_err(|error| boundary(&error))?;

    let total = u32::try_from(entries.len()).unwrap_or(u32::MAX);
    let truncated = entries.len() > FILE_LISTING_MAX_ENTRIES;

    Ok(Json(DirectoryListingProjection {
        schema_version: WEB_SCHEMA_V1,
        path: payload.path,
        entries: entries
            .into_iter()
            .take(FILE_LISTING_MAX_ENTRIES)
            .map(|entry| DirectoryEntryProjection {
                name: entry.name,
                is_dir: entry.is_dir,
                size_bytes: entry.size_bytes,
            })
            .collect(),
        total_entries: total,
        truncated,
    }))
}

/// Read one text file inside the sandbox.
///
/// # Errors
///
/// Refuses if the caller holds no seat, or the path names nothing readable within the byte budget.
pub async fn read_file_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<FilePathRequest>,
) -> Result<Json<FileContentProjection>, Refusal> {
    let owner = state.shell_seat(&headers).ok_or_else(no_seat)?;

    // The size is read before the bytes so a file too large to serve can still say how large it is.
    // Answering "cannot show this" without saying what was too big is a refusal a person cannot act
    // on.
    let size_bytes = state
        .files
        .resolve(&payload.path)
        .ok()
        .and_then(|resolved| std::fs::metadata(resolved).ok())
        .map_or(0, |metadata| metadata.len());

    let text = state
        .files
        .read_to_string(&payload.path, FILE_READ_MAX_BYTES)
        .map_err(|error| boundary(&error))?;

    Ok(Json(FileContentProjection {
        schema_version: WEB_SCHEMA_V1,
        path: payload.path.clone(),
        location: jail_location(&owner, payload.path),
        content_sha256: sha256(text.as_bytes()),
        text,
        size_bytes,
    }))
}

/// Conditionally replace one UTF-8 file inside the request owner's sandbox.
///
/// # Errors
///
/// Returns a governed refusal when the request has no private seat, the
/// location was not issued for that seat, the payload exceeds the write
/// limit, the expected digest is stale, or the sandbox cannot atomically
/// replace and verify the file.
pub async fn write_file_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<FileWriteRequest>,
) -> Result<Json<FileWriteProjection>, Refusal> {
    let owner = state.shell_seat(&headers).ok_or_else(no_seat)?;
    let LocationRef::SafeShellJail { path, .. } = &payload.location else {
        return Err(no_seat());
    };
    if jail_location(&owner, path.clone()) != payload.location {
        return Err(no_seat());
    }
    if payload.text.len() > FILE_WRITE_MAX_BYTES {
        return Err(boundary(&JailError::SizeLimitExceeded {
            max_bytes: FILE_WRITE_MAX_BYTES,
            actual_bytes: payload.text.len(),
        }));
    }

    let _write_guard = FILE_WRITES.lock().unwrap_or_else(PoisonError::into_inner);
    let current = state
        .files
        .read_to_string(path, FILE_READ_MAX_BYTES)
        .map_err(|error| boundary(&error))?;
    if sha256(current.as_bytes()) != payload.expected_sha256 {
        return Err(conflict());
    }
    state
        .files
        .replace_bytes_atomic(path, payload.text.as_bytes(), FILE_WRITE_MAX_BYTES)
        .map_err(|error| boundary(&error))?;
    let verified = state
        .files
        .read_to_string(path, FILE_READ_MAX_BYTES)
        .map_err(|error| boundary(&error))?;
    if verified != payload.text {
        return Err(boundary(&JailError::Io(
            "post-write verification failed".to_string(),
        )));
    }

    Ok(Json(FileWriteProjection {
        schema_version: WEB_SCHEMA_V1,
        location: payload.location,
        content_sha256: sha256(verified.as_bytes()),
        size_bytes: u64::try_from(verified.len()).unwrap_or(u64::MAX),
    }))
}

/// Exclusively create a new UTF-8 file inside the request owner's sandbox.
///
/// # Errors
///
/// Returns a governed refusal when the request has no private seat, the
/// location was not issued for that seat, the payload exceeds the write
/// limit, the file already exists, or the sandbox cannot create the file.
pub async fn create_file_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<FileCreateRequest>,
) -> Result<Json<FileWriteProjection>, Refusal> {
    let owner = state.shell_seat(&headers).ok_or_else(no_seat)?;
    let location = jail_location(&owner, payload.path.clone());
    if payload.text.len() > FILE_WRITE_MAX_BYTES {
        return Err(boundary(&JailError::SizeLimitExceeded {
            max_bytes: FILE_WRITE_MAX_BYTES,
            actual_bytes: payload.text.len(),
        }));
    }

    let _write_guard = FILE_WRITES.lock().unwrap_or_else(PoisonError::into_inner);
    state
        .files
        .create_file_exclusive(&payload.path, payload.text.as_bytes(), FILE_WRITE_MAX_BYTES)
        .map_err(|error| boundary(&error))?;
    let verified = state
        .files
        .read_to_string(&payload.path, FILE_READ_MAX_BYTES)
        .map_err(|error| boundary(&error))?;
    if verified != payload.text {
        return Err(boundary(&JailError::Io(
            "post-creation verification failed".to_string(),
        )));
    }

    Ok(Json(FileWriteProjection {
        schema_version: WEB_SCHEMA_V1,
        location,
        content_sha256: sha256(verified.as_bytes()),
        size_bytes: u64::try_from(verified.len()).unwrap_or(u64::MAX),
    }))
}

/// The bytes a download hands back, and the name a browser should save them under.
///
/// Not a JSON projection, because the payload is the file. Everything the caller needs to save it
/// travels in the headers, which is where a browser already looks for it.
pub struct FileDownload {
    /// What the browser should call the saved file.
    file_name: String,
    /// The file.
    bytes: Vec<u8>,
}

/// Characters a file name may keep inside a quoted `filename=` parameter.
///
/// A quote or a newline in a file name would end the parameter early and let the rest of the name
/// become header syntax. Everything outside this set is dropped from the quoted form; the exact
/// name still reaches the browser through the `filename*` parameter, which is percent-encoded and
/// cannot carry syntax at all.
const DISPOSITION_SAFE: &percent_encoding::AsciiSet = &percent_encoding::NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

impl IntoResponse for FileDownload {
    fn into_response(self) -> Response {
        let quoted: String = self
            .file_name
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_' | ' '))
            .collect();
        let quoted = if quoted.trim().is_empty() {
            "download".to_owned()
        } else {
            quoted
        };
        let encoded =
            percent_encoding::utf8_percent_encode(&self.file_name, DISPOSITION_SAFE).to_string();

        let disposition =
            format!("attachment; filename=\"{quoted}\"; filename*=UTF-8''{encoded}");

        (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "application/octet-stream".to_owned()),
                (header::CONTENT_DISPOSITION, disposition),
                // The sandbox serves what a seat put there. Telling the browser not to guess a
                // type keeps an uploaded .html from being rendered as a page on this origin.
                (header::X_CONTENT_TYPE_OPTIONS, "nosniff".to_owned()),
            ],
            self.bytes,
        )
            .into_response()
    }
}

/// The last path segment, which is what a browser should call the saved file.
fn file_name_of(path: &str) -> String {
    path.rsplit(['/', '\\'])
        .find(|segment| !segment.is_empty() && *segment != "." && *segment != "..")
        .unwrap_or("download")
        .to_owned()
}

/// Hand back one file from the sandbox as bytes.
///
/// Separate from [`read_file_handler`] rather than a mode of it. That route answers *what does this
/// file say*, and refuses anything it cannot decode as UTF-8 within a panel-sized budget. This one
/// answers *give me this file*, which is a different question about the same bytes: an image is not
/// unreadable, it is not text.
///
/// # Errors
///
/// Refuses if the caller holds no seat, the path names nothing readable, or the file is larger
/// than one transfer carries.
pub async fn download_file_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<FilePathRequest>,
) -> Result<FileDownload, Refusal> {
    state.shell_seat(&headers).ok_or_else(no_seat)?;

    let bytes = state
        .files
        .read_bytes(&payload.path, FILE_TRANSFER_MAX_BYTES)
        .map_err(|error| boundary(&error))?;

    Ok(FileDownload {
        file_name: file_name_of(&payload.path),
        bytes,
    })
}

/// Place one file into the sandbox.
///
/// Exclusive creation rather than a write: a drop onto a directory names a destination, and a
/// destination that already holds something is a collision the person has to see. Silently
/// replacing it would make losing a file indistinguishable from placing one.
///
/// # Errors
///
/// Refuses if the caller holds no seat, the payload is not base64 or exceeds the transfer bound,
/// the path already names something, or the sandbox cannot create and read back the file.
pub async fn upload_file_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(payload): Json<FileUploadRequest>,
) -> Result<Json<FileUploadProjection>, Refusal> {
    let owner = state.shell_seat(&headers).ok_or_else(no_seat)?;

    // Checked before decoding rather than after. Base64 is four characters per three bytes, so the
    // decoded size is known from the encoded length, and refusing here means an oversized upload
    // is never held twice.
    let declared_bytes = payload.content_base64.len() / 4 * 3;
    if declared_bytes > FILE_TRANSFER_MAX_BYTES {
        return Err(boundary(&JailError::SizeLimitExceeded {
            max_bytes: FILE_TRANSFER_MAX_BYTES,
            actual_bytes: declared_bytes,
        }));
    }

    let bytes = base64::engine::general_purpose::STANDARD
        .decode(payload.content_base64.as_bytes())
        .map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorBody {
                    schema_version: WEB_SCHEMA_V1,
                    error: "uploadNotBase64",
                    retryable: false,
                }),
            )
        })?;

    let location = jail_location(&owner, payload.path.clone());

    let _write_guard = FILE_WRITES.lock().unwrap_or_else(PoisonError::into_inner);
    state
        .files
        .create_file_exclusive(&payload.path, &bytes, FILE_TRANSFER_MAX_BYTES)
        .map_err(|error| boundary(&error))?;

    // Read back rather than trust the write, the same way the text routes do. A transfer that
    // reported success for bytes nobody can read again is the failure a person discovers later.
    let verified = state
        .files
        .read_bytes(&payload.path, FILE_TRANSFER_MAX_BYTES)
        .map_err(|error| boundary(&error))?;
    if verified != bytes {
        return Err(boundary(&JailError::Io(
            "post-upload verification failed".to_string(),
        )));
    }

    Ok(Json(FileUploadProjection {
        schema_version: WEB_SCHEMA_V1,
        location,
        path: payload.path,
        content_sha256: sha256(&verified),
        size_bytes: u64::try_from(verified.len()).unwrap_or(u64::MAX),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_download_name_is_the_last_segment_and_never_a_traversal() {
        assert_eq!(file_name_of("/notes/report.pdf"), "report.pdf");
        assert_eq!(file_name_of("report.pdf"), "report.pdf");
        assert_eq!(file_name_of("/notes/"), "notes");
        // A path ending in `..` must not name the saved file `..`, which is a directory entry a
        // browser would refuse or, worse, resolve.
        assert_eq!(file_name_of("/notes/.."), "notes");
        assert_eq!(file_name_of("/"), "download");
        assert_eq!(file_name_of(""), "download");
    }

    #[test]
    fn a_file_name_cannot_carry_header_syntax_into_the_disposition() {
        let response = FileDownload {
            // A quote closes the quoted parameter, a newline ends the header, and a semicolon
            // starts another one. None of the three may survive into the quoted form.
            file_name: "in\"voice;\nx: y.pdf".to_owned(),
            bytes: Vec::new(),
        }
        .into_response();

        let disposition = response
            .headers()
            .get(header::CONTENT_DISPOSITION)
            .expect("a download says how to save itself")
            .to_str()
            .expect("the header is printable ASCII");

        assert!(disposition.starts_with("attachment; filename=\"invoicex y.pdf\""), "{disposition}");
        // The exact name still reaches the browser, where it cannot be read as syntax.
        assert!(disposition.contains("filename*=UTF-8''in%22voice%3B%0Ax%3A%20y.pdf"), "{disposition}");
    }

    #[test]
    fn a_download_tells_the_browser_not_to_guess_the_type() {
        // The sandbox serves what a seat put in it. Without this an uploaded .html would render
        // as a page on the gateway's own origin.
        let response = FileDownload {
            file_name: "page.html".to_owned(),
            bytes: b"<script>".to_vec(),
        }
        .into_response();

        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/octet-stream"
        );
        assert_eq!(
            response.headers().get(header::X_CONTENT_TYPE_OPTIONS).unwrap(),
            "nosniff"
        );
    }

    #[test]
    fn already_existing_file_is_a_non_retryable_conflict() {
        let (status, Json(body)) = boundary(&JailError::AlreadyExists("notes.txt".into()));

        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body.error, "fileAlreadyExists");
        assert!(!body.retryable);
    }
}
