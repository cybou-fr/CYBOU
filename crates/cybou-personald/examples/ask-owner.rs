// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Speak to one Personal Core owner exactly as the gateway does, for host proofs.
//!
//! Usage: `ask-owner <socket> notes` or `ask-owner <socket> create <title>`.
//!
//! It prints the note titles the owner answered with, one per line, and nothing else, so a gate can
//! assert on what one account's owner does and does not hold.

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("ask-owner is Linux-only");
    std::process::exit(1);
}

#[cfg(target_os = "linux")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use cybou_personald::{Request, Response};
    use cybou_web_contracts::CreateNoteRequest;
    use std::io::{Read as _, Write as _};

    let mut arguments = std::env::args().skip(1);
    let socket = arguments.next().ok_or("a socket path is required")?;
    let verb = arguments.next().ok_or("a verb is required")?;
    let request = match verb.as_str() {
        "notes" => Request::Notes,
        "create" => Request::CreateNote(Box::new(CreateNoteRequest {
            title: arguments.next().ok_or("a title is required")?,
            content_markdown: "private".to_owned(),
            tags: Vec::new(),
            is_pinned: false,
            referenced_subject: None,
        })),
        other => return Err(format!("unknown verb {other}").into()),
    };

    let mut stream = std::os::unix::net::UnixStream::connect(socket)?;
    let mut encoded = Vec::new();
    ciborium::into_writer(&request, &mut encoded)?;
    stream.write_all(&encoded)?;
    stream.shutdown(std::net::Shutdown::Write)?;
    let mut answer = Vec::new();
    stream.read_to_end(&mut answer)?;

    match ciborium::from_reader::<Response, _>(answer.as_slice())? {
        Response::Notes(projection) => {
            for note in projection.notes {
                println!("{}", note.title);
            }
            Ok(())
        }
        Response::Note(note) => {
            println!("{}", note.title);
            Ok(())
        }
        other => Err(format!("unexpected answer: {other:?}").into()),
    }
}
