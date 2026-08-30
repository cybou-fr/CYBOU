// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Runtime state, subscription lifecycle, and formatting helpers for Living Canvas.

use cybou_protocol::{CapabilityState, KnowledgeState};
use cybou_web_contracts::{
    DisclosureProjection, Freshness, InsightProjection, MindProjection, SessionMode,
    SessionProjection, SnapshotProjection,
};
use leptos::prelude::RwSignal;

/// High-level runtime connection and projection state.
#[derive(Clone, Debug)]
pub enum RuntimeState {
    /// Initializing connection to the Mind gateway.
    Loading,
    /// Connected with server-established session and projections.
    Ready {
        /// Gateway session mode (`LocalDesktop`, `RemoteBrowser`, `PublicPreview`).
        mode: SessionMode,
        /// Server-established session projection.
        session: SessionProjection,
        /// Current state snapshot projection.
        snapshot: SnapshotProjection,
        /// Full Mind owner projection if available.
        mind: Option<MindProjection>,
        /// What this reader was last supplied, and what was kept from them.
        ///
        /// `None` means the gateway could not be asked, which is a different fact from a delivery
        /// that has not happened — the projection carries that one itself.
        disclosure: Option<DisclosureProjection>,
        /// What this host makes of itself, if the telemetry organ could be asked.
        ///
        /// `None` means the gateway could not be asked at all. The projection carries the two finer
        /// distinctions itself — the organ not answering, and the organ not having watched long
        /// enough — because a surface that showed one thing for all three would let "nobody looked"
        /// read as "nothing is wrong".
        insight: Option<InsightProjection>,
        /// Every agent session the runtime is holding, if it could be asked.
        ///
        /// `None` is *the runtime did not answer* and an empty list is *nothing is running*. They
        /// are drawn differently on purpose: on a host where the agent runtime is not installed at
        /// all, a card that showed the second would be telling somebody their agents had stopped.
        agents: Option<Vec<cybou_protocol::agent::SessionView>>,
        /// Action records held by Action1.
        actions: Option<Vec<cybou_web_contracts::ActionRecordProjection>>,
        /// Agent launch offers and runtime readiness.
        agent_offers: Option<cybou_protocol::agent::AgentOffersResponse>,
    },
    /// Connection or protocol error.
    Error(String),
    /// This deployment serves nothing until somebody signs in, and nobody has.
    ///
    /// Its own state rather than an `Error`, because it is not one. Reading the session, finding
    /// the surface closed and reporting "unavailable" drew a whole desktop of em-dashes: it told a
    /// stranger the machine was broken, and showed them the entire structure of the Mind while
    /// doing it. Nothing is wrong here. Nothing is being shown, which is different.
    SignInRequired,
}

/// Managed subscription to gateway runtime state and SSE live event stream.
pub struct DesktopRuntimeSubscription {
    #[cfg(target_arch = "wasm32")]
    es: Option<web_sys::EventSource>,
}

impl DesktopRuntimeSubscription {
    /// Subscribe to the SSE event stream, updating the runtime signal on snapshots.
    #[must_use]
    pub fn subscribe(runtime: RwSignal<RuntimeState>) -> Self {
        #[cfg(target_arch = "wasm32")]
        {
            use leptos::prelude::*;
            use wasm_bindgen::{JsCast, closure::Closure};
            use web_sys::{EventSource, MessageEvent};

            if let Ok(es) = EventSource::new("/api/v1/events") {
                let on_snap =
                    Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
                        let Some(data) = event.data().as_string() else {
                            return;
                        };
                        let Ok(new_snapshot) = serde_json::from_str::<SnapshotProjection>(&data)
                        else {
                            return;
                        };
                        runtime.update(|state| {
                            if let RuntimeState::Ready { snapshot, .. } = state {
                                *snapshot = new_snapshot;
                            }
                        });
                    });
                let _ = es
                    .add_event_listener_with_callback("snapshot", on_snap.as_ref().unchecked_ref());
                on_snap.forget();
                return Self { es: Some(es) };
            }
        }
        let _ = runtime;
        Self {
            #[cfg(target_arch = "wasm32")]
            es: None,
        }
    }
}

impl Drop for DesktopRuntimeSubscription {
    fn drop(&mut self) {
        #[cfg(target_arch = "wasm32")]
        if let Some(es) = &self.es {
            es.close();
        }
    }
}

/// Placeholder string for unread/withheld data fields.
#[must_use]
pub fn unread() -> String {
    "—".to_owned()
}

/// Human-readable label for a capability state.
#[must_use]
pub const fn capability_state_label(state: CapabilityState) -> &'static str {
    match state {
        CapabilityState::Available => "Available",
        CapabilityState::Unavailable => "Unavailable",
        CapabilityState::Unknown => "Unknown",
    }
}

/// Human-readable label for a knowledge state.
#[must_use]
pub const fn knowledge_label(state: KnowledgeState) -> &'static str {
    match state {
        KnowledgeState::Known => "Known",
        KnowledgeState::Unknown => "Unknown",
    }
}

/// Human-readable label for projection freshness.
#[must_use]
pub const fn freshness_label(state: Freshness) -> &'static str {
    match state {
        Freshness::Current => "Current",
        Freshness::Stale => "Stale",
        Freshness::Unknown => "Unknown freshness",
    }
}

/// Helper matching command palette queries.
#[must_use]
pub fn command_matches(query: &str, haystack: &str) -> bool {
    let query = query.trim().to_ascii_lowercase();
    query.is_empty() || haystack.contains(&query)
}

/// Match first matching panel for a command query.
#[must_use]
pub fn first_command_match(query: &str) -> Option<&'static str> {
    [
        ("capabilities", "capabilities health"),
        ("identity", "identity subject continuity"),
        ("session", "session trust mode"),
        ("journal", "journal contributions event1"),
        ("lifecycle", "lifecycle sleep wake"),
        ("commitments", "commitments obligations intention1"),
        ("self", "self assessment narration self1"),
        ("attention", "attention focus workspace1"),
        ("beliefs", "beliefs epistemic1 validity"),
        ("perception", "perception host observation"),
        ("context", "context association concepts context1"),
        ("shell", "shell terminal body capability"),
        (
            "insight",
            "insight telemetry machine health findings status",
        ),
        ("agents", "agents agent1 launch opencode task"),
    ]
    .into_iter()
    .find_map(|(panel, label)| command_matches(query, label).then_some(panel))
}

/// A deterministic answer produced by the Ask CYBOU engine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AskCybouAnswer {
    /// Short high-level answer headline.
    pub headline: String,
    /// Detailed factual response.
    pub detail: String,
    /// Optional button / panel link label and target `CardId`.
    pub target: Option<(&'static str, crate::CardId)>,
}

/// Deterministically answer operator questions about host health, actions, and agents.
#[must_use]
pub fn ask_cybou(query: &str, state: &RuntimeState) -> Option<AskCybouAnswer> {
    let q = query.trim().to_ascii_lowercase();
    if q.is_empty() || q.len() < 3 {
        return None;
    }

    // Health / What is wrong? / Status
    if q.contains("wrong")
        || q.contains("problem")
        || q.contains("issue")
        || q.contains("status")
        || q.contains("health")
        || q.contains("сервер")
        || q.contains("что с")
        || q.contains("проблем")
    {
        return match state {
            RuntimeState::Ready {
                insight: Some(insight),
                ..
            } => {
                if insight.knowledge != KnowledgeState::Known {
                    return Some(AskCybouAnswer {
                        headline: "Observations converging".to_string(),
                        detail: "Telemetry observations are currently converging and establishing baseline metrics.".to_string(),
                        target: Some(("Open System Insight", crate::CardId::Insight)),
                    });
                }
                if !insight.watched_enough {
                    return Some(AskCybouAnswer {
                        headline: "Baseline establishing".to_string(),
                        detail: "Telemetry has not observed enough baseline history yet to establish normal limits.".to_string(),
                        target: Some(("Open System Insight", crate::CardId::Insight)),
                    });
                }
                if !insight.unobserved.is_empty() {
                    return Some(AskCybouAnswer {
                        headline: format!("{} unobserved resource(s)", insight.unobserved.len()),
                        detail: format!(
                            "Some declared resources could not be read: {}.",
                            insight.unobserved.join(", ")
                        ),
                        target: Some(("View System Insight", crate::CardId::Insight)),
                    });
                }
                let finding_count = insight.findings.len();
                if finding_count > 0 {
                    let first_finding = &insight.findings[0];
                    let target_name = first_finding
                        .about
                        .as_deref()
                        .unwrap_or(first_finding.finding.as_str());
                    Some(AskCybouAnswer {
                        headline: format!("{finding_count} issue(s) detected"),
                        detail: format!(
                            "{target_name}: {}. CYBOU self-healing is monitoring or remediating.",
                            first_finding.means
                        ),
                        target: Some(("View in System Insight", crate::CardId::Insight)),
                    })
                } else {
                    Some(AskCybouAnswer {
                        headline: "All watched systems are healthy".to_string(),
                        detail: format!(
                            "All {} watched metrics, services, and system capabilities are operating within ordinary baseline limits.",
                            insight.watched.len()
                        ),
                        target: Some(("Open System Insight", crate::CardId::Insight)),
                    })
                }
            }
            RuntimeState::Ready {
                insight: None, ..
            } => Some(AskCybouAnswer {
                headline: "System insight unavailable".to_string(),
                detail: "I could not read System Insight. I cannot establish whether the host is healthy right now.".to_string(),
                target: Some(("Open System Insight", crate::CardId::Insight)),
            }),
            _ => None,
        };
    }

    // What did you fix? / Remediation / Actions
    if q.contains("fix")
        || q.contains("repar")
        || q.contains("remediat")
        || q.contains("action")
        || q.contains("исправ")
        || q.contains("почин")
        || q.contains("сделал")
    {
        return match state {
            RuntimeState::Ready {
                actions: Some(actions),
                ..
            } => {
                if actions.is_empty() {
                    Some(AskCybouAnswer {
                        headline: "No autonomous actions executed".to_string(),
                        detail: "No remediation actions have been executed recently. Standing self-healing policies are armed.".to_string(),
                        target: Some(("View System Insight", crate::CardId::Insight)),
                    })
                } else {
                    let first = &actions[0];
                    let relief_desc = first.outcome.as_ref().map_or("in-progress", |o| o.relief.as_str());
                    Some(AskCybouAnswer {
                        headline: format!("{} autonomous action(s) recorded", actions.len()),
                        detail: format!(
                            "Most recent: {} on {} (policy verdict: {}, outcome: {}).",
                            first.operation, first.target_resource, first.verdict, relief_desc
                        ),
                        target: Some(("View System Insight", crate::CardId::Insight)),
                    })
                }
            }
            RuntimeState::Ready { actions: None, .. } => Some(AskCybouAnswer {
                headline: "Action history unavailable".to_string(),
                detail: "Action1 authorization service could not be reached. Cannot establish past remediation history.".to_string(),
                target: Some(("View System Insight", crate::CardId::Insight)),
            }),
            _ => None,
        };
    }

    // Agents / Tasks / Running
    if q.contains("agent")
        || q.contains("opencode")
        || q.contains("task")
        || q.contains("running")
        || q.contains("агент")
        || q.contains("задач")
        || q.contains("работает")
    {
        return match state {
            RuntimeState::Ready {
                agents: Some(sessions),
                agent_offers,
                ..
            } => {
                let live: Vec<_> = sessions.iter().filter(|s| s.is_live()).collect();
                if live.is_empty() {
                    let setup_required = agent_offers.as_ref().is_some_and(|o| {
                        o.profiles_state != "ready" || o.capacity_state != "ready"
                    });
                    if setup_required {
                        Some(AskCybouAnswer {
                            headline: "No agents running (Setup required)".to_string(),
                            detail: "Agent runtime is idle and requires operator profile and capacity configuration before launch.".to_string(),
                            target: Some(("Open Agents", crate::CardId::Agents)),
                        })
                    } else {
                        Some(AskCybouAnswer {
                            headline: "No agents currently running".to_string(),
                            detail:
                                "Agent runtime is idle. Ready to launch sandboxed OpenCode agents."
                                    .to_string(),
                            target: Some(("Launch Agent", crate::CardId::Agents)),
                        })
                    }
                } else {
                    let first = &live[0];
                    let task_desc = first
                        .task
                        .as_ref()
                        .map_or("unspecified task", |t| t.prompt.as_str());
                    let spend_desc = match first.spend {
                        Some(cybou_protocol::agent::SpendView::Capped {
                            limit,
                            spent: Some(spent),
                        }) => format!("{spent} / {limit} units"),
                        Some(cybou_protocol::agent::SpendView::Capped { limit, .. }) => {
                            format!("limit {limit}")
                        }
                        Some(cybou_protocol::agent::SpendView::ZeroCost { spent: Some(spent) })
                            if spent > 0 =>
                        {
                            format!("{spent} charged (zero-cost policy)")
                        }
                        Some(cybou_protocol::agent::SpendView::ZeroCost { .. }) => {
                            "zero-cost".to_string()
                        }
                        None => match &first.model_class {
                            Some(m) => format!("model {m}, spending not read"),
                            None => "no model granted".to_string(),
                        },
                    };
                    Some(AskCybouAnswer {
                        headline: format!("{} agent session(s) active", live.len()),
                        detail: format!(
                            "{} running in {} (\"{task_desc}\"). Spend: {spend_desc}.",
                            first.agent, first.workspace
                        ),
                        target: Some(("Open Agents", crate::CardId::Agents)),
                    })
                }
            }
            RuntimeState::Ready { agents: None, .. } => Some(AskCybouAnswer {
                headline: "Agent runtime unreachable".to_string(),
                detail:
                    "I could not reach Agent1. I cannot establish whether any agents are running."
                        .to_string(),
                target: Some(("Open Agents", crate::CardId::Agents)),
            }),
            _ => None,
        };
    }

    // Reach / Security / Boundary
    if q.contains("reach")
        || q.contains("network")
        || q.contains("bound")
        || q.contains("secur")
        || q.contains("доступ")
        || q.contains("границ")
    {
        return Some(AskCybouAnswer {
            headline: "Technically Enforced Capsule Isolation".to_string(),
            detail: "CYBOU enforces capsule boundaries using Linux user namespaces, Landlock filesystem restrictions, cgroup resource limits, and network egress brokers.".to_string(),
            target: Some(("Inspect Agents", crate::CardId::Agents)),
        });
    }

    None
}
