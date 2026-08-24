// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! ADR-0041 S0, walked from what the kernel wrote to what a person reads.
//!
//! ```text
//! /proc text → reading → bounded window → baseline → finding → plan → prose
//! ```
//!
//! S0 is the gate that decides whether this is a system or a client for somebody else's model:
//!
//! > Cut internet access and every external model API. On a minimal VPS, Cybou continues to observe
//! > its Body, answer basic questions about its own state, detect known problems, explain them
//! > through evidence, remember its open intentions, and form typed action proposals.
//!
//! Everything below runs with no network, no model, and no `/proc` — the kernel's output is a string
//! literal, which is what makes it a test rather than an observation about the machine it runs on.
//! Nothing here imports an inference runtime, and there is none to import.
//!
//! What this walks is *observe → detect → explain → propose → authorize*. Carrying an action out is
//! not built, and this file does not pretend otherwise: there is no executor, and the last two
//! stages of S0 — *act* and *observe outcome* — do not exist. What is walked is everything up to the
//! point where a person would have to say yes.

use cybou_meaning::{Language, plan_system_state, realize};
use cybou_protocol::action::AuthorizationVerdict;
use cybou_protocol::telemetry::{Finding, MetricKey, Reading, Subject, SystemInsight};
use cybou_remediation::{
    Operation, StandingPolicy, authorize, criticise, permits_unattended, propose,
};
use cybou_telemetryd::TelemetryCore;
use cybou_telemetryd::probe;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

/// What a small VPS under memory pressure writes, and what a calm one writes.
const CALM_MEMINFO: &str = "MemTotal:        4025536 kB\nMemFree:          812304 kB\nMemAvailable:    2610214 kB\nSwapTotal:       2097148 kB\nSwapFree:        2087436 kB\n";
const STRAINED_MEMINFO: &str = "MemTotal:        4025536 kB\nMemFree:           41204 kB\nMemAvailable:     121004 kB\nSwapTotal:       2097148 kB\nSwapFree:         430118 kB\n";
const CALM_PRESSURE: &str = "some avg10=1.20 avg60=0.90 avg300=0.40 total=1284920\nfull avg10=0.00 avg60=0.00 avg300=0.00 total=0\n";
const STRAINED_PRESSURE: &str = "some avg10=87.40 avg60=71.20 avg300=44.10 total=9284920\nfull avg10=41.00 avg60=30.00 avg300=12.00 total=41284\n";

fn at(offset: i64) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(1_787_000_000 + offset).expect("a fixed instant")
}

fn core() -> TelemetryCore {
    TelemetryCore::new(Duration::hours(6), 2160)
}

/// One round of sampling, parsed exactly as the daemon parses it.
fn sample(
    core: &TelemetryCore,
    meminfo: &str,
    pressure: &str,
    disk: (u64, u64),
    at: OffsetDateTime,
) {
    let note = |subject: Subject, value: Option<f64>| {
        if let Some(value) = value {
            core.observe(Reading {
                key: MetricKey::host(subject),
                value,
                at,
            });
        }
    };
    note(Subject::MemoryUsed, probe::memory_used(meminfo));
    note(Subject::SwapUsed, probe::swap_used(meminfo));
    note(
        Subject::MemoryPressure,
        probe::pressure_some_avg10(pressure),
    );
    note(
        Subject::RootFilesystemUsed,
        probe::filesystem_used(disk.0, disk.1),
    );
    note(
        Subject::LoadAverage,
        probe::load_average("0.41 0.38 0.35 1/238 9182"),
    );
    note(Subject::FailedUnits, Some(probe::failed_units("")));
}

/// Ask the host what is going on with it, entirely from the deterministic layer.
fn ask(core: &TelemetryCore, now: OffsetDateTime) -> (Vec<SystemInsight>, String) {
    let insights = core.insights(now);
    let observed: Vec<MetricKey> = core
        .latest()
        .into_iter()
        .map(|reading| reading.key)
        .collect();
    let plan = plan_system_state(
        &insights,
        &observed,
        core.has_watched_enough(),
        Uuid::from_u128(9),
    );
    (insights, realize(&plan, Language::English))
}

/// Half a day of a calm host, then whatever comes next.
fn calm_history(core: &TelemetryCore) {
    for tick in 0..40 {
        sample(
            core,
            CALM_MEMINFO,
            CALM_PRESSURE,
            (1_000_000, 610_000),
            at(tick * 10),
        );
    }
}

#[test]
fn a_host_under_memory_pressure_says_so_with_the_readings_that_show_it() {
    // The whole of S0's observe → detect → explain, from kernel text to a sentence, with no
    // network and no model anywhere in the path.
    let core = core();
    calm_history(&core);
    for tick in 40..48 {
        sample(
            &core,
            STRAINED_MEMINFO,
            STRAINED_PRESSURE,
            (1_000_000, 610_000),
            at(tick * 10),
        );
    }

    let (insights, prose) = ask(&core, at(500));

    assert!(
        insights
            .iter()
            .any(|insight| insight.finding == Finding::MemoryPressure),
        "the host did not notice it was under memory pressure: {insights:?}"
    );
    assert!(prose.contains("memory-pressure"), "{prose}");
    assert!(prose.contains("memory.pressure is 87.40"), "{prose}");
    assert!(
        prose.contains("where 1.20 is ordinary here"),
        "the answer does not say what ordinary is for this host: {prose}"
    );
}

#[test]
fn the_answer_is_a_hypothesis_and_never_a_cause() {
    // The one thing a fluent renderer would get wrong, checked at the far end. "The cause is memory
    // pressure" is a claim the evidence does not support and the same struct would have produced.
    let core = core();
    calm_history(&core);
    for tick in 40..48 {
        sample(
            &core,
            STRAINED_MEMINFO,
            STRAINED_PRESSURE,
            (1_000_000, 610_000),
            at(tick * 10),
        );
    }

    let (_, prose) = ask(&core, at(500));
    assert!(!prose.contains("the cause"), "{prose}");
    assert!(!prose.contains("caused by"), "{prose}");
    assert!(
        prose.contains("consistent with") || prose.contains("readings show"),
        "{prose}"
    );
}

#[test]
fn a_calm_host_says_nothing_needs_attention_and_does_not_hedge_about_it() {
    // The control. Without it every assertion above passes on a system that reports a problem
    // whatever it sees, which is the same as a system that reports nothing.
    let core = core();
    calm_history(&core);

    let (insights, prose) = ask(&core, at(500));
    assert!(insights.is_empty(), "{insights:?}");
    assert!(prose.contains("Nothing needs attention"), "{prose}");
    assert!(
        !prose.contains("not the whole of it"),
        "a fully-observed calm host hedged: {prose}"
    );
}

#[test]
fn a_host_that_has_only_just_started_says_it_does_not_know_yet() {
    // The failure this is most likely to have: four readings, a confident all-clear, and a person
    // who believes it. "I have not watched long enough" and "nothing is wrong" are different
    // answers and only one of them is true here.
    let core = core();
    for tick in 0..4 {
        sample(
            &core,
            CALM_MEMINFO,
            CALM_PRESSURE,
            (1_000_000, 610_000),
            at(tick * 10),
        );
    }

    let (_, prose) = ask(&core, at(100));
    assert!(prose.contains("not been watching"), "{prose}");
    assert!(!prose.contains("Nothing needs attention"), "{prose}");
    assert!(
        prose.contains("never read"),
        "the answer is unqualified: {prose}"
    );
}

#[test]
fn a_kernel_without_pressure_accounting_produces_an_answer_that_says_what_it_could_not_see() {
    // The gap between "nothing is wrong" and "nothing is wrong among the things I can watch". A
    // host reporting the first while blind to memory pressure has told a person it is safe to go
    // back to sleep on evidence it does not have.
    let core = core();
    for tick in 0..40 {
        // Every sample, with the pressure file absent as it is on a kernel built without PSI.
        sample(&core, CALM_MEMINFO, "", (1_000_000, 610_000), at(tick * 10));
    }

    let (_, prose) = ask(&core, at(500));
    assert!(prose.contains("Nothing needs attention"), "{prose}");
    assert!(
        prose.contains("no readings for") && prose.contains("memory.pressure"),
        "the answer did not say what it could not see: {prose}"
    );
    assert!(
        prose.contains("never read"),
        "the answer is not qualified: {prose}"
    );
}

#[test]
fn a_full_disk_is_reported_even_where_it_has_always_been_full() {
    // The categorical half. A purely statistical detector says nothing here, precisely because 96%
    // is perfectly ordinary for this host — and an operator would still like to know.
    let core = core();
    for tick in 0..40 {
        sample(
            &core,
            CALM_MEMINFO,
            CALM_PRESSURE,
            (1_000_000, 38_000),
            at(tick * 10),
        );
    }

    let (_, prose) = ask(&core, at(500));
    assert!(prose.contains("storage-exhaustion"), "{prose}");
    assert!(prose.contains("filesystem.root.used is 0.96"), "{prose}");
}

#[test]
fn the_same_host_state_always_produces_the_same_answer() {
    // Determinism at the far end, which is the only place it can be compared by the person it is
    // for.
    let core = core();
    calm_history(&core);
    for tick in 40..48 {
        sample(
            &core,
            STRAINED_MEMINFO,
            STRAINED_PRESSURE,
            (1_000_000, 610_000),
            at(tick * 10),
        );
    }

    let (_, first) = ask(&core, at(500));
    for _ in 0..8 {
        let (_, again) = ask(&core, at(500));
        assert_eq!(again, first);
    }
}

#[test]
fn the_answer_reads_in_russian_without_changing_what_it_claims() {
    // ADR-0031 C7 on this path too: the surface language is a boundary concern, and the findings and
    // readings are the canonical thing underneath.
    let core = core();
    calm_history(&core);
    for tick in 40..48 {
        sample(
            &core,
            STRAINED_MEMINFO,
            STRAINED_PRESSURE,
            (1_000_000, 610_000),
            at(tick * 10),
        );
    }

    let insights = core.insights(at(500));
    let observed: Vec<MetricKey> = core
        .latest()
        .into_iter()
        .map(|reading| reading.key)
        .collect();
    let plan = plan_system_state(&insights, &observed, true, Uuid::from_u128(9));

    let english = realize(&plan, Language::English);
    let russian = realize(&plan, Language::Russian);
    assert_ne!(english, russian);
    for rendered in [&english, &russian] {
        assert!(rendered.contains("memory-pressure"), "{rendered}");
        assert!(rendered.contains("87.40"), "{rendered}");
    }
}

/// Everything the host would offer to do about one finding, and what it decided about each.
fn offers(insight: &SystemInsight) -> Vec<(String, AuthorizationVerdict)> {
    propose(insight, at(0), |operation| {
        Uuid::from_u128(operation.verb().len() as u128)
    })
    .into_iter()
    .map(|proposal| {
        let checks = criticise(&proposal, insight);
        let decision = authorize(
            &proposal,
            &checks,
            insight.strength == cybou_protocol::telemetry::EvidenceStrength::Weak,
            &StandingPolicy::nothing_pre_authorized(),
            at(0),
        );
        (proposal.operation, decision.verdict)
    })
    .collect()
}

#[test]
fn a_full_disk_produces_offers_and_not_one_of_them_may_happen_unattended() {
    // The stage this file gained, walked from the same kernel text as everything above. The host
    // noticed, explained, and now offers — and on a machine nobody has configured, every offer waits
    // for a person.
    let core = core();
    for tick in 0..40 {
        sample(
            &core,
            CALM_MEMINFO,
            CALM_PRESSURE,
            (1_000_000, 38_000),
            at(tick * 10),
        );
    }
    let insights = core.insights(at(500));
    let storage = insights
        .iter()
        .find(|insight| insight.finding == Finding::StorageExhaustion)
        .expect("the host noticed the disk");

    let decided = offers(storage);
    assert!(!decided.is_empty(), "the host offered nothing at all");
    for (operation, verdict) in &decided {
        assert!(
            !matches!(verdict, AuthorizationVerdict::Granted),
            "{operation} was granted on a machine nobody configured"
        );
    }
    assert!(
        decided.iter().any(
            |(operation, verdict)| operation == Operation::CleanPackageCache.verb()
                && matches!(
                    verdict,
                    AuthorizationVerdict::RequiresUserConfirmation { .. }
                )
        ),
        "{decided:?}"
    );
}

#[test]
fn nothing_the_host_offers_is_ever_destructive() {
    // Walked across every finding the detector can reach, rather than asserted about one. The
    // example that motivated this — deleting a database's data directory to free space — is not
    // something the host declines to do. It is something it never offers.
    for finding in [
        Finding::StorageExhaustion,
        Finding::ServiceFailure,
        Finding::MemoryPressure,
        Finding::IoSaturation,
        Finding::CpuSaturation,
        Finding::UnexplainedDeviation,
    ] {
        let insight = SystemInsight {
            insight_id: Uuid::from_u128(3),
            finding,
            about: None,
            because: Vec::new(),
            strength: cybou_protocol::telemetry::EvidenceStrength::Strong,
            concluded_at: at(0),
            since: at(0),
        };
        for (operation, _) in offers(&insight) {
            let known = cybou_remediation::ALL_OPERATIONS
                .iter()
                .find(|candidate| candidate.verb() == operation)
                .expect("a known operation");
            assert!(!known.forbidden(), "{finding:?} offered {operation}");
        }
    }
}

#[test]
fn a_host_that_is_only_guessing_offers_to_look_and_refuses_to_change_anything() {
    // One uncorroborated reading out of range is a reason to investigate, not to act. A system that
    // acted on its weakest conclusions would spend its credibility on the cases it is least sure of.
    let guessing = SystemInsight {
        insight_id: Uuid::from_u128(4),
        finding: Finding::ServiceFailure,
        about: None,
        because: Vec::new(),
        strength: cybou_protocol::telemetry::EvidenceStrength::Weak,
        concluded_at: at(0),
        since: at(0),
    };
    for (operation, verdict) in offers(&guessing) {
        if operation == Operation::InspectServiceStatus.verb() {
            assert!(
                matches!(
                    verdict,
                    AuthorizationVerdict::RequiresUserConfirmation { .. }
                ),
                "looking was refused: {verdict:?}"
            );
        } else {
            assert!(
                matches!(verdict, AuthorizationVerdict::Denied { .. }),
                "{operation} was offered on one uncorroborated reading"
            );
        }
    }
}

#[test]
fn a_configured_operator_can_let_one_ordinary_thing_happen_unattended() {
    // The control. Every assertion above passes on a gate that refuses everything, and a gate that
    // refuses everything is not a policy boundary, it is an off switch.
    let insight = SystemInsight {
        insight_id: Uuid::from_u128(5),
        finding: Finding::StorageExhaustion,
        about: None,
        because: Vec::new(),
        strength: cybou_protocol::telemetry::EvidenceStrength::Strong,
        concluded_at: at(0),
        since: at(0),
    };
    let proposal = propose(&insight, at(0), |operation| {
        Uuid::from_u128(operation.verb().len() as u128)
    })
    .into_iter()
    .find(|proposal| proposal.operation == Operation::CleanPackageCache.verb())
    .expect("offered");

    let decision = authorize(
        &proposal,
        &criticise(&proposal, &insight),
        false,
        &StandingPolicy {
            pre_authorized: vec![Operation::CleanPackageCache],
        },
        at(0),
    );
    assert!(permits_unattended(&decision));
}
