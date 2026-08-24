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
//! ## Nothing here reaches outside the graph
//!
//! A seed is turned into a label and looked up. It does not open a file, resolve a person, or ask
//! any organ what an identifier means. A seed naming something the graph has never held is reported
//! as unknown, which is the honest answer and not an error: *nothing is associated with this yet* is
//! a fact about the graph, and an activation that quietly returned an empty list would be saying
//! something much stronger.

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
    /// By name from the frozen vocabulary — `storage.exhaustion`, `certificate.expiring` — rather
    /// than by insight identity. The identity belongs to one occurrence; what a person means by
    /// *bring to mind what relates to this* is the kind of thing, across occurrences.
    Finding(String),
    /// Something the host measures about itself.
    ///
    /// Named by subject and, where there is one, which one — so two certificates are two seeds.
    Metric {
        /// What is measured.
        subject: String,
        /// Which one, for a subject about a named thing.
        instance: Option<String>,
    },
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
            Self::Metric { .. } => "metric",
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
            Self::Finding(name) => format!("finding:{name}"),
            Self::Metric { subject, instance } => match instance {
                Some(instance) => format!("metric:{subject}({instance})"),
                None => format!("metric:{subject}"),
            },
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
            Self::Finding(name) => format!("a finding about this host ({name})"),
            Self::Metric {
                subject,
                instance: Some(instance),
            } => format!("something measured here ({subject}, {instance})"),
            Self::Metric {
                subject,
                instance: None,
            } => format!("something measured here ({subject})"),
            Self::Episode(id) => format!("an episode ({id})"),
        }
    }
}

#[cfg(test)]
mod tests {
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
        let first = Seed::Metric {
            subject: "certificate.days.remaining".to_owned(),
            instance: Some("/etc/ssl/a.pem".to_owned()),
        };
        let second = Seed::Metric {
            subject: "certificate.days.remaining".to_owned(),
            instance: Some("/etc/ssl/b.pem".to_owned()),
        };
        assert_ne!(first.label(), second.label());

        // And neither is the subject itself, which is about the kind rather than either file.
        let kind = Seed::Metric {
            subject: "certificate.days.remaining".to_owned(),
            instance: None,
        };
        assert_ne!(first.label(), kind.label());
    }

    #[test]
    fn a_seed_reads_as_something_a_person_can_check() {
        // A path beginning `intention:0f3a…` is a path nobody can check against anything they know,
        // and A12 is answered by a path a person can follow.
        let described = Seed::Finding("storage.exhaustion".to_owned()).describe();
        assert!(described.contains("storage.exhaustion"), "{described}");
        assert!(described.contains("finding"), "{described}");
    }

    #[test]
    fn a_seed_survives_the_wire() {
        let seeds = vec![
            Seed::concept("lemon"),
            Seed::Intention(Uuid::from_u128(7)),
            Seed::Metric {
                subject: "memory.pressure".to_owned(),
                instance: None,
            },
        ];
        let mut encoded = Vec::new();
        ciborium::into_writer(&seeds, &mut encoded).expect("encodes");
        let decoded: Vec<Seed> = ciborium::from_reader(encoded.as_slice()).expect("decodes");
        assert_eq!(decoded, seeds);
    }
}
