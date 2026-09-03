// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Gateway transport to an authenticated user's unprivileged filesystem owner.

use std::path::PathBuf;

use async_trait::async_trait;
use cybou_host_filesd::{Request, Response};
use cybou_web_contracts::{
    FileContentProjection, FileWriteProjection, HostDirectoryListingProjection,
};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

use crate::state::{GatewayError, HostUserFileSource};

const RESPONSE_MAX_BYTES: u64 = 1024 * 1024;
const ROUND_TRIP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// Per-UID Unix-socket client for host-user filesystem owners.
pub struct SocketHostUserFiles {
    socket_directory: PathBuf,
}

impl SocketHostUserFiles {
    /// Address owners as `<directory>/<uid>/owner.sock`.
    #[must_use]
    pub fn in_directory(directory: impl Into<PathBuf>) -> Self {
        Self {
            socket_directory: directory.into(),
        }
    }

    async fn ask(&self, uid: u32, request: &Request) -> Result<Response, GatewayError> {
        let socket = self
            .socket_directory
            .join(uid.to_string())
            .join("owner.sock");
        let exchange = async {
            let mut stream = tokio::net::UnixStream::connect(socket)
                .await
                .map_err(|_| GatewayError::Unavailable)?;
            let mut encoded = Vec::new();
            ciborium::into_writer(request, &mut encoded).map_err(|_| GatewayError::Unavailable)?;
            stream
                .write_all(&encoded)
                .await
                .map_err(|_| GatewayError::Unavailable)?;
            stream
                .shutdown()
                .await
                .map_err(|_| GatewayError::Unavailable)?;
            let mut answer = Vec::new();
            (&mut stream)
                .take(RESPONSE_MAX_BYTES + 1)
                .read_to_end(&mut answer)
                .await
                .map_err(|_| GatewayError::Unavailable)?;
            if u64::try_from(answer.len()).map_or(true, |len| len > RESPONSE_MAX_BYTES) {
                return Err(GatewayError::InvalidProjection);
            }
            ciborium::from_reader(answer.as_slice()).map_err(|_| GatewayError::InvalidProjection)
        };
        tokio::time::timeout(ROUND_TRIP_TIMEOUT, exchange)
            .await
            .map_err(|_| GatewayError::Timeout)?
    }
}

#[async_trait]
impl HostUserFileSource for SocketHostUserFiles {
    async fn list_directory(
        &self,
        uid: u32,
        _home: &str,
        path: &str,
    ) -> Result<HostDirectoryListingProjection, GatewayError> {
        match self
            .ask(
                uid,
                &Request::ListDirectory {
                    path: path.to_owned(),
                },
            )
            .await?
        {
            Response::Directory(projection) => Ok(projection),
            Response::File(_) | Response::Written(_) | Response::Success => {
                Err(GatewayError::InvalidProjection)
            }
            // Only a conditional write can conflict; anything else saying so is not an

            // answer this boundary can pass on.
            Response::Conflict => Err(GatewayError::InvalidProjection),
            Response::Refused => Err(GatewayError::Unavailable),
        }
    }

    async fn read_file(
        &self,
        uid: u32,
        _home: &str,
        path: &str,
    ) -> Result<FileContentProjection, GatewayError> {
        match self
            .ask(
                uid,
                &Request::ReadFile {
                    path: path.to_owned(),
                },
            )
            .await?
        {
            Response::File(projection) => Ok(projection),
            Response::Directory(_) | Response::Written(_) | Response::Success => {
                Err(GatewayError::InvalidProjection)
            }
            // Only a conditional write can conflict; anything else saying so is not an

            // answer this boundary can pass on.
            Response::Conflict => Err(GatewayError::InvalidProjection),
            Response::Refused => Err(GatewayError::Unavailable),
        }
    }

    async fn write_file(
        &self,
        uid: u32,
        _home: &str,
        path: &str,
        expected_sha256: Option<String>,
        text: &str,
    ) -> Result<FileWriteProjection, GatewayError> {
        match self
            .ask(
                uid,
                &Request::WriteFile {
                    path: path.to_owned(),
                    expected_sha256,
                    text: text.to_owned(),
                },
            )
            .await?
        {
            Response::Written(projection) => Ok(projection),
            // The file changed since it was read. Not unavailability, and not something to retry.
            Response::Conflict => Err(GatewayError::Conflict),
            Response::File(_) | Response::Directory(_) | Response::Success => {
                Err(GatewayError::InvalidProjection)
            }
            Response::Refused => Err(GatewayError::Unavailable),
        }
    }

    async fn create_file(
        &self,
        uid: u32,
        _home: &str,
        path: &str,
        text: &str,
        exclusive: bool,
    ) -> Result<FileWriteProjection, GatewayError> {
        match self
            .ask(
                uid,
                &Request::CreateFile {
                    path: path.to_owned(),
                    text: text.to_owned(),
                    exclusive,
                },
            )
            .await?
        {
            Response::Written(projection) => Ok(projection),
            Response::File(_) | Response::Directory(_) | Response::Success => {
                Err(GatewayError::InvalidProjection)
            }
            // Only a conditional write can conflict; anything else saying so is not an

            // answer this boundary can pass on.
            Response::Conflict => Err(GatewayError::InvalidProjection),
            Response::Refused => Err(GatewayError::Unavailable),
        }
    }

    async fn create_directory(
        &self,
        uid: u32,
        _home: &str,
        path: &str,
        recursive: bool,
    ) -> Result<(), GatewayError> {
        match self
            .ask(
                uid,
                &Request::CreateDirectory {
                    path: path.to_owned(),
                    recursive,
                },
            )
            .await?
        {
            Response::Success => Ok(()),
            Response::Refused => Err(GatewayError::Unavailable),
            _ => Err(GatewayError::InvalidProjection),
        }
    }

    async fn rename_path(
        &self,
        uid: u32,
        _home: &str,
        from_path: &str,
        to_path: &str,
    ) -> Result<(), GatewayError> {
        match self
            .ask(
                uid,
                &Request::RenamePath {
                    from_path: from_path.to_owned(),
                    to_path: to_path.to_owned(),
                },
            )
            .await?
        {
            Response::Success => Ok(()),
            Response::Refused => Err(GatewayError::Unavailable),
            _ => Err(GatewayError::InvalidProjection),
        }
    }

    async fn delete_path(
        &self,
        uid: u32,
        _home: &str,
        path: &str,
        recursive: bool,
    ) -> Result<(), GatewayError> {
        match self
            .ask(
                uid,
                &Request::DeletePath {
                    path: path.to_owned(),
                    recursive,
                },
            )
            .await?
        {
            Response::Success => Ok(()),
            Response::Refused => Err(GatewayError::Unavailable),
            _ => Err(GatewayError::InvalidProjection),
        }
    }

    async fn copy_path(
        &self,
        uid: u32,
        _home: &str,
        from_path: &str,
        to_path: &str,
    ) -> Result<(), GatewayError> {
        match self
            .ask(
                uid,
                &Request::CopyPath {
                    from_path: from_path.to_owned(),
                    to_path: to_path.to_owned(),
                },
            )
            .await?
        {
            Response::Success => Ok(()),
            Response::Refused => Err(GatewayError::Unavailable),
            _ => Err(GatewayError::InvalidProjection),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cybou_protocol::LocationRef;
    use cybou_web_contracts::WEB_SCHEMA_V1;

    #[tokio::test]
    async fn authenticated_uid_selects_its_owner_socket() {
        let directory = tempfile::tempdir().expect("socket directory");
        std::fs::create_dir(directory.path().join("1000")).expect("uid directory");
        let socket = directory.path().join("1000/owner.sock");
        let listener = tokio::net::UnixListener::bind(&socket).expect("listener");
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("connection");
            let mut request = Vec::new();
            stream.read_to_end(&mut request).await.expect("request");
            assert_eq!(
                ciborium::from_reader::<Request, _>(request.as_slice()).expect("typed request"),
                Request::ReadFile {
                    path: "/home/alice/note.txt".to_owned()
                }
            );
            let response = Response::File(FileContentProjection {
                schema_version: WEB_SCHEMA_V1,
                path: "/home/alice/note.txt".to_owned(),
                location: LocationRef::HostUserPath("/home/alice/note.txt".to_owned()),
                text: "hello".to_owned(),
                size_bytes: 5,
                content_sha256: "00".repeat(32),
            });
            let mut encoded = Vec::new();
            ciborium::into_writer(&response, &mut encoded).expect("response");
            stream.write_all(&encoded).await.expect("write response");
        });

        let client = SocketHostUserFiles::in_directory(directory.path());
        let response = client
            .read_file(1000, "/home/alice", "/home/alice/note.txt")
            .await
            .expect("owner response");
        assert_eq!(response.text, "hello");
        server.await.expect("server task");
    }
}
