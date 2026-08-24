// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! What an activation may start from, when it is not a word.
//!
//! ADR-0029 is explicit about this and states the cost of getting it wrong: *restricting seeds to
//! text would make the whole layer an accessory to a chat box, which is precisely the accident this
//! ADR is written to avoid.* A context layer that can only be asked in words is a prompt builder
//! with a graph inside it, and every question put to it has to be phrased before it can be asked —
//! which rules out the questions a machine asks itself.
//!
//! The concrete ones this build can already produce: what the Workspace is looking at, an intention
//! being held, a finding the host reached about itself, a metric it watches. None of those is a
//! word somebody typed, and all four are things a person would want brought to mind.
//!
//! ## Kind is part of identity
//!
//! The reason this is a type and not a string with a prefix convention. A file called `lemon` and
//! the concept `lemon` are not the same seed, and under plain strings they are the same key — so
//! activating from one returns whatever was associated with the other, with a path that reads
//! entirely plausibly and is about the wrong thing. Namespacing by kind is what stops two unrelated
//! things sharing a node because they share a name.
//!
//! ## The vocabulary is the other layer's, not a copy of it
//!
//! A finding is a [`Finding`] and a metric is a [`MetricKey`], not strings that happen to look like
//! them. The first version of this file took both as text, and the drift was already visible before
//! anything used it: the tests here wrote `storage.exhaustion` while `Finding::name` says
//! `storage-exhaustion`. Two layers spelling one thing differently is how a seed comes to find
//! nothing and be reported as an empty corner of the graph.
//!
//! This crate already depends on the protocol, so there was never a reason to restate the
//! vocabulary here beyond the shape of the first draft.
//!
//! ## Nothing here reaches outside the graph
//!
//! A seed is turned into a label and looked up. It does not open a file, resolve a person, or ask
//! any organ what an identifier means. A seed naming something the graph has never held is reported
//! as unknown, which is the honest answer and not an error: *nothing is associated with this yet* is
//! a fact about the graph, and an activation that quietly returned an empty list would be saying
//! something much stronger.

use cybou_protocol::telemetry::{Finding, MetricKey};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Something an activation can start from.
///
/// A closed set. Open-ended seeding would put the mapping from a thing to a label at each call
/// site, which is where two call sites come to disagree about what `intention:…` means.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "of")]
pub enum Seed {
    /// A concept named directly, which is what a person typing a word supplies.
    Concept(String),
    /// What the Workspace is currently looking at.
    Focus(String),
    /// An intention being held.
    Intention(Uuid),
    /// A finding the host reached about itself.
    ///
    /// The kind of finding, not one occurrence of it. The identity of an insight belongs to one
    /// episode; what a person means by *bring to mind what relates to this* is the kind of thing,
    /// across episodes.
    Finding(Finding),
    /// Something the host measures about itself.
    ///
    /// The whole key, so two certificates are two seeds — the same rule the telemetry path spent a
    /// rewrite establishing, arriving here rather than being decided again.
    Metric(MetricKey),
    /// An episode in the biography.
    Episode(Uuid),
}

impl Seed {
    /// A concept seed, which is what a word amounts to.
    #[must_use]
    pub fn concept(label: impl Into<String>) -> Self {
        Self::Concept(label.into())
    }

    /// Which kind of thing this is.
    ///
    /// Part of the label rather than decoration on it: see the note above about a file and a
    /// concept sharing a name.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Concept(_) => "concept",
            Self::Focus(_) => "focus",
            Self::Intention(_) => "intention",
            Self::Finding(_) => "finding",
            Self::Metric(_) => "metric",
            Self::Episode(_) => "episode",
        }
    }

    /// The graph label this seed looks for.
    ///
    /// A concept keeps its bare label, so a graph built by people naming things is unchanged and
    /// every existing association still resolves. Everything else is namespaced by kind, because
    /// everything else is an identifier that happens to be spellable rather than a word chosen to
    /// mean something.
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Self::Concept(label) => label.clone(),
            Self::Focus(item) => format!("focus:{item}"),
            Self::Intention(id) => format!("intention:{id}"),
            Self::Finding(finding) => format!("finding:{}", finding.name()),
            Self::Metric(key) => format!("metric:{}", key.label()),
            Self::Episode(id) => format!("episode:{id}"),
        }
    }

    /// How this seed reads to a person.
    ///
    /// Distinct from [`Self::label`], which is a key. A reader asking *why was this brought to
    /// mind* is answered with the path, and a path beginning `intention:0f3a…` is a path they
    /// cannot check against anything they know.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Concept(label) => format!("the concept {label}"),
            Self::Focus(item) => format!("what the workspace is looking at ({item})"),
            Self::Intention(id) => format!("an intention being held ({id})"),
            Self::Finding(finding) => format!("a finding about this host ({})", finding.name()),
            Self::Metric(key) => format!("something measured here ({})", key.label()),
            Self::Episode(id) => format!("an episode ({id})"),
        }
    }
}

#[cfg(test)]
mod tests {
    use cybou_protocol::telemetry::Subject;

    use super::*;

    #[test]
    fn a_file_named_lemon_is_not_the_concept_lemon() {
        // Under plain strings these are one key, so activating from one returns what was associated
        // with the other, along a path that reads entirely plausibly and is about the wrong thing.
        let concept = Seed::concept("lemon");
        let focus = Seed::Focus("lemon".to_owned());
        assert_ne!(concept.label(), focus.label());
        assert_ne!(concept, focus);
    }

    #[test]
    fn a_word_still_looks_for_itself() {
        // A graph built by people naming things must keep working. Namespacing concepts too would
        // mean every existing association resolves to nothing, which is a migration wearing the
        // look of an empty memory.
        assert_eq!(Seed::concept("kernel-version").label(), "kernel-version");
    }

    #[test]
    fn two_certificates_are_two_seeds() {
        // The same rule the telemetry path spent a rewrite establishing, arriving here rather than
        // being re-decided: a subject about a named thing is not one thing.
        let of = |name: &str| {
            Seed::Metric(MetricKey::named(
                Subject::CertificateDaysRemaining,
                name.to_owned(),
            ))
        };
        assert_ne!(of("/etc/ssl/a.pem").label(), of("/etc/ssl/b.pem").label());

        // And neither is the subject itself, which is about the kind rather than either file.
        let kind = Seed::Metric(MetricKey::host(Subject::CertificateDaysRemaining));
        assert_ne!(of("/etc/ssl/a.pem").label(), kind.label());
    }

    #[test]
    fn a_seed_spells_a_finding_the_way_the_finding_spells_itself() {
        // The drift this closes was visible before anything used it: this file's own tests wrote
        // `storage.exhaustion` while `Finding::name` says `storage-exhaustion`. Two layers spelling
        // one thing differently is how a seed comes to find nothing and be reported as an empty
        // corner of the graph.
        let seed = Seed::Finding(Finding::StorageExhaustion);
        assert_eq!(
            seed.label(),
            format!("finding:{}", Finding::StorageExhaustion.name())
        );
    }

    #[test]
    fn a_seed_reads_as_something_a_person_can_check() {
        // A path beginning `intention:0f3a…` is a path nobody can check against anything they know,
        // and A12 is answered by a path a person can follow.
        let described = Seed::Finding(Finding::StorageExhaustion).describe();
        assert!(described.contains("storage-exhaustion"), "{described}");
        assert!(described.contains("finding"), "{described}");
    }

    #[test]
    fn a_seed_survives_the_wire() {
        let seeds = vec![
            Seed::concept("lemon"),
            Seed::Intention(Uuid::from_u128(7)),
            Seed::Metric(MetricKey::host(Subject::MemoryPressure)),
        ];
        let mut encoded = Vec::new();
        ciborium::into_writer(&seeds, &mut encoded).expect("encodes");
        let decoded: Vec<Seed> = ciborium::from_reader(encoded.as_slice()).expect("decodes");
        assert_eq!(decoded, seeds);
    }
}
