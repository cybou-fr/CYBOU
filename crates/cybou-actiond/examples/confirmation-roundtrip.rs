// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Live gate client for the path a host takes when nobody pre-authorized anything.
//!
//! The A1 gate beside this one proves the pre-authorized path: an operator granted
//! `service.restart` in advance, so a finding becomes a permit with nobody present. That is the
//! path a standing policy opens, and it is not the path a fresh installation is on. With no
//! standing policy — the default, and the only state an installation has until somebody changes it
//! — every proposal decides to `RequiresUserConfirmation`, and until now that was where the host
//! stopped.
//!
//! This drives the other one end to end: finding → a proposal waiting on a person → a person's
//! answer → permit → restart → an independent observation that it happened, with the answer
//! refused a second time and the permit refused a second time.

use cybou_fabric::{ACTION, EXECUTOR, decode, encode};
use cybou_protocol::{
    action::{ActionRecord, AttemptReport, AuthorizationVerdict, ExecutionAttempt},
    telemetry::{EvidenceStrength, Finding, InsightEvidence, MetricKey, Subject, SystemInsight},
};
use std::error::Error;
use time::OffsetDateTime;
use uuid::Uuid;
use zbus::Proxy;

const UNIT: &str = "cybou-confirmation-gate.service";

/// The seat a real deployment would take from whatever authenticated the person.
///
/// Supplied by the caller here because this gate stands in for the gateway, which is the only
/// party that can establish it. What matters for the gate is that it arrives, is recorded, and is
/// not something the proposal could have named about itself.
const SEAT: &str = "gate-operator";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let session = if std::env::var_os("CYBOU_ACTION_SYSTEM_BUS").is_some() {
        zbus::Connection::system().await?
    } else {
        zbus::Connection::session().await?
    };
    let action = Proxy::new(
        &session,
        ACTION.service,
        ACTION.object_path,
        ACTION.interface,
    )
    .await?;

    let since = OffsetDateTime::now_utc();
    let key = MetricKey::named(Subject::ServiceActive, UNIT.to_owned());
    let insight = SystemInsight {
        insight_id: Uuid::new_v4(),
        finding: Finding::ServiceFailure,
        about: Some(key.clone()),
        // A finding has to be able to say why. The Journal will not hold an inference that cites
        // nothing, and a proposal confirmed against one would be a person agreeing to a diagnosis
        // with no reading behind it.
        because: vec![InsightEvidence {
            key,
            observed: 0.0,
            deviation: None,
        }],
        strength: EvidenceStrength::Strong,
        concluded_at: since,
        since,
    };

    // The finding goes in first because a proposal cites it, and a contribution may only cite
    // something the Journal already holds. Written through `cybou_telemetryd::journal`, which is
    // what the organ that observes findings uses, so this is the shape a real one arrives in.
    observe(&insight).await?;

    // ---------------------------------------------------------------- it stops and asks
    let (encoded, permit_id): (Vec<u8>, String) = action
        .call(
            "EvaluateInsight",
            &(encode(&insight)?, "service.restart".to_owned()),
        )
        .await?;
    let record: ActionRecord = decode(&encoded)?;

    if !permit_id.is_empty() {
        return Err(
            "a host with nothing pre-authorized minted a permit without being asked".into(),
        );
    }
    let AuthorizationVerdict::RequiresUserConfirmation { prompt } = &record.decision.verdict else {
        return Err(format!(
            "expected a proposal waiting on a person, got {:?}",
            record.decision.verdict
        )
        .into());
    };
    if prompt.trim().is_empty() {
        return Err("the host asked a question with nothing in it".into());
    }
    let proposal_id = record.proposal.proposal_id;
    let asked = record.decision.decision_id;

    // ------------------------------------------------- agreeing to a prompt nobody showed
    if action
        .call::<_, _, (Vec<u8>, String)>(
            "Confirm",
            &(
                proposal_id.to_string(),
                Uuid::new_v4().to_string(),
                SEAT.to_owned(),
            ),
        )
        .await
        .is_ok()
    {
        return Err("a confirmation naming a decision nobody showed was accepted".into());
    }

    // ------------------------------------------------------------------ a person says yes
    let (encoded, permit_id): (Vec<u8>, String) = action
        .call(
            "Confirm",
            &(proposal_id.to_string(), asked.to_string(), SEAT.to_owned()),
        )
        .await?;
    let confirmed: ActionRecord = decode(&encoded)?;
    if permit_id.is_empty() {
        return Err("a confirmed proposal produced no permit".into());
    }

    // The record must say a person authorized this, not that a policy did. On this host the policy
    // authorized nothing, so a record reading `granted` would be attributing the decision to a
    // grant that does not exist.
    match &confirmed.decision.verdict {
        AuthorizationVerdict::GrantedOnConfirmation { confirmed_by } if confirmed_by == SEAT => {}
        other => {
            return Err(
                format!("expected a confirmed grant naming its seat, got {other:?}").into(),
            );
        }
    }
    if confirmed.decision.decision_id == asked {
        return Err("the answer and the question were recorded as one decision".into());
    }

    // --------------------------------------------------------- the same yes cannot be reused
    if action
        .call::<_, _, (Vec<u8>, String)>(
            "Confirm",
            &(proposal_id.to_string(), asked.to_string(), SEAT.to_owned()),
        )
        .await
        .is_ok()
    {
        return Err("one agreement minted a second permit".into());
    }

    // -------------------------------------------------------------------- and it is carried out
    let executor_connection = if std::env::var_os("CYBOU_EXECUTOR_SYSTEM_BUS").is_some() {
        zbus::Connection::system().await?
    } else {
        session.clone()
    };
    let executor = Proxy::new(
        &executor_connection,
        EXECUTOR.service,
        EXECUTOR.object_path,
        EXECUTOR.interface,
    )
    .await?;
    let encoded: Vec<u8> = executor.call("Execute", &(permit_id.clone())).await?;
    let attempt: ExecutionAttempt = decode(&encoded)?;
    if !matches!(attempt.report, AttemptReport::Completed) {
        return Err(format!("executor did not complete: {:?}", attempt.report).into());
    }

    // Re-observation does not ask the executor whether it worked. It asks systemd again over a
    // separate connection, which is the disagreement ADR-0022 requires the architecture to allow.
    let system = zbus::Connection::system().await?;
    let manager = Proxy::new(
        &system,
        "org.freedesktop.systemd1",
        "/org/freedesktop/systemd1",
        "org.freedesktop.systemd1.Manager",
    )
    .await?;
    let path: zbus::zvariant::OwnedObjectPath = manager.call("GetUnit", &(UNIT)).await?;
    let unit = Proxy::new(
        &system,
        "org.freedesktop.systemd1",
        path,
        "org.freedesktop.systemd1.Unit",
    )
    .await?;
    let mut active = String::new();
    for _ in 0..50 {
        active = unit.get_property("ActiveState").await?;
        if active == "active" {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    if active != "active" {
        return Err(format!("independent re-observation says {UNIT} is {active}").into());
    }

    // The capability was consumed before the adapter ran. Replaying it must fail at Action1.
    if executor
        .call::<_, _, Vec<u8>>("Execute", &(permit_id))
        .await
        .is_ok()
    {
        return Err("single-use permit was replayed".into());
    }

    println!(
        "finding → question → answer by {SEAT} → permit → restart → independent active observation"
    );
    Ok(())
}

/// Put the finding in the Journal, together with the readings it rests on.
///
/// The readings enter as `Observation`s — the two kinds that may cite nothing — and the finding as
/// a `Hypothesis` citing them, because what was observed is the readings and that they add up to a
/// failure is an inference.
async fn observe(insight: &SystemInsight) -> Result<(), Box<dyn Error>> {
    let client = cybou_fabric::event_client::EventClient::session().await?;
    for envelope in cybou_telemetryd::journal::contributions(insight, OffsetDateTime::now_utc())? {
        client.submit(&envelope).await?;
    }
    Ok(())
}
