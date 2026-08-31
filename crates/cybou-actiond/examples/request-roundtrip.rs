// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Live gate client for the door a person walks through.
//!
//! The two gates beside this one drive proposals Mind made from its own findings: one where a
//! standing policy pre-authorized the operation, one where nobody had and a person answered the
//! question that followed. Both start from something this host concluded about itself.
//!
//! This one starts from a person. Nothing observes anything, no finding is written, and there is no
//! question to answer — ADR-0048 says the asking is the confirmation — so what this proves is the
//! whole of the other entrance: request → permit → restart → an independent observation that it
//! happened, with what is forbidden still forbidden and the permit still single-use.

use cybou_fabric::{ACTION, EXECUTOR, decode};
use cybou_protocol::action::{ActionRecord, AttemptReport, AuthorizationVerdict, ExecutionAttempt};
use zbus::Proxy;

const UNIT: &str = "cybou-request-gate.service";

/// The seat a deployment would take from whatever authenticated the person.
///
/// Supplied by the caller here because this gate stands in for the gateway, which is the only party
/// that can establish it.
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

    let target = format!("systemd:{UNIT}");

    // ------------------------------------------------------- what is forbidden stays forbidden
    // First, because a gate that proved the happy path and then asked this would be reporting that
    // the door works before checking that it is a door rather than a hole.
    for forbidden in [
        "filesystem.format",
        "system.poweroff",
        "service.data.delete",
    ] {
        let (encoded, permit): (Vec<u8>, String) = action
            .call(
                "Request",
                &(forbidden.to_owned(), target.clone(), SEAT.to_owned()),
            )
            .await?;
        let refused: ActionRecord = decode(&encoded)?;
        if !permit.is_empty() {
            return Err(format!("{forbidden} produced a permit").into());
        }
        if !matches!(
            refused.decision.verdict,
            AuthorizationVerdict::Denied { .. }
        ) {
            return Err(format!(
                "{forbidden} was not refused: {:?}",
                refused.decision.verdict
            )
            .into());
        }
    }

    // A verb outside the table is not an operation with unknown risk; it is not an operation, and
    // Action1 refuses the call rather than deciding about it.
    if action
        .call::<_, _, (Vec<u8>, String)>(
            "Request",
            &(
                "service.obliterate".to_owned(),
                target.clone(),
                SEAT.to_owned(),
            ),
        )
        .await
        .is_ok()
    {
        return Err("a verb this build cannot express was accepted".into());
    }

    // --------------------------------------------------------------------- a person asks
    let (encoded, permit_id): (Vec<u8>, String) = action
        .call(
            "Request",
            &(
                "service.restart".to_owned(),
                target.clone(),
                SEAT.to_owned(),
            ),
        )
        .await?;
    let record: ActionRecord = decode(&encoded)?;

    if permit_id.is_empty() {
        return Err("a request this build can carry out produced no permit".into());
    }

    // The asking is the confirmation, and the record says who asked. `Granted` here would attribute
    // a person's decision to a standing policy, on a host whose policy authorized nothing.
    match &record.decision.verdict {
        AuthorizationVerdict::GrantedOnConfirmation { confirmed_by } if confirmed_by == SEAT => {}
        other => return Err(format!("expected a grant naming its seat, got {other:?}").into()),
    }
    if !record.proposal.proposed_by.is_a_person() {
        return Err("the proposal does not say a person made it".into());
    }
    if record.proposal.proposed_by.brings_its_own_evidence() {
        return Err("a person's request claims to bring its own evidence".into());
    }
    if record.proposal.cause_id.is_some() {
        // A cause here would be this host claiming it had concluded something it did not.
        return Err("a person's request cites a finding nobody made".into());
    }

    // ------------------------------------------------------------------ and it is carried out
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

    // One permit, once.
    if executor
        .call::<_, _, Vec<u8>>("Execute", &(permit_id))
        .await
        .is_ok()
    {
        return Err("single-use permit was replayed".into());
    }

    println!("{SEAT} asked → permit → restart → independent active observation");
    Ok(())
}
