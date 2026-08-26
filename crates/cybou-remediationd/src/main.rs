// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! The organ that takes a finding as far as an outcome.
//!
//! Everything this needs existed and nothing ran it. `Telemetry1` finds things and the web gateway
//! shows them to a person; `Action1` proposes, criticises and decides; the executor carries out three
//! typed operations; `observe_outcome` concludes what actually happened. The only thing that had ever
//! joined them was an example written for a gate, so a host left to itself reached *explain* and
//! stopped. This is the join.
//!
//! ## It has no bus name, and that is the point
//!
//! It offers nothing to anybody. It reads what the telemetry organ concluded, asks the authorization
//! owner what may be done, hands an opaque permit to the executor, and reports back what happened. A
//! surface here would be a second place to ask about actions, and there already is one — `Action1`
//! owns the lifecycle and answers for it, which is why this reports its results there rather than
//! keeping them.
//!
//! It sits in the governance layer beside `Action1` for a structural reason rather than a tidy one: an
//! organ may read the layers above it and not the ones below, and this must read telemetry *and* call
//! authorization. Placed anywhere above governance it would be reaching downward, which is the rule
//! ADR-0029 exists to keep.
//!
//! ## What stops it doing something rash
//!
//! Four things, and none of them is this file being careful.
//!
//! It cannot choose an operation: the remedies for a finding are a closed table in
//! `cybou-remediation`, ordered least committal first, and it takes the first.
//!
//! It cannot authorize itself. Every proposal goes to `Action1`, which criticises it and applies the
//! operator's standing policy. On a host where nobody pre-authorized anything, this proposes and is
//! refused, every time — and the refusal is recorded, which is the useful part: an operator can read
//! what their host wanted to do and decide whether to let it.
//!
//! It cannot act twice on one finding, because [`initiative`] answers that and it asks.
//!
//! It cannot conclude success. What it carried out and what the host saw afterwards are gathered
//! separately, and the second comes from the telemetry organ, which did not carry the action out and
//! has no notion that one happened.

#[cfg(not(target_os = "linux"))]
compile_error!("cybou-remediationd drives Linux system maintenance");

use std::collections::HashMap;

use cybou_actiond::ActionRecord;
use cybou_fabric::{ACTION, EXECUTOR, TELEMETRY, decode, encode};
use cybou_protocol::action::ExecutionAttempt;
use cybou_protocol::telemetry::{SystemInsight, WatchedResource};
use cybou_remediation::{Initiative, Tried, initiative, observe_outcome, remedies_for, wait_for};
use time::OffsetDateTime;
use uuid::Uuid;

/// How often this looks at what the host has concluded about itself.
///
/// Slower than sampling on purpose. A finding is a conclusion drawn over a window, so asking more
/// often than one is formed would only be asking the same question again.
const CONSIDER_EVERY: std::time::Duration = std::time::Duration::from_secs(15);

/// What this remembers about a finding it has already taken somewhere.
struct Handled {
    /// What was carried out, and what was concluded about it once anybody looked.
    tried: Tried,
    /// What the host was concluding when the action was carried out.
    ///
    /// Kept because relief is a comparison. Without the earlier reading there is nothing to compare
    /// the later one against, and *the finding is gone* would be indistinguishable from *this host
    /// never thought it was there*.
    before: Vec<SystemInsight>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let session = zbus::Connection::session().await?;
    // Action1 and the executor meet on the system bus, because the executor is root and the two have
    // to authenticate to the same transport.
    let system = zbus::Connection::system().await?;
    println!("[cybou-remediationd] Watching what this host concludes about itself");

    let mut handled: HashMap<Uuid, Handled> = HashMap::new();
    loop {
        if let Err(why) = consider(&session, &system, &mut handled).await {
            // A pass that could not run is not a host that should stop watching itself. The reason
            // is said out loud; the next pass tries again.
            eprintln!("[cybou-remediationd] This pass did not complete: {why}");
        }
        tokio::time::sleep(CONSIDER_EVERY).await;
    }
}

/// One look at everything the host currently concludes about itself.
async fn consider(
    session: &zbus::Connection,
    system: &zbus::Connection,
    handled: &mut HashMap<Uuid, Handled>,
) -> Result<(), Box<dyn std::error::Error>> {
    let findings = insights(session).await?;
    let now = OffsetDateTime::now_utc();

    for finding in &findings {
        let known = handled.get(&finding.insight_id).map(|entry| &entry.tried);
        match initiative(finding, known) {
            Initiative::Act => {
                if let Some(handled_now) = attempt(system, finding, &findings).await? {
                    handled.insert(finding.insight_id, handled_now);
                }
            }
            waiting @ Initiative::Wait { .. } => {
                if wait_for(&waiting, now) == Some(time::Duration::ZERO) {
                    conclude(session, system, finding, handled, now).await?;
                }
            }
            // Nothing more to do about this one, and saying so once is enough. It stays remembered so
            // the same conclusion is not reached again out loud every fifteen seconds.
            Initiative::Leave { .. } => {}
        }
    }
    Ok(())
}

/// Propose what this finding calls for, and carry it out if the owner permits it.
///
/// Returns nothing when the proposal was refused, which on a host where nobody pre-authorized
/// anything is every time. That is the ordinary case and not a failure: the refusal is recorded by
/// `Action1`, and an operator reading it learns what their host wanted to do.
async fn attempt(
    system: &zbus::Connection,
    finding: &SystemInsight,
    findings: &[SystemInsight],
) -> Result<Option<Handled>, Box<dyn std::error::Error>> {
    // The least committal remedy the closed table offers for this finding. Choosing is not this
    // organ's to do: something arguing for its own proposal is the wrong party to rank the options.
    let Some(operation) = remedies_for(finding.finding).first().copied() else {
        return Ok(None);
    };

    let reply: (Vec<u8>, String) = system
        .call_method(
            Some(ACTION.service),
            ACTION.object_path,
            Some(ACTION.interface),
            "EvaluateInsight",
            &(encode(finding)?, operation.verb()),
        )
        .await?
        .body()
        .deserialize()?;

    // The decision is decoded so a malformed answer is a failure here rather than a silent success.
    // What it said is already recorded by its owner; nothing further is kept.
    let _decided: ActionRecord = decode(&reply.0)?;
    if reply.1.is_empty() {
        println!(
            "[cybou-remediationd] {} was not permitted for {}",
            operation.verb(),
            finding.finding.name()
        );
        return Ok(None);
    }

    println!(
        "[cybou-remediationd] Carrying out {} for {}",
        operation.verb(),
        finding.finding.name()
    );
    let encoded: Vec<u8> = system
        .call_method(
            Some(EXECUTOR.service),
            EXECUTOR.object_path,
            Some(EXECUTOR.interface),
            "Execute",
            &(reply.1,),
        )
        .await?
        .body()
        .deserialize()?;
    let attempt: ExecutionAttempt = decode(&encoded)?;

    // Reported to the owner of the lifecycle rather than kept here. What was authorized and what was
    // done belong in one record, or somebody has to correlate two afterwards.
    record_with(system, "RecordAttempt", &encode(&attempt)?).await?;
    Ok(Some(Handled {
        tried: Tried {
            attempt,
            outcome: None,
        },
        before: findings.to_vec(),
    }))
}

/// Look again, and conclude what the action actually did.
async fn conclude(
    session: &zbus::Connection,
    system: &zbus::Connection,
    finding: &SystemInsight,
    handled: &mut HashMap<Uuid, Handled>,
    now: OffsetDateTime,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(entry) = handled.get_mut(&finding.insight_id) else {
        return Ok(());
    };
    if entry.tried.outcome.is_some() {
        return Ok(());
    }

    // Asked of the telemetry organ, which did not carry the action out and has no notion that one
    // happened. An executor grading its own homework is refused everywhere else in this tree and
    // would be no better here.
    let after = insights(session).await?;
    let watched = watching(session).await?;

    let proposal = proposal_for(system, entry.tried.attempt.proposal_id).await?;
    let outcome = observe_outcome(
        &entry.tried.attempt,
        &proposal,
        Some(finding),
        &cybou_remediation::Reobservation {
            before: &entry.before,
            after: &after,
            watched_after: &watched,
            at: now,
        },
        Uuid::new_v4(),
    );

    println!(
        "[cybou-remediationd] {} for {}: {:?}",
        entry.tried.attempt.operation,
        finding.finding.name(),
        outcome.observed
    );
    record_with(system, "RecordOutcome", &encode(&outcome)?).await?;
    entry.tried.outcome = Some(outcome);
    Ok(())
}

/// What was proposed, asked of the owner that proposed it.
async fn proposal_for(
    system: &zbus::Connection,
    proposal_id: Uuid,
) -> Result<cybou_protocol::action::ActionProposal, Box<dyn std::error::Error>> {
    let encoded: Vec<u8> = system
        .call_method(
            Some(ACTION.service),
            ACTION.object_path,
            Some(ACTION.interface),
            "Record",
            &(proposal_id.to_string(),),
        )
        .await?
        .body()
        .deserialize()?;
    let record: ActionRecord = decode(&encoded)?;
    Ok(record.proposal)
}

async fn record_with(
    system: &zbus::Connection,
    method: &str,
    payload: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    system
        .call_method(
            Some(ACTION.service),
            ACTION.object_path,
            Some(ACTION.interface),
            method,
            &(payload.to_vec(),),
        )
        .await?;
    Ok(())
}

async fn insights(
    session: &zbus::Connection,
) -> Result<Vec<SystemInsight>, Box<dyn std::error::Error>> {
    let encoded: Vec<u8> = session
        .call_method(
            Some(TELEMETRY.service),
            TELEMETRY.object_path,
            Some(TELEMETRY.interface),
            "Insights",
            &(),
        )
        .await?
        .body()
        .deserialize()?;
    Ok(decode(&encoded)?)
}

async fn watching(
    session: &zbus::Connection,
) -> Result<Vec<WatchedResource>, Box<dyn std::error::Error>> {
    let encoded: Vec<u8> = session
        .call_method(
            Some(TELEMETRY.service),
            TELEMETRY.object_path,
            Some(TELEMETRY.interface),
            "Watching",
            &(),
        )
        .await?
        .body()
        .deserialize()?;
    Ok(decode(&encoded)?)
}
