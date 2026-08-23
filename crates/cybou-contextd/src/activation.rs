// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Spreading activation from seeds, under a budget that is enforced rather than advised.
//!
//! Until now `contextd` could say what was active and what was associated with what, but not what
//! one word brings to mind. A filter over every node by salience is not that: it answers "what is
//! loud right now", which is the same answer whatever you asked. Activation starts from seeds and
//! walks the associations, so the answer depends on the question.
//!
//! Three properties from ADR-0029 are what make this worth having, and each one is a refusal:
//!
//! - **Bounded** (A2). Every dimension — nodes, edges, depth, time, tokens — stops the walk. A
//!   budget that only some dimensions honour is a budget nobody can rely on, and an associative
//!   graph is exactly the structure where one unbounded dimension reaches everything.
//! - **Inspectable** (A12). Every retrieved concept carries the path it was reached by and the
//!   edges along it. "Why did you think of honey?" is answered from the graph — `lemon → honey`,
//!   used-with, 0.84 — and never by asking something to compose a plausible story. A generated
//!   explanation of a retrieval is not evidence about that retrieval.
//! - **Partial says so** (A6). A walk cut short by a budget reports what cut it, and is not
//!   complete. A short list presented as the whole list is the substrate's oldest failure.
//!
//! ## Determinism and the clock
//!
//! A1 asks that the same snapshot, seeds and instant produce the same bundle. A2 asks that a wall
//! clock be able to stop the walk. Taken naively those contradict: a slower machine would return a
//! different bundle.
//!
//! They are reconciled by where the clock is allowed to act. Expansion order is fully determined by
//! the graph — highest relevance first, ties broken by label — so there is one canonical sequence
//! for a given snapshot and seeds. The time budget may only **truncate a prefix** of that sequence;
//! it can never reorder it or admit something a longer run would not have reached. So a bundle cut
//! short by time is always a prefix of the bundle a patient machine would produce, and it says that
//! it was cut. That is the strongest honest form of both gates at once, and it is stated here
//! because a reader finding a clock inside a "deterministic" walk deserves to know why it is safe.

use std::{
    collections::{HashMap, HashSet},
    hash::BuildHasher,
    time::Duration,
};

use cybou_protocol::attention::AttentionProposal;
use cybou_protocol::epistemic::EpistemicStatus;
use serde::{Deserialize, Serialize};

use crate::types::{Association, ConceptNode};

/// Which dimension of a budget stopped the walk.
///
/// A closed set, and reported rather than summarised, because "the budget ran out" leaves an
/// operator no way to tell a graph that is too wide from one that is too deep from a machine that
/// is too slow. Those need different responses.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Exhausted {
    /// The concept budget was reached.
    Nodes,
    /// The association budget was reached.
    Edges,
    /// The walk reached its depth limit with more to follow.
    Depth,
    /// The clock ran out.
    Time,
    /// The token estimate reached its ceiling.
    Tokens,
    /// A seed named a concept the graph does not hold.
    ///
    /// Not a budget, and listed here anyway: it is the other way an activation is less than what
    /// was asked for, and a caller that treats "found nothing from this seed" as "there is nothing
    /// associated with it" has been told something false.
    UnknownSeed,
}

/// How far an activation is allowed to go.
///
/// Every field is a stop, not a hint. The defaults are the ones ADR-0029 writes down.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActivationBudget {
    /// Most concepts the session may return.
    pub nodes: usize,
    /// Most associations the walk may follow.
    pub edges: usize,
    /// How many links from a seed the walk may travel.
    pub depth: u32,
    /// How long the walk may take.
    pub time: Duration,
    /// The token ceiling of everything returned.
    pub tokens: usize,
}

impl Default for ActivationBudget {
    fn default() -> Self {
        Self {
            nodes: 32,
            edges: 64,
            depth: 3,
            time: Duration::from_millis(30),
            tokens: 1800,
        }
    }
}

/// One concept the walk reached, and how.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivatedConcept {
    /// The concept.
    pub label: String,
    /// How strongly the walk reached it, in [0.0, 1.0].
    ///
    /// The product of the association strengths along the path taken. Chosen because it is the one
    /// quantity the path itself can account for: a person reading the path can arrive at the same
    /// number. A score assembled from freshness, recency and personal relevance may well rank
    /// better, and would not be explainable by looking at what it came from.
    pub relevance: f64,
    /// How many links from a seed.
    pub depth: u32,
    /// The concepts walked through to reach it, seed first.
    pub path: Vec<String>,
    /// Why it was retrieved, in a form derived entirely from the graph.
    pub reason: String,
    /// What returning it is estimated to cost.
    pub estimated_tokens: usize,
    /// How the epistemic owner stood on what this concept was derived from.
    ///
    /// ADR-0029 A4. Its own standing, and never the path's: a concept reached *through* a disputed
    /// one is not thereby disputed. Propagating along the walk would be association conferring
    /// epistemic force, which is the thing A5 forbids — `lemon` being contested says nothing about
    /// whether honey exists.
    #[serde(default)]
    pub epistemic_status: EpistemicStatus,
}

impl ActivationSession {
    /// What this walk offers attention, and nothing more.
    ///
    /// ADR-0014's amendment permits exactly this and no more: `contextd` may produce proposals;
    /// activation itself never enters the Workspace. The conversion is a named function rather
    /// than a shared struct so that the moment association becomes a proposal is a line somebody
    /// can point at — and so that `workspaced` still decides what becomes of them.
    #[must_use]
    pub fn proposals(&self) -> Vec<AttentionProposal> {
        self.items
            .iter()
            .map(|item| AttentionProposal {
                label: item.label.clone(),
                relevance: item.relevance,
                reason: item.reason.clone(),
                epistemic_status: item.epistemic_status,
            })
            .collect()
    }

    /// Whether a budget stopped this walk before it ran out of graph.
    ///
    /// Narrower than `!complete`, and the difference matters to whoever turns this into a sentence.
    /// A seed the graph does not hold makes a session incomplete — the answer is less than what was
    /// asked for — but the walk that did happen finished, and "nothing is associated with bergamot"
    /// is a true thing to say. A budget cutting the walk short is the other case entirely: nothing
    /// came back and the graph was never asked, so the same sentence would be a claim about the
    /// world made from a search that did not run.
    #[must_use]
    pub fn was_cut_short(&self) -> bool {
        self.exhausted
            .iter()
            .any(|reason| *reason != Exhausted::UnknownSeed)
    }

    /// Whether anything reached carries a standing a reader has to be told about.
    ///
    /// Offered so a consumer cannot present a bundle as settled without having looked. It is a
    /// summary of the items and never a substitute for them: which concept is disputed is the part
    /// a person needs, and that stays on the concept.
    #[must_use]
    pub fn carries_qualified(&self) -> bool {
        self.items
            .iter()
            .any(|item| item.epistemic_status.qualifies())
    }
}

/// One bounded walk, and everything needed to argue with it.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivationSession {
    /// The seeds the walk started from.
    pub seeds: Vec<String>,
    /// What was reached, most relevant first.
    pub items: Vec<ActivatedConcept>,
    /// How many associations were followed.
    pub edges_followed: usize,
    /// The estimated cost of everything returned.
    pub estimated_tokens: usize,
    /// Whether the walk finished everything it had left to do.
    pub complete: bool,
    /// What stopped it, when something did. Sorted, so two runs compare.
    pub exhausted: Vec<Exhausted>,
}

/// What one concept is estimated to cost to carry.
///
/// Deliberately crude and deliberately fixed: four characters to a token. An estimate that varied
/// with a tokenizer would make the token budget a property of whichever model happened to be
/// installed, and ADR-0029 A10 says switching models does not change associative memory.
fn estimated_tokens(label: &str, reason: &str) -> usize {
    (label.chars().count() + reason.chars().count()).div_ceil(4)
}

/// Walk the associations from `seeds`, stopping at the first budget that binds.
///
/// `elapsed` is asked how long the walk has taken. It is a parameter rather than a call to a clock
/// so that the time budget can be tested at all: a bound nothing exercises is a bound that will be
/// wrong when it first matters.
///
/// The result is ordered by relevance, ties by label, and is a prefix of what a larger budget would
/// have returned.
pub fn activate_from<S: BuildHasher>(
    nodes: &HashMap<String, ConceptNode, S>,
    associations: &[Association],
    seeds: &[String],
    budget: &ActivationBudget,
    mut elapsed: impl FnMut() -> Duration,
) -> ActivationSession {
    let mut exhausted: HashSet<Exhausted> = HashSet::new();
    let mut items: Vec<ActivatedConcept> = Vec::new();
    let mut settled: HashSet<String> = HashSet::new();
    let mut edges_followed = 0usize;
    let mut total_tokens = 0usize;

    // The frontier, held sorted so that what comes off it is the canonical next step rather than
    // whatever a hash happened to yield.
    let mut frontier = seed_frontier(nodes, seeds, &mut exhausted);

    loop {
        sort_by_relevance(&mut frontier);
        let Some(current) = pop_first(&mut frontier) else {
            break;
        };
        if !settled.insert(current.label.clone()) {
            continue;
        }

        // Checked before admitting, so a session never returns more than it says it may.
        if items.len() >= budget.nodes {
            exhausted.insert(Exhausted::Nodes);
            break;
        }
        if total_tokens + current.estimated_tokens > budget.tokens {
            exhausted.insert(Exhausted::Tokens);
            break;
        }
        if elapsed() >= budget.time {
            exhausted.insert(Exhausted::Time);
            break;
        }

        total_tokens += current.estimated_tokens;
        let depth = current.depth;
        let relevance = current.relevance;
        let label = current.label.clone();
        let path = current.path.clone();
        items.push(current);

        if depth >= budget.depth {
            // Only an exhaustion if there was somewhere further to go. A walk that reached the
            // edge of the graph exactly at its depth limit finished; saying otherwise would report
            // a complete answer as a truncated one, which is the same lie in the other direction.
            // Somewhere further means somewhere not already reached — a link back to a concept the
            // walk already holds is not a concept the depth limit kept from anyone.
            if neighbours(associations, &label)
                .any(|(neighbour, _)| !settled.contains(neighbour) && nodes.contains_key(neighbour))
            {
                exhausted.insert(Exhausted::Depth);
            }
            continue;
        }

        for (neighbour, link) in neighbours(associations, &label) {
            if settled.contains(neighbour) || !nodes.contains_key(neighbour) {
                continue;
            }
            if edges_followed >= budget.edges {
                exhausted.insert(Exhausted::Edges);
                break;
            }
            edges_followed += 1;

            let reason = format!(
                "{} → {} ({:?}, strength {:.2}) at depth {}",
                label,
                neighbour,
                link.origin,
                link.strength,
                depth + 1
            );
            let mut next_path = path.clone();
            next_path.push(neighbour.clone());
            frontier.push(ActivatedConcept {
                estimated_tokens: estimated_tokens(neighbour, &reason),
                // The neighbour's own standing. Not the one it was reached from: see the field.
                epistemic_status: standing_of(nodes, neighbour),
                label: neighbour.clone(),
                relevance: relevance * link.strength,
                depth: depth + 1,
                path: next_path,
                reason,
            });
        }
        if exhausted.contains(&Exhausted::Edges) {
            break;
        }
    }

    let mut exhausted: Vec<Exhausted> = exhausted.into_iter().collect();
    exhausted.sort_unstable();

    ActivationSession {
        seeds: seeds.to_vec(),
        items,
        edges_followed,
        estimated_tokens: total_tokens,
        // Complete means the walk ran out of graph, not out of budget. An unreached seed counts:
        // the answer is less than what was asked for either way.
        complete: exhausted.is_empty() && frontier.is_empty(),
        exhausted,
    }
}

/// How the epistemic owner stood on one concept, or `Unknown` if the graph does not hold it.
fn standing_of<S: BuildHasher>(
    nodes: &HashMap<String, ConceptNode, S>,
    label: &str,
) -> EpistemicStatus {
    nodes
        .get(label)
        .map_or(EpistemicStatus::Unknown, |node| node.epistemic_status)
}

/// The starting frontier: one entry per distinct seed the graph actually holds.
///
/// A seed the graph does not hold is recorded as an exhaustion rather than skipped. A caller that
/// reads "nothing came back" as "nothing is associated with it" has been told something false.
fn seed_frontier<S: BuildHasher>(
    nodes: &HashMap<String, ConceptNode, S>,
    seeds: &[String],
    exhausted: &mut HashSet<Exhausted>,
) -> Vec<ActivatedConcept> {
    let mut frontier = Vec::new();
    let mut seen: HashSet<&String> = HashSet::new();
    for seed in seeds {
        if !seen.insert(seed) {
            continue;
        }
        if !nodes.contains_key(seed) {
            exhausted.insert(Exhausted::UnknownSeed);
            continue;
        }
        let reason = "named as a seed".to_owned();
        frontier.push(ActivatedConcept {
            estimated_tokens: estimated_tokens(seed, &reason),
            epistemic_status: standing_of(nodes, seed),
            label: seed.clone(),
            relevance: 1.0,
            depth: 0,
            path: vec![seed.clone()],
            reason,
        });
    }
    frontier
}

/// Order a frontier canonically: strongest first, ties by shallower, then by label.
///
/// The tiebreak is what makes A1 hold. Two concepts reached equally strongly must come off the
/// frontier in the same order every run, or a bundle could not be compared with the one a person
/// was shown.
fn sort_by_relevance(frontier: &mut [ActivatedConcept]) {
    frontier.sort_by(|left, right| {
        right
            .relevance
            .partial_cmp(&left.relevance)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(left.depth.cmp(&right.depth))
            .then_with(|| left.label.cmp(&right.label))
    });
}

/// Take the front of a sorted frontier.
fn pop_first(frontier: &mut Vec<ActivatedConcept>) -> Option<ActivatedConcept> {
    if frontier.is_empty() {
        None
    } else {
        Some(frontier.remove(0))
    }
}

/// Everything one concept links to, in either direction, in a fixed order.
///
/// Associations are undirected for the purpose of a walk — `lemon → honey` means honey is brought
/// to mind by lemon and lemon by honey — but the stored direction is preserved in the reason, so a
/// person reading a path sees the link as it was recorded.
fn neighbours<'graph>(
    associations: &'graph [Association],
    label: &'graph str,
) -> impl Iterator<Item = (&'graph String, &'graph Association)> {
    associations.iter().filter_map(move |link| {
        if link.source == label {
            Some((&link.target, link))
        } else if link.target == label {
            Some((&link.source, link))
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use time::OffsetDateTime;
    use uuid::Uuid;

    use super::*;
    use crate::types::AssociationOrigin;

    fn at() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_787_000_000).expect("a fixed instant")
    }

    fn node(label: &str) -> (String, ConceptNode) {
        (
            label.to_owned(),
            ConceptNode {
                label: label.to_owned(),
                salience: 0.5,
                activation_reason: "observed".to_owned(),
                last_activated_at: at(),
                sensitivity: 0,
                epistemic_status: EpistemicStatus::Observed,
            },
        )
    }

    /// The same graph, with one concept the epistemic owner is contesting.
    fn kitchen_with_a_dispute() -> (HashMap<String, ConceptNode>, Vec<Association>) {
        let (mut nodes, links) = kitchen();
        nodes
            .get_mut("honey")
            .expect("honey is in the kitchen")
            .epistemic_status = EpistemicStatus::Disputed;
        (nodes, links)
    }

    fn link(source: &str, target: &str, strength: f64) -> Association {
        Association {
            source: source.to_owned(),
            target: target.to_owned(),
            strength,
            origin: AssociationOrigin::Episodic,
            evidence: vec![Uuid::from_u128(1)],
            privacy: 0,
            sensitivity: 0,
            retention_class: 0,
        }
    }

    /// lemon — honey — tea, and a yellow off to the side.
    fn kitchen() -> (HashMap<String, ConceptNode>, Vec<Association>) {
        let nodes = ["lemon", "honey", "tea", "yellow", "unrelated"]
            .into_iter()
            .map(node)
            .collect();
        let links = vec![
            link("lemon", "honey", 0.8),
            link("honey", "tea", 0.5),
            link("lemon", "yellow", 0.9),
        ];
        (nodes, links)
    }

    fn patient() -> impl FnMut() -> Duration {
        || Duration::ZERO
    }

    fn seeds(labels: &[&str]) -> Vec<String> {
        labels.iter().map(|label| (*label).to_owned()).collect()
    }

    #[test]
    fn a_disputed_state_is_still_disputed_after_retrieval() {
        // A4. A retrieval that handed back the value and dropped the standing would lose the
        // dispute at exactly the boundary where losing it looks like there having been nothing.
        let (nodes, links) = kitchen_with_a_dispute();
        let session = activate_from(
            &nodes,
            &links,
            &seeds(&["lemon"]),
            &ActivationBudget::default(),
            patient(),
        );
        let honey = session
            .items
            .iter()
            .find(|item| item.label == "honey")
            .expect("honey was reached");
        assert_eq!(honey.epistemic_status, EpistemicStatus::Disputed);
        assert!(session.carries_qualified());
    }

    #[test]
    fn a_concept_reached_through_a_disputed_one_is_not_thereby_disputed() {
        // The other half, and the one an instinct gets wrong. In `compose` a hedge on any part
        // qualifies the whole, because the parts were claims about one answer. Here the walk is
        // association, not inference: lemon being contested says nothing about whether honey
        // exists, and propagating would be association conferring epistemic force.
        let (nodes, links) = kitchen_with_a_dispute();
        let session = activate_from(
            &nodes,
            &links,
            &seeds(&["lemon"]),
            &ActivationBudget::default(),
            patient(),
        );
        let tea = session
            .items
            .iter()
            .find(|item| item.label == "tea")
            .expect("tea was reached through honey");
        assert_eq!(tea.path, vec!["lemon", "honey", "tea"]);
        assert_eq!(
            tea.epistemic_status,
            EpistemicStatus::Observed,
            "the dispute travelled along the path"
        );
    }

    #[test]
    fn a_bundle_with_nothing_contested_does_not_claim_to_be_qualified() {
        let (nodes, links) = kitchen();
        let session = activate_from(
            &nodes,
            &links,
            &seeds(&["lemon"]),
            &ActivationBudget::default(),
            patient(),
        );
        assert!(!session.carries_qualified());
    }

    #[test]
    fn a_disputed_seed_carries_its_own_standing() {
        let (nodes, links) = kitchen_with_a_dispute();
        let session = activate_from(
            &nodes,
            &links,
            &seeds(&["honey"]),
            &ActivationBudget::default(),
            patient(),
        );
        assert_eq!(session.items[0].label, "honey");
        assert_eq!(session.items[0].epistemic_status, EpistemicStatus::Disputed);
    }

    #[test]
    fn every_retrieved_concept_can_say_why_it_was_retrieved() {
        // A12, the gate ADR-0029 says to defend hardest. The answer comes from the graph, and
        // names the link it came along.
        let (nodes, links) = kitchen();
        let session = activate_from(
            &nodes,
            &links,
            &seeds(&["lemon"]),
            &ActivationBudget::default(),
            patient(),
        );

        let honey = session
            .items
            .iter()
            .find(|item| item.label == "honey")
            .expect("honey is one link from lemon");
        assert!(honey.reason.contains("lemon → honey"), "{}", honey.reason);
        assert!(honey.reason.contains("0.80"), "{}", honey.reason);
        assert_eq!(honey.path, vec!["lemon", "honey"]);
        for item in &session.items {
            assert!(!item.reason.is_empty());
            assert_eq!(item.path.first().map(String::as_str), Some("lemon"));
        }
    }

    #[test]
    fn the_same_graph_and_seeds_produce_the_same_session() {
        // A1. Nothing here may depend on hash order.
        let (nodes, links) = kitchen();
        let budget = ActivationBudget::default();
        let first = activate_from(&nodes, &links, &seeds(&["lemon"]), &budget, patient());
        for _ in 0..8 {
            let again = activate_from(&nodes, &links, &seeds(&["lemon"]), &budget, patient());
            assert_eq!(first, again);
        }
    }

    #[test]
    fn a_walk_stopped_by_the_clock_is_a_prefix_of_one_that_was_not() {
        // How A1 and A2 are reconciled: the clock truncates a canonical sequence and never
        // reorders it.
        let (nodes, links) = kitchen();
        let budget = ActivationBudget::default();
        let whole = activate_from(&nodes, &links, &seeds(&["lemon"]), &budget, patient());

        let mut ticks = 0u32;
        let hurried = activate_from(&nodes, &links, &seeds(&["lemon"]), &budget, move || {
            ticks += 1;
            if ticks > 2 {
                Duration::from_secs(1)
            } else {
                Duration::ZERO
            }
        });

        assert!(hurried.items.len() < whole.items.len());
        assert!(hurried.exhausted.contains(&Exhausted::Time));
        assert!(!hurried.complete);
        assert_eq!(hurried.items[..], whole.items[..hurried.items.len()]);
    }

    #[test]
    fn each_dimension_of_the_budget_stops_the_walk_by_itself() {
        // A2. A budget only some dimensions honour is a budget nobody can rely on.
        let (nodes, links) = kitchen();
        let base = ActivationBudget::default();

        let by_nodes = activate_from(
            &nodes,
            &links,
            &seeds(&["lemon"]),
            &ActivationBudget { nodes: 2, ..base },
            patient(),
        );
        assert_eq!(by_nodes.items.len(), 2);
        assert!(by_nodes.exhausted.contains(&Exhausted::Nodes));

        let by_depth = activate_from(
            &nodes,
            &links,
            &seeds(&["lemon"]),
            &ActivationBudget { depth: 1, ..base },
            patient(),
        );
        assert!(by_depth.exhausted.contains(&Exhausted::Depth));
        assert!(
            !by_depth.items.iter().any(|item| item.label == "tea"),
            "tea is two links away and the walk was allowed one"
        );

        let by_tokens = activate_from(
            &nodes,
            &links,
            &seeds(&["lemon"]),
            &ActivationBudget { tokens: 4, ..base },
            patient(),
        );
        assert!(by_tokens.exhausted.contains(&Exhausted::Tokens));
        assert!(by_tokens.estimated_tokens <= 4);

        let by_edges = activate_from(
            &nodes,
            &links,
            &seeds(&["lemon"]),
            &ActivationBudget { edges: 1, ..base },
            patient(),
        );
        assert!(by_edges.exhausted.contains(&Exhausted::Edges));
        assert!(by_edges.edges_followed <= 1);
    }

    #[test]
    fn a_walk_cut_short_never_reports_itself_complete() {
        // A6. A short list presented as the whole list is the oldest failure here.
        let (nodes, links) = kitchen();
        let cut = activate_from(
            &nodes,
            &links,
            &seeds(&["lemon"]),
            &ActivationBudget {
                nodes: 1,
                ..ActivationBudget::default()
            },
            patient(),
        );
        assert!(!cut.complete);
        assert!(!cut.exhausted.is_empty());
    }

    #[test]
    fn a_walk_that_ran_out_of_graph_rather_than_budget_is_complete() {
        let (nodes, links) = kitchen();
        let whole = activate_from(
            &nodes,
            &links,
            &seeds(&["lemon"]),
            &ActivationBudget::default(),
            patient(),
        );
        assert!(whole.complete, "{:?}", whole.exhausted);
        assert!(whole.exhausted.is_empty());
        // "unrelated" is in the graph and reachable from nothing, so it is not brought to mind.
        assert!(!whole.items.iter().any(|item| item.label == "unrelated"));
    }

    #[test]
    fn a_seed_that_is_not_there_is_not_a_walk_that_was_cut_short() {
        // Two ways to come back with nothing, and only one of them means the graph was never asked.
        let (nodes, links) = kitchen();
        let absent = activate_from(
            &nodes,
            &links,
            &seeds(&["bergamot"]),
            &ActivationBudget::default(),
            patient(),
        );
        assert!(
            !absent.complete,
            "the answer is less than what was asked for"
        );
        assert!(
            !absent.was_cut_short(),
            "but the walk that happened finished"
        );

        let hurried = activate_from(
            &nodes,
            &links,
            &seeds(&["lemon"]),
            &ActivationBudget {
                time: Duration::ZERO,
                ..ActivationBudget::default()
            },
            patient(),
        );
        assert!(hurried.was_cut_short());
    }

    #[test]
    fn a_seed_the_graph_does_not_hold_is_said_rather_than_swallowed() {
        // "Nothing came back" and "there is nothing associated with it" are different answers.
        let (nodes, links) = kitchen();
        let session = activate_from(
            &nodes,
            &links,
            &seeds(&["bergamot"]),
            &ActivationBudget::default(),
            patient(),
        );
        assert!(session.items.is_empty());
        assert!(session.exhausted.contains(&Exhausted::UnknownSeed));
        assert!(!session.complete);
    }

    #[test]
    fn relevance_is_the_product_of_the_links_walked_and_the_path_accounts_for_it() {
        // The number a person can arrive at themselves from the path they were shown.
        let (nodes, links) = kitchen();
        let session = activate_from(
            &nodes,
            &links,
            &seeds(&["lemon"]),
            &ActivationBudget::default(),
            patient(),
        );
        let tea = session
            .items
            .iter()
            .find(|item| item.label == "tea")
            .expect("tea is two links from lemon");
        assert!((tea.relevance - 0.8 * 0.5).abs() < f64::EPSILON);
        assert_eq!(tea.path, vec!["lemon", "honey", "tea"]);
        assert_eq!(tea.depth, 2);
    }

    #[test]
    fn what_is_reached_more_strongly_comes_first() {
        let (nodes, links) = kitchen();
        let session = activate_from(
            &nodes,
            &links,
            &seeds(&["lemon"]),
            &ActivationBudget::default(),
            patient(),
        );
        let order: Vec<&str> = session
            .items
            .iter()
            .map(|item| item.label.as_str())
            .collect();
        assert_eq!(order, vec!["lemon", "yellow", "honey", "tea"]);
    }

    #[test]
    fn a_concept_reached_two_ways_is_one_concept_reached_the_stronger_way() {
        let nodes: HashMap<String, ConceptNode> = ["a", "b", "c"].into_iter().map(node).collect();
        let links = vec![
            link("a", "b", 0.2),
            link("a", "c", 0.9),
            link("c", "b", 0.9),
        ];

        let session = activate_from(
            &nodes,
            &links,
            &seeds(&["a"]),
            &ActivationBudget::default(),
            patient(),
        );
        let b: Vec<_> = session
            .items
            .iter()
            .filter(|item| item.label == "b")
            .collect();
        assert_eq!(b.len(), 1, "one concept, listed once");
        assert_eq!(b[0].path, vec!["a", "c", "b"], "the stronger route won");
    }

    #[test]
    fn a_cycle_does_not_make_the_walk_run_forever() {
        let nodes: HashMap<String, ConceptNode> = ["a", "b", "c"].into_iter().map(node).collect();
        let links = vec![
            link("a", "b", 0.9),
            link("b", "c", 0.9),
            link("c", "a", 0.9),
        ];
        let session = activate_from(
            &nodes,
            &links,
            &seeds(&["a"]),
            &ActivationBudget::default(),
            patient(),
        );
        assert_eq!(session.items.len(), 3);
    }

    #[test]
    fn two_seeds_both_reach_and_neither_is_counted_twice() {
        let (nodes, links) = kitchen();
        let session = activate_from(
            &nodes,
            &links,
            &seeds(&["lemon", "tea", "lemon"]),
            &ActivationBudget::default(),
            patient(),
        );
        assert_eq!(
            session
                .items
                .iter()
                .filter(|item| item.label == "lemon")
                .count(),
            1
        );
        for expected in ["lemon", "tea", "honey", "yellow"] {
            assert!(
                session.items.iter().any(|item| item.label == expected),
                "{expected} was not reached"
            );
        }
    }

    #[test]
    fn an_activation_from_no_seeds_brings_nothing_to_mind() {
        // Not "everything that is loud right now". Nothing was asked, so nothing is answered.
        let (nodes, links) = kitchen();
        let session = activate_from(&nodes, &links, &[], &ActivationBudget::default(), patient());
        assert!(session.items.is_empty());
        assert!(session.complete, "there was nothing left undone");
    }

    #[test]
    fn a_walk_that_ends_exactly_at_its_depth_limit_is_not_called_truncated() {
        // Reporting a complete answer as a cut one is the same lie in the other direction.
        let nodes: HashMap<String, ConceptNode> = ["a", "b"].into_iter().map(node).collect();
        let links = vec![link("a", "b", 0.9)];
        let session = activate_from(
            &nodes,
            &links,
            &seeds(&["a"]),
            &ActivationBudget {
                depth: 1,
                ..ActivationBudget::default()
            },
            patient(),
        );
        assert_eq!(session.items.len(), 2);
        assert!(session.complete, "{:?}", session.exhausted);
    }
}
