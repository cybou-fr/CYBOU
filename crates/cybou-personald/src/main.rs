// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Unix-socket entry point for one unprivileged Personal Core owner.

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("cybou-personald is Linux-only");
    std::process::exit(1);
}

#[cfg(target_os = "linux")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::{
        os::unix::fs::{FileTypeExt as _, PermissionsExt as _},
        path::PathBuf,
        sync::Arc,
    };

    use cybou_personald::{MAX_REQUEST_BYTES, Owner, REQUEST_TIMEOUT, Request, Response};
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

    // Running as root would make one process the owner of everybody's personal records again,
    // which is the arrangement this daemon exists to end.
    if rustix::process::geteuid().is_root() {
        return Err("refusing to own personal records as root".into());
    }
    let store = std::env::var_os("CYBOU_PERSONAL_STORE")
        .map(PathBuf::from)
        .ok_or("CYBOU_PERSONAL_STORE is required")?;
    let socket = std::env::var_os("CYBOU_PERSONAL_SOCKET")
        .map(PathBuf::from)
        .ok_or("CYBOU_PERSONAL_SOCKET is required")?;
    if let Ok(metadata) = std::fs::symlink_metadata(&socket) {
        if !metadata.file_type().is_socket() {
            return Err(format!("refusing to replace non-socket path {}", socket.display()).into());
        }
        if std::os::unix::net::UnixStream::connect(&socket).is_ok() {
            return Err(format!("refusing to replace active socket {}", socket.display()).into());
        }
        std::fs::remove_file(&socket)?;
    }
    let owner = Arc::new(Owner::open(store)?);
    let listener = tokio::net::UnixListener::bind(&socket)?;
    std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o666))?;

    loop {
        let (mut stream, _) = listener.accept().await?;
        let owner = Arc::clone(&owner);
        tokio::spawn(async move {
            let mut encoded = Vec::new();
            let mut limited = (&mut stream).take(MAX_REQUEST_BYTES + 1);
            let read = limited.read_to_end(&mut encoded);
            let response = match tokio::time::timeout(REQUEST_TIMEOUT, read).await {
                Ok(Ok(_))
                    if u64::try_from(encoded.len()).is_ok_and(|len| len <= MAX_REQUEST_BYTES) =>
                {
                    match ciborium::from_reader::<Request, _>(encoded.as_slice()) {
                        Ok(request) => owner.answer(request),
                        Err(_) => Response::Refused,
                    }
                }
                _ => Response::Refused,
            };
            let mut answer = Vec::new();
            if ciborium::into_writer(&response, &mut answer).is_ok() {
                let _ = stream.write_all(&answer).await;
            }
        });
    }
}
