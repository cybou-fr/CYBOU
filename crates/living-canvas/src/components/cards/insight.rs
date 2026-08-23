// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! System Insight card: what this host makes of itself, and what it would offer to do about it.
//!
//! Every other card shows a projection of Mind. This one shows a projection of the *machine* — and
//! it is the one card whose whole content is produced with no model and no network, which is why it
//! still works at the moment it is most wanted.
//!
//! Four things are drawn that a dashboard would leave out:
//!
//! - **The reading behind each finding**, so *why do you think that* is answered by looking rather
//!   than by trusting.
//! - **What is ordinary for this host**, beside what was observed. A number without its baseline is
//!   a number a reader has to take on faith.
//! - **What was never looked at.** An all-clear on a kernel with no pressure accounting is an
//!   all-clear about a subset, and a card that did not say so would be answering a question it was
//!   not asked.
//! - **What the host would offer to do, and what the gate says about doing it.** None of it can be
//!   carried out — there is no executor — and showing the verdict now is deliberate: a person should
//!   see what the system would ask permission for while the answer is still theoretical.

use cybou_protocol::KnowledgeState;
use cybou_web_contracts::{FindingProjection, InsightProjection};
use leptos::prelude::*;
use lucide_leptos::Activity;
use std::sync::Arc;

use crate::{
    CardId, DesktopItemId, DesktopLayout,
    components::card_frame::CardFrame,
    instant::instant_label,
    interaction::{DragState, ResizeState},
    state::RuntimeState,
};

/// What the host makes of itself, if the gateway could be asked.
fn insight_of(runtime: RwSignal<RuntimeState>) -> Option<InsightProjection> {
    match runtime.get() {
        RuntimeState::Ready { insight, .. } => insight,
        RuntimeState::Loading | RuntimeState::Error(_) | RuntimeState::SignInRequired => None,
    }
}

/// One line saying where this host stands.
///
/// Four states, deliberately distinguished. The surface could not be asked; the organ could not be
/// read; it has not watched long enough to have a notion of ordinary; and it has an answer.
/// Collapsing any pair of the first three would let *nobody looked* read as *nothing is wrong*.
fn headline(insight: Option<&InsightProjection>) -> String {
    match insight {
        None => "Not read".to_owned(),
        Some(state) if state.knowledge == KnowledgeState::Unknown => {
            "Telemetry did not answer".to_owned()
        }
        Some(state) if !state.watched_enough => "Not watching long enough yet".to_owned(),
        Some(state) => match state.findings.len() {
            0 => "Nothing needs attention".to_owned(),
            1 => "1 thing needs attention".to_owned(),
            count => format!("{count} things need attention"),
        },
    }
}

/// How a verdict reads to the person who would have to answer it.
const fn verdict_text(verdict: &str) -> &'static str {
    match verdict.as_bytes() {
        b"granted" => "pre-authorized",
        b"requires-confirmation" => "would ask you",
        b"denied" => "refused",
        _ => "verdict not recognised",
    }
}

/// System Insight domain content presentation.
#[component]
pub fn InsightContent(runtime: RwSignal<RuntimeState>) -> impl IntoView {
    let state = move || insight_of(runtime);
    let label = move || headline(state().as_ref());

    let findings = move || {
        state().map_or_else(Vec::new, |state| {
            state.findings.into_iter().enumerate().collect::<Vec<_>>()
        })
    };

    // Said outright rather than left to be inferred from an absence. An all-clear about a subset
    // and an all-clear about everything look identical on a screen unless one of them says so.
    let unobserved = move || match state() {
        Some(state) if !state.unobserved.is_empty() => {
            format!("No readings for: {}", state.unobserved.join(", "))
        }
        _ => String::new(),
    };

    let said = move || state().map(|state| state.said).unwrap_or_default();

    view! {
        <div class="insight-card-body">
            <strong>{label}</strong>
            <span class="insight-unobserved">{unobserved}</span>

            <div class="finding-list">
                <For
                    each=findings
                    key=|(index, finding)| format!("{index}:{}", finding.finding)
                    children=move |(_, finding)| view! { <FindingRow finding=finding /> }
                />
            </div>

            <details class="insight-said">
                <summary>"What it would say"</summary>
                <pre>{said}</pre>
            </details>
            <span class="panel-link">"Nothing here can be carried out"</span>
        </div>
    }
}

/// One finding, its readings, and what could be offered about it.
#[component]
fn FindingRow(finding: FindingProjection) -> impl IntoView {
    let since = instant_label(&finding.since);
    let strength = finding.strength.clone();
    let readings = finding.readings.clone();
    let offers = finding.offers.clone();
    let has_offers = !offers.is_empty();

    view! {
        <div class="finding-line">
            <span class="finding-head">
                <b>{finding.means.clone()}</b>
                <small class="finding-strength">{strength}</small>
            </span>
            <small class="finding-since">{format!("since {since}")}</small>

            <div class="reading-list">
                {readings
                    .into_iter()
                    .map(|reading| {
                        // The observation beside what is ordinary for this host. A number without
                        // its baseline is a number a reader has to take on faith.
                        view! {
                            <span class="reading-line">
                                <code>{reading.subject}</code>
                                <b>{format!("{:.2}", reading.observed)}</b>
                                <small>{format!("ordinary {:.2}", reading.ordinary)}</small>
                            </span>
                        }
                    })
                    .collect_view()}
            </div>

            <Show when=move || has_offers>
                <span class="offer-label">"Could offer"</span>
            </Show>
            <div class="offer-list">
                {offers
                    .into_iter()
                    .map(|offer| {
                        let undo = if offer.reversible { "reversible" } else { "cannot be undone" };
                        view! {
                            <span class="offer-line">
                                <code>{offer.operation}</code>
                                <small class="offer-risk">{offer.risk}</small>
                                <small class="offer-undo">{undo}</small>
                                <small class="offer-verdict">{verdict_text(&offer.verdict)}</small>
                            </span>
                        }
                    })
                    .collect_view()}
            </div>
        </div>
    }
}

/// System Insight cognitive card component.
#[component]
pub fn InsightCard(
    layout: RwSignal<DesktopLayout>,
    selected: ReadSignal<Option<DesktopItemId>>,
    set_selected: WriteSignal<Option<DesktopItemId>>,
    dragging: RwSignal<Option<DragState>>,
    resizing: RwSignal<Option<ResizeState>>,
    runtime: RwSignal<RuntimeState>,
) -> impl IntoView {
    let collapsed = move || {
        let state = insight_of(runtime);
        let label = headline(state.as_ref());
        let offered = state.map_or(0, |state| {
            state
                .findings
                .iter()
                .map(|finding| finding.offers.len())
                .sum()
        });
        view! {
            <div class="card-collapsed-summary">
                <b>"System Insight"</b>
                <span>{label}</span>
                <span>
                    {if offered == 0 { String::new() } else { format!("{offered} offered") }}
                </span>
            </div>
        }
        .into_any()
    };

    view! {
        <CardFrame
            card=CardId::Insight
            layout=layout
            selected=selected
            set_selected=set_selected
            dragging=dragging
            resizing=resizing
            kicker_title="Telemetry1"
            kicker_icon=Arc::new(|| view! { <Activity size=14 /> }.into_any())
            collapsed_summary=Arc::new(collapsed)
        >
            <InsightContent runtime=runtime />
        </CardFrame>
    }
}
