// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Live A1 gate client: finding → Action1 → executor → independent systemd observation.

use cybou_fabric::{ACTION, EXECUTOR, decode, encode};
use cybou_protocol::{
    action::{AttemptReport, ExecutionAttempt},
    telemetry::{EvidenceStrength, Finding, MetricKey, Subject, SystemInsight},
};
use time::OffsetDateTime;
use uuid::Uuid;
use zbus::Proxy;

const UNIT: &str = "cybou-action-gate.service";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let session = zbus::Connection::session().await?;
    let action = Proxy::new(
        &session,
        ACTION.service,
        ACTION.object_path,
        ACTION.interface,
    )
    .await?;
    let insight = SystemInsight {
        insight_id: Uuid::new_v4(),
        finding: Finding::ServiceInactive,
        about: Some(MetricKey::named(Subject::ServiceActive, UNIT.to_owned())),
        because: Vec::new(),
        strength: EvidenceStrength::Strong,
        concluded_at: OffsetDateTime::now_utc(),
        since: OffsetDateTime::now_utc(),
    };
    let (record, permit_id): (Vec<u8>, String) = action
        .call(
            "EvaluateInsight",
            &(encode(&insight)?, "service.restart".to_owned()),
        )
        .await?;
    if record.is_empty() || permit_id.is_empty() {
        return Err("Action1 did not produce a granted lifecycle record and permit".into());
    }

    let executor = Proxy::new(
        &session,
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
    println!("proposal → decision → permit → restart → independent active observation");
    Ok(())
}
