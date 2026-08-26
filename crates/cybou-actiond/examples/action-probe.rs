// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Drive a running `Action1` from outside it, for the durability gate.
//!
//! Exists so that gate tests this crate's own lifecycle rather than a D-Bus exchange written out by
//! hand in a shell script. A gate against a hand-written exchange tests the shell script, and passes
//! forever after the code stops agreeing with it.
//!
//! Three things, because the claim needs three: decide one action, count what reached the Journal,
//! and read back what the owner still knows.

use std::error::Error;

use cybou_actiond::ActionRecord;
use cybou_fabric::{ACTION, decode, encode};
use cybou_protocol::action::AuthorizationVerdict;
use cybou_protocol::telemetry::{EvidenceStrength, Finding, MetricKey, Subject, SystemInsight};
use time::OffsetDateTime;
use uuid::Uuid;

const UNIT: &str = "cybou-durability-gate.service";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = std::env::args().skip(1);
    match arguments.next().as_deref() {
        Some("decide") => decide().await,
        Some("journal") => journal().await,
        Some("record") => {
            let proposal = arguments.next().ok_or("record needs a proposal id")?;
            record(&proposal).await
        }
        _ => Err("usage: action-probe decide | journal | record <proposal-id>".into()),
    }
}

/// One finding, put in the Journal, then criticised and decided.
///
/// The finding goes in first because a proposal cites it, and a contribution may only cite something
/// that exists. Standing in for the organ that observes findings: this probe is not entitled to
/// record one in a running system, and does it here only so the gate has a cause to point at.
async fn decide() -> Result<(), Box<dyn Error>> {
    let insight = SystemInsight {
        insight_id: Uuid::new_v4(),
        finding: Finding::ServiceFailure,
        about: Some(MetricKey::named(Subject::ServiceActive, UNIT.to_owned())),
        because: Vec::new(),
        strength: EvidenceStrength::Strong,
        concluded_at: OffsetDateTime::now_utc(),
        since: OffsetDateTime::now_utc(),
    };

    observe(&insight).await?;

    let reply: (Vec<u8>, String) = zbus::Connection::session()
        .await?
        .call_method(
            Some(ACTION.service),
            ACTION.object_path,
            Some(ACTION.interface),
            "EvaluateInsight",
            &(encode(&insight)?, "service.restart"),
        )
        .await?
        .body()
        .deserialize()?;

    let record: ActionRecord = decode(&reply.0)?;
    if record.decision.verdict != AuthorizationVerdict::Granted {
        return Err(format!("the standing policy refused: {:?}", record.decision.verdict).into());
    }
    println!("{}", record.proposal.proposal_id);
    Ok(())
}

/// Put the finding in the Journal under its own identity, so a proposal can cite it.
///
/// An `Observation`, which is one of the two kinds that record something that happened outside the
/// Journal and therefore cite nothing themselves. Everything else, a proposal included, is derived
/// and must point at something already there.
async fn observe(insight: &SystemInsight) -> Result<(), Box<dyn Error>> {
    let mut payload = Vec::new();
    ciborium::into_writer(insight, &mut payload)?;
    let envelope = cybou_protocol::canonical::CanonicalEnvelope {
        schema_version: 3,
        message_id: insight.insight_id,
        correlation_id: insight.insight_id,
        causation_id: Uuid::nil(),
        origin_organ: "telemetryd".to_owned(),
        origin_node: String::new(),
        kind: cybou_protocol::admission::Kind::Observation as u16,
        wall_time_ms: i64::try_from(OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000)
            .unwrap_or(i64::MAX),
        monotonic_time: 0,
        logical_clock: 1,
        confidence: 1.0,
        evidence: Vec::new(),
        payload,
        privacy: 1,
        capability_scope: String::new(),
        sealed: false,
        key_domain_id: Uuid::nil(),
        key_epoch: 0,
        retention_class: 2,
        retention_policy_version: 0,
        retain_until_ms: 0,
        sensitivity: 0,
    };
    // A refusal comes back as an error rather than a field: the client already tells "the Journal
    // would not take this" apart from "the Journal could not be reached".
    cybou_fabric::event_client::EventClient::session()
        .await?
        .submit(&envelope)
        .await?;
    Ok(())
}

/// How many contributions this organ has put in the Journal.
///
/// Counted by asking Event1 rather than by reading a file, because the Journal's own owner is the
/// only thing entitled to say what is in it.
async fn journal() -> Result<(), Box<dyn Error>> {
    let client = cybou_fabric::event_client::EventClient::session().await?;
    let mut counted = 0_usize;
    let mut after = 0_u64;
    loop {
        let page = client.replay(after, 256).await?;
        if page.is_empty() {
            break;
        }
        after += page.len() as u64;
        let complete = page.len() < 256;
        counted += page
            .iter()
            .filter(|envelope| envelope.origin_organ == "actiond")
            .count();
        if complete {
            break;
        }
    }
    println!("{counted}");
    Ok(())
}

/// What the owner still knows about one proposal, in the few facts the gate judges.
async fn record(proposal: &str) -> Result<(), Box<dyn Error>> {
    let encoded: Vec<u8> = zbus::Connection::session()
        .await?
        .call_method(
            Some(ACTION.service),
            ACTION.object_path,
            Some(ACTION.interface),
            "Record",
            &(proposal,),
        )
        .await?
        .body()
        .deserialize()?;

    let record: ActionRecord = decode(&encoded)?;
    println!(
        "{}",
        serde_json::json!({
            "verdict": format!("{:?}", record.decision.verdict).to_lowercase(),
            "operation": record.proposal.operation,
            "target": record.proposal.target_resource,
            "checks": record.checks.len(),
            "permitId": record.permit_id.map(|id| id.to_string()),
        })
    );
    Ok(())
}
