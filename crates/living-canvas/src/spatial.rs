// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! The desktop describing itself to the spatial policy, and reading its answer.
//!
//! [`cybou_spatial_policy`] holds no knowledge of this desktop: not which panels exist, not which
//! of them shows a service, not where any of them is. It is handed facts and returns suggestions.
//! This module is the half of that conversation the desktop owns — it turns a layout, a camera and
//! a Workspace1 projection into a [`SpatialContext`], and turns a suggestion back into the card it
//! is about.
//!
//! Deliberately outside `components`: nothing here draws anything, and keeping it out of the
//! wasm-only half means the rules that decide what a person is offered can be tested without a
//! browser.

use cybou_protocol::attention::AttendedSubject;
use cybou_protocol::{SubjectKind, SubjectQuery};
use cybou_spatial_policy::{Engagement, SpatialContext, SpatialSuggestion, SurfaceFacts};
use cybou_web_contracts::{AttendedSubjectProjection, AttentionProjection};
use uuid::Uuid;

use crate::card::CardId;
use crate::layout::DesktopLayout;
use crate::layout::camera::is_within_view;

/// Where the camera is, for deciding what the person can currently see.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraView {
    /// Stage translation, in screen pixels.
    pub pan: (f64, f64),
    /// Stage scale.
    pub zoom: f64,
    /// The window, as last measured.
    pub viewport: (f64, f64),
}

/// Everything the policy is allowed to know, assembled from what the desktop knows.
///
/// `standing` names the cards that are currently about a particular subject — an inspector pointed
/// at a unit, say. The desktop passes it in rather than the layout holding it, because standing for
/// something is a fact about what a card is displaying this second and not about where it sits.
#[must_use]
pub fn spatial_context(
    layout: &DesktopLayout,
    attention: &AttentionProjection,
    camera: Option<CameraView>,
    standing: &[(CardId, SubjectQuery)],
    engagement: &[Engagement],
) -> SpatialContext {
    SpatialContext {
        correlation: attention.focus.as_deref().and_then(|id| id.parse().ok()),
        attention: attention.subjects.iter().filter_map(attended).collect(),
        surfaces: surfaces(layout, camera, standing),
        engagement: engagement.to_vec(),
    }
}

/// One projected contribution as the policy takes it.
///
/// A projection whose identities do not parse is dropped rather than repaired with a nil UUID.
/// A rationale citing `00000000-0000-0000-0000-000000000000` reads as evidence and is not any, and
/// the point of carrying grounds at all is that a person can follow them.
fn attended(subject: &AttendedSubjectProjection) -> Option<AttendedSubject> {
    let contribution = subject.contribution.parse::<Uuid>().ok()?;
    let evidence = subject
        .evidence
        .iter()
        .map(|id| id.parse::<Uuid>())
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    Some(AttendedSubject {
        contribution,
        organ: subject.organ.clone(),
        kind: 0,
        confidence: subject.confidence,
        evidence,
        reading: subject.reading.clone(),
    })
}

/// Every surface the policy could offer, most specific first.
///
/// The ordering is the desktop's decision and not the policy's. The policy takes the first surface
/// that can show a kind, and the inspector can show all of them, so an unordered list would have
/// every offer land on the inspector and the Services panel never be suggested at all. Cards that
/// present no subject are left out entirely: they are not candidates for anything.
fn surfaces(
    layout: &DesktopLayout,
    camera: Option<CameraView>,
    standing: &[(CardId, SubjectQuery)],
) -> Vec<SurfaceFacts> {
    let mut surfaces: Vec<SurfaceFacts> = candidate_cards(layout)
        .into_iter()
        .filter(|card| !card.shows().is_empty())
        .map(|card| SurfaceFacts {
            key: card.instance_key(),
            shows: card.shows().to_vec(),
            open: layout.contains_card(card),
            in_view: in_view(layout, card, camera),
            subject: standing
                .iter()
                .find(|(standing_card, _)| *standing_card == card)
                .map(|(_, subject)| subject.clone()),
        })
        .collect();
    surfaces.sort_by_key(|surface| surface.shows.len());
    surfaces
}

/// The cards worth describing: the ones on the canvas, plus one closed candidate of each kind.
///
/// A closed card has to be describable or nothing could ever be suggested for opening — that is the
/// whole of `Reveal`. Instance zero stands in for a kind nobody has opened; where a person already
/// has panels of that kind, theirs are described instead and the placeholder is not added.
fn candidate_cards(layout: &DesktopLayout) -> Vec<CardId> {
    let mut cards: Vec<CardId> = layout.cards.iter().map(|card| card.id).collect();
    for key in [
        "services",
        "processes",
        "files",
        "agents",
        "packages",
        "operations",
        "inspector",
    ] {
        let Some(candidate) = CardId::from_key(key) else {
            continue;
        };
        if !cards.iter().any(|card| card.key() == candidate.key()) {
            cards.push(candidate);
        }
    }
    cards
}

/// Whether the person can currently see a card.
///
/// A card that is not on the canvas is not in view, and a camera nobody has measured shows
/// everything — the same answer [`is_within_view`] gives, for the same reason: the failure of this
/// has to be offering too little movement, never hiding a panel that is on the screen.
fn in_view(layout: &DesktopLayout, card: CardId, camera: Option<CameraView>) -> bool {
    if !layout.contains_card(card) {
        return false;
    }
    camera.is_none_or(|camera| {
        is_within_view(
            layout.geometry(card),
            camera.pan,
            camera.zoom,
            camera.viewport,
        )
    })
}

/// The card a suggestion is about, when the key still names one.
#[must_use]
pub fn target_card(suggestion: &SpatialSuggestion) -> Option<CardId> {
    match suggestion {
        SpatialSuggestion::Highlight { surface, .. }
        | SpatialSuggestion::OfferFocus { surface, .. }
        | SpatialSuggestion::Reveal { surface, .. } => CardId::from_instance_key(surface),
        SpatialSuggestion::Gather { surfaces, .. } => surfaces
            .first()
            .and_then(|key| CardId::from_instance_key(key)),
    }
}

/// Every card a suggestion would have the desktop show, in the order the policy ranked them.
#[must_use]
pub fn target_cards(suggestion: &SpatialSuggestion) -> Vec<CardId> {
    match suggestion {
        SpatialSuggestion::Gather { surfaces, .. } => surfaces
            .iter()
            .filter_map(|key| CardId::from_instance_key(key))
            .collect(),
        other => target_card(other).into_iter().collect(),
    }
}

/// What a suggestion is about, as one line a person can read.
#[must_use]
pub fn suggestion_label(suggestion: &SpatialSuggestion) -> String {
    match suggestion {
        SpatialSuggestion::Highlight { subject, .. }
        | SpatialSuggestion::OfferFocus { subject, .. }
        | SpatialSuggestion::Reveal { subject, .. } => subject.identifier().to_owned(),
        SpatialSuggestion::Gather { subjects, .. } => subjects
            .iter()
            .map(SubjectQuery::identifier)
            .collect::<Vec<_>>()
            .join(", "),
    }
}

/// What accepting a suggestion would do, in the words offered on its button.
#[must_use]
pub const fn suggestion_action(suggestion: &SpatialSuggestion) -> &'static str {
    match suggestion {
        SpatialSuggestion::Highlight { .. } => "Select",
        SpatialSuggestion::OfferFocus { .. } => "Focus",
        SpatialSuggestion::Reveal { .. } => "Open",
        SpatialSuggestion::Gather { .. } => "Open all",
    }
}

/// The subject a suggestion is about, for recording that a person turned it down.
#[must_use]
pub fn suggestion_subjects(suggestion: &SpatialSuggestion) -> Vec<SubjectQuery> {
    match suggestion {
        SpatialSuggestion::Highlight { subject, .. }
        | SpatialSuggestion::OfferFocus { subject, .. }
        | SpatialSuggestion::Reveal { subject, .. } => vec![subject.clone()],
        SpatialSuggestion::Gather { subjects, .. } => subjects.clone(),
    }
}

/// The card to place a revealed surface beside, when the policy named one.
#[must_use]
pub fn near_card(suggestion: &SpatialSuggestion) -> Option<CardId> {
    match suggestion {
        SpatialSuggestion::Reveal { near, .. } => {
            near.as_deref().and_then(CardId::from_instance_key)
        }
        _ => None,
    }
}

/// Whether a card kind can present anything at all, for callers that only need the question.
#[must_use]
pub fn presents(card: CardId, kind: SubjectKind) -> bool {
    card.shows().contains(&kind)
}

#[cfg(test)]
mod tests {
    use cybou_protocol::KnowledgeState;
    use cybou_protocol::attention::SubjectReading;
    use cybou_spatial_policy::suggest;

    use super::*;

    fn projected(subject: &str, confidence: f64) -> AttendedSubjectProjection {
        AttendedSubjectProjection {
            contribution: "1c9d3e2f-0000-4000-8000-000000000002".to_owned(),
            organ: "perceptiond".to_owned(),
            kind: "observation".to_owned(),
            confidence,
            evidence: Vec::new(),
            reading: SubjectReading::Classified(SubjectQuery::Service(subject.to_owned())),
        }
    }

    fn attention(subjects: Vec<AttendedSubjectProjection>) -> AttentionProjection {
        AttentionProjection {
            knowledge: KnowledgeState::Known,
            focus: Some("1c9d3e2f-0000-4000-8000-000000000001".to_owned()),
            salience: Some(0.8),
            organs: vec!["perceptiond".to_owned()],
            subjects,
        }
    }

    #[test]
    fn the_desktop_describes_a_closed_panel_so_it_can_be_offered_for_opening() {
        let layout = DesktopLayout::default();
        let context = spatial_context(
            &layout,
            &attention(vec![projected("nginx.service", 0.9)]),
            None,
            &[],
            &[],
        );

        let services = context
            .surfaces
            .iter()
            .find(|surface| surface.key == "services:0")
            .expect("a services surface is described");
        assert_eq!(services.shows, vec![SubjectKind::Service]);
        assert!(!services.open);

        match suggest(&context).as_slice() {
            [SpatialSuggestion::Reveal { surface, .. }] => assert_eq!(surface, "services:0"),
            other => panic!("expected one reveal, got {other:?}"),
        }
    }

    #[test]
    fn the_inspector_is_the_last_surface_offered_rather_than_the_first() {
        // It can present every kind, so an unordered list would send every offer to it and the
        // panel a person actually wanted would never be suggested.
        let context = spatial_context(
            &DesktopLayout::default(),
            &attention(vec![projected("nginx.service", 0.9)]),
            None,
            &[],
            &[],
        );
        let inspector = context
            .surfaces
            .iter()
            .position(|surface| surface.key == "inspector:0")
            .expect("an inspector surface is described");
        let services = context
            .surfaces
            .iter()
            .position(|surface| surface.key == "services:0")
            .expect("a services surface is described");
        assert!(services < inspector);
    }

    #[test]
    fn a_card_standing_for_something_says_so() {
        let subject = SubjectQuery::Service("sshd.service".to_owned());
        let context = spatial_context(
            &DesktopLayout::default(),
            &attention(Vec::new()),
            None,
            &[(CardId::Inspector(0), subject.clone())],
            &[],
        );
        let inspector = context
            .surfaces
            .iter()
            .find(|surface| surface.key == "inspector:0")
            .expect("an inspector surface is described");
        assert_eq!(inspector.subject, Some(subject));
    }

    #[test]
    fn a_projection_whose_identities_do_not_parse_is_dropped_rather_than_repaired() {
        let mut broken = projected("nginx.service", 0.9);
        broken.contribution = "not a uuid".to_owned();
        let context = spatial_context(
            &DesktopLayout::default(),
            &attention(vec![broken]),
            None,
            &[],
            &[],
        );
        assert!(context.attention.is_empty());
        assert!(suggest(&context).is_empty());
    }

    #[test]
    fn a_suggestion_names_the_exact_panel_and_not_the_first_of_its_kind() {
        let mut layout = DesktopLayout::default();
        layout.open_card(CardId::Services(3), 100.0, 100.0);
        let context = spatial_context(
            &layout,
            &attention(vec![projected("nginx.service", 0.9)]),
            None,
            &[],
            &[],
        );
        let suggestions = suggest(&context);
        assert_eq!(
            target_card(&suggestions[0]),
            Some(CardId::Services(3)),
            "the person's own panel, not instance zero"
        );
    }

    #[test]
    fn an_unmeasured_camera_counts_everything_as_visible() {
        let mut layout = DesktopLayout::default();
        layout.open_card(CardId::Services(0), 100.0, 100.0);
        let context = spatial_context(
            &layout,
            &attention(vec![projected("nginx.service", 0.9)]),
            None,
            &[],
            &[],
        );
        assert!(matches!(
            suggest(&context).as_slice(),
            [SpatialSuggestion::Highlight { .. }]
        ));
    }

    #[test]
    fn a_panel_the_person_has_panned_away_from_is_offered_rather_than_marked() {
        let mut layout = DesktopLayout::default();
        layout.open_card(CardId::Services(0), 90_000.0, 90_000.0);
        let context = spatial_context(
            &layout,
            &attention(vec![projected("nginx.service", 0.9)]),
            Some(CameraView {
                pan: (0.0, 0.0),
                zoom: 1.0,
                viewport: (1440.0, 900.0),
            }),
            &[],
            &[],
        );
        assert!(matches!(
            suggest(&context).as_slice(),
            [SpatialSuggestion::OfferFocus { .. }]
        ));
    }
}
