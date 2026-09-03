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
//! whole of the other entrance: request → permit → the thing itself → an independent observation
//! that it happened, with what is forbidden still forbidden and the permit still single-use.
//!
//! It walks restart, stop, start and reload rather than restart alone, because those four differ by
//! one word to systemd and a dispatch that sent two of them to the same adapter would still pass a
//! gate that only ever asked for one. Stop and start are checked against the state systemd reports
//! afterwards, so a Stop button that restarts fails here rather than in front of somebody.

use cybou_fabric::{ACTION, EXECUTOR, decode};
use cybou_protocol::action::{ActionRecord, AttemptReport, AuthorizationVerdict, ExecutionAttempt};
use zbus::Proxy;

const UNIT: &str = "cybou-request-gate.service";

/// The seat a deployment would take from whatever authenticated the person.
///
/// Supplied by the caller here because this gate stands in for the gateway, which is the only party
/// that can establish it.
const SEAT: &str = "gate-operator";

/// Ask for one operation as a person, and carry out the permit it produces.
///
/// Returns nothing: what the operation did is established afterwards by asking systemd, not by
/// believing the executor's own account of itself.
async fn asked_and_done(
    action: &Proxy<'_>,
    executor: &Proxy<'_>,
    verb: &str,
    target: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let (encoded, permit_id): (Vec<u8>, String) = action
        .call(
            "Request",
            &(verb.to_owned(), target.to_owned(), SEAT.to_owned()),
        )
        .await?;
    let record: ActionRecord = decode(&encoded)?;
    if permit_id.is_empty() {
        return Err(format!("{verb} produced no permit").into());
    }
    // The asking is the confirmation, and the record says who asked. `Granted` here would attribute
    // a person's decision to a standing policy, on a host whose policy authorized nothing.
    match &record.decision.verdict {
        AuthorizationVerdict::GrantedOnConfirmation { confirmed_by } if confirmed_by == SEAT => {}
        other => {
            return Err(format!("{verb}: expected a grant naming its seat, got {other:?}").into());
        }
    }
    if !record.proposal.proposed_by.is_a_person() {
        return Err(format!("{verb}: the proposal does not say a person made it").into());
    }
    if record.proposal.proposed_by.brings_its_own_evidence() {
        return Err(format!("{verb}: a person's request claims to bring its own evidence").into());
    }
    if record.proposal.cause_id.is_some() {
        // A cause here would be this host claiming it had concluded something it did not.
        return Err(format!("{verb}: a person's request cites a finding nobody made").into());
    }

    let encoded: Vec<u8> = executor.call("Execute", &(permit_id.clone())).await?;
    let attempt: ExecutionAttempt = decode(&encoded)?;
    if !matches!(attempt.report, AttemptReport::Completed) {
        return Err(format!("{verb}: executor did not complete: {:?}", attempt.report).into());
    }
    if attempt.operation != verb {
        return Err(format!("{verb}: the attempt reports {}", attempt.operation).into());
    }

    // One permit, once.
    if executor
        .call::<_, _, Vec<u8>>("Execute", &(permit_id))
        .await
        .is_ok()
    {
        return Err(format!("{verb}: single-use permit was replayed").into());
    }
    Ok(())
}

/// Ask systemd itself what the unit is doing, allowing for the job taking a moment.
///
/// A separate connection to a separate party from the one that carried the operation out: this is
/// the disagreement ADR-0022 requires the architecture to allow.
async fn settles_at(unit: &Proxy<'_>, expected: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut active = String::new();
    for _ in 0..50 {
        active = unit.get_property("ActiveState").await?;
        if active == expected {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    Err(format!("independent re-observation says {UNIT} is {active}, expected {expected}").into())
}

#[tokio::main]
#[expect(
    clippy::too_many_lines,
    reason = "one request carried end to end; the steps are the proof and splitting them would hide the order they must happen in"
)]
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

    // ------------------------------------------------------------------ the parties who will act
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

    let system = zbus::Connection::system().await?;
    let manager = Proxy::new(
        &system,
        "org.freedesktop.systemd1",
        "/org/freedesktop/systemd1",
        "org.freedesktop.systemd1.Manager",
    )
    .await?;
    // `LoadUnit` rather than `GetUnit`: the unit is deliberately stopped before this runs, and a
    // stopped unit systemd has not been asked about is not loaded at all.
    let path: zbus::zvariant::OwnedObjectPath = manager.call("LoadUnit", &(UNIT)).await?;
    let unit = Proxy::new(
        &system,
        "org.freedesktop.systemd1",
        path,
        "org.freedesktop.systemd1.Unit",
    )
    .await?;

    // ------------------------------------------------- a person asks, four times, for four things
    // The order is deliberate. Stop is asked while the unit is running and start while it is not,
    // so each one is asked for the state it actually changes; a stop that quietly restarted, or a
    // start that restarted an already-running unit, both leave the unit active and would pass a
    // sequence that did not care which state it started from.
    asked_and_done(&action, &executor, "service.restart", &target).await?;
    settles_at(&unit, "active").await?;

    asked_and_done(&action, &executor, "service.stop", &target).await?;
    settles_at(&unit, "inactive").await?;

    asked_and_done(&action, &executor, "service.start", &target).await?;
    settles_at(&unit, "active").await?;

    // Enable and then disable, in that order, so each is asked for the state it changes. Both are
    // checked afterwards by asking systemd, and the second one leaves the host as this gate found
    // it — a gate that enabled a unit and walked away would be a gate that changes the machine.
    // Asked of the manager rather than read from the unit object. A proxy caches properties and
    // refreshes them from `PropertiesChanged`, which is how the active-state checks above see a
    // stop the moment it happens — but enabling changes a file on disk rather than the unit's
    // state, and the first version of this check read a cached "disabled" straight through a
    // successful enable. `GetUnitFileState` asks systemd the question every time.
    asked_and_done(&action, &executor, "service.enable", &target).await?;
    let enabled: String = manager.call("GetUnitFileState", &(UNIT)).await?;
    if enabled != "enabled" {
        return Err(format!("after service.enable systemd says {UNIT} is {enabled}").into());
    }

    asked_and_done(&action, &executor, "service.disable", &target).await?;
    let disabled: String = manager.call("GetUnitFileState", &(UNIT)).await?;
    if disabled != "disabled" {
        return Err(format!("after service.disable systemd says {UNIT} is {disabled}").into());
    }

    // ------------------------------------------------------------------ and a real process
    // Signalling is the first operation whose adapter refuses on its own account: it reads /proc
    // at the moment it acts and compares the owner against the one the permit was decided for. So
    // this asks twice. Once about a uid that owns nothing here, which must be refused by the
    // executor even though Action1 granted it — and once truthfully, which must end the process.
    if let (Ok(pid), Ok(uid)) = (
        std::env::var("CYBOU_GATE_VICTIM_PID"),
        std::env::var("CYBOU_GATE_VICTIM_UID"),
    ) {
        let wrong_owner = format!("process:{}:{pid}", u32::MAX - 1);
        let (encoded, permit_id): (Vec<u8>, String) = action
            .call(
                "Request",
                &("process.terminate".to_owned(), wrong_owner, SEAT.to_owned()),
            )
            .await?;
        let record: ActionRecord = decode(&encoded)?;
        if !matches!(
            record.decision.verdict,
            AuthorizationVerdict::GrantedOnConfirmation { .. }
        ) {
            return Err("Action1 refused a well-formed request before the executor could".into());
        }
        let encoded: Vec<u8> = executor.call("Execute", &(permit_id)).await?;
        let attempt: ExecutionAttempt = decode(&encoded)?;
        match &attempt.report {
            AttemptReport::Failed { because } if because.contains("belongs to uid") => {}
            other => {
                return Err(format!(
                    "the executor accepted a permit naming the wrong owner: {other:?}"
                )
                .into());
            }
        }

        asked_and_done(
            &action,
            &executor,
            "process.terminate",
            &format!("process:{uid}:{pid}"),
        )
        .await?;
    }

    // Reload has no state of its own to observe from outside: a unit that re-read its configuration
    // looks exactly like one that did not. What is checked is that it was permitted, carried out
    // and reported under its own name, and that the unit is still up afterwards — a reload wired to
    // stop would not survive that last line.
    asked_and_done(&action, &executor, "service.reload", &target).await?;
    settles_at(&unit, "active").await?;

    println!(
        "{SEAT} asked → permit → restart, stop, start, reload{} → independent observation",
        if std::env::var_os("CYBOU_GATE_VICTIM_PID").is_some() {
            ", enable, disable, terminate"
        } else {
            ", enable, disable"
        }
    );
    Ok(())
}
