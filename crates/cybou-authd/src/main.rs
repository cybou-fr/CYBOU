// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! `cybou-authd`: the only Cybou process with the privilege to check a password.
//!
//! It runs as root because PAM needs that to read the shadow database, and everything about it is
//! arranged so that being root buys an attacker as little as possible. It speaks one message type,
//! answers one bit, listens on a socket only the gateway's user can open, refuses accounts outside
//! one group, and never writes what it was told.

#[cfg(not(unix))]
fn main() {
    eprintln!("cybou-authd is only supported on Linux/Unix platforms");
}

#[cfg(unix)]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    unix::run().await
}

#[cfg(unix)]
mod unix {
    use std::{os::unix::fs::PermissionsExt, path::Path};

    use cybou_authd::{ACCESS_GROUP, Answer, MAX_REQUEST_BYTES, Request};
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

    /// Where the gateway looks for the helper.
    const SOCKET_PATH: &str = "/run/cybou/auth.sock";

    /// How long a failed attempt is held before answering.
    const FAILURE_DELAY: std::time::Duration = std::time::Duration::from_millis(750);

    pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
        println!("[cybou-authd] Starting the credential helper");

        let path = std::env::var("CYBOU_AUTH_SOCKET").unwrap_or_else(|_| SOCKET_PATH.to_owned());
        let path = Path::new(&path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let _ = std::fs::remove_file(path);

        let listener = tokio::net::UnixListener::bind(path)?;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o660))?;
        println!("[cybou-authd] Listening on {}", path.display());
        println!("[cybou-authd] Only members of {ACCESS_GROUP} may authenticate");

        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                continue;
            };
            tokio::spawn(async move {
                let mut buffer = Vec::new();
                let mut limited = (&mut stream).take(MAX_REQUEST_BYTES as u64);
                if limited.read_to_end(&mut buffer).await.is_err() {
                    return;
                }
                let Ok(request) = ciborium::from_reader::<Request, _>(buffer.as_slice()) else {
                    return;
                };
                let authenticated = decide(request).await;

                let mut encoded = Vec::new();
                if ciborium::into_writer(&Answer { authenticated }, &mut encoded).is_ok() {
                    let _ = stream.write_all(&encoded).await;
                }
                let _ = stream.shutdown().await;
            });
        }
    }

    async fn decide(request: Request) -> bool {
        if request.username.is_empty() || request.password.is_empty() {
            tokio::time::sleep(FAILURE_DELAY).await;
            return false;
        }

        if !in_access_group(&request.username) {
            tokio::time::sleep(FAILURE_DELAY).await;
            return false;
        }

        let username = request.username.clone();
        let password = request.password.clone();
        drop(request);
        let authenticated = tokio::task::spawn_blocking(move || {
            let decision = authenticate(&username, &password);
            let mut bytes = password.into_bytes();
            bytes.fill(0);
            decision
        })
        .await
        .unwrap_or(false);

        if authenticated {
            true
        } else {
            tokio::time::sleep(FAILURE_DELAY).await;
            false
        }
    }

    #[cfg(target_os = "linux")]
    fn authenticate(username: &str, password: &str) -> bool {
        let mut client = match pam::Client::with_password("cybou") {
            Ok(client) => client,
            Err(error) => {
                eprintln!("[cybou-authd] PAM is not usable: {error}");
                return false;
            }
        };
        client
            .conversation_mut()
            .set_credentials(username, password);
        client.authenticate().is_ok()
    }

    #[cfg(not(target_os = "linux"))]
    fn authenticate(_username: &str, _password: &str) -> bool {
        false
    }

    fn in_access_group(username: &str) -> bool {
        let Ok(groups) = std::fs::read_to_string("/etc/group") else {
            return false;
        };
        for line in groups.lines() {
            let mut fields = line.split(':');
            let (Some(name), Some(_), Some(gid), Some(members)) =
                (fields.next(), fields.next(), fields.next(), fields.next())
            else {
                continue;
            };
            if name != ACCESS_GROUP {
                continue;
            }
            if members.split(',').any(|member| member == username) {
                return true;
            }
            return primary_group_of(username).as_deref() == Some(gid);
        }
        false
    }

    fn primary_group_of(username: &str) -> Option<String> {
        let passwd = std::fs::read_to_string("/etc/passwd").ok()?;
        for line in passwd.lines() {
            let mut fields = line.split(':');
            let (Some(name), Some(_), Some(_), Some(gid)) =
                (fields.next(), fields.next(), fields.next(), fields.next())
            else {
                continue;
            };
            if name == username {
                return Some(gid.to_owned());
            }
        }
        None
    }
}
