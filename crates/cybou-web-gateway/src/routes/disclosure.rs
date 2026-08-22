// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! What this reader was supplied, and what was kept from them (ADR-0030 B1, B6).
//!
//! The deliveries were already recorded: every supply of the Mind projection across a boundary
//! writes a `ContextDisclosed` naming the consumer, the contributions the supplied items came
//! from, and what was held back and why. Until this route existed those records were in the
//! Journal and only a developer with `busctl` could read them, which makes the transparency real
//! for the wrong person.
//!
//! The scope is the Mind projection and not everything the gateway serves, because that is where
//! context about the person is. The snapshot and the event stream carry capability states, which
//! are facts about whether an organ is answering — nothing about them is the person's to release,
//! so filtering them would record a decision nobody made.
//!
//! What the route adds is not the record but the *gap*: how much was supplied against how much of
//! it can be accounted for, and what was refused against the reason. Every system that assembles
//! context does this bookkeeping internally. What none of them do is let the person it is about
//! read it.

use axum::{Json, extract::State, http::HeaderMap};
use cybou_protocol::disclosure::{ConsumerTrust, WithheldBecause};
use cybou_web_contracts::{
    DISCLOSURE_ITEM_SAMPLE, DisclosureProjection, WEB_SCHEMA_V1, WithheldProjection,
};

use crate::state::GatewayState;

/// Name a reason in the frozen vocabulary, rather than in prose assembled here.
///
/// A closed set stays closed across the boundary: a reader that could not distinguish "more
/// exposing than your trust permits" from "about the person by construction" would be reading two
/// different decisions as one.
const fn reason(because: WithheldBecause) -> &'static str {
    match because {
        WithheldBecause::AboveConsumerTrust => "aboveConsumerTrust",
        WithheldBecause::BelongsToThePerson => "belongsToThePerson",
    }
}

/// Return what this consumer was last supplied, and what was withheld from them.
///
/// Answers for the caller and nobody else. The record is keyed by consumer, so a reader sees their
/// own deliveries: this is a window onto what was done to you, not a log of what was done to
/// everyone.
pub async fn disclosure_handler(
    State(state): State<GatewayState>,
    headers: HeaderMap,
) -> Json<DisclosureProjection> {
    let session = state.session_for(&headers);
    let destination = GatewayState::destination_for(session.as_ref());
    let last = state.disclosures.last_for(&destination.id);

    // No delivery is not an empty delivery. On a gateway nobody has read from yet, the honest
    // answer is that nothing has been supplied — not that nothing was.
    let delivered = last.is_some();
    let last = last.unwrap_or_default();

    // A stranger is told how much was refused and on what grounds, never what it was about. The
    // subject of a refused concept is that concept's label, so naming it here to explain the
    // refusal would publish exactly what the refusal withheld — the surface that reports a filter
    // must not be a way around it.
    let subjects_visible = destination.trust == ConsumerTrust::Owner;

    Json(DisclosureProjection {
        schema_version: WEB_SCHEMA_V1,
        consumer_id: destination.id,
        external_boundary: destination.external_boundary,
        retains: destination.retains,
        supplied: last.item_count,
        // Not the length of `items`: that is a set of source contributions on a different scale,
        // and reading it as a count of accounted-for items reported more accounted for than were
        // supplied on the first live deployment of this surface.
        accounted_for: last.accounted_for,
        provenance_count: u32::try_from(last.items.len()).unwrap_or(u32::MAX),
        items: last
            .items
            .into_iter()
            .take(DISCLOSURE_ITEM_SAMPLE)
            .collect(),
        withheld: last
            .withheld
            .into_iter()
            .map(|withheld| WithheldProjection {
                subject: if subjects_visible {
                    withheld.subject
                } else {
                    None
                },
                because: reason(withheld.because).to_owned(),
            })
            .collect(),
        subjects_visible,
        delivered,
    })
}
