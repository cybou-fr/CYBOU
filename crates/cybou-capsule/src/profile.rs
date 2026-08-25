// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! The boundary between a profile somebody selected and the standing lease an agent receives.
//!
//! A profile is reusable policy. A capsule id, agent and workspace are not: they name one launch.
//! Keeping those apart prevents a saved profile from quietly becoming authority over whichever
//! workspace happens to be current when it is reused.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::compile::{CannotCompile, compile};
use crate::grant::{CapsuleGrant, ModelGrant, NetworkGrant, ResourceBudget, Workspace};
use crate::lease::Lease;

/// A stable name for a capability profile shown at launch and recorded on its lease.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct ProfileId(String);

impl ProfileId {
    /// Parse a profile id.
    ///
    /// IDs are deliberately smaller than display names: lowercase ASCII words separated by `-`
    /// survive logs, files and wire formats without acquiring aliases.
    ///
    /// # Errors
    ///
    /// Returns [`CannotIssueLease::InvalidProfileId`] when the value is empty, too long, contains
    /// anything other than lowercase ASCII letters, digits and internal hyphens, or begins or ends
    /// with a hyphen.
    pub fn parse(value: impl Into<String>) -> Result<Self, CannotIssueLease> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.len() <= 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
            && !value.starts_with('-')
            && !value.ends_with('-');
        if !valid {
            return Err(CannotIssueLease::InvalidProfileId(value));
        }
        Ok(Self(value))
    }

    /// The wire and audit representation.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Reusable bounds a person can select on the launch surface.
///
/// All fields remain data, rather than an enum of Cybou-owned presets. The product may offer
/// presets, but the authority is the exact profile shown and selected, not the preset's label.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityProfile {
    /// Which profile was selected.
    pub id: ProfileId,
    /// Exact outbound hosts, with no patterns.
    pub network: NetworkGrant,
    /// Physical and time ceilings.
    pub budget: ResourceBudget,
    /// Optional model class and spending ceiling.
    pub model: Option<ModelGrant>,
    /// Host-mediated tools, by exact name.
    pub tools: Vec<String>,
    /// Whether arbitrary programs may run inside the capsule namespaces.
    pub may_execute: bool,
}

impl CapabilityProfile {
    /// Start a named profile with no network, model, tools or execution authority.
    ///
    /// # Errors
    ///
    /// Returns [`CannotIssueLease::InvalidProfileId`] when `id` is not a stable profile id.
    pub fn bounded(
        id: impl Into<String>,
        budget: ResourceBudget,
    ) -> Result<Self, CannotIssueLease> {
        Ok(Self {
            id: ProfileId::parse(id)?,
            network: NetworkGrant::default(),
            budget,
            model: None,
            tools: Vec::new(),
            may_execute: false,
        })
    }
}

/// The launch-specific values beside the selected profile.
#[derive(Clone, Debug, PartialEq)]
pub struct LeaseRequest {
    /// The selected reusable policy.
    pub selected_profile: CapabilityProfile,
    /// Fresh identity for this capsule.
    pub capsule_id: Uuid,
    /// Agent implementation this launch will run.
    pub agent: String,
    /// The one host directory mounted as the workspace.
    pub workspace: Workspace,
}

/// Why a selected profile cannot become a standing lease.
#[derive(Clone, Debug, PartialEq)]
pub enum CannotIssueLease {
    /// A profile id is not a stable lowercase identifier.
    InvalidProfileId(String),
    /// No agent was bound to the launch.
    AgentIsBlank,
    /// A network entry is not one exact DNS host or IP address.
    InvalidNetworkHost(String),
    /// The same host appeared more than once, including aliases that differ only by case.
    DuplicateNetworkHost(String),
    /// A mediated tool has no usable exact name.
    InvalidTool(String),
    /// The same mediated tool was granted more than once.
    DuplicateTool(String),
    /// A model grant did not name a class.
    ModelClassIsBlank,
    /// The resulting kernel grant cannot safely run.
    GrantCannotCompile(CannotCompile),
}

impl core::fmt::Display for CannotIssueLease {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidProfileId(id) => write!(formatter, "'{id}' is not a valid profile id"),
            Self::AgentIsBlank => formatter.write_str("the launch does not name an agent"),
            Self::InvalidNetworkHost(host) => {
                write!(formatter, "'{host}' is not one exact network host")
            }
            Self::DuplicateNetworkHost(host) => {
                write!(formatter, "network host '{host}' is granted more than once")
            }
            Self::InvalidTool(tool) => write!(formatter, "'{tool}' is not an exact tool name"),
            Self::DuplicateTool(tool) => {
                write!(formatter, "tool '{tool}' is granted more than once")
            }
            Self::ModelClassIsBlank => formatter.write_str("the model grant has no class"),
            Self::GrantCannotCompile(reason) => write!(formatter, "the grant cannot run: {reason}"),
        }
    }
}

impl core::error::Error for CannotIssueLease {}

impl From<CannotCompile> for CannotIssueLease {
    fn from(value: CannotCompile) -> Self {
        Self::GrantCannotCompile(value)
    }
}

/// Issue the standing lease represented by one explicit launch selection.
///
/// This is the only public mint. It copies the exact selected bounds, adds only the launch binding,
/// and refuses malformed or un-runnable profiles before a lease exists.
///
/// # Errors
///
/// Returns [`CannotIssueLease`] when the profile or launch binding is ambiguous, or when the exact
/// resulting grant cannot be compiled into a runnable kernel capsule.
pub fn issue_lease(
    request: LeaseRequest,
    issued_at: OffsetDateTime,
) -> Result<Lease, CannotIssueLease> {
    validate_request(&request)?;

    let LeaseRequest {
        selected_profile,
        capsule_id,
        agent,
        workspace,
    } = request;
    let grant = CapsuleGrant {
        capsule_id,
        agent,
        workspace,
        network: selected_profile.network,
        budget: selected_profile.budget,
        model: selected_profile.model,
        tools: selected_profile.tools,
        may_execute: selected_profile.may_execute,
    };

    // Validation and enforcement consume the same grant. Constructing a second approximation here
    // would let the launch screen approve one thing and the backend receive another.
    compile(&grant)?;
    Ok(Lease::issued_from_profile(
        selected_profile.id,
        grant,
        issued_at,
    ))
}

fn validate_request(request: &LeaseRequest) -> Result<(), CannotIssueLease> {
    // Deserialisation can construct these types without their convenience constructors, so every
    // invariant is checked again at the mint.
    ProfileId::parse(request.selected_profile.id.as_str())?;
    if request.agent.trim().is_empty() || request.agent.trim() != request.agent {
        return Err(CannotIssueLease::AgentIsBlank);
    }

    let mut hosts = HashSet::new();
    for host in &request.selected_profile.network.hosts {
        if !exact_host(host) {
            return Err(CannotIssueLease::InvalidNetworkHost(host.clone()));
        }
        if !hosts.insert(host.to_ascii_lowercase()) {
            return Err(CannotIssueLease::DuplicateNetworkHost(host.clone()));
        }
    }

    let mut tools = HashSet::new();
    for tool in &request.selected_profile.tools {
        if tool.is_empty()
            || tool.trim() != tool
            || !tool
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        {
            return Err(CannotIssueLease::InvalidTool(tool.clone()));
        }
        if !tools.insert(tool.as_str()) {
            return Err(CannotIssueLease::DuplicateTool(tool.clone()));
        }
    }

    if request
        .selected_profile
        .model
        .as_ref()
        .is_some_and(|model| model.class.trim().is_empty() || model.class.trim() != model.class)
    {
        return Err(CannotIssueLease::ModelClassIsBlank);
    }
    Ok(())
}

fn exact_host(host: &str) -> bool {
    if host.is_empty() || host.trim() != host || host.contains('*') || host.contains('/') {
        return false;
    }
    if host.parse::<std::net::IpAddr>().is_ok() {
        return true;
    }
    host.len() <= 253
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                && !label.starts_with('-')
                && !label.ends_with('-')
        })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use time::Duration;

    use super::*;
    use crate::{Reach, Verdict, decide_under_lease};

    fn at(offset: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_787_000_000 + offset).expect("a fixed instant")
    }

    fn development() -> CapabilityProfile {
        let mut profile = CapabilityProfile::bounded(
            "sandboxed-development",
            ResourceBudget {
                memory_mib: 4096,
                cpus: 2,
                tasks_max: 512,
                lifetime: Duration::hours(4),
            },
        )
        .expect("valid profile id");
        profile.network = NetworkGrant::to(&["github.com", "registry.npmjs.org"]);
        profile.model = Some(ModelGrant {
            class: "Strong".to_owned(),
            spend_limit: 100,
        });
        profile.tools = vec!["git".to_owned(), "tests".to_owned()];
        profile.may_execute = true;
        profile
    }

    fn request(profile: CapabilityProfile) -> LeaseRequest {
        LeaseRequest {
            selected_profile: profile,
            capsule_id: Uuid::from_u128(8472),
            agent: "opencode".to_owned(),
            workspace: Workspace::at("/srv/project"),
        }
    }

    #[test]
    fn the_lease_is_exactly_the_selected_profile_plus_launch_binding() {
        let profile = development();
        let lease = issue_lease(request(profile.clone()), at(0)).expect("lease is issued");

        assert_eq!(lease.profile_id, profile.id);
        assert_eq!(lease.grant.capsule_id, Uuid::from_u128(8472));
        assert_eq!(lease.grant.agent, "opencode");
        assert_eq!(lease.grant.workspace, Workspace::at("/srv/project"));
        assert_eq!(lease.grant.network, profile.network);
        assert_eq!(lease.grant.budget, profile.budget);
        assert_eq!(lease.grant.model, profile.model);
        assert_eq!(lease.grant.tools, profile.tools);
        assert_eq!(lease.grant.may_execute, profile.may_execute);
        assert_eq!(lease.model_spent, 0);
        assert_eq!(lease.revoked_at, None);
    }

    #[test]
    fn one_selection_is_silent_for_every_reach_inside_it() {
        let lease = issue_lease(request(development()), at(0)).expect("lease is issued");
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
            assert_eq!(decide_under_lease(&lease, reach, at(60)), Verdict::Allowed);
        }
    }

    #[test]
    fn issuance_adds_no_ambient_capability() {
        let mut profile = development();
        profile.network = NetworkGrant::default();
        profile.model = None;
        profile.tools.clear();
        profile.may_execute = false;
        let lease = issue_lease(request(profile), at(0)).expect("bounded lease is issued");

        for reach in [
            Reach::RunProgram {
                program: "true".to_owned(),
            },
            Reach::ConnectHost {
                host: "github.com".to_owned(),
            },
            Reach::CallTool {
                tool: "git".to_owned(),
            },
            Reach::UseModel {
                class: "Strong".to_owned(),
            },
        ] {
            assert!(!decide_under_lease(&lease, &reach, at(60)).is_allowed());
        }
    }

    #[test]
    fn an_unrunnable_or_unbound_selection_never_becomes_a_lease() {
        let mut empty = development();
        empty.budget.memory_mib = 0;
        assert!(matches!(
            issue_lease(request(empty), at(0)),
            Err(CannotIssueLease::GrantCannotCompile(
                CannotCompile::BudgetPermitsNothing("memory")
            ))
        ));

        let mut no_agent = request(development());
        no_agent.agent = " ".to_owned();
        assert_eq!(
            issue_lease(no_agent, at(0)),
            Err(CannotIssueLease::AgentIsBlank)
        );

        let mut relative = request(development());
        relative.workspace = Workspace::at("project");
        assert!(matches!(
            issue_lease(relative, at(0)),
            Err(CannotIssueLease::GrantCannotCompile(
                CannotCompile::WorkspaceIsNotAbsolute(_)
            ))
        ));
    }

    #[test]
    fn patterns_duplicates_and_ambiguous_names_fail_closed() {
        for host in ["*.example.com", "example.com/path", " example.com"] {
            let mut profile = development();
            profile.network.hosts = vec![host.to_owned()];
            assert!(matches!(
                issue_lease(request(profile), at(0)),
                Err(CannotIssueLease::InvalidNetworkHost(_))
            ));
        }

        let mut duplicate = development();
        duplicate.network.hosts = vec!["github.com".to_owned(), "GitHub.com".to_owned()];
        assert!(matches!(
            issue_lease(request(duplicate), at(0)),
            Err(CannotIssueLease::DuplicateNetworkHost(_))
        ));

        let mut blank_model = development();
        blank_model.model.as_mut().expect("model").class = " ".to_owned();
        assert_eq!(
            issue_lease(request(blank_model), at(0)),
            Err(CannotIssueLease::ModelClassIsBlank)
        );
    }
}
