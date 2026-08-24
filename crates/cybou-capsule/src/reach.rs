// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! What an agent is asking to do.
//!
//! A closed set. An open one — a string, a command line, an arbitrary syscall description — would
//! put the decision about what a request *means* at the place that decides whether to allow it, and
//! those two questions have to be answered by different code or neither is answerable.
//!
//! It is deliberately coarse. This is not a syscall filter; the kernel is the syscall filter. These
//! are the kinds of thing a person granting a profile has an opinion about.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Something an agent wants to do.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "reach")]
pub enum Reach {
    /// Read a path.
    #[serde(rename_all = "camelCase")]
    ReadPath {
        /// What it wants to read.
        path: PathBuf,
    },
    /// Write a path.
    #[serde(rename_all = "camelCase")]
    WritePath {
        /// What it wants to write.
        path: PathBuf,
    },
    /// Run a program inside its own namespaces.
    #[serde(rename_all = "camelCase")]
    RunProgram {
        /// What it wants to run, for the record rather than for the decision.
        ///
        /// The decision is whether this capsule may execute at all. Deciding per program would be
        /// a name-based allow-list, which is a rule about spelling: `sh -c` defeats it, and so does
        /// a build script. Inside a capsule the agent already owns its processes.
        program: String,
    },
    /// Connect to a host.
    #[serde(rename_all = "camelCase")]
    ConnectHost {
        /// The host it wants to reach.
        host: String,
    },
    /// Call a tool through the host's mediation.
    #[serde(rename_all = "camelCase")]
    CallTool {
        /// Which tool.
        tool: String,
    },
    /// Ask the model gateway for a completion.
    #[serde(rename_all = "camelCase")]
    UseModel {
        /// Which class of model.
        class: String,
    },
    /// Act on the host itself.
    ///
    /// Restart a service, change the firewall, publish a port. Not a capsule capability at all: it
    /// is the boundary crossing ADR-0022 exists for, and it arrives here so that it is *named* as
    /// one rather than being an unmatched case.
    #[serde(rename_all = "camelCase")]
    ActOnHost {
        /// What it wants done, in the operation vocabulary the action boundary understands.
        operation: String,
        /// What it wants it done to.
        target: String,
    },
    /// Reach another capsule.
    ReachAnotherCapsule,
    /// Reach Cybou's own durable state.
    ReachTheJournal,
    /// Reach the key store.
    ReachTheKeyStore,
}

impl Reach {
    /// A short name for the record.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::ReadPath { .. } => "read-path",
            Self::WritePath { .. } => "write-path",
            Self::RunProgram { .. } => "run-program",
            Self::ConnectHost { .. } => "connect-host",
            Self::CallTool { .. } => "call-tool",
            Self::UseModel { .. } => "use-model",
            Self::ActOnHost { .. } => "act-on-host",
            Self::ReachAnotherCapsule => "reach-another-capsule",
            Self::ReachTheJournal => "reach-the-journal",
            Self::ReachTheKeyStore => "reach-the-key-store",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reach_survives_the_wire() {
        // It travels from whatever observes an agent to whatever decides about it, and those are
        // different processes by design.
        let reaches = vec![
            Reach::ReadPath {
                path: PathBuf::from("/srv/project/src/main.rs"),
            },
            Reach::ConnectHost {
                host: "github.com".to_owned(),
            },
            Reach::ActOnHost {
                operation: "service.restart".to_owned(),
                target: "systemd:postgresql.service".to_owned(),
            },
            Reach::ReachTheKeyStore,
        ];
        let mut encoded = Vec::new();
        ciborium::into_writer(&reaches, &mut encoded).expect("encodes");
        let decoded: Vec<Reach> = ciborium::from_reader(encoded.as_slice()).expect("decodes");
        assert_eq!(decoded, reaches);
    }

    #[test]
    fn every_reach_has_a_name_and_they_are_all_different() {
        // A record where two kinds of request share a label is a record that cannot answer what
        // happened.
        let all = [
            Reach::ReadPath {
                path: PathBuf::new(),
            },
            Reach::WritePath {
                path: PathBuf::new(),
            },
            Reach::RunProgram {
                program: String::new(),
            },
            Reach::ConnectHost {
                host: String::new(),
            },
            Reach::CallTool {
                tool: String::new(),
            },
            Reach::UseModel {
                class: String::new(),
            },
            Reach::ActOnHost {
                operation: String::new(),
                target: String::new(),
            },
            Reach::ReachAnotherCapsule,
            Reach::ReachTheJournal,
            Reach::ReachTheKeyStore,
        ];
        let mut names: Vec<&str> = all.iter().map(Reach::name).collect();
        let total = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), total);
    }
}
