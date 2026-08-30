// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! One connection, one pseudoterminal, and the bounds between them.

use std::ffi::OsStr;

use crate::{
    FromGateway, FromOwner, IDLE_TIMEOUT, MAX_BACKLOG_BYTES, MAX_FRAME_BYTES, Refusal,
    window_is_possible,
};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

/// How much is read from the pseudoterminal at once.
///
/// Output arrives as a stream with no message boundaries in it, so the size here is a trade
/// between syscalls and latency and nothing more. It is well under [`MAX_FRAME_BYTES`], which is
/// what the resulting frame has to fit inside.
const OUTPUT_CHUNK_BYTES: usize = 8 * 1024;

/// How long the owner waits for a shell it believes has ended to be reaped.
///
/// Short, because this runs only after the pseudoterminal has already reached end of file — the
/// process is gone and what is left is the kernel handing back its status. A bound rather than an
/// unbounded wait, because a session must not be held open by a child that will not be collected.
const REAP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

/// Read one length-prefixed frame, or say why there will not be one.
///
/// The length is read before the body and checked before anything is allocated for it. A peer that
/// declares four gigabytes gets a refusal rather than an allocation, which is the difference
/// between a bound and a comment saying frames are small.
async fn read_frame<R>(reader: &mut R) -> Result<Option<FromGateway>, Refusal>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut length = [0_u8; 4];
    match reader.read_exact(&mut length).await {
        Ok(_) => {}
        // The gateway closed, which is the ordinary way a terminal ends: somebody shut the tab.
        Err(_) => return Ok(None),
    }
    let length = u32::from_be_bytes(length) as usize;
    if length > MAX_FRAME_BYTES {
        return Err(Refusal::FrameTooLarge);
    }

    let mut body = vec![0_u8; length];
    if reader.read_exact(&mut body).await.is_err() {
        return Ok(None);
    }
    ciborium::from_reader(body.as_slice())
        .map(Some)
        .map_err(|_| Refusal::OutOfOrder)
}

/// Write one length-prefixed frame.
async fn write_frame<W>(writer: &mut W, frame: &FromOwner) -> std::io::Result<()>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    let mut body = Vec::new();
    ciborium::into_writer(frame, &mut body)
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    let length = u32::try_from(body.len())
        .map_err(|_| std::io::Error::other("frame length does not fit a u32"))?;
    writer.write_all(&length.to_be_bytes()).await?;
    writer.write_all(&body).await?;
    writer.flush().await
}

/// Everything that has to be true before there is a terminal at all.
///
/// Separate from the loop that carries it, because this is where every refusal lives and that is
/// where none of them do. `Ok(None)` means the session was refused and the caller has nothing left
/// to do; the refusal has already been sent.
async fn open_session<W>(
    to_gateway: &mut W,
    first: Result<Option<FromGateway>, Refusal>,
    shell: &OsStr,
) -> std::io::Result<Option<(pty_process::Pty, tokio::process::Child)>>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    // The first frame opens the session and nothing else may. A connection that started sending
    // input before saying how large the screen is would be a program drawing into a size nobody
    // established, and a default eighty by twenty-four would be a guess a person then looks at.
    let (columns, rows) = match first {
        Ok(Some(FromGateway::Open { columns, rows })) => (columns, rows),
        Ok(None) => return Ok(None),
        Ok(Some(_)) => {
            write_frame(to_gateway, &FromOwner::Refused(Refusal::OutOfOrder)).await?;
            return Ok(None);
        }
        Err(refusal) => {
            write_frame(to_gateway, &FromOwner::Refused(refusal)).await?;
            return Ok(None);
        }
    };
    if !window_is_possible(columns, rows) {
        write_frame(to_gateway, &FromOwner::Refused(Refusal::ImpossibleWindow)).await?;
        return Ok(None);
    }

    let Ok((pty, pts)) = pty_process::open() else {
        write_frame(to_gateway, &FromOwner::Refused(Refusal::CouldNotStart)).await?;
        return Ok(None);
    };
    if pty.resize(pty_process::Size::new(rows, columns)).is_err() {
        write_frame(to_gateway, &FromOwner::Refused(Refusal::CouldNotStart)).await?;
        return Ok(None);
    }

    // A login shell, because a person opening a terminal expects their profile to have run. This
    // is the account's own shell and the account's own profile; nothing here chooses either.
    let Ok(child) = pty_process::Command::new(shell).arg("-l").spawn(pts) else {
        write_frame(to_gateway, &FromOwner::Refused(Refusal::CouldNotStart)).await?;
        return Ok(None);
    };

    write_frame(to_gateway, &FromOwner::Opened).await?;
    Ok(Some((pty, child)))
}

/// Carry one terminal from its first frame to its last.
///
/// # Errors
///
/// Returns the write failure when the gateway's end of the socket cannot be written to at all.
/// Every other ending — a refusal, an idle session, a shell that exited — is a frame rather than
/// an error, because it is something the person at the terminal has to be told.
pub async fn run(
    stream: tokio::net::UnixStream,
    shell: &OsStr,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (mut from_gateway, mut to_gateway) = tokio::io::split(stream);

    let first = read_frame(&mut from_gateway).await;
    let Some((pty, mut child)) = open_session(&mut to_gateway, first, shell).await? else {
        return Ok(());
    };

    let (mut pty_out, mut pty_in) = pty.into_split();
    let mut buffer = vec![0_u8; OUTPUT_CHUNK_BYTES];
    let mut unread: usize = 0;

    let ending = loop {
        tokio::select! {
            // Output from the program.
            read = pty_out.read(&mut buffer) => {
                let Ok(count) = read else { break None };
                if count == 0 {
                    break None;
                }
                unread = unread.saturating_add(count);
                if unread > MAX_BACKLOG_BYTES {
                    // The program is doing what it was asked; the session is what could not keep
                    // up. Held bytes are this host's memory, and dropped bytes are a terminal that
                    // lies about what it printed, so the session ends instead.
                    break Some(Refusal::OutputOutranTheReader);
                }
                if write_frame(&mut to_gateway, &FromOwner::Output(buffer[..count].to_vec()))
                    .await
                    .is_err()
                {
                    break None;
                }
                // The frame reached the socket, so what was outstanding no longer is. The bound is
                // on what this process is holding, not on how much a session has ever printed.
                unread = 0;
            }

            // Keystrokes, and window changes.
            frame = read_frame(&mut from_gateway) => {
                match frame {
                    Ok(Some(FromGateway::Input(bytes))) => {
                        if pty_in.write_all(&bytes).await.is_err() {
                            break None;
                        }
                    }
                    Ok(Some(FromGateway::Resize { columns, rows })) => {
                        if !window_is_possible(columns, rows) {
                            break Some(Refusal::ImpossibleWindow);
                        }
                        let _ = pty_in.resize(pty_process::Size::new(rows, columns));
                    }
                    // A second Open is either a confused peer or an attempt to get a second shell
                    // out of one connection. Neither is a thing to answer.
                    Ok(Some(FromGateway::Open { .. })) => break Some(Refusal::OutOfOrder),
                    Ok(None) => break None,
                    Err(refusal) => break Some(refusal),
                }
            }

            // Nothing typed and nothing printed. This collects the tab that was closed without the
            // socket noticing, not a person who stopped to think.
            () = tokio::time::sleep(IDLE_TIMEOUT) => break Some(Refusal::Idle),

            // The program ended on its own, which is the ordinary way a terminal closes.
            status = child.wait() => {
                let frame = status.map_or(
                    FromOwner::Ended { code: None, signal: None },
                    |status| {
                        use std::os::unix::process::ExitStatusExt as _;
                        FromOwner::Ended { code: status.code(), signal: status.signal() }
                    },
                );
                let _ = write_frame(&mut to_gateway, &frame).await;
                return Ok(());
            }
        }
    };

    if let Some(refusal) = ending {
        let _ = write_frame(&mut to_gateway, &FromOwner::Refused(refusal)).await;
    } else {
        // The loop ended without a refusal, which on the ordinary path means the shell exited: the
        // pseudoterminal reaches end of file the moment its last writer is gone, and that read can
        // win the race against noticing the child. Reporting nothing here would turn "you typed
        // exit 7" into a terminal that simply stopped, so the status is collected rather than
        // inferred from which branch happened to fire first.
        if let Ok(Ok(status)) = tokio::time::timeout(REAP_TIMEOUT, child.wait()).await {
            use std::os::unix::process::ExitStatusExt as _;
            let _ = write_frame(
                &mut to_gateway,
                &FromOwner::Ended {
                    code: status.code(),
                    signal: status.signal(),
                },
            )
            .await;
        }
    }

    // Whatever ended the loop, the shell does not outlive it. A terminal whose connection is gone
    // is a shell nobody is attached to and nobody authenticated.
    let _ = child.start_kill();
    let _ = child.wait().await;
    Ok(())
}
