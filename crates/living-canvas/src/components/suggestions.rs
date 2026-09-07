// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! What the desktop could offer, offered — and nothing taken without being asked.
//!
//! The rules live in `cybou-spatial-policy` and the desktop's half of the conversation lives in
//! [`crate::spatial`]. This is only the surface: a short list under the Attention card, one line per
//! suggestion, each with a button that does the thing and a button that says no.
//!
//! Nothing here happens on its own. No panel opens, no camera moves, no card is closed until a
//! person clicks. That is ADR-0044's rule that presentation is not authorisation, kept at the one
//! place where it would be easiest to break: a canvas that flew to whatever Mind noticed would feel
//! clever for about ten minutes and then be unusable.

use cybou_protocol::SubjectQuery;
use cybou_spatial_policy::{Engagement, EngagementKind, SpatialSuggestion, suggest};
use leptos::prelude::*;

use crate::card::CardId;
use crate::layout::{DesktopItemId, DesktopLayout, UsableViewport};
use crate::spatial::{
    CameraView, near_card, spatial_context, suggestion_action, suggestion_label,
    suggestion_subjects, target_cards,
};
use crate::state::RuntimeState;
use crate::tool_state::ToolCardStates;

/// What the person has already answered, for as long as this desktop is open.
///
/// Session-local on purpose. A dismissal is a person saying no to an offer, and it has to outlive
/// the next projection or the same line would come back a second later; it does not have to outlive
/// the tab, because the grounds for the offer will have changed by then and re-asking is honest.
pub type SpatialEngagement = RwSignal<Vec<Engagement>>;

/// The suggestions the desktop is currently making, and the means to answer them.
#[allow(
    clippy::too_many_lines,
    reason = "one component: the context it reads, the two answers it offers, and the rows"
)]
#[component]
pub fn AttentionSuggestions(runtime: RwSignal<RuntimeState>) -> impl IntoView {
    let layout = use_context::<RwSignal<DesktopLayout>>();
    let set_selected = use_context::<WriteSignal<Option<DesktopItemId>>>();
    let tool_states = use_context::<ToolCardStates>();
    let camera = use_context::<crate::components::camera_context::CanvasCamera>();
    let pan = use_context::<ReadSignal<(f64, f64)>>();
    let set_pan = use_context::<WriteSignal<(f64, f64)>>();
    let zoom = use_context::<ReadSignal<f64>>();
    let set_zoom = use_context::<WriteSignal<f64>>();
    let camera_history = use_context::<RwSignal<crate::CameraHistory>>();
    // A desktop that provided none is a card mounted outside one, in a test or a story. It gets a
    // list of its own rather than a panic, and dismissing there answers only that card.
    let engagement =
        use_context::<SpatialEngagement>().unwrap_or_else(|| RwSignal::new(Vec::new()));

    let suggestions = move || {
        let (Some(layout), Some(attention)) = (layout, attention_of(runtime)) else {
            return Vec::new();
        };
        let layout = layout.get();
        let standing = standing_subjects(&layout, tool_states.as_ref());
        suggest(&spatial_context(
            &layout,
            &attention,
            camera.map(view_of),
            &standing,
            &engagement.get(),
        ))
    };

    let accept = move |suggestion: SpatialSuggestion| {
        let Some(layout) = layout else {
            return;
        };
        let cards = target_cards(&suggestion);
        match &suggestion {
            SpatialSuggestion::Highlight { .. } => {
                if let (Some(card), Some(set_selected)) = (cards.first(), set_selected) {
                    set_selected.set(Some(DesktopItemId::Card(*card)));
                }
            }
            SpatialSuggestion::OfferFocus { .. } => {
                if let Some(card) = cards.first() {
                    fly_to(layout, *card, pan, set_pan, zoom, set_zoom, camera_history);
                }
            }
            SpatialSuggestion::Reveal { .. } | SpatialSuggestion::Gather { .. } => {
                let near = near_card(&suggestion);
                let viewport = camera.map(|camera| {
                    let (width, height) = camera.viewport.get();
                    UsableViewport { width, height }
                });
                // In the order the policy ranked them, each beside the one before it, so a gathered
                // set arrives as a group rather than scattered across the plane.
                let mut beside = near;
                for card in &cards {
                    layout.update(|layout| layout.open_card_near(*card, beside, viewport));
                    beside = Some(*card);
                }
                if let (Some(card), Some(set_selected)) = (cards.first(), set_selected) {
                    set_selected.set(Some(DesktopItemId::Card(*card)));
                }
                layout.get_untracked().save();
            }
        }
        // Acting on an offer is engagement with what it was about: the person is there now, and the
        // desktop has nothing left to suggest about it.
        record(engagement, &suggestion, EngagementKind::Selected);
    };

    let dismiss = move |suggestion: SpatialSuggestion| {
        record(engagement, &suggestion, EngagementKind::Dismissed);
    };

    view! {
        <Show when=move || !suggestions().is_empty()>
            <span class="heading-label">"Could show"</span>
            <div class="attention-suggestions">
                {move || {
                    suggestions()
                        .into_iter()
                        .map(|suggestion| {
                            let accepted = suggestion.clone();
                            let dismissed = suggestion.clone();
                            let label = suggestion_label(&suggestion);
                            let action = suggestion_action(&suggestion);
                            let why = suggestion.why().clone();
                            let grounds = format!(
                                "{} · confidence {:.2} · {} pieces of evidence",
                                why.organ,
                                why.confidence,
                                why.evidence.len(),
                            );
                            view! {
                                <div class="attention-suggestion">
                                    <b title=grounds.clone()>{label}</b>
                                    <span class="attention-suggestion-why">{grounds}</span>
                                    <button
                                        class="attention-suggestion-accept"
                                        on:click=move |_| accept(accepted.clone())
                                    >
                                        {action}
                                    </button>
                                    <button
                                        class="attention-suggestion-dismiss"
                                        title="Do not offer this again"
                                        on:click=move |_| dismiss(dismissed.clone())
                                    >
                                        "No"
                                    </button>
                                </div>
                            }
                        })
                        .collect_view()
                }}
            </div>
        </Show>
    }
}

/// Workspace1's answer, when there is one to read.
fn attention_of(
    runtime: RwSignal<RuntimeState>,
) -> Option<cybou_web_contracts::AttentionProjection> {
    match runtime.get() {
        RuntimeState::Ready { mind, .. } => mind.map(|mind| mind.attention),
        RuntimeState::Loading | RuntimeState::Error(_) | RuntimeState::SignInRequired => None,
    }
}

/// Where the camera is, as the context builder wants it.
fn view_of(camera: crate::components::camera_context::CanvasCamera) -> CameraView {
    CameraView {
        pan: camera.pan.get(),
        zoom: camera.zoom.get(),
        viewport: camera.viewport.get(),
    }
}

/// The open inspectors and what each is currently pointed at.
fn standing_subjects(
    layout: &DesktopLayout,
    tool_states: Option<&ToolCardStates>,
) -> Vec<(CardId, SubjectQuery)> {
    let Some(tool_states) = tool_states else {
        return Vec::new();
    };
    layout
        .cards
        .iter()
        .map(|card| card.id)
        .filter(|card| matches!(card, CardId::Inspector(_)))
        .filter_map(|card| {
            tool_states
                .inspector(card)
                .subject_query
                .get()
                .map(|subject| (card, subject))
        })
        .collect()
}

/// Record what the person did with an offer, once per subject it was about.
fn record(engagement: SpatialEngagement, suggestion: &SpatialSuggestion, kind: EngagementKind) {
    engagement.update(|engagement| {
        for subject in suggestion_subjects(suggestion) {
            if engagement.iter().any(|held| held.subject == subject) {
                continue;
            }
            engagement.push(Engagement { subject, kind });
        }
    });
}

/// Take the person to a card they asked to be taken to.
fn fly_to(
    layout: RwSignal<DesktopLayout>,
    card: CardId,
    pan: Option<ReadSignal<(f64, f64)>>,
    set_pan: Option<WriteSignal<(f64, f64)>>,
    zoom: Option<ReadSignal<f64>>,
    set_zoom: Option<WriteSignal<f64>>,
    camera_history: Option<RwSignal<crate::CameraHistory>>,
) {
    let (Some(pan), Some(set_pan), Some(zoom), Some(set_zoom)) = (pan, set_pan, zoom, set_zoom)
    else {
        return;
    };
    let geometry = layout.get_untracked().geometry(card);
    crate::apply_camera_fly_to(
        camera_history,
        pan,
        set_pan,
        zoom,
        set_zoom,
        geometry.x + geometry.width / 2.0,
        geometry.y + geometry.height / 2.0,
        zoom.get_untracked(),
    );
}
