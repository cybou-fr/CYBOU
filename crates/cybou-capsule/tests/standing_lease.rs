// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Public-boundary gate for profile selection and standing lease issuance.

use std::path::PathBuf;

use cybou_capsule::{
    CapabilityProfile, LeaseRequest, ModelGrant, NetworkGrant, Reach, ResourceBudget, Verdict,
    Workspace, decide_under_lease, issue_lease,
};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

fn selected_development_profile() -> CapabilityProfile {
    let mut profile = CapabilityProfile::bounded(
        "sandboxed-development",
        ResourceBudget {
            memory_mib: 4096,
            cpus: 2,
            tasks_max: 512,
            lifetime: Duration::hours(4),
        },
    )
    .expect("the launch surface uses a stable profile id");
    profile.network = NetworkGrant::to(&["github.com", "registry.npmjs.org"]);
    profile.model = Some(ModelGrant {
        class: "Strong".to_owned(),
        spend_limit: 100,
    });
    profile.tools = vec!["git".to_owned(), "tests".to_owned()];
    profile.may_execute = true;
    profile
}

#[test]
fn a_public_launch_selection_becomes_one_silent_standing_lease() {
    let issued_at = OffsetDateTime::from_unix_timestamp(1_787_000_000).expect("fixed instant");
    let lease = issue_lease(
        LeaseRequest {
            selected_profile: selected_development_profile(),
            capsule_id: Uuid::from_u128(8472),
            agent: "opencode".to_owned(),
            workspace: Workspace::at("/srv/project"),
        },
        issued_at,
    )
    .expect("the selected profile is issuable");

    assert_eq!(lease.profile_id().as_str(), "sandboxed-development");
    for reach in [
        Reach::ReadPath {
            path: PathBuf::from("/srv/project/src/main.rs"),
        },
        Reach::WritePath {
            path: PathBuf::from("/srv/project/target/output"),
        },
        Reach::RunProgram {
            program: "cargo test".to_owned(),
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
    ] {
        assert_eq!(
            decide_under_lease(&lease, &reach, issued_at + Duration::minutes(1)),
            Verdict::Allowed,
            "the selected profile interrupted {}",
            reach.name()
        );
    }
}

#[test]
fn the_public_mint_refuses_a_profile_that_cannot_run() {
    let mut profile = selected_development_profile();
    profile.budget.lifetime = Duration::ZERO;
    assert!(
        issue_lease(
            LeaseRequest {
                selected_profile: profile,
                capsule_id: Uuid::from_u128(8472),
                agent: "opencode".to_owned(),
                workspace: Workspace::at("/srv/project"),
            },
            OffsetDateTime::UNIX_EPOCH,
        )
        .is_err()
    );
}
