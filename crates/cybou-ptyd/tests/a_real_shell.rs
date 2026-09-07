// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! What the owner does with a real pseudoterminal and a real shell.
//!
//! Not a mock. The point of this crate is that a program on the far end believes it is talking to a
//! screen, and nothing short of allocating one proves that. The session runs over a socket pair, so
//! the test is the gateway.

#![cfg(target_os = "linux")]

use cybou_ptyd::{FromGateway, FromOwner, MAX_FRAME_BYTES, Refusal, session};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

/// Drive one session, and hand back the end the gateway would hold.
fn start() -> (tokio::net::UnixStream, tokio::task::JoinHandle<()>) {
    let (gateway, owner) = tokio::net::UnixStream::pair().expect("a socket pair");
    let handle = tokio::spawn(async move {
        let shell = std::ffi::OsString::from("/bin/sh");
        let _ = session::run(owner, &shell).await;
    });
    (gateway, handle)
}

async fn send(stream: &mut tokio::net::UnixStream, frame: &FromGateway) {
    let mut body = Vec::new();
    ciborium::into_writer(frame, &mut body).expect("encode");
    let length = u32::try_from(body.len()).expect("length");
    stream
        .write_all(&length.to_be_bytes())
        .await
        .expect("write");
    stream.write_all(&body).await.expect("write");
}

/// Send a frame whose declared length is a lie, without ever sending that many bytes.
async fn send_declared_length(stream: &mut tokio::net::UnixStream, length: u32) {
    stream
        .write_all(&length.to_be_bytes())
        .await
        .expect("write");
}

async fn receive(stream: &mut tokio::net::UnixStream) -> Option<FromOwner> {
    let mut length = [0_u8; 4];
    stream.read_exact(&mut length).await.ok()?;
    let mut body = vec![0_u8; u32::from_be_bytes(length) as usize];
    stream.read_exact(&mut body).await.ok()?;
    ciborium::from_reader(body.as_slice()).ok()
}

#[tokio::test]
async fn a_program_in_this_terminal_is_talking_to_a_screen() {
    let (mut gateway, _handle) = start();
    send(
        &mut gateway,
        &FromGateway::Open {
            columns: 80,
            rows: 24,
        },
    )
    .await;
    assert_eq!(receive(&mut gateway).await, Some(FromOwner::Opened));

    // `tty` answers from the file descriptor it was given, and `test -t 0` asks the kernel. A pipe
    // fails both. This is the entire difference between this crate and the sandboxed shell beside
    // it, so it is asked of the kernel rather than assumed from the fact that a crate was called.
    send(
        &mut gateway,
        &FromGateway::Input(b"test -t 0 && echo IS_A_TERMINAL\n".to_vec()),
    )
    .await;

    let mut seen = String::new();
    for _ in 0..40 {
        match tokio::time::timeout(std::time::Duration::from_secs(5), receive(&mut gateway)).await {
            Ok(Some(FromOwner::Output(bytes))) => {
                seen.push_str(&String::from_utf8_lossy(&bytes));
                if seen.contains("IS_A_TERMINAL") {
                    break;
                }
            }
            Ok(Some(_) | None) | Err(_) => break,
        }
    }
    assert!(
        seen.contains("IS_A_TERMINAL"),
        "the shell did not report a terminal on its standard input; saw {seen:?}"
    );
}

#[tokio::test]
async fn the_window_the_browser_reports_is_the_window_the_program_sees() {
    let (mut gateway, _handle) = start();
    send(
        &mut gateway,
        &FromGateway::Open {
            columns: 132,
            rows: 43,
        },
    )
    .await;
    assert_eq!(receive(&mut gateway).await, Some(FromOwner::Opened));

    // A size that reached `TIOCSWINSZ` is a size `stty` can read back. If it had only been stored
    // somewhere in this process, a full-screen program would still draw into eighty by twenty-four.
    send(&mut gateway, &FromGateway::Input(b"stty size\n".to_vec())).await;

    let mut seen = String::new();
    for _ in 0..40 {
        match tokio::time::timeout(std::time::Duration::from_secs(5), receive(&mut gateway)).await {
            Ok(Some(FromOwner::Output(bytes))) => {
                seen.push_str(&String::from_utf8_lossy(&bytes));
                if seen.contains("43 132") {
                    break;
                }
            }
            Ok(Some(_) | None) | Err(_) => break,
        }
    }
    assert!(
        seen.contains("43 132"),
        "the program saw a different window than the one that was reported; saw {seen:?}"
    );
}

#[tokio::test]
async fn a_shell_that_exits_says_so_rather_than_going_quiet() {
    let (mut gateway, _handle) = start();
    send(
        &mut gateway,
        &FromGateway::Open {
            columns: 80,
            rows: 24,
        },
    )
    .await;
    assert_eq!(receive(&mut gateway).await, Some(FromOwner::Opened));

    send(&mut gateway, &FromGateway::Input(b"exit 7\n".to_vec())).await;

    let mut ending = None;
    for _ in 0..40 {
        match tokio::time::timeout(std::time::Duration::from_secs(5), receive(&mut gateway)).await {
            Ok(Some(FromOwner::Output(_))) => {}
            Ok(Some(frame)) => {
                ending = Some(frame);
                break;
            }
            Ok(None) | Err(_) => break,
        }
    }

    // The status is carried, not flattened. A terminal that reported every ending identically would
    // make "the command failed" and "you typed exit" the same event.
    assert_eq!(
        ending,
        Some(FromOwner::Ended {
            code: Some(7),
            signal: None
        })
    );
}

#[tokio::test]
async fn a_first_frame_that_is_not_open_starts_nothing() {
    let (mut gateway, _handle) = start();

    // Input before a window size would be a program drawing into a size nobody established. There
    // is no default here on purpose: a guessed eighty by twenty-four is a guess a person would
    // then be looking at.
    send(&mut gateway, &FromGateway::Input(b"echo hello\n".to_vec())).await;

    assert_eq!(
        receive(&mut gateway).await,
        Some(FromOwner::Refused(Refusal::OutOfOrder))
    );
}

#[tokio::test]
async fn a_second_open_does_not_get_a_second_shell() {
    let (mut gateway, _handle) = start();
    send(
        &mut gateway,
        &FromGateway::Open {
            columns: 80,
            rows: 24,
        },
    )
    .await;
    assert_eq!(receive(&mut gateway).await, Some(FromOwner::Opened));

    send(
        &mut gateway,
        &FromGateway::Open {
            columns: 80,
            rows: 24,
        },
    )
    .await;

    let mut refusal = None;
    for _ in 0..40 {
        match tokio::time::timeout(std::time::Duration::from_secs(5), receive(&mut gateway)).await {
            Ok(Some(FromOwner::Output(_))) => {}
            Ok(Some(frame)) => {
                refusal = Some(frame);
                break;
            }
            Ok(None) | Err(_) => break,
        }
    }
    assert_eq!(refusal, Some(FromOwner::Refused(Refusal::OutOfOrder)));
}

#[tokio::test]
async fn a_terminal_is_never_given_a_size_it_cannot_have() {
    let (mut gateway, _handle) = start();
    send(
        &mut gateway,
        &FromGateway::Open {
            columns: 0,
            rows: 24,
        },
    )
    .await;

    assert_eq!(
        receive(&mut gateway).await,
        Some(FromOwner::Refused(Refusal::ImpossibleWindow))
    );
}

#[tokio::test]
async fn a_declared_length_nobody_intends_to_send_allocates_nothing() {
    let (mut gateway, _handle) = start();

    // The refusal has to come from the declared length alone. A reader that allocated first and
    // checked afterwards would answer this by reserving four gigabytes, which is the whole reason
    // the check is where it is.
    send_declared_length(&mut gateway, u32::MAX).await;

    assert_eq!(
        receive(&mut gateway).await,
        Some(FromOwner::Refused(Refusal::FrameTooLarge))
    );
    assert!(u32::MAX as usize > MAX_FRAME_BYTES);
}

async fn assert_start_directory(directory: &std::path::Path) {
    let expected = directory
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let (mut gateway, handle) = start();
    send(
        &mut gateway,
        &FromGateway::OpenAt {
            columns: 80,
            rows: 24,
            directory: expected.clone(),
        },
    )
    .await;
    assert_eq!(receive(&mut gateway).await, Some(FromOwner::Opened));
    // Put the answer on its own line even when a prompt has already been printed.
    send(
        &mut gateway,
        &FromGateway::Input(b"printf '\\n'; pwd -P\n".to_vec()),
    )
    .await;
    let mut seen = String::new();
    let result = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            match receive(&mut gateway).await {
                Some(FromOwner::Output(bytes)) => {
                    seen.push_str(&String::from_utf8_lossy(&bytes));
                    if seen
                        .lines()
                        .any(|line| line.trim_end_matches('\r') == expected)
                    {
                        break;
                    }
                }
                other => {
                    panic!("session ended before reporting its directory: {other:?}; {seen:?}")
                }
            }
        }
    })
    .await;
    assert!(
        result.is_ok(),
        "expected directory {expected:?}; actual output {seen:?}"
    );
    drop(gateway);
    tokio::time::timeout(std::time::Duration::from_secs(5), handle)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn directories_are_child_local_and_shell_metacharacters_stay_literal() {
    let before = std::env::current_dir().unwrap();
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("cybou-pty-{}-{unique}", std::process::id()));
    let first = root.join("project space ' ; $(printf injected)");
    let second = root.join("second");
    std::fs::create_dir_all(&first).unwrap();
    std::fs::create_dir(&second).unwrap();
    tokio::join!(
        assert_start_directory(&first),
        assert_start_directory(&second)
    );
    assert_eq!(std::env::current_dir().unwrap(), before);
    std::fs::remove_dir(first).unwrap();
    std::fs::remove_dir(second).unwrap();
    std::fs::remove_dir(root).unwrap();
}

#[tokio::test]
async fn an_unusable_directory_never_falls_back_to_another_folder() {
    for directory in ["relative", "", "/etc/passwd", "/tmp/../tmp", "/tmp/\0bad"] {
        let (mut gateway, handle) = start();
        send(
            &mut gateway,
            &FromGateway::OpenAt {
                columns: 80,
                rows: 24,
                directory: directory.into(),
            },
        )
        .await;
        assert_eq!(
            receive(&mut gateway).await,
            Some(FromOwner::Refused(Refusal::DirectoryUnavailable)),
            "{directory:?}"
        );
        handle.await.unwrap();
    }
    let (mut gateway, handle) = start();
    send(
        &mut gateway,
        &FromGateway::OpenAt {
            columns: 0,
            rows: 24,
            directory: "/tmp".into(),
        },
    )
    .await;
    assert_eq!(
        receive(&mut gateway).await,
        Some(FromOwner::Refused(Refusal::ImpossibleWindow))
    );
    handle.await.unwrap();
}
