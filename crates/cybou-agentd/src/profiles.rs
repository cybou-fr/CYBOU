// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! The profiles an operator approved, and the only thing a caller gets to choose between them.
//!
//! `launch` takes ceilings as arguments, which is right for bring-up on a host somebody is sitting
//! at: whoever can run it is already `cybou`. It is wrong for anything reachable — a bus method or a
//! web endpoint carrying the same shape would be asking its caller to invent a `CapsuleGrant`, which
//! is the one thing a capability profile exists to prevent.
//!
//! So there is a second door. A caller names a profile, an agent and a workspace; every bound comes
//! from the profile, and the profile comes from a file only root can write.
//!
//! ```text
//! the caller chooses    which profile, which agent, which workspace, which offered model
//! the profile decides   memory, CPUs, tasks, lifetime, network, spending, token ceilings,
//!                       sensitivity, execution
//! ```
//!
//! ## Three things a caller could otherwise have widened
//!
//! Naming a profile is not the whole of it, and the parts that are easy to miss are the interesting
//! ones.
//!
//! **The workspace.** It is the one directory an agent may change, and a caller supplying it freely
//! could supply `/etc`. So a profile carries the roots a workspace may live under, and a path outside
//! all of them is refused — before anything is minted, and lexically, so `/projects/../etc` is not a
//! project.
//!
//! **The agent.** A profile that named ceilings but let any pack run under them would let a caller
//! pick a different agent than the one those ceilings were approved for.
//!
//! **The model.** A caller picks from what the profile offers, by class. It cannot name a class the
//! profile does not carry, and it supplies no bound that goes with one: the spending policy, the
//! token ceilings and the sensitivity are attached to the class the operator approved. So choosing
//! "Free" cannot come with a ceiling of a hundred beside it, and no caller decides how exposing a
//! prompt its agent may send.
//!
//! ## What this deliberately is not
//!
//! Authorization. Nothing here asks *who* is calling: it decides what a named profile permits, and it
//! would answer the same for anyone. Deciding whether a particular caller may use a particular
//! profile is a different question, and one this file would answer badly by guessing.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use time::Duration;

use cybou_capsule::{
    CapabilityProfile, ModelGrant, NetworkGrant, ResourceBudget, SpendPolicy, Workspace,
};

use crate::plan::Ceilings;

/// One model an operator approved for a profile, with the policy that comes with it.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferedModel {
    /// The class a caller names to choose this.
    pub class: String,
    /// The spending policy attached to it.
    ///
    /// Attached here rather than chosen by the caller, so picking a free model cannot arrive with a
    /// ceiling of a hundred beside it.
    pub spend: SpendPolicy,
    /// Total input plus output tokens one task's bearer may consume.
    pub token_limit: u64,
    /// Per-request output ceiling.
    pub max_output_tokens: u32,
    /// The most exposing content a bearer for this class may carry.
    ///
    /// Here rather than on the request for the same reason as the spending policy, and with more
    /// force: sensitivity decides which routes may see a prompt at all, so a caller naming its own
    /// would be choosing what a model is allowed to be told.
    pub sensitivity: u8,
}

/// One profile, exactly as an operator wrote it down.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferedProfile {
    /// The name a caller uses.
    pub id: String,
    /// Which agents may run under these bounds.
    pub agents: Vec<String>,
    /// The directories a workspace may live under.
    pub workspace_roots: Vec<PathBuf>,
    /// Memory ceiling, in mebibytes.
    pub memory_mib: u32,
    /// CPU ceiling.
    pub cpus: u32,
    /// Process ceiling.
    pub tasks_max: u32,
    /// The longest a session under this profile may live, in seconds.
    pub lifetime_seconds: i64,
    /// Exactly the hosts a capsule may reach.
    #[serde(default)]
    pub hosts: Vec<String>,
    /// The models on offer. Empty is a profile for work that needs none.
    #[serde(default)]
    pub models: Vec<OfferedModel>,
    /// Whether arbitrary programs may run inside the capsule's own namespaces.
    #[serde(default)]
    pub may_execute: bool,
}

/// Everything an operator approved on this host.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
pub struct ProfileCatalogue {
    /// The profiles, in the order they were written.
    pub profiles: Vec<OfferedProfile>,
}

/// What a caller asked for.
#[derive(Clone, Debug, PartialEq)]
pub struct Wanted {
    /// Which profile.
    pub profile: String,
    /// Which agent.
    pub agent: String,
    /// Which directory the agent may change.
    pub workspace: PathBuf,
    /// Which offered model, by class, or none.
    pub model_class: Option<String>,
}

/// Why a request cannot become a lease.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CannotOffer {
    /// The catalogue could not be read.
    Unreadable,
    /// No profile by that name is approved on this host.
    NoSuchProfile(String),
    /// The profile does not permit that agent.
    AgentNotOffered(String),
    /// The workspace is not under any root this profile permits.
    WorkspaceOutsideRoots(PathBuf),
    /// The profile does not offer that model class.
    ModelNotOffered(String),
    /// The profile itself is not something that can be granted.
    NotGrantable(String),
}

impl core::fmt::Display for CannotOffer {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Unreadable => formatter.write_str("the approved profiles could not be read"),
            Self::NoSuchProfile(id) => write!(formatter, "'{id}' is not an approved profile"),
            Self::AgentNotOffered(agent) => {
                write!(formatter, "this profile does not run '{agent}'")
            }
            Self::WorkspaceOutsideRoots(path) => write!(
                formatter,
                "{} is not under a directory this profile permits",
                path.display()
            ),
            Self::ModelNotOffered(class) => {
                write!(formatter, "this profile does not offer a '{class}' model")
            }
            Self::NotGrantable(why) => write!(formatter, "this profile cannot be granted: {why}"),
        }
    }
}

impl core::error::Error for CannotOffer {}

impl ProfileCatalogue {
    /// Read what an operator approved.
    ///
    /// # Errors
    ///
    /// Returns [`CannotOffer::Unreadable`] when the bytes are not a catalogue this build understands.
    /// A malformed catalogue is refused whole rather than partly accepted: a file half of which
    /// parsed would silently offer fewer profiles than an operator wrote, and the missing ones would
    /// look like a caller's mistake.
    pub fn read(bytes: &[u8]) -> Result<Self, CannotOffer> {
        serde_json::from_slice(bytes).map_err(|_| CannotOffer::Unreadable)
    }

    /// The profile by that name, if it is approved here.
    #[must_use]
    pub fn find(&self, id: &str) -> Option<&OfferedProfile> {
        self.profiles.iter().find(|profile| profile.id == id)
    }

    /// Every profile's name, for a surface offering a choice.
    #[must_use]
    pub fn names(&self) -> Vec<&str> {
        self.profiles
            .iter()
            .map(|profile| profile.id.as_str())
            .collect()
    }

    /// Convert to the protocol offers response.
    #[must_use]
    pub fn to_response(
        &self,
        capacity_bounded: bool,
        provider_connected: bool,
    ) -> cybou_protocol::agent::AgentOffersResponse {
        cybou_protocol::agent::AgentOffersResponse {
            profiles: self
                .profiles
                .iter()
                .map(|p| cybou_protocol::agent::OfferedProfileView {
                    id: p.id.clone(),
                    agents: p.agents.clone(),
                    workspace_roots: p
                        .workspace_roots
                        .iter()
                        .map(|w| w.display().to_string())
                        .collect(),
                    memory_mib: p.memory_mib,
                    cpus: p.cpus,
                    tasks_max: p.tasks_max,
                    lifetime_seconds: p.lifetime_seconds,
                    hosts: p.hosts.clone(),
                    models: p
                        .models
                        .iter()
                        .map(|m| cybou_protocol::agent::OfferedModelView {
                            class: m.class.clone(),
                            zero_cost: matches!(m.spend, cybou_capsule::SpendPolicy::ZeroCostOnly),
                            spend_limit: match m.spend {
                                cybou_capsule::SpendPolicy::Capped(limit) => Some(limit),
                                cybou_capsule::SpendPolicy::ZeroCostOnly => None,
                            },
                        })
                        .collect(),
                    may_execute: p.may_execute,
                })
                .collect(),
            capacity_bounded,
            provider_connected,
        }
    }

    /// Turn one request into the exact bounds an operator approved for it.
    ///
    /// # Errors
    ///
    /// Returns [`CannotOffer`] when the profile is unknown, does not run that agent, does not permit
    /// a workspace there, or does not offer that model.
    pub fn grant(
        &self,
        wanted: &Wanted,
    ) -> Result<(CapabilityProfile, Workspace, Option<Ceilings>), CannotOffer> {
        let profile = self
            .find(&wanted.profile)
            .ok_or_else(|| CannotOffer::NoSuchProfile(wanted.profile.clone()))?;

        if !profile.agents.iter().any(|agent| agent == &wanted.agent) {
            return Err(CannotOffer::AgentNotOffered(wanted.agent.clone()));
        }
        let workspace = permitted_workspace(profile, &wanted.workspace)?;

        let offered = match &wanted.model_class {
            Some(class) => Some(
                profile
                    .models
                    .iter()
                    .find(|offered| &offered.class == class)
                    .ok_or_else(|| CannotOffer::ModelNotOffered(class.clone()))?,
            ),
            None => None,
        };
        let model = offered.map(|offered| ModelGrant {
            class: offered.class.clone(),
            spend: offered.spend,
        });
        let ceilings = offered.map(|offered| Ceilings {
            token_limit: offered.token_limit,
            max_output_tokens: offered.max_output_tokens,
            sensitivity: offered.sensitivity,
        });

        let mut granted = CapabilityProfile::bounded(
            profile.id.clone(),
            ResourceBudget {
                memory_mib: profile.memory_mib,
                cpus: profile.cpus,
                tasks_max: profile.tasks_max,
                lifetime: Duration::seconds(profile.lifetime_seconds),
            },
        )
        .map_err(|why| CannotOffer::NotGrantable(why.to_string()))?;
        granted.network = NetworkGrant {
            hosts: profile.hosts.clone(),
        };
        granted.model = model;
        granted.may_execute = profile.may_execute;
        Ok((granted, workspace, ceilings))
    }
}

/// Whether this workspace is under something the profile permits.
///
/// Resolved lexically before comparing, so a path that climbs out of a permitted root is refused
/// rather than accepted by string prefix. `/projects/../etc` is inside `/projects` by spelling and
/// outside it by meaning, and it is the second that decides.
///
/// Symlinks are not followed, for the reason `Workspace::contains` gives: that answer depends on when
/// it was asked, and the mount and the Landlock rule are what actually hold it.
fn permitted_workspace(profile: &OfferedProfile, wanted: &Path) -> Result<Workspace, CannotOffer> {
    let outside = || CannotOffer::WorkspaceOutsideRoots(wanted.to_path_buf());
    if profile.workspace_roots.is_empty() {
        // A profile naming no roots permits no workspace. Reading that as "anywhere" would make the
        // most permissive configuration the one an operator gets by leaving a field out.
        return Err(outside());
    }
    for root in &profile.workspace_roots {
        if Workspace::at(root.clone()).contains(wanted) {
            return Ok(Workspace::at(wanted.to_path_buf()));
        }
    }
    Err(outside())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalogue() -> ProfileCatalogue {
        ProfileCatalogue {
            profiles: vec![OfferedProfile {
                id: "sandboxed-autonomous".to_owned(),
                agents: vec!["opencode".to_owned()],
                workspace_roots: vec![PathBuf::from("/projects")],
                memory_mib: 4096,
                cpus: 2,
                tasks_max: 512,
                lifetime_seconds: 4 * 60 * 60,
                hosts: vec!["github.com".to_owned()],
                models: vec![
                    OfferedModel {
                        class: "Free".to_owned(),
                        spend: SpendPolicy::ZeroCostOnly,
                        token_limit: 50_000,
                        max_output_tokens: 1024,
                        sensitivity: 0,
                    },
                    OfferedModel {
                        class: "Strong".to_owned(),
                        spend: SpendPolicy::Capped(100),
                        token_limit: 200_000,
                        max_output_tokens: 4096,
                        sensitivity: 1,
                    },
                ],
                may_execute: true,
            }],
        }
    }

    fn wanting(workspace: &str, model: Option<&str>) -> Wanted {
        Wanted {
            profile: "sandboxed-autonomous".to_owned(),
            agent: "opencode".to_owned(),
            workspace: PathBuf::from(workspace),
            model_class: model.map(ToOwned::to_owned),
        }
    }

    #[test]
    fn every_bound_comes_from_the_profile_and_none_from_the_caller() {
        let (granted, workspace, ceilings) = catalogue()
            .grant(&wanting("/projects/app", Some("Strong")))
            .expect("granted");

        assert_eq!(granted.budget.memory_mib, 4096);
        assert_eq!(granted.budget.cpus, 2);
        assert_eq!(granted.budget.tasks_max, 512);
        assert_eq!(granted.budget.lifetime, Duration::hours(4));
        assert_eq!(granted.network.hosts, ["github.com"]);
        assert!(granted.may_execute);
        assert_eq!(workspace.root, PathBuf::from("/projects/app"));
        let ceilings = ceilings.expect("a model was chosen");
        assert_eq!(ceilings.token_limit, 200_000);
        assert_eq!(ceilings.max_output_tokens, 4096);
        assert_eq!(ceilings.sensitivity, 1);
    }

    #[test]
    fn a_spending_policy_travels_with_the_class_it_was_approved_for() {
        // Otherwise choosing "Free" could arrive with a ceiling of a hundred beside it, which is the
        // caller writing half the grant.
        let catalogue = catalogue();
        let free = catalogue
            .grant(&wanting("/projects/app", Some("Free")))
            .expect("granted");
        let strong = catalogue
            .grant(&wanting("/projects/app", Some("Strong")))
            .expect("granted");

        assert_eq!(
            free.0.model.expect("a model").spend,
            SpendPolicy::ZeroCostOnly
        );
        assert_eq!(
            strong.0.model.expect("a model").spend,
            SpendPolicy::Capped(100)
        );
        // And the ceilings travel with the class too. A caller choosing the cheaper model cannot
        // bring the stronger one's allowance along with it.
        assert_eq!(free.2.expect("ceilings").sensitivity, 0);
        assert_eq!(strong.2.expect("ceilings").sensitivity, 1);
    }

    #[test]
    fn a_workspace_outside_every_permitted_root_is_refused() {
        // The one directory an agent may change. A caller supplying it freely could supply /etc.
        assert_eq!(
            catalogue().grant(&wanting("/etc", Some("Strong"))),
            Err(CannotOffer::WorkspaceOutsideRoots(PathBuf::from("/etc")))
        );
    }

    #[test]
    fn a_workspace_that_climbs_out_of_a_root_is_outside_it() {
        // Inside by spelling, outside by meaning, and it is the second that decides. A prefix test
        // would have admitted this.
        let climbing = "/projects/../etc/shadow";
        assert_eq!(
            catalogue().grant(&wanting(climbing, None)),
            Err(CannotOffer::WorkspaceOutsideRoots(PathBuf::from(climbing)))
        );
    }

    #[test]
    fn a_profile_naming_no_roots_permits_no_workspace() {
        // Reading an absent field as "anywhere" would make the most permissive configuration the one
        // an operator gets by leaving something out.
        let mut catalogue = catalogue();
        catalogue.profiles[0].workspace_roots.clear();

        assert!(matches!(
            catalogue.grant(&wanting("/projects/app", None)),
            Err(CannotOffer::WorkspaceOutsideRoots(_))
        ));
    }

    #[test]
    fn a_profile_runs_only_the_agents_it_names() {
        // Ceilings are approved for an agent, not in the abstract.
        let mut wanted = wanting("/projects/app", Some("Strong"));
        wanted.agent = "some-other-pack".to_owned();

        assert_eq!(
            catalogue().grant(&wanted),
            Err(CannotOffer::AgentNotOffered("some-other-pack".to_owned()))
        );
    }

    #[test]
    fn a_model_the_profile_does_not_offer_is_refused() {
        assert_eq!(
            catalogue().grant(&wanting("/projects/app", Some("Enormous"))),
            Err(CannotOffer::ModelNotOffered("Enormous".to_owned()))
        );
    }

    #[test]
    fn asking_for_no_model_is_an_ordinary_request() {
        // A capsule that was never going to ask. The same case the capsule crate holds open, and it
        // has to survive this door too.
        let (granted, _, ceilings) = catalogue()
            .grant(&wanting("/projects/app", None))
            .expect("granted");
        assert!(granted.model.is_none());
        assert!(
            ceilings.is_none(),
            "there is no bearer, so there is nothing for a ceiling to bound"
        );
    }

    #[test]
    fn an_unknown_profile_is_named_rather_than_substituted() {
        let mut wanted = wanting("/projects/app", None);
        wanted.profile = "whatever-is-handy".to_owned();

        assert_eq!(
            catalogue().grant(&wanted),
            Err(CannotOffer::NoSuchProfile("whatever-is-handy".to_owned()))
        );
    }

    #[test]
    fn a_catalogue_survives_the_file_it_is_written_in() {
        let original = catalogue();
        let rendered = serde_json::to_vec(&original).expect("encodes");

        assert_eq!(
            ProfileCatalogue::read(&rendered).expect("decodes"),
            original
        );
        assert_eq!(original.names(), ["sandboxed-autonomous"]);
    }

    #[test]
    fn a_malformed_catalogue_is_refused_whole_rather_than_partly_accepted() {
        // A file half of which parsed would offer fewer profiles than an operator wrote, and the
        // missing ones would look like a caller's mistake.
        assert_eq!(
            ProfileCatalogue::read(b"[{\"id\": \"broken\"}]"),
            Err(CannotOffer::Unreadable)
        );
    }
}
