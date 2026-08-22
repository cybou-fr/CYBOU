// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Runtime state, subscription lifecycle, and formatting helpers for Living Canvas.

use cybou_protocol::{CapabilityState, KnowledgeState};
use cybou_web_contracts::{
    DisclosureProjection, Freshness, MindProjection, SessionMode, SessionProjection,
    SnapshotProjection,
};
use leptos::prelude::RwSignal;

/// High-level runtime connection and projection state.
#[derive(Clone, Debug)]
pub enum RuntimeState {
    /// Initializing connection to the Mind gateway.
    Loading,
    /// Connected with server-established session and projections.
    Ready {
        /// Gateway session mode (LocalDesktop, RemoteBrowser, PublicPreview).
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
    ]
    .into_iter()
    .find_map(|(panel, label)| command_matches(query, label).then_some(panel))
}
