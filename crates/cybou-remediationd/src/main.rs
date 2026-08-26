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

use std::collections::{HashMap, HashSet};

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
    /// The finding itself, kept rather than looked up again.
    ///
    /// Because the best outcome makes it disappear. A remedy that worked is a finding the telemetry
    /// organ stops reporting, so an episode concluded only from what is currently found would
    /// conclude every failure and never a success — the one result worth recording would be the one
    /// that never was.
    ///
    /// Absent for an episode rebuilt from `Action1` after a restart. The record holds what was
    /// proposed and decided; the finding that gave rise to it was the telemetry organ's and nobody
    /// wrote it here. Saying so is the point: an outcome concluded without it says less, and saying
    /// less is what this host is entitled to say.
    finding: Option<SystemInsight>,
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

    // What this host started and never finished, asked for before anything else happens. Its own
    // memory is gone; the owner's is not. Looking only at what the host currently concludes would
    // miss exactly the episodes that worked, because a remedy that succeeded makes its finding
    // disappear — so the successful ones would stay unfinished forever.
    let mut handled = adopt_unfinished(&system).await;
    if !handled.is_empty() {
        println!(
            "[cybou-remediationd] Taking over {} episode(s) this host began earlier",
            handled.len()
        );
    }
    // Findings this host may do nothing about. Remembered so the fact is stated once: a line every
    // fifteen seconds about a thing that will not change is how a log stops being read, and a log
    // nobody reads is where the interesting line hides.
    let mut nothing_permitted: HashSet<Uuid> = HashSet::new();
    loop {
        if let Err(why) = consider(&session, &system, &mut handled, &mut nothing_permitted).await {
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
    nothing_permitted: &mut HashSet<Uuid>,
) -> Result<(), Box<dyn std::error::Error>> {
    let findings = insights(session).await?;
    let now = OffsetDateTime::now_utc();

    // Unfinished episodes first, and from what this remembers rather than from what is currently
    // found. A remedy that worked makes its finding disappear, so concluding only about findings
    // still present would conclude every failure and never a success.
    let due: Vec<Uuid> = handled
        .iter()
        .filter(|(_, entry)| entry.tried.outcome.is_none())
        .filter(|(_, entry)| {
            wait_for(&initiative(entry.finding.as_ref(), Some(&entry.tried)), now)
                == Some(time::Duration::ZERO)
        })
        .map(|(insight_id, _)| *insight_id)
        .collect();
    for insight_id in due {
        conclude(session, system, insight_id, handled, now).await?;
    }

    // Then whatever the host currently concludes about itself.
    for finding in &findings {
        // Asked of the owner when this process has no memory of this finding, which is every finding
        // after a restart. The durable record is what survives; the map is a cache of it. Without
        // this the guarantee is only that it cannot act twice during one uninterrupted process, and
        // a crash must not be able to cause a second restart of a service.
        if !handled.contains_key(&finding.insight_id) {
            match episode_for(system, finding.insight_id).await {
                Ok(Some(already)) => {
                    println!(
                        "[cybou-remediationd] Remembering the episode already carried out for {}",
                        finding.finding.name()
                    );
                    handled.insert(finding.insight_id, already);
                }
                Ok(None) => {}
                Err(why) => {
                    // Failure to ask the owner is not evidence that nothing was tried. Acting on
                    // that answer would make a transient bus failure permission to repeat a
                    // mutation. Leave this finding alone for this pass and ask again next time.
                    eprintln!(
                        "[cybou-remediationd] Could not check earlier work for {}: {why}",
                        finding.finding.name()
                    );
                    continue;
                }
            }
        }

        // An episode adopted from the owner has no finding of its own; this is where it gets one, so
        // a problem that is still present can be concluded about properly rather than vaguely.
        if let Some(entry) = handled.get_mut(&finding.insight_id)
            && entry.finding.is_none()
        {
            entry.finding = Some(finding.clone());
        }

        let known = handled.get(&finding.insight_id).map(|entry| &entry.tried);
        if !matches!(initiative(Some(finding), known), Initiative::Act) {
            continue;
        }
        if nothing_permitted.contains(&finding.insight_id) {
            continue;
        }
        match attempt(system, finding, &findings).await? {
            Some(handled_now) => {
                handled.insert(finding.insight_id, handled_now);
            }
            None => {
                nothing_permitted.insert(finding.insight_id);
            }
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
    // Every remedy the closed table offers for this finding, in the operator's own order, skipping
    // anything that relieves nothing.
    //
    // The inspection is skipped because `relieves()` says plainly that reading a unit's state
    // relieves nothing. It leads the table so a person investigating is offered the option that
    // changes nothing; a host that proposed it and considered the finding handled would look at a
    // stopped service, learn that it is stopped, and repair nothing.
    //
    // Each is proposed until one is permitted, and that is not escalation past what anybody allowed:
    // `Action1` refuses every one the operator did not authorize, so walking the order means *the
    // gentlest thing this host is actually allowed to do*. Stopping at the first refusal instead
    // would make an authorization unusable unless everything gentler was authorized too — an
    // operator who permits a restart and nothing else would have granted something their host can
    // never reach.
    let remedies: Vec<_> = remedies_for(finding.finding)
        .into_iter()
        .filter(|operation| operation.relieves().contains(&finding.finding))
        .collect();

    for operation in remedies {
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

        // Decoded so a malformed answer is a failure here rather than a silent success. What it said
        // is already recorded by its owner; nothing further is kept.
        let _decided: ActionRecord = decode(&reply.0)?;
        if reply.1.is_empty() {
            continue;
        }
        return carry_out(system, finding, operation, &reply.1, findings).await;
    }

    // Nothing this host knows how to do is permitted here. Said once rather than every pass: the
    // caller remembers, because a line every fifteen seconds is how a log stops being read.
    println!(
        "[cybou-remediationd] Nothing permitted for {}",
        finding.finding.name()
    );
    Ok(None)
}

/// Hand the permit to the executor and report what came back.
async fn carry_out(
    system: &zbus::Connection,
    finding: &SystemInsight,
    operation: cybou_remediation::Operation,
    permit_id: &str,
    findings: &[SystemInsight],
) -> Result<Option<Handled>, Box<dyn std::error::Error>> {
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
            &(permit_id.to_owned(),),
        )
        .await?
        .body()
        .deserialize()?;
    let attempt: ExecutionAttempt = decode(&encoded)?;

    // Reported to the owner of the lifecycle rather than kept here. What was authorized and what was
    // done belong in one record, or somebody has to correlate two afterwards.
    record_with(system, "RecordAttempt", &encode(&attempt)?).await?;
    Ok(Some(Handled {
        finding: Some(finding.clone()),
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
    insight_id: Uuid,
    handled: &mut HashMap<Uuid, Handled>,
    now: OffsetDateTime,
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(entry) = handled.get_mut(&insight_id) else {
        return Ok(());
    };
    if entry.tried.outcome.is_some() {
        return Ok(());
    }
    let finding = entry.finding.clone();

    // Asked of the telemetry organ, which did not carry the action out and has no notion that one
    // happened. An executor grading its own homework is refused everywhere else in this tree and
    // would be no better here.
    let after = insights(session).await?;
    let watched = watching(session).await?;

    let proposal = proposal_for(system, entry.tried.attempt.proposal_id).await?;
    let outcome = observe_outcome(
        &entry.tried.attempt,
        &proposal,
        finding.as_ref(),
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
        entry.tried.attempt.operation, entry.tried.attempt.target_resource, outcome.observed
    );
    record_with(system, "RecordOutcome", &encode(&outcome)?).await?;
    entry.tried.outcome = Some(outcome);
    Ok(())
}

/// Take over every episode this host carried out and never concluded.
///
/// Asked for at startup, and it is the half a per-finding lookup cannot cover: that one only fires
/// for findings still being reported, and a remedy that *worked* makes its finding disappear. So
/// without this the successful episodes are exactly the ones left unfinished forever, which is the
/// same defect as concluding only about current findings, one level up.
///
/// Empty when the owner cannot be asked. Nothing is adopted and nothing is acted on either: a finding
/// still present is checked against the owner before anything is proposed about it.
async fn adopt_unfinished(system: &zbus::Connection) -> HashMap<Uuid, Handled> {
    let Ok(reply) = system
        .call_method(
            Some(ACTION.service),
            ACTION.object_path,
            Some(ACTION.interface),
            "UnfinishedEpisodes",
            &(),
        )
        .await
    else {
        eprintln!("[cybou-remediationd] The owner could not be asked what was left unfinished");
        return HashMap::new();
    };
    let Ok(encoded) = reply.body().deserialize::<Vec<u8>>() else {
        return HashMap::new();
    };
    let Ok(records) = decode::<Vec<ActionRecord>>(&encoded) else {
        return HashMap::new();
    };

    records
        .into_iter()
        .filter_map(|record| {
            let cause = record.proposal.cause_id?;
            let attempt = record.attempt?;
            Some((
                cause,
                Handled {
                    finding: None,
                    tried: Tried {
                        attempt,
                        outcome: record.outcome,
                    },
                    before: Vec::new(),
                },
            ))
        })
        .collect()
}

/// What this host already did about one finding, asked of the owner that would know.
///
/// Absence and failure are different answers. Absence means this cause is new and may proceed to the
/// initiative decision. Failure means the owner could not answer, which is never grounds to act: a
/// driver must not turn a transient bus failure into permission to repeat a mutation.
async fn episode_for(
    system: &zbus::Connection,
    cause_id: Uuid,
) -> Result<Option<Handled>, Box<dyn std::error::Error>> {
    let encoded: Vec<u8> = system
        .call_method(
            Some(ACTION.service),
            ACTION.object_path,
            Some(ACTION.interface),
            "EpisodeForCause",
            &(cause_id.to_string(),),
        )
        .await?
        .body()
        .deserialize()?;
    let Some(record) = decode::<Option<ActionRecord>>(&encoded)? else {
        return Ok(None);
    };
    let Some(attempt) = record.attempt else {
        return Ok(None);
    };

    Ok(Some(Handled {
        // Not invented from the record: it holds what was proposed and decided, and the finding was
        // the telemetry organ's. The caller fills it in when the problem is still being reported.
        finding: None,
        tried: Tried {
            attempt,
            outcome: record.outcome,
        },
        before: Vec::new(),
    }))
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
    // Plain CBOR, not a fabric envelope. The telemetry organ answers in the shape it has always
    // answered in, and the web gateway reads it the same way; decoding it as an envelope here was
    // this organ assuming a convention rather than reading the one in use.
    Ok(ciborium::from_reader(encoded.as_slice())?)
}

/// What the telemetry organ could see when it last looked.
///
/// Read as a property rather than called as a method, because that is what it is. Guessing the shape
/// of somebody else's surface from the name of a Rust function is how the first version of this
/// failed every pass, silently, while still repairing the host.
///
/// It is needed because a finding can vanish for two opposite reasons — the condition cleared, or
/// nothing could read the thing it was about — and an outcome concluded without knowing which would
/// report a host that went blind as a host that was repaired.
async fn watching(
    session: &zbus::Connection,
) -> Result<Vec<WatchedResource>, Box<dyn std::error::Error>> {
    let value: zbus::zvariant::OwnedValue = session
        .call_method(
            Some(TELEMETRY.service),
            TELEMETRY.object_path,
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            &(TELEMETRY.interface, "Watching"),
        )
        .await?
        .body()
        .deserialize()?;
    let encoded: Vec<u8> = value.try_into()?;
    Ok(ciborium::from_reader(encoded.as_slice())?)
}
