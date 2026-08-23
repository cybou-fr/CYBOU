// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! The whole path a word travels, walked end to end.
//!
//! ```text
//! utterance → act → activation → proposals → attention → plan → prose
//! ```
//!
//! Every joint of this existed and was tested on its own before this file did. That is exactly the
//! condition under which things get lost: each layer holds its own invariant, nobody holds the
//! composition, and a hedge that survives five boundaries and dies at the sixth looks — from the
//! only place a person stands — identical to a hedge that was never raised.
//!
//! Three things travel the whole way, and each is asserted at the far end rather than at the joint
//! that produced it:
//!
//! - a **dispute** the epistemic owner set (ADR-0029 A4),
//! - the fact that a **budget or a quota cut the answer short** (A6, A11),
//! - **why** each concept came back at all (A12).
//!
//! Held as a dev-dependency test rather than an organ. Nothing here is how the daemons actually
//! talk — they are separate processes over D-Bus — so this composes the libraries the way a caller
//! composes the interfaces, and its subject is the loss, not the transport.

use cybou_contextd::types::AssociationOrigin;
use cybou_contextd::{ActivationBudget, ContextCore};
use cybou_meaning::{Language, plan_attention, realize};
use cybou_protocol::epistemic::EpistemicStatus;
use cybou_protocol::meaning::CognitiveActKind;
use cybou_workspaced::WorkspaceCore;
use time::OffsetDateTime;
use uuid::Uuid;

fn plan_id() -> Uuid {
    Uuid::from_u128(77)
}

/// lemon — honey — tea, with a yellow off to the side.
fn kitchen(disputed: Option<&str>) -> ContextCore {
    let context = ContextCore::new();
    let now = OffsetDateTime::now_utc();
    for concept in ["lemon", "honey", "tea", "yellow"] {
        let standing = if disputed == Some(concept) {
            EpistemicStatus::Disputed
        } else {
            EpistemicStatus::Observed
        };
        context.activate_with_standing(concept, 1.0, "observed", now, 0, standing);
    }
    for (from, to, strength) in [
        ("lemon", "honey", 0.8),
        ("honey", "tea", 0.5),
        ("lemon", "yellow", 0.9),
    ] {
        context.associate(
            from,
            to,
            strength,
            AssociationOrigin::Episodic,
            vec![Uuid::from_u128(1)],
        );
    }
    context
}

/// Walk the whole path once and return what a person would read.
fn walk(
    context: &ContextCore,
    workspace: &WorkspaceCore,
    utterance: &str,
    budget: &ActivationBudget,
) -> String {
    let interpreted = cybou_meaning::interpret(utterance, "person", OffsetDateTime::now_utc())
        .expect("the vocabulary recognises this opening");
    assert_eq!(interpreted.primary_act.kind, CognitiveActKind::Ask);

    let seed = interpreted.primary_act.subject.clone();
    let session = context.bring_to_mind(std::slice::from_ref(&seed), budget);
    let admission = workspace.consider(&session.proposals(), !session.was_cut_short());
    let plan = plan_attention(&seed, &admission, plan_id());
    realize(&plan, Language::English)
}

#[test]
fn a_dispute_set_five_boundaries_earlier_reaches_the_sentence() {
    // A4, walked rather than asserted at the joint that produced it. epistemicd contested honey;
    // the activation carried it, the proposal carried it, admission carried it, the plan turned it
    // into a typed qualification, and the renderer said it out loud. Any one of those five dropping
    // it produces prose that reads as settled, and reads that way to the only person who can tell.
    let prose = walk(
        &kitchen(Some("honey")),
        &WorkspaceCore::new(32),
        "what is lemon",
        &ActivationBudget::default(),
    );
    assert!(prose.contains("contested"), "{prose}");
}

#[test]
fn nothing_contested_produces_prose_that_does_not_hedge() {
    // The control. Without it the test above would pass on a renderer that hedges everything, and
    // a hedge that is always there is the same as no hedge at all.
    let prose = walk(
        &kitchen(None),
        &WorkspaceCore::new(32),
        "what is lemon",
        &ActivationBudget::default(),
    );
    assert!(!prose.contains("contested"), "{prose}");
    assert!(!prose.contains("not the whole of it"), "{prose}");
}

#[test]
fn a_walk_the_budget_cut_short_says_so_in_the_sentence() {
    // A6 the whole way. The activation stopped at one concept; the prose must not present that one
    // concept as everything lemon brings to mind.
    let prose = walk(
        &kitchen(None),
        &WorkspaceCore::new(32),
        "what is lemon",
        &ActivationBudget {
            nodes: 1,
            ..ActivationBudget::default()
        },
    );
    assert!(prose.contains("not the whole of it"), "{prose}");
}

#[test]
fn a_flood_reaches_the_sentence_as_a_flood_that_was_turned_away() {
    // A11 the whole way. Hundreds of things came to mind, the quota admitted a handful, and the
    // person is told both numbers rather than shown the handful.
    //
    // The graph is kept inside the organ's own concept budget on purpose. Overflowing that budget
    // is a different behaviour with its own test, and mixing the two here made this test about
    // which concepts survived eviction rather than about what attention did with them.
    let context = ContextCore::new();
    let now = OffsetDateTime::now_utc();
    context.activate_with_standing("lemon", 1.0, "observed", now, 0, EpistemicStatus::Observed);
    for index in 0..400 {
        let label = format!("concept-{index:04}");
        context.activate_with_standing(&label, 1.0, "observed", now, 0, EpistemicStatus::Observed);
        context.associate(
            "lemon",
            &label,
            0.9,
            AssociationOrigin::Episodic,
            vec![Uuid::from_u128(1)],
        );
    }

    let workspace = WorkspaceCore::new(32);
    let interpreted = cybou_meaning::interpret("what is lemon", "person", now).expect("recognised");
    let seed = interpreted.primary_act.subject.clone();
    let session = context.bring_to_mind(
        std::slice::from_ref(&seed),
        &ActivationBudget {
            nodes: 64,
            edges: 4096,
            // Generous on purpose. This test is about the quota, and the default 30ms budget makes
            // it about whichever machine is running it — a walk over four thousand edges on a busy
            // builder can be cut before its first step. That behaviour is correct and has its own
            // test; leaving it here made this one flaky, and a flaky test is a test nobody reads.
            time: std::time::Duration::from_secs(45),
            ..ActivationBudget::default()
        },
    );
    let admission = workspace.consider(&session.proposals(), !session.was_cut_short());
    let prose = realize(
        &plan_attention(&seed, &admission, plan_id()),
        Language::English,
    );

    assert!(
        admission.admitted.len() <= 8,
        "the quota let too many through"
    );
    assert!(
        prose.contains(&format!("of {}", admission.considered)),
        "the size of what was turned away is not in the prose: {prose}"
    );
    assert!(prose.contains("not the whole of it"), "{prose}");
}

#[test]
fn a_clock_that_cuts_the_walk_before_its_first_step_does_not_produce_an_empty_graph() {
    // The flake, pinned. A walk given no time at all reaches nothing, and "nothing came back" must
    // not be rendered as "nothing is associated with lemon" — the first is about the search, the
    // second is a claim about the world.
    let prose = walk(
        &kitchen(None),
        &WorkspaceCore::new(32),
        "what is lemon",
        &ActivationBudget {
            time: std::time::Duration::ZERO,
            ..ActivationBudget::default()
        },
    );
    assert!(
        !prose.contains("Nothing is associated"),
        "an unfinished search reported an empty graph: {prose}"
    );
    assert!(prose.contains("did not finish"), "{prose}");
    assert!(prose.contains("not the whole of it"), "{prose}");
}

#[test]
fn why_a_concept_came_to_mind_is_still_readable_at_the_far_end() {
    // A12 walked. The path from the graph — which link, which origin, which strength — is in the
    // sentence a person reads, so "why did you think of honey?" is answered by looking rather than
    // by asking something to compose a story.
    let prose = walk(
        &kitchen(None),
        &WorkspaceCore::new(32),
        "what is lemon",
        &ActivationBudget::default(),
    );
    assert!(prose.contains("lemon → honey"), "{prose}");
    assert!(prose.contains("Episodic"), "{prose}");
}

#[test]
fn an_erasure_reaches_the_sentence_as_nothing_associated() {
    // The full path after ADR-0028 erases. What must not happen is prose that still names an erased
    // concept; what must happen is an answer that says there is nothing, not one that says nothing.
    let context = kitchen(None);
    assert!(context.invalidate_for_epoch(1));
    let prose = walk(
        &context,
        &WorkspaceCore::new(32),
        "what is lemon",
        &ActivationBudget::default(),
    );
    assert!(
        !prose.contains("honey"),
        "an erased concept was named: {prose}"
    );
    assert!(
        prose.contains("Nothing is associated with lemon"),
        "{prose}"
    );
}

#[test]
fn the_moment_the_workspace_was_attending_to_survives_the_whole_walk() {
    // The seam ADR-0014's amendment protects, exercised through the real path rather than through
    // `admit` directly: a question asked of Mind may not cost Mind what it was already doing.
    let workspace = WorkspaceCore::new(32);
    let before = workspace.moment_state(OffsetDateTime::now_utc());
    let _ = walk(
        &kitchen(None),
        &workspace,
        "what is lemon",
        &ActivationBudget::default(),
    );
    let after = workspace.moment_state(OffsetDateTime::now_utc());
    assert_eq!(
        before, after,
        "asking a question changed what Mind was attending to"
    );
}

#[test]
fn the_same_question_asked_twice_reads_the_same_way() {
    // A1 at the far end. Determinism inside the walk is only worth having if it survives to the
    // thing a person compares — the sentence.
    let context = kitchen(Some("honey"));
    let workspace = WorkspaceCore::new(32);
    let first = walk(
        &context,
        &workspace,
        "what is lemon",
        &ActivationBudget::default(),
    );
    for _ in 0..8 {
        assert_eq!(
            walk(
                &context,
                &workspace,
                "what is lemon",
                &ActivationBudget::default()
            ),
            first
        );
    }
}
