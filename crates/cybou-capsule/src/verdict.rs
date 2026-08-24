// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Whether a request is inside the grant, outside it, or outside anything anybody may grant.

use serde::{Deserialize, Serialize};

use crate::grant::CapsuleGrant;
use crate::reach::Reach;

/// Why something can never be permitted.
///
/// A closed set, and short. These are the things that would end the capsule as a concept if a
/// profile could grant them, which is why they are refusals rather than expensive proposals — there
/// is no answer a person could give that would make them safe, so offering the question would be
/// offering a choice that does not exist.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Refusal {
    /// One capsule reaching another.
    ///
    /// Capsules are isolated from each other or they are one capsule with extra steps.
    AnotherCapsule,
    /// The Journal.
    ///
    /// An agent that could read the biography could read everything every other agent did, plus
    /// everything the person did. An agent that could write it could rewrite what happened.
    TheJournal,
    /// The key store.
    ///
    /// The whole erasure guarantee is that a destroyed key makes a record unreadable. An agent
    /// holding the keys is that guarantee ending at the least trusted process on the machine.
    TheKeyStore,
}

impl Refusal {
    /// How this reads to a person.
    #[must_use]
    pub const fn describe(self) -> &'static str {
        match self {
            Self::AnotherCapsule => "one capsule may not reach another",
            Self::TheJournal => "the biography is not an agent's to read or change",
            Self::TheKeyStore => {
                "the keys that make an erasure real are not reachable from a capsule"
            }
        }
    }
}

/// What was decided about a request.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "verdict")]
pub enum Verdict {
    /// Inside the grant. Nobody is asked, and nothing is recorded beyond the ordinary telemetry.
    Allowed,
    /// Outside the capsule, and answerable.
    ///
    /// Becomes an `ActionProposal` and crosses ADR-0022 — criticism, standing policy, confirmation
    /// or a grant already given, and an executor that is not this. The agent does not get an answer
    /// here; it gets one when the boundary produces one.
    #[serde(rename_all = "camelCase")]
    CrossesBoundary {
        /// What it wants done.
        operation: String,
        /// What it wants it done to.
        target: String,
    },
    /// Outside the capsule and never answerable.
    #[serde(rename_all = "camelCase")]
    Refused {
        /// Why.
        because: Refusal,
    },
    /// Inside the capsule's kind of thing, and not in this capsule's grant.
    ///
    /// Distinct from `Refused`, and the distinction matters: this is a profile that could have
    /// included it and does not. A person seeing it learns their grant is too narrow, which is a
    /// different thing to learn than that they asked for something impossible.
    #[serde(rename_all = "camelCase")]
    NotGranted {
        /// What was asked for, in words.
        wanted: String,
    },
}

impl Verdict {
    /// Whether the agent may proceed without anybody being asked.
    #[must_use]
    pub const fn is_allowed(&self) -> bool {
        matches!(self, Self::Allowed)
    }
}

/// Decide one request against one grant.
///
/// Total over [`Reach`], deliberately: every kind of request has an answer here, including the ones
/// that are never permitted. A default arm would mean the next variant added is silently allowed or
/// silently refused, depending on which way somebody wrote it, and neither is a decision anybody
/// made.
#[must_use]
pub fn decide(grant: &CapsuleGrant, reach: &Reach) -> Verdict {
    match reach {
        // Reading and writing are the same question here. A capsule's filesystem view is its own;
        // what it may reach outside the workspace is nothing, in either direction. Reading is not
        // the milder case — exfiltration is a read.
        Reach::ReadPath { path } | Reach::WritePath { path } => {
            if grant.workspace.contains(path) {
                Verdict::Allowed
            } else {
                Verdict::NotGranted {
                    wanted: format!("{} outside the workspace", path.display()),
                }
            }
        }
        Reach::RunProgram { program } => {
            if grant.may_execute {
                Verdict::Allowed
            } else {
                Verdict::NotGranted {
                    wanted: format!("running {program}"),
                }
            }
        }
        Reach::ConnectHost { host } => {
            if grant.network.permits(host) {
                Verdict::Allowed
            } else {
                Verdict::NotGranted {
                    wanted: format!("a connection to {host}"),
                }
            }
        }
        Reach::CallTool { tool } => {
            if grant.tools.iter().any(|granted| granted == tool) {
                Verdict::Allowed
            } else {
                Verdict::NotGranted {
                    wanted: format!("the {tool} tool"),
                }
            }
        }
        // Whether the grant names this class at all. Whether there is any budget left is a fact
        // about the lease, not the grant, and  asks it.
        Reach::UseModel { class } => {
            if grant
                .model
                .as_ref()
                .is_some_and(|model| model.class == *class)
            {
                Verdict::Allowed
            } else {
                Verdict::NotGranted {
                    wanted: format!("a {class} model"),
                }
            }
        }
        // Never allowed from here, and never refused from here either. The capsule has no opinion
        // about whether restarting a service is a good idea; it only knows this is not something a
        // capsule does, so it goes to the party whose job that is.
        Reach::ActOnHost { operation, target } => Verdict::CrossesBoundary {
            operation: operation.clone(),
            target: target.clone(),
        },
        Reach::ReachAnotherCapsule => Verdict::Refused {
            because: Refusal::AnotherCapsule,
        },
        Reach::ReachTheJournal => Verdict::Refused {
            because: Refusal::TheJournal,
        },
        Reach::ReachTheKeyStore => Verdict::Refused {
            because: Refusal::TheKeyStore,
        },
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use time::Duration;
    use uuid::Uuid;

    use super::*;
    use crate::grant::{ModelGrant, NetworkGrant, ResourceBudget, Workspace};

    /// A development profile: the shape a person actually grants.
    fn developer() -> CapsuleGrant {
        CapsuleGrant {
            capsule_id: Uuid::from_u128(8472),
            agent: "opencode".to_owned(),
            workspace: Workspace::at("/srv/project"),
            network: NetworkGrant::to(&["github.com", "registry.npmjs.org", "api.mistral.ai"]),
            budget: ResourceBudget {
                memory_mib: 4096,
                cpus: 2,
                tasks_max: 512,
                lifetime: Duration::hours(4),
            },
            model: Some(ModelGrant {
                class: "Strong".to_owned(),
                spend_limit: 100,
            }),
            tools: vec!["git".to_owned(), "tests".to_owned()],
            may_execute: true,
        }
    }

    #[test]
    fn an_agent_working_inside_its_grant_is_never_asked_anything() {
        // The point of the whole design. A profile granted once, and then hours of work with
        // nobody interrupted — because an interface that asks anyway has not made a weaker promise,
        // it has made the grant meaningless.
        let grant = developer();
        let ordinary = [
            Reach::ReadPath {
                path: PathBuf::from("/srv/project/src/main.rs"),
            },
            Reach::WritePath {
                path: PathBuf::from("/srv/project/target/debug/thing"),
            },
            Reach::RunProgram {
                program: "cargo".to_owned(),
            },
            Reach::ConnectHost {
                host: "registry.npmjs.org".to_owned(),
            },
            Reach::CallTool {
                tool: "git".to_owned(),
            },
            Reach::UseModel {
                class: "Strong".to_owned(),
            },
        ];
        for reach in &ordinary {
            assert!(
                decide(&grant, reach).is_allowed(),
                "a development profile refused {}",
                reach.name()
            );
        }
    }

    #[test]
    fn acting_on_the_host_is_a_proposal_and_not_an_answer() {
        // The capsule has no opinion about whether restarting a service is wise. It knows this is
        // not something a capsule does, and hands it to the party whose job that is.
        let verdict = decide(
            &developer(),
            &Reach::ActOnHost {
                operation: "service.restart".to_owned(),
                target: "systemd:postgresql.service".to_owned(),
            },
        );
        assert_eq!(
            verdict,
            Verdict::CrossesBoundary {
                operation: "service.restart".to_owned(),
                target: "systemd:postgresql.service".to_owned(),
            }
        );
        assert!(!verdict.is_allowed());
    }

    #[test]
    fn the_three_things_no_profile_may_grant_are_refused_by_the_widest_profile_there_is() {
        // Not "not in this grant" — never. There is no answer a person could give that would make
        // these safe, so offering the question would be offering a choice that does not exist.
        let mut generous = developer();
        generous.tools = vec!["everything".to_owned()];
        generous.network = NetworkGrant::to(&["github.com"]);
        generous.may_execute = true;

        for (reach, expected) in [
            (Reach::ReachAnotherCapsule, Refusal::AnotherCapsule),
            (Reach::ReachTheJournal, Refusal::TheJournal),
            (Reach::ReachTheKeyStore, Refusal::TheKeyStore),
        ] {
            assert_eq!(
                decide(&generous, &reach),
                Verdict::Refused { because: expected },
                "{} was not refused",
                reach.name()
            );
        }
    }

    #[test]
    fn reading_outside_the_workspace_is_refused_as_firmly_as_writing() {
        // Reading is not the milder case. Exfiltration is a read, and a design that guarded writes
        // more carefully than reads would be guarding the wrong direction.
        let grant = developer();
        let outside = PathBuf::from("/etc/ssh/ssh_host_ed25519_key");
        assert!(
            !decide(
                &grant,
                &Reach::ReadPath {
                    path: outside.clone()
                }
            )
            .is_allowed()
        );
        assert!(!decide(&grant, &Reach::WritePath { path: outside }).is_allowed());
    }

    #[test]
    fn climbing_out_of_the_workspace_is_refused() {
        // The escape that a prefix comparison admits, decided here rather than left to whoever
        // wires this to a filesystem.
        let verdict = decide(
            &developer(),
            &Reach::ReadPath {
                path: PathBuf::from("/srv/project/../../etc/shadow"),
            },
        );
        assert!(!verdict.is_allowed(), "{verdict:?}");
    }

    #[test]
    fn a_narrow_grant_says_it_is_narrow_rather_than_saying_it_is_impossible() {
        // What a person learns from this matters. "Your profile does not include that" sends them
        // to widen the profile; "that can never happen" sends them to argue with the architecture.
        let mut narrow = developer();
        narrow.may_execute = false;
        narrow.network = NetworkGrant::default();

        let verdict = decide(
            &narrow,
            &Reach::ConnectHost {
                host: "github.com".to_owned(),
            },
        );
        match verdict {
            Verdict::NotGranted { wanted } => assert!(wanted.contains("github.com"), "{wanted}"),
            other => panic!("a narrow grant produced {other:?}"),
        }

        assert!(matches!(
            decide(
                &narrow,
                &Reach::RunProgram {
                    program: "sh".to_owned()
                }
            ),
            Verdict::NotGranted { .. }
        ));
    }

    #[test]
    fn a_grant_that_permits_nothing_permits_nothing() {
        // The default a profile is built up from. Every capability should be a line somebody wrote.
        let nothing =
            CapsuleGrant::nothing_but(Workspace::at("/srv/empty"), Uuid::from_u128(1), "opencode");
        assert!(
            !decide(
                &nothing,
                &Reach::RunProgram {
                    program: "sh".to_owned()
                }
            )
            .is_allowed()
        );
        assert!(
            !decide(
                &nothing,
                &Reach::ConnectHost {
                    host: "github.com".to_owned()
                }
            )
            .is_allowed()
        );
        assert!(
            !decide(
                &nothing,
                &Reach::UseModel {
                    class: "Strong".to_owned()
                }
            )
            .is_allowed()
        );
    }

    #[test]
    fn a_verdict_survives_the_wire() {
        let verdicts = vec![
            Verdict::Allowed,
            Verdict::CrossesBoundary {
                operation: "service.restart".to_owned(),
                target: "systemd:postgresql.service".to_owned(),
            },
            Verdict::Refused {
                because: Refusal::TheKeyStore,
            },
            Verdict::NotGranted {
                wanted: "a connection to example.com".to_owned(),
            },
        ];
        let mut encoded = Vec::new();
        ciborium::into_writer(&verdicts, &mut encoded).expect("encodes");
        let decoded: Vec<Verdict> = ciborium::from_reader(encoded.as_slice()).expect("decodes");
        assert_eq!(decoded, verdicts);
    }
}
