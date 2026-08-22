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
    use std::{os::unix::fs::PermissionsExt, path::Path, sync::Arc, time::Instant};

    use cybou_authd::{
        ACCESS_GROUP, Answer, MAX_CONCURRENT_ATTEMPTS, MAX_REQUEST_BYTES, MAX_USERNAME_BYTES,
        Request, Throttle,
    };
    use tokio::{
        io::{AsyncReadExt as _, AsyncWriteExt as _},
        sync::Semaphore,
    };

    /// Where the gateway looks for the helper.
    const SOCKET_PATH: &str = "/run/cybou/auth.sock";

    /// How long the helper will wait for a caller to finish sending its request.
    ///
    /// A connection that opens and then says nothing would otherwise hold a task forever, and
    /// enough of them would be a way to exhaust the helper without ever guessing a password.
    const READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

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

        // One throttle and one gate for the whole process. Both are shared deliberately: backoff
        // that a caller could reset by reconnecting would measure nothing, and a limit that were
        // per-connection would be no limit at all.
        let throttle = Arc::new(Throttle::new());
        let attempts = Arc::new(Semaphore::new(MAX_CONCURRENT_ATTEMPTS));

        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                continue;
            };
            let throttle = Arc::clone(&throttle);
            let attempts = Arc::clone(&attempts);
            tokio::spawn(async move {
                let mut buffer = Vec::new();
                let mut limited = (&mut stream).take(MAX_REQUEST_BYTES as u64);
                let read = tokio::time::timeout(READ_TIMEOUT, limited.read_to_end(&mut buffer));
                if !matches!(read.await, Ok(Ok(_))) {
                    return;
                }
                let Ok(request) = ciborium::from_reader::<Request, _>(buffer.as_slice()) else {
                    return;
                };
                // The permit is taken here rather than at accept, and held across the delay below.
                // Reading is cheap and already bounded; deciding is what must queue, because a
                // delay that runs in parallel with every other delay costs an attacker nothing.
                let Ok(_permit) = attempts.acquire().await else {
                    return;
                };
                let authenticated = decide(request, &throttle).await;

                let mut encoded = Vec::new();
                if ciborium::into_writer(&Answer { authenticated }, &mut encoded).is_ok() {
                    let _ = stream.write_all(&encoded).await;
                }
                let _ = stream.shutdown().await;
            });
        }
    }

    /// Whether this attempt is accepted, and what it costs if it is not.
    ///
    /// Every path that answers `false` leaves through the same delay, so the reason for a refusal
    /// — no name, an account outside the group, a wrong password — is not readable from how long
    /// the answer took.
    async fn decide(request: Request, throttle: &Throttle) -> bool {
        let username = request.username.clone();

        if username.is_empty()
            || username.len() > MAX_USERNAME_BYTES
            || request.password.is_empty()
            || !in_access_group(&username)
        {
            refuse(throttle, &username).await;
            return false;
        }

        let password = request.password.clone();
        let for_pam = username.clone();
        drop(request);
        let authenticated = tokio::task::spawn_blocking(move || {
            let decision = authenticate(&for_pam, &password);
            let mut bytes = password.into_bytes();
            bytes.fill(0);
            decision
        })
        .await
        .unwrap_or(false);

        if authenticated {
            throttle.record_success(&username);
            true
        } else {
            refuse(throttle, &username).await;
            false
        }
    }

    /// Hold a failed attempt for what this account currently owes, then count it.
    ///
    /// The penalty is read before the failure is recorded so that the first wrong password costs
    /// the floor rather than double it: the delay is what the attempt earned on arrival.
    async fn refuse(throttle: &Throttle, username: &str) {
        let now = Instant::now();
        let penalty = throttle.penalty(username, now);
        throttle.record_failure(username, now);
        tokio::time::sleep(penalty).await;
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
