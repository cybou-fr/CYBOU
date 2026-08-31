// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! One unprivileged terminal owner, for one Linux account.
//!
//! The process is that account. It refuses to be root, binds a socket somebody else's systemd
//! instance created for it, and answers one connection with one pseudoterminal. The gateway proves
//! who is at the keyboard and connects the two ends; it never spawns a shell, because it runs as
//! `cybou` and has no business becoming anybody.

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("cybou-ptyd is Linux-only");
    std::process::exit(1);
}

#[cfg(target_os = "linux")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::{
        os::unix::fs::{FileTypeExt as _, PermissionsExt as _},
        path::PathBuf,
    };

    // The first line of ADR-0047's boundary, and the cheapest. Everything else in this process
    // assumes it is one person; a root terminal would be a browser session that is the machine.
    if rustix::process::geteuid().is_root() {
        return Err("refusing to own a terminal as root".into());
    }

    let socket = std::env::var_os("CYBOU_PTY_SOCKET")
        .map(PathBuf::from)
        .ok_or("CYBOU_PTY_SOCKET is required")?;

    // The shell is this account's, read from the environment its systemd unit established. Not
    // guessed from a list of likely paths: a terminal that silently opened a different shell from
    // the one `chsh` says would be answering a question nobody asked.
    let shell = std::env::var_os("CYBOU_PTY_SHELL")
        .or_else(|| std::env::var_os("SHELL"))
        .ok_or("CYBOU_PTY_SHELL or SHELL is required")?;

    if let Ok(metadata) = std::fs::symlink_metadata(&socket) {
        if !metadata.file_type().is_socket() {
            return Err(format!("refusing to replace non-socket path {}", socket.display()).into());
        }
        if std::os::unix::net::UnixStream::connect(&socket).is_ok() {
            return Err(format!("refusing to replace active socket {}", socket.display()).into());
        }
        std::fs::remove_file(&socket)?;
    }

    let listener = tokio::net::UnixListener::bind(&socket)?;
    // The directory is the boundary, not this mode. The socket sits in `<dir>/<uid>/`, which the
    // runner makes `0750 <account>:cybou`: the account owns it and the gateway's group may enter,
    // and nobody else can reach this path to connect to it whatever it says here.
    //
    // Worth stating because the two are easy to confuse and the failure mode is silent. On
    // 2026-09-01 the parent directory was created under `UMask=0077` and came out `0700 root`, so
    // the account could not enter the directory made for it and the owner died with `Permission
    // denied` — a directory bit that read exactly like a broken daemon. The gate now asserts the
    // modes so a change to either side has to face the other.
    std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o666))?;
    eprintln!(
        "[cybou-ptyd] terminal owner listening at {}",
        socket.display()
    );

    loop {
        let (stream, _) = listener.accept().await?;
        let shell = shell.clone();
        // One connection is one terminal. A session that outlived its connection would be a shell
        // nobody is attached to and nobody authenticated, which is the thing ADR-0047 spends most
        // of its length refusing.
        tokio::spawn(async move {
            if let Err(error) = cybou_ptyd::session::run(stream, &shell).await {
                eprintln!("[cybou-ptyd] a session ended: {error}");
            }
        });
    }
}
