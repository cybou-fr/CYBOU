// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! What Mind attends to, turned into suggestions a desktop may offer — and never into commands.
//!
//! Between an organ that knows what matters and a canvas that knows where things are, something has
//! to decide what is worth showing. Without that layer the two are only adjacent: `workspaced`
//! picks a coalition, the canvas draws a card that says which coalition was picked, and the person
//! reads a correlation identity. This crate is that layer, and its whole design is in one word in
//! the return type.
//!
//! [`SpatialSuggestion`], not command. ADR-0044 requires that presentation never become cognition
//! or authorisation, and a policy that returned commands would quietly cross both lines: it would
//! be deciding what the person is working on, and moving their workspace to match. Nothing here
//! moves a camera, opens a panel, or closes one. It says what could be offered; a person accepts.
//!
//! The crate reaches nothing — no D-Bus, no HTTP, no filesystem, no clock, no randomness — so
//! every suggestion is a function of the [`SpatialContext`] it was handed and can be reproduced
//! from it. Two calls on the same context return the same list in the same order.
//!
//! No language model, either, and not as a placeholder for one. What is relevant to an episode is
//! a question about identity and structure, and the deterministic answer is the one a person can
//! argue with.

use std::cmp::Ordering;

use cybou_protocol::attention::{AttendedSubject, SubjectReading};
use cybou_protocol::{SubjectKind, SubjectQuery};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// How many suggestions a desktop is offered at once.
///
/// A cap rather than a ranking preference. The moment is already bounded upstream, so this is not
/// flood protection: it is the difference between a desktop offering a person something and a
/// desktop presenting them with a second workspace to triage.
pub const SUGGESTION_LIMIT: usize = 5;

/// How many closed surfaces it takes before opening them is one suggestion instead of several.
///
/// Below this a person reads three separate offers and can take the one they want. At and above it
/// the offers stop being individually meaningful — what the episode is about is the set — and a
/// list of six things to open is a chore rather than an offer.
pub const GATHER_THRESHOLD: usize = 3;

/// Why a suggestion was made, carried from the contribution that caused it.
///
/// Every suggestion has one, and it is not decoration. A suggestion whose grounds a person cannot
/// inspect is indistinguishable from a guess, and this layer's only claim on their attention is
/// that it is not guessing: the organ that observed it, how sure that organ was, and the evidence
/// it cited, so the person can go and look.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rationale {
    /// The episode this belongs to, when the context named one.
    pub correlation: Option<Uuid>,
    /// The contribution the subject was read from.
    pub contribution: Uuid,
    /// The organ that wrote it.
    pub organ: String,
    /// The confidence that contribution carried.
    pub confidence: f64,
    /// The evidence it cited, in the order it cited it.
    pub evidence: Vec<Uuid>,
}

/// What a person is already doing with a subject.
///
/// Every variant suppresses suggestions about that subject, and the distinctions are kept because
/// they do not have the same lifetime: a selection lasts as long as the selection, and a dismissal
/// is a person saying no to an offer, which a desktop has no business making again a second later.
/// Folding them into one flag would make it impossible to honour the second correctly.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EngagementKind {
    /// The person selected the subject.
    Selected,
    /// The person is working through the subject.
    Investigating,
    /// The person turned down a suggestion about the subject.
    Dismissed,
}

/// One thing the person is already engaged with.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Engagement {
    /// What they are engaged with.
    pub subject: SubjectQuery,
    /// How.
    pub kind: EngagementKind,
}

/// One surface of the desktop, as the desktop describes it.
///
/// The policy is told what exists rather than knowing it. A crate that held its own table of which
/// panel shows which kind of thing would be a second copy of the desktop's own vocabulary, drifting
/// from it silently, and would have to be edited every time a panel was added.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceFacts {
    /// Stable identity of the surface in the desktop's own spelling.
    pub key: String,
    /// The kinds of subject this surface can present.
    pub shows: Vec<SubjectKind>,
    /// Whether it is currently on the canvas at all.
    pub open: bool,
    /// Whether the person can currently see it.
    pub in_view: bool,
    /// The subject it currently stands for, when it stands for one.
    pub subject: Option<SubjectQuery>,
}

/// Everything the policy is allowed to know.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpatialContext {
    /// The episode currently holding attention, when one does.
    pub correlation: Option<Uuid>,
    /// What the contributions holding attention are about, newest first.
    pub attention: Vec<AttendedSubject>,
    /// The surfaces of the desktop.
    pub surfaces: Vec<SurfaceFacts>,
    /// What the person is already doing.
    pub engagement: Vec<Engagement>,
}

/// Something a desktop may offer a person. None of these happens on its own.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "suggestion", rename_all = "kebab-case")]
pub enum SpatialSuggestion {
    /// A surface already in front of the person is about what Mind is attending to.
    ///
    /// The weakest thing this crate can say, and the most common: nothing moves, nothing opens, a
    /// surface the person is already looking at is marked.
    Highlight {
        /// The surface to mark.
        surface: String,
        /// What it is about.
        subject: SubjectQuery,
        /// Why.
        why: Rationale,
    },
    /// A surface is open but out of view, and the person may want to be taken to it.
    ///
    /// An offer, never a camera move. A canvas that flew to whatever Mind noticed would be
    /// unusable within ten minutes, and worse, it would be taking a position on what the person
    /// should be doing.
    OfferFocus {
        /// The surface to travel to, if the person asks.
        surface: String,
        /// What it is about.
        subject: SubjectQuery,
        /// Why.
        why: Rationale,
    },
    /// Nothing on the canvas shows this, and a surface could.
    Reveal {
        /// The surface that could show it.
        surface: String,
        /// What it would show.
        subject: SubjectQuery,
        /// An open surface to place it beside, when there is a sensible one.
        near: Option<String>,
        /// Why.
        why: Rationale,
    },
    /// Enough of one episode is missing from the canvas that it is worth offering as a whole.
    Gather {
        /// The episode this belongs to, when the context named one.
        correlation: Option<Uuid>,
        /// What it is about, in the order the policy ranked them.
        subjects: Vec<SubjectQuery>,
        /// The surfaces that would show them, positionally matching `subjects`.
        surfaces: Vec<String>,
        /// Why, taken from the strongest contribution in the set.
        why: Rationale,
    },
}

impl SpatialSuggestion {
    /// The grounds this suggestion rests on.
    #[must_use]
    pub const fn why(&self) -> &Rationale {
        match self {
            Self::Highlight { why, .. }
            | Self::OfferFocus { why, .. }
            | Self::Reveal { why, .. }
            | Self::Gather { why, .. } => why,
        }
    }
}

/// One candidate before it is known what kind of suggestion it becomes.
struct Candidate {
    subject: SubjectQuery,
    surface: SurfaceFacts,
    why: Rationale,
}

/// What the desktop could offer, given what Mind is attending to.
///
/// Deterministic and total: the same context produces the same list in the same order, and a
/// context this cannot make anything of produces an empty list rather than a filler suggestion.
///
/// Order is by the confidence the contribution carried, strongest first, with ties broken by the
/// subject's canonical spelling so that two equally-confident suggestions never swap places.
#[must_use]
pub fn suggest(context: &SpatialContext) -> Vec<SpatialSuggestion> {
    let candidates = candidates(context);
    let (mut suggestions, reveals) = classify(candidates, context);

    // Below the threshold each closed surface is offered on its own, so the person can take one.
    // At it, the set is the point, and six separate offers would be a list of chores.
    if reveals.len() >= GATHER_THRESHOLD {
        suggestions.push(gather(reveals, context.correlation));
    } else {
        suggestions.extend(reveals);
    }

    suggestions.sort_by(|left, right| rank(left).cmp_to(&rank(right)));
    suggestions.truncate(SUGGESTION_LIMIT);
    suggestions
}

/// The subjects worth considering, strongest-first, each with the surface that could show it.
fn candidates(context: &SpatialContext) -> Vec<Candidate> {
    let mut seen: Vec<SubjectQuery> = Vec::new();
    let mut candidates = Vec::new();

    for attended in &context.attention {
        // An unread payload and an unclassifiable key are both real facts about the moment, and
        // neither is a subject a desktop could open. They are shown by the attention card as what
        // they are; making a spatial offer out of one would be inventing the entity.
        let SubjectReading::Classified(subject) = &attended.reading else {
            continue;
        };
        // Newest first upstream, so the first mention of a subject carries the freshest grounds.
        if seen.contains(subject) {
            continue;
        }
        seen.push(subject.clone());

        // The person is already there. Competing with them for their own attention is the failure
        // mode that makes this kind of system intolerable, so engagement suppresses rather than
        // boosts — including a dismissal, which is them having already answered this offer.
        if context
            .engagement
            .iter()
            .any(|engagement| &engagement.subject == subject)
        {
            continue;
        }

        let Some(surface) = surface_for(subject, &context.surfaces) else {
            continue;
        };

        candidates.push(Candidate {
            subject: subject.clone(),
            surface,
            why: Rationale {
                correlation: context.correlation,
                contribution: attended.contribution,
                organ: attended.organ.clone(),
                confidence: attended.confidence,
                evidence: attended.evidence.clone(),
            },
        });
    }

    candidates
}

/// The surface that would show a subject: the one already standing for it, else a free one of its
/// kind.
///
/// A surface standing for something else is not a candidate, even though it is of the right kind. A
/// detail panel showing `sshd.service` can present a service in general and is presenting that one
/// in particular, and marking it for `nginx.service` would tell a person their panel is about
/// something it is not. It remains a perfectly good thing to place a new surface beside, which is a
/// different question and [`neighbour_for`] answers it.
fn surface_for(subject: &SubjectQuery, surfaces: &[SurfaceFacts]) -> Option<SurfaceFacts> {
    surfaces
        .iter()
        .find(|surface| surface.subject.as_ref() == Some(subject))
        .or_else(|| {
            surfaces.iter().find(|surface| {
                surface.subject.is_none() && surface.shows.contains(&subject.kind())
            })
        })
        .cloned()
}

/// Split candidates into what can be said about them now, and what would have to be opened.
fn classify(
    candidates: Vec<Candidate>,
    context: &SpatialContext,
) -> (Vec<SpatialSuggestion>, Vec<SpatialSuggestion>) {
    let mut immediate = Vec::new();
    let mut reveals = Vec::new();

    for candidate in candidates {
        let Candidate {
            subject,
            surface,
            why,
        } = candidate;
        match (surface.open, surface.in_view) {
            (true, true) => immediate.push(SpatialSuggestion::Highlight {
                surface: surface.key,
                subject,
                why,
            }),
            (true, false) => immediate.push(SpatialSuggestion::OfferFocus {
                surface: surface.key,
                subject,
                why,
            }),
            (false, _) => {
                let near = neighbour_for(&subject, &context.surfaces);
                reveals.push(SpatialSuggestion::Reveal {
                    surface: surface.key,
                    subject,
                    near,
                    why,
                });
            }
        }
    }

    (immediate, reveals)
}

/// An open surface a new one could sensibly stand beside: one the person can see, of the same kind.
///
/// `None` is a perfectly good answer and the caller must handle it. Naming an arbitrary open
/// surface would place a service beside a mail folder because both happened to be open.
fn neighbour_for(subject: &SubjectQuery, surfaces: &[SurfaceFacts]) -> Option<String> {
    surfaces
        .iter()
        .find(|surface| surface.open && surface.in_view && surface.shows.contains(&subject.kind()))
        .map(|surface| surface.key.clone())
}

/// Collapse several reveals into one offer, keeping their order and the strongest grounds.
fn gather(reveals: Vec<SpatialSuggestion>, correlation: Option<Uuid>) -> SpatialSuggestion {
    let mut ordered = reveals;
    ordered.sort_by(|left, right| rank(left).cmp_to(&rank(right)));

    let why = ordered.first().map_or_else(
        || Rationale {
            correlation,
            contribution: Uuid::nil(),
            organ: String::new(),
            confidence: 0.0,
            evidence: Vec::new(),
        },
        |suggestion| suggestion.why().clone(),
    );

    let mut subjects = Vec::with_capacity(ordered.len());
    let mut surfaces = Vec::with_capacity(ordered.len());
    for suggestion in ordered {
        if let SpatialSuggestion::Reveal {
            surface, subject, ..
        } = suggestion
        {
            subjects.push(subject);
            surfaces.push(surface);
        }
    }

    SpatialSuggestion::Gather {
        correlation,
        subjects,
        surfaces,
        why,
    }
}

/// The sort key of a suggestion: how sure its grounds were, then something that never ties.
struct Rank {
    confidence: f64,
    tiebreak: String,
}

impl Rank {
    /// Strongest first, and equally strong suggestions in a fixed order rather than an arbitrary
    /// one — a list that reshuffled between two identical runs would be impossible to trust.
    fn cmp_to(&self, other: &Self) -> Ordering {
        other
            .confidence
            .partial_cmp(&self.confidence)
            .unwrap_or(Ordering::Equal)
            .then_with(|| self.tiebreak.cmp(&other.tiebreak))
    }
}

/// Read a suggestion's sort key.
fn rank(suggestion: &SpatialSuggestion) -> Rank {
    let tiebreak = match suggestion {
        SpatialSuggestion::Highlight { subject, .. }
        | SpatialSuggestion::OfferFocus { subject, .. }
        | SpatialSuggestion::Reveal { subject, .. } => subject_key(subject),
        SpatialSuggestion::Gather { subjects, .. } => {
            subjects.first().map_or_else(String::new, subject_key)
        }
    };
    Rank {
        confidence: suggestion.why().confidence,
        tiebreak,
    }
}

/// A stable, comparable spelling of a subject.
fn subject_key(subject: &SubjectQuery) -> String {
    format!("{}/{}", subject.kind_name(), subject.identifier())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attended(id: u128, subject: SubjectReading, confidence: f64) -> AttendedSubject {
        AttendedSubject {
            contribution: Uuid::from_u128(id),
            organ: "perceptiond".to_owned(),
            kind: 1,
            confidence,
            evidence: vec![Uuid::from_u128(900 + id)],
            reading: subject,
        }
    }

    fn service(name: &str) -> SubjectQuery {
        SubjectQuery::Service(name.to_owned())
    }

    fn classified(name: &str) -> SubjectReading {
        SubjectReading::Classified(service(name))
    }

    fn services_panel(key: &str, open: bool, in_view: bool) -> SurfaceFacts {
        SurfaceFacts {
            key: key.to_owned(),
            shows: vec![SubjectKind::Service],
            open,
            in_view,
            subject: None,
        }
    }

    fn context(attention: Vec<AttendedSubject>, surfaces: Vec<SurfaceFacts>) -> SpatialContext {
        SpatialContext {
            correlation: Some(Uuid::from_u128(42)),
            attention,
            surfaces,
            engagement: Vec::new(),
        }
    }

    #[test]
    fn a_surface_in_front_of_the_person_is_marked_rather_than_moved() {
        let suggestions = suggest(&context(
            vec![attended(1, classified("nginx.service"), 0.9)],
            vec![services_panel("services", true, true)],
        ));
        assert_eq!(suggestions.len(), 1);
        match &suggestions[0] {
            SpatialSuggestion::Highlight {
                surface,
                subject,
                why,
            } => {
                assert_eq!(surface, "services");
                assert_eq!(subject, &service("nginx.service"));
                // The grounds travel with the offer, all the way from the contribution.
                assert_eq!(why.organ, "perceptiond");
                assert_eq!(why.contribution, Uuid::from_u128(1));
                assert_eq!(why.evidence, vec![Uuid::from_u128(901)]);
                assert_eq!(why.correlation, Some(Uuid::from_u128(42)));
            }
            other => panic!("expected a highlight, got {other:?}"),
        }
    }

    #[test]
    fn a_surface_out_of_view_is_offered_and_never_travelled_to() {
        let suggestions = suggest(&context(
            vec![attended(1, classified("nginx.service"), 0.9)],
            vec![services_panel("services", true, false)],
        ));
        assert!(matches!(
            suggestions.as_slice(),
            [SpatialSuggestion::OfferFocus { .. }]
        ));
    }

    #[test]
    fn something_nothing_shows_is_offered_beside_something_of_its_kind() {
        let mut open_panel = services_panel("services", true, true);
        open_panel.subject = Some(service("sshd.service"));
        let suggestions = suggest(&context(
            vec![attended(1, classified("nginx.service"), 0.9)],
            vec![open_panel, services_panel("service-detail", false, false)],
        ));
        match &suggestions[0] {
            SpatialSuggestion::Reveal { surface, near, .. } => {
                assert_eq!(surface, "service-detail");
                assert_eq!(near.as_deref(), Some("services"));
            }
            other => panic!("expected a reveal, got {other:?}"),
        }
    }

    #[test]
    fn a_surface_already_standing_for_the_subject_wins_over_one_of_its_kind() {
        let mut standing = services_panel("service-detail", true, true);
        standing.subject = Some(service("nginx.service"));
        let suggestions = suggest(&context(
            vec![attended(1, classified("nginx.service"), 0.9)],
            vec![services_panel("services", true, true), standing],
        ));
        match &suggestions[0] {
            SpatialSuggestion::Highlight { surface, .. } => assert_eq!(surface, "service-detail"),
            other => panic!("expected the standing surface, got {other:?}"),
        }
    }

    #[test]
    fn what_a_reader_could_not_make_out_is_never_made_into_an_offer() {
        // Both are real facts about the moment and neither names an entity. A suggestion built on
        // one would put something on the canvas that no organ ever observed.
        let suggestions = suggest(&context(
            vec![
                attended(1, SubjectReading::Unread, 0.9),
                attended(
                    2,
                    SubjectReading::Uninterpreted("operating-system".to_owned()),
                    0.9,
                ),
            ],
            vec![services_panel("services", true, true)],
        ));
        assert!(suggestions.is_empty());
    }

    #[test]
    fn nothing_is_offered_about_what_no_surface_could_show() {
        let suggestions = suggest(&context(
            vec![attended(1, classified("nginx.service"), 0.9)],
            vec![SurfaceFacts {
                key: "mail".to_owned(),
                shows: vec![SubjectKind::File],
                open: true,
                in_view: true,
                subject: None,
            }],
        ));
        assert!(suggestions.is_empty());
    }

    #[test]
    fn the_desktop_does_not_compete_with_the_person_for_their_own_attention() {
        for kind in [
            EngagementKind::Selected,
            EngagementKind::Investigating,
            EngagementKind::Dismissed,
        ] {
            let mut ctx = context(
                vec![attended(1, classified("nginx.service"), 0.9)],
                vec![services_panel("services", true, true)],
            );
            ctx.engagement = vec![Engagement {
                subject: service("nginx.service"),
                kind,
            }];
            assert!(suggest(&ctx).is_empty(), "{kind:?} should suppress");
        }
    }

    #[test]
    fn one_subject_mentioned_twice_is_offered_once_on_its_freshest_grounds() {
        let suggestions = suggest(&context(
            vec![
                attended(2, classified("nginx.service"), 0.4),
                attended(1, classified("nginx.service"), 0.9),
            ],
            vec![services_panel("services", true, true)],
        ));
        assert_eq!(suggestions.len(), 1);
        // Newest first upstream, so the newer contribution supplies the grounds even though the
        // older one was more confident.
        assert_eq!(suggestions[0].why().contribution, Uuid::from_u128(2));
    }

    #[test]
    fn a_few_things_to_open_stay_separate_and_a_setful_becomes_one_offer() {
        let surfaces = vec![
            services_panel("services", false, false),
            SurfaceFacts {
                key: "processes".to_owned(),
                shows: vec![SubjectKind::Process],
                open: false,
                in_view: false,
                subject: None,
            },
            SurfaceFacts {
                key: "files".to_owned(),
                shows: vec![SubjectKind::File],
                open: false,
                in_view: false,
                subject: None,
            },
        ];
        let two = suggest(&context(
            vec![
                attended(1, classified("nginx.service"), 0.9),
                attended(
                    2,
                    SubjectReading::Classified(SubjectQuery::Process("1234".to_owned())),
                    0.8,
                ),
            ],
            surfaces.clone(),
        ));
        assert_eq!(two.len(), 2);
        assert!(
            two.iter()
                .all(|suggestion| matches!(suggestion, SpatialSuggestion::Reveal { .. }))
        );

        let three = suggest(&context(
            vec![
                attended(1, classified("nginx.service"), 0.9),
                attended(
                    2,
                    SubjectReading::Classified(SubjectQuery::Process("1234".to_owned())),
                    0.8,
                ),
                attended(
                    3,
                    SubjectReading::Classified(SubjectQuery::File("/etc/nginx.conf".to_owned())),
                    0.7,
                ),
            ],
            surfaces,
        ));
        match three.as_slice() {
            [
                SpatialSuggestion::Gather {
                    subjects,
                    surfaces,
                    why,
                    correlation,
                },
            ] => {
                assert_eq!(subjects.len(), 3);
                assert_eq!(surfaces.len(), 3);
                // Strongest first, and the offer rests on the strongest contribution in the set.
                assert_eq!(subjects[0], service("nginx.service"));
                assert_eq!(surfaces[0], "services");
                assert!((why.confidence - 0.9).abs() < f64::EPSILON);
                assert_eq!(*correlation, Some(Uuid::from_u128(42)));
            }
            other => panic!("expected one gather, got {other:?}"),
        }
    }

    #[test]
    fn the_same_context_twice_produces_the_same_list_in_the_same_order() {
        // Equal confidences on purpose: this is exactly where an unstable order would show.
        let ctx = context(
            vec![
                attended(1, classified("b.service"), 0.5),
                attended(2, classified("a.service"), 0.5),
                attended(3, classified("c.service"), 0.5),
            ],
            vec![services_panel("services", true, true)],
        );
        let first = suggest(&ctx);
        assert_eq!(first, suggest(&ctx));
        let names: Vec<&str> = first
            .iter()
            .map(|suggestion| match suggestion {
                SpatialSuggestion::Highlight { subject, .. } => subject.identifier(),
                other => panic!("expected highlights, got {other:?}"),
            })
            .collect();
        assert_eq!(names, ["a.service", "b.service", "c.service"]);
    }

    #[test]
    fn a_person_is_offered_a_bounded_number_of_things() {
        let attention = (1..=12u128)
            .map(|index| attended(index, classified(&format!("unit-{index}.service")), 0.5))
            .collect();
        let suggestions = suggest(&context(
            attention,
            vec![services_panel("services", true, true)],
        ));
        assert_eq!(suggestions.len(), SUGGESTION_LIMIT);
    }

    #[test]
    fn a_suggestion_survives_the_trip_to_a_browser() {
        let suggestion = SpatialSuggestion::Reveal {
            surface: "services".to_owned(),
            subject: service("nginx.service"),
            near: None,
            why: Rationale {
                correlation: Some(Uuid::from_u128(42)),
                contribution: Uuid::from_u128(1),
                organ: "perceptiond".to_owned(),
                confidence: 0.9,
                evidence: Vec::new(),
            },
        };
        let encoded = serde_json::to_string(&suggestion).expect("serialize suggestion");
        let decoded: SpatialSuggestion =
            serde_json::from_str(&encoded).expect("deserialize suggestion");
        assert_eq!(decoded, suggestion);
    }
}
