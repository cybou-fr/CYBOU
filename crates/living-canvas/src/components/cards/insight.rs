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
    MindClient,
    CardId, DesktopItemId, DesktopLayout,
    components::card_frame::CardFrame,
    heading::heading_line,
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

    // The declared things that produced nothing. Above the prose, because a person who reads one
    // line must not read an all-clear about a host that was partly not looked at.
    let unseen = move || {
        state()
            .as_ref()
            .and_then(|state| crate::heading::unseen_line(&state.watched))
            .unwrap_or_default()
    };

    let said = move || state().map(|state| state.said).unwrap_or_default();

    let headings = move || {
        state().map_or_else(Vec::new, |state| {
            state
                .projections
                .iter()
                .filter_map(heading_line)
                .enumerate()
                .collect::<Vec<_>>()
        })
    };

    view! {
        <div class="insight-card-body">
            <strong>{label}</strong>
            <span class="insight-unobserved">{unobserved}</span>
            <span class="insight-unseen">{unseen}</span>

            <Show when=move || !headings().is_empty()>
                <span class="heading-label">"Where things are going"</span>
            </Show>
            <div class="heading-list">
                <For
                    each=headings
                    key=|(index, line)| format!("{index}:{line}")
                    children=move |(_, line)| view! { <span class="heading-line">{line}</span> }
                />
            </div>

            <div class="finding-list">
                <For
                    each=findings
                    key=|(index, finding)| format!("{index}:{}", finding.finding)
                    children=move |(_, finding)| view! { <FindingRow finding=finding runtime=runtime /> }
                />
            </div>

            <details class="insight-said">
                <summary>"What it would say"</summary>
                <pre>{said}</pre>
            </details>
        </div>
    }
}

/// One finding, its readings, and what could be offered about it.
#[component]
fn FindingRow(finding: FindingProjection, runtime: RwSignal<RuntimeState>) -> impl IntoView {
    let since = instant_label(&finding.since);
    let strength = finding.strength.clone();
    let readings = finding.readings.clone();
    let offers = finding.offers.clone();
    let has_offers = !offers.is_empty();

    let finding_id = finding.id;
    let action_state = move || {
        if let RuntimeState::Ready {
            actions: Some(records),
            ..
        } = runtime.get()
        {
            if let Some(fid) = finding_id {
                if let Some(record) = records.iter().find(|r| r.cause_id == Some(fid)) {
                    let exec = record.execution_started.is_some() || record.attempt.is_some();
                    let rel = record
                        .outcome
                        .as_ref()
                        .map_or(false, |o| o.relief == "relieved");
                    let still = record
                        .outcome
                        .as_ref()
                        .map_or(false, |o| o.relief == "still-present");
                    let unest = record
                        .outcome
                        .as_ref()
                        .map_or(false, |o| o.relief == "not-established");
                    return (exec, rel, still, unest);
                }
            }
        }
        (false, false, false, false)
    };

    view! {
        <div class="finding-line">
            <span class="finding-head">
                <b>{crate::heading::finding_title(&finding)}</b>
                <small class="finding-strength">{strength}</small>
            </span>
            <small class="finding-since">{format!("since {since}")}</small>

            <div class="reading-list">
                {readings
                    .into_iter()
                    .map(|reading| {
                        // The observation beside what is ordinary for this host. A number without
                        // its baseline is a number a reader has to take on faith.
                        let baseline = crate::heading::baseline_line(&reading);
                        let why = crate::heading::why_explanation(reading.observed, reading.ordinary, reading.spread);
                        view! {
                            <div class="reading-block">
                                <span class="reading-line">
                                    <code>{reading.subject}</code>
                                    <b>{format!("{:.2}", reading.observed)}</b>
                                    <small>{baseline}</small>
                                </span>
                                <small class="reading-why">{why}</small>
                            </div>
                        }
                    })
                    .collect_view()}
            </div>

            <Show when=move || has_offers>
                <span class="offer-label">"Self-Healing Actions"</span>
            </Show>
            <div class="offer-list">
                {offers
                    .into_iter()
                    .map(|offer| {
                        let undo = if offer.reversible { "reversible" } else { "cannot be undone" };
                        let target = crate::heading::offer_target(&offer);
                        let (executed, relieved, still_present, outcome_unestablished) = action_state();
                        let timeline = crate::heading::self_healing_timeline(&offer.verdict, executed, relieved);
                        view! {
                            <div class="offer-item">
                                <span class="offer-line">
                                    <code>{offer.operation}</code>
                                    <small class="offer-target">{target}</small>
                                    <small class="offer-risk">{offer.risk}</small>
                                    <small class="offer-undo">{undo}</small>
                                    <small class="offer-verdict">{verdict_text(&offer.verdict)}</small>
                                </span>
                                <div class="self-healing-timeline">
                                    {timeline
                                        .into_iter()
                                        .map(|stage| {
                                            let class = if stage.completed {
                                                "stage-completed"
                                            } else if stage.active {
                                                "stage-active"
                                            } else {
                                                "stage-pending"
                                            };
                                            view! {
                                                <span class=format!("timeline-stage {class}")>
                                                    {stage.name}
                                                </span>
                                            }
                                        })
                                        .collect_view()}
                                </div>
                                <Show when=move || still_present>
                                    <div class="timeline-outcome-warning">"Remedy completed, but condition remains present"</div>
                                </Show>
                                <Show when=move || outcome_unestablished>
                                    <div class="timeline-outcome-warning">"Outcome could not be established independently"</div>
                                </Show>
                            </div>
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
