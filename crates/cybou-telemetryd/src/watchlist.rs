// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! What this host has been told to watch beyond what is universally true.
//!
//! Every subject the organ watches by default is readable on any Linux host with no configuration:
//! load, memory, pressure, filesystem, descriptors, failed units. That is deliberate, and it is why
//! the three things an operator most often asks for are not among them. A certificate, a service and
//! a backup all need to be told *which one*, and a subject that needs naming is a different kind of
//! thing from one that is universally true.
//!
//! So they are declared, in a file, one per line:
//!
//! ```text
//! # what this host cares about beyond the universal
//! certificate /etc/letsencrypt/live/example.org/fullchain.pem
//! ```
//!
//! ## A line this build cannot read is an error, not a comment
//!
//! The tempting behaviour is to skip what is not understood and carry on. It is wrong here, and the
//! reason is the whole point of the file: an operator who writes `certficate /etc/...` has told
//! their machine to watch a certificate. If the line is ignored, they believe it is watched, nothing
//! is watching it, and the first they hear of it is an expired certificate.
//!
//! A refused file is loud and recoverable. A silently ignored line is quiet and produces exactly the
//! outage the declaration existed to prevent, months later.
//!
//! ## Empty by default, and that is not a degraded state
//!
//! A host with no file, or an empty one, watches the universal subjects and nothing else. That is
//! the true description of such a host, not a reduced one — there is nothing it has been asked to
//! watch and is failing to.

use std::path::PathBuf;

/// Something named that this host was told to watch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Watched {
    /// A TLS certificate, watched for how long it has left.
    Certificate(PathBuf),
}

/// Why a declaration file was refused.
///
/// Carries the line number, because a file refused without one moves the work of finding the problem
/// onto the person who already made a typo.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Refused {
    /// Which line, counting from one.
    pub line: usize,
    /// What was wrong with it.
    pub because: Because,
}

/// What was wrong with a line.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Because {
    /// The first word is not a kind this build watches.
    UnknownKind(String),
    /// The kind was recognised and nothing was named.
    NothingNamed(String),
    /// The path is not absolute.
    ///
    /// Refused rather than resolved. A relative path in a daemon's declaration resolves against
    /// whatever directory systemd happened to start it in, which is not a thing the person writing
    /// the line was thinking about.
    NotAbsolute(String),
}

impl core::fmt::Display for Because {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnknownKind(word) => {
                write!(formatter, "'{word}' is not something this build can watch")
            }
            Self::NothingNamed(kind) => write!(formatter, "'{kind}' names nothing"),
            Self::NotAbsolute(path) => {
                write!(formatter, "'{path}' is not an absolute path")
            }
        }
    }
}

impl core::fmt::Display for Refused {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "line {}: {}", self.line, self.because)
    }
}

impl core::error::Error for Refused {}

/// Read a declaration file.
///
/// # Errors
///
/// Returns every line that could not be read, rather than the first. An operator fixing a
/// hand-written file wants all of it at once; reporting one error per run makes correcting three
/// typos three restarts.
pub fn parse(contents: &str) -> Result<Vec<Watched>, Vec<Refused>> {
    let mut watched = Vec::new();
    let mut refused = Vec::new();

    for (index, raw) in contents.lines().enumerate() {
        let line = raw.trim();
        // A blank line and a comment are the two things that legitimately say nothing.
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let number = index + 1;
        let mut words = line.split_whitespace();
        let Some(kind) = words.next() else {
            continue;
        };
        let named = words.next().unwrap_or_default();

        match kind {
            "certificate" => {
                if named.is_empty() {
                    refused.push(Refused {
                        line: number,
                        because: Because::NothingNamed(kind.to_owned()),
                    });
                } else if !named.starts_with('/') {
                    refused.push(Refused {
                        line: number,
                        because: Because::NotAbsolute(named.to_owned()),
                    });
                } else {
                    watched.push(Watched::Certificate(PathBuf::from(named)));
                }
            }
            other => refused.push(Refused {
                line: number,
                because: Because::UnknownKind(other.to_owned()),
            }),
        }
    }

    if refused.is_empty() {
        Ok(watched)
    } else {
        Err(refused)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_kind_this_build_cannot_watch_refuses_the_file() {
        // The failure this rule exists for. An operator who writes `certficate` has told their
        // machine to watch a certificate; if the line is skipped they believe it is watched, nothing
        // is, and the first they hear of it is an expired certificate months later.
        let typo = "certficate /etc/ssl/example.pem\n";
        let refused = parse(typo).expect_err("a refusal");
        assert_eq!(refused.len(), 1);
        assert_eq!(
            refused[0].because,
            Because::UnknownKind("certficate".to_owned())
        );
        assert_eq!(refused[0].line, 1);
    }

    #[test]
    fn every_bad_line_is_reported_rather_than_the_first() {
        // Reporting one error per run makes correcting three typos three restarts.
        let messy = "certficate /a\nservice nginx\ncertificate relative/path\n";
        let refused = parse(messy).expect_err("refusals");
        assert_eq!(refused.len(), 3);
        assert_eq!(
            refused.iter().map(|r| r.line).collect::<Vec<_>>(),
            [1, 2, 3]
        );
    }

    #[test]
    fn a_relative_path_is_refused_rather_than_resolved() {
        // It would resolve against whatever directory systemd happened to start the daemon in,
        // which is not a thing the person writing the line was thinking about.
        let relative = "certificate etc/ssl/example.pem\n";
        let refused = parse(relative).expect_err("a refusal");
        assert_eq!(
            refused[0].because,
            Because::NotAbsolute("etc/ssl/example.pem".to_owned())
        );
    }

    #[test]
    fn a_kind_that_names_nothing_is_refused() {
        let bare = "certificate\n";
        let refused = parse(bare).expect_err("a refusal");
        assert_eq!(
            refused[0].because,
            Because::NothingNamed("certificate".to_owned())
        );
    }

    #[test]
    fn blank_lines_and_comments_are_the_two_things_that_may_say_nothing() {
        let commented = "# what this host cares about\n\n   \ncertificate /etc/ssl/a.pem\n\n";
        let watched = parse(commented).expect("a readable file");
        assert_eq!(watched, vec![Watched::Certificate("/etc/ssl/a.pem".into())]);
    }

    #[test]
    fn a_host_with_nothing_declared_watches_nothing_extra_and_is_not_in_error() {
        // The true description of such a host, not a reduced one: there is nothing it has been asked
        // to watch and is failing to.
        assert_eq!(parse("").expect("an empty file is readable"), Vec::new());
        assert_eq!(parse("# nothing yet\n").expect("comments only"), Vec::new());
    }

    #[test]
    fn a_refusal_says_which_line_and_what_was_wrong_with_it() {
        // A file refused without a line number moves the work of finding the problem onto the person
        // who already made the typo.
        let refused = parse("certificate /a\nnonsense here\n").expect_err("a refusal");
        let said = refused[0].to_string();
        assert!(said.contains("line 2"), "{said}");
        assert!(said.contains("nonsense"), "{said}");
    }

    #[test]
    fn several_certificates_are_all_watched() {
        let many = "certificate /etc/ssl/a.pem\ncertificate /etc/ssl/b.pem\n";
        let watched = many_watched(many);
        assert_eq!(watched.len(), 2);
    }

    fn many_watched(contents: &str) -> Vec<Watched> {
        parse(contents).expect("a readable file")
    }
}
