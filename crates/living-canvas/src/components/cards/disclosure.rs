// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Disclosure card: what this reader was supplied, and what was kept from them.
//!
//! Every other card on this desktop answers "what does the system hold?". This one answers "what
//! was done to me?", which is a question no assembled-context system usually lets its subject ask.
//!
//! Two things are shown that a summary would hide. The gap between what was supplied and what can
//! be accounted for, because a projection that lost track of a row's provenance must say so rather
//! than quietly report the smaller number. And the refusals, with their reasons — an item dropped
//! for policy and an item that was never relevant look identical unless the surface insists on the
//! difference.

use cybou_web_contracts::DisclosureProjection;
use leptos::prelude::*;
use lucide_leptos::EyeOff;
use std::sync::Arc;

use crate::{
    CardId, DesktopLayout,
    components::card_frame::CardFrame,
    interaction::{DragState, ResizeState},
    state::RuntimeState,
};

/// The disclosure this reader's session carries, if the gateway could be asked.
fn disclosure_of(runtime: RwSignal<RuntimeState>) -> Option<DisclosureProjection> {
    match runtime.get() {
        RuntimeState::Ready { disclosure, .. } => disclosure,
        RuntimeState::Loading | RuntimeState::Error(_) | RuntimeState::SignInRequired => None,
    }
}

/// One line saying what this consumer stands in relation to the record.
///
/// Three states, deliberately distinguished: the surface could not be asked, nothing has been
/// supplied yet, and something was supplied. Collapsing the first two would let "not asked" read
/// as "nothing happened".
fn headline(disclosure: Option<&DisclosureProjection>) -> String {
    match disclosure {
        None => "Disclosure record not read".to_owned(),
        Some(record) if !record.delivered => "Nothing supplied to you yet".to_owned(),
        Some(record) => match record.supplied {
            0 => "An empty delivery was recorded".to_owned(),
            1 => "1 item supplied to you".to_owned(),
            count => format!("{count} items supplied to you"),
        },
    }
}

/// What a reason code means in the words a person would use to ask about it.
fn reason_text(because: &str) -> &'static str {
    match because {
        "aboveConsumerTrust" => "above your trust",
        "belongsToThePerson" => "yours by construction",
        // A reason this build cannot name is shown as unnameable rather than guessed at, for the
        // same reason the contract refuses to default it.
        _ => "reason not recognised",
    }
}

/// Disclosure domain content presentation.
#[component]
pub fn DisclosureContent(runtime: RwSignal<RuntimeState>) -> impl IntoView {
    let record = move || disclosure_of(runtime);
    let label = move || headline(record().as_ref());

    let provenance = move || {
        let Some(record) = record() else {
            return String::new();
        };
        if !record.delivered {
            return String::new();
        }
        let unaccounted = record.supplied.saturating_sub(record.accounted_for);
        if unaccounted == 0 {
            format!(
                "all of them name where they came from, across {} contributions",
                record.provenance_count
            )
        } else {
            // The whole reason two numbers are carried. Reporting only the accounted-for ones
            // would be claiming provenance the delivery does not have.
            format!(
                "{unaccounted} of them cannot say where they came from; the rest name {} contributions",
                record.provenance_count
            )
        }
    };

    let withheld = move || record().map_or_else(Vec::new, |record| record.withheld);
    let subjects_visible = move || record().is_some_and(|record| record.subjects_visible);
    // Said outright rather than left to be inferred from a list of blanks. "No subject could be
    // named" and "you are not the person this record is about" are different facts, and a reader
    // who cannot tell them apart learns the wrong thing from the same screen.
    let subject_notice = move || match record() {
        Some(record) if !record.subjects_visible && !record.withheld.is_empty() => {
            "Subjects are shown to the person this record is about. Sign in to see them."
        }
        _ => "",
    };
    let withheld_label = move || match record() {
        None => String::new(),
        Some(record) => match record.withheld.len() {
            0 => "Nothing was held back".to_owned(),
            1 => "1 item held back".to_owned(),
            count => format!("{count} items held back"),
        },
    };

    let consumer = move || {
        record()
            .map(|record| record.consumer_id)
            .unwrap_or_default()
    };

    // Indexed so two refusals with the same subject and reason stay two rows. A key that collapsed
    // them would report one refusal where two happened.
    let withheld_rows = move || withheld().into_iter().enumerate().collect::<Vec<_>>();

    view! {
        <div class="disclosure-card-body">
            <strong>{label}</strong>
            <span class="disclosure-provenance">{provenance}</span>
            <span class="disclosure-consumer">{consumer}</span>

            <strong class="disclosure-withheld-label">{withheld_label}</strong>
            <span class="disclosure-subject-notice">{subject_notice}</span>
            <div class="withheld-list">
                <For
                    each=withheld_rows
                    key=|(index, item)| format!("{index}:{}:{:?}", item.because, item.subject)
                    children=move |(_, item)| {
                        // A refusal whose subject is absent is still listed. An unnamed refusal
                        // is a smaller loss than a silent one, and the two ways a subject can be
                        // absent are distinguished rather than shown as the same blank.
                        let named = item.subject.is_some();
                        let subject = item.subject.unwrap_or_else(|| {
                            if subjects_visible() {
                                "unnamed".to_owned()
                            } else {
                                "withheld".to_owned()
                            }
                        });
                        let subject_class = if named { "" } else { "withheld-unnamed" };
                        view! {
                            <span class="withheld-line">
                                <b class=subject_class>{subject}</b>
                                <small>{reason_text(&item.because)}</small>
                            </span>
                        }
                    }
                />
            </div>
            <span class="panel-link">"Delivery is not permission"</span>
        </div>
    }
}

/// Disclosure cognitive card component.
#[component]
pub fn DisclosureCard(
    layout: RwSignal<DesktopLayout>,
    selected: ReadSignal<&'static str>,
    set_selected: WriteSignal<&'static str>,
    dragging: RwSignal<Option<DragState>>,
    resizing: RwSignal<Option<ResizeState>>,
    runtime: RwSignal<RuntimeState>,
) -> impl IntoView {
    let collapsed = move || {
        let record = disclosure_of(runtime);
        let label = headline(record.as_ref());
        let held = record.map_or(0, |record| record.withheld.len());
        view! {
            <div class="card-collapsed-summary">
                <b>"Disclosure"</b>
                <span>{label}</span>
                <span>{if held == 0 { String::new() } else { format!("{held} held back") }}</span>
            </div>
        }
        .into_any()
    };

    view! {
        <CardFrame
            card=CardId::Disclosure
            layout=layout
            selected=selected
            set_selected=set_selected
            dragging=dragging
            resizing=resizing
            kicker_title="ContextDisclosed"
            kicker_icon=Arc::new(|| view! { <EyeOff size=14 /> }.into_any())
            collapsed_summary=Arc::new(collapsed)
        >
            <DisclosureContent runtime=runtime />
        </CardFrame>
    }
}
