// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Services Manager card component for managing systemd units and daemons.

use cybou_protocol::SubjectRef;
use cybou_protocol::system::{ServiceAction, ServiceState};
use leptos::prelude::*;

use crate::{
    CardId, MindClient,
    components::{freshness::FreshnessControls, icons::IconLayers},
    refresh::Freshness,
    tool_state::ToolCardStates,
};

/// How often the service list is re-read while the panel is open and visible.
///
/// Fifteen seconds. Units change state when something changes them, not continuously, and this
/// list is long enough that reading it more often would cost the host more than it tells anyone.
const SERVICES_INTERVAL_MS: u32 = 15_000;

#[component]
pub fn ServicesContent(card: CardId) -> impl IntoView {
    let client = crate::GatewayMindClient;
    let tool_states = expect_context::<ToolCardStates>();
    let signals = tool_states.services(card);

    // A service list is also the place a person looks straight after pressing Stop, so an age
    // here doubles as the answer to whether that took effect.
    let freshness = Freshness::new();

    let load_services = move || {
        signals.loading.set(true);
        leptos::task::spawn_local(async move {
            match client.list_services().await {
                Ok(projection) => {
                    signals.services.set(projection.services);
                    signals.status_msg.set(None);
                    freshness.arrived();
                }
                Err(err) => {
                    signals
                        .status_msg
                        .set(Some(format!("Failed to load services: {err}")));
                }
            }
            signals.loading.set(false);
        });
    };

    let trigger_action = move |name: String, action: ServiceAction| {
        leptos::task::spawn_local(async move {
            match client.execute_service_action(&name, action).await {
                Ok(record) => {
                    // The boundary's own words. A card that composed its own sentence here would
                    // be reporting what it asked for rather than what was decided — and a refusal
                    // is a record too, with the reason the boundary gave.
                    let said = match record.verdict.as_str() {
                        "granted-on-confirmation" | "granted" => {
                            record.attempt.as_ref().map_or_else(
                                || format!("{name}: authorized, waiting to be carried out."),
                                |attempt| format!("{name}: {}.", attempt.report),
                            )
                        }
                        _ => record.verdict_reason.clone().unwrap_or_else(|| {
                            format!("{name}: refused, and no reason was given.")
                        }),
                    };
                    signals.status_msg.set(Some(said));
                    load_services();
                }
                Err(err) => {
                    signals
                        .status_msg
                        .set(Some(format!("Service action failed: {err}")));
                }
            }
        });
    };

    let inspect_service = move |name: String| {
        let inspector_signals = tool_states.inspector(CardId::Inspector(0));
        inspector_signals
            .target_subject
            .set(Some(SubjectRef::Service {
                name,
                node_id: None,
            }));
    };

    // Trigger initial load
    Effect::new(move |_| {
        load_services();
    });

    crate::refresh::keep_reading(
        SERVICES_INTERVAL_MS,
        signals.auto_refresh,
        signals.loading,
        load_services,
    );

    let filtered_services = move || {
        let all = signals.services.get();
        let state_filter = signals.filter_state.get();
        let search = signals.search_query.get().to_lowercase();

        all.into_iter()
            .filter(|s| {
                if let Some(st) = state_filter {
                    s.state == st
                } else {
                    true
                }
            })
            .filter(|s| {
                if search.is_empty() {
                    true
                } else {
                    s.name.to_lowercase().contains(&search)
                        || s.description.to_lowercase().contains(&search)
                }
            })
            .collect::<Vec<_>>()
    };

    let active_count = move || {
        signals
            .services
            .get()
            .iter()
            .filter(|s| s.state == ServiceState::Active)
            .count()
    };
    let failed_count = move || {
        signals
            .services
            .get()
            .iter()
            .filter(|s| s.state == ServiceState::Failed)
            .count()
    };

    view! {
        <div class="services-panel" style="display: flex; flex-direction: column; height: 100%; width: 100%; background: var(--bg-card); color: var(--text-main); font-family: system-ui, -apple-system, sans-serif;">
            // Toolbar
            <div style="display: flex; flex-direction: column; gap: 8px; padding: 10px 12px; background: var(--bg-sunken); border-bottom: 1px solid var(--line);">
                <div style="display: flex; align-items: center; justify-content: space-between;">
                    <div style="display: flex; align-items: center; gap: 8px;">
                        <span style="font-weight: 600; font-size: 13px;">"System Services"</span>
                        <span style="background: var(--ok-fill); color: var(--ok); font-size: 11px; padding: 2px 6px; border-radius: 10px; font-weight: 600;">
                            {move || format!("{} Active", active_count())}
                        </span>
                        {move || {
                            let failed = failed_count();
                            if failed > 0 {
                                Some(view! {
                                    <span style="background: var(--danger-fill-strong); color: var(--danger); font-size: 11px; padding: 2px 6px; border-radius: 10px; font-weight: 600;">
                                        {format!("{failed} Failed")}
                                    </span>
                                })
                            } else {
                                None
                            }
                        }}
                    </div>

                    <FreshnessControls
                        freshness=freshness
                        auto_refresh=signals.auto_refresh
                        loading=signals.loading
                        refresh_now=move |()| load_services()
                    />
                </div>

                // Filter tabs & search
                <div style="display: flex; align-items: center; justify-content: space-between; gap: 8px;">
                    <div style="display: flex; gap: 4px;">
                        <button
                            style=move || format!(
                                "background: {}; border: none; font-size: 11px; padding: 3px 8px; border-radius: 12px; color: inherit; cursor: pointer;",
                                if signals.filter_state.get().is_none() { "var(--accent-line)" } else { "var(--fill-faintest)" }
                            )
                            on:click=move |_| signals.filter_state.set(None)
                        >
                            "All"
                        </button>
                        <button
                            style=move || format!(
                                "background: {}; border: none; font-size: 11px; padding: 3px 8px; border-radius: 12px; color: var(--ok); cursor: pointer;",
                                if signals.filter_state.get() == Some(ServiceState::Active) { "var(--ok-line)" } else { "var(--fill-faintest)" }
                            )
                            on:click=move |_| signals.filter_state.set(Some(ServiceState::Active))
                        >
                            "Active"
                        </button>
                        <button
                            style=move || format!(
                                "background: {}; border: none; font-size: 11px; padding: 3px 8px; border-radius: 12px; color: var(--danger); cursor: pointer;",
                                if signals.filter_state.get() == Some(ServiceState::Failed) { "var(--danger-line)" } else { "var(--fill-faintest)" }
                            )
                            on:click=move |_| signals.filter_state.set(Some(ServiceState::Failed))
                        >
                            "Failed"
                        </button>
                    </div>

                    <input
                        type="text"
                        placeholder="Search services..."
                        prop:value=move || signals.search_query.get()
                        on:input=move |e| signals.search_query.set(event_target_value(&e))
                        style="background: var(--bg-sunken-strong); border: 1px solid var(--fill-hover); border-radius: 4px; padding: 3px 8px; font-size: 11px; color: inherit; width: 140px;"
                    />
                </div>
            </div>

            // Status message toast
            {move || signals.status_msg.get().map(|msg| {
                view! {
                    <div class="card-status-line" role="status" aria-live="polite">
                        <span>{msg}</span>
                        <button class="card-status-dismiss" title="Dismiss" on:click=move |_| signals.status_msg.set(None)>"×"</button>
                    </div>
                }
            })}

            // Services Feed
            <div style="flex: 1; overflow-y: auto; padding: 8px; display: flex; flex-direction: column; gap: 6px;">
                <For
                    each=filtered_services
                    key=|s| s.name.clone()
                    children=move |svc| {
                        let name = svc.name.clone();
                        let name_restart = svc.name.clone();
                        let name_stop = svc.name.clone();
                        let name_inspect = svc.name.clone();
                        let is_active = svc.state == ServiceState::Active;

                        let (badge_bg, badge_color) = match svc.state {
                            ServiceState::Active => ("var(--ok-fill-strong)", "var(--ok)"),
                            ServiceState::Failed => ("var(--danger-fill-strong)", "var(--danger)"),
                            ServiceState::Inactive => ("rgba(156, 163, 175, 0.2)", "#9ca3af"),
                            _ => ("var(--caution-fill-strong)", "var(--caution)"),
                        };

                        view! {
                            <div
                                style="background: var(--fill-faint); border: 1px solid var(--fill-subtle); border-radius: 6px; padding: 8px 12px; display: flex; align-items: center; justify-content: space-between; gap: 12px;"
                            >
                                <div style="flex: 1; min-width: 0;">
                                    <div style="display: flex; align-items: center; gap: 8px; margin-bottom: 2px;">
                                        <span style="font-weight: 600; font-size: 12px; color: var(--text-bright); font-family: monospace;">
                                            {name}
                                        </span>
                                        <span style=format!("background: {badge_bg}; color: {badge_color}; font-size: 9px; font-weight: 700; padding: 1px 5px; border-radius: 3px; text-transform: uppercase;")>
                                            {svc.state.label()}
                                        </span>
                                        {svc.main_pid.map(|pid| view! {
                                            <span style="font-size: 10px; color: var(--text-faint); font-family: monospace;">
                                                {format!("PID {pid}")}
                                            </span>
                                        })}
                                    </div>
                                    <div style="font-size: 11px; color: var(--text-second); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                                        {svc.description}
                                    </div>
                                </div>

                                // Action buttons
                                <div style="display: flex; align-items: center; gap: 6px; flex-shrink: 0;">
                                    <button
                                        style="background: var(--fill-subtle); border: 1px solid var(--fill-hover); border-radius: 4px; padding: 3px 8px; font-size: 10px; color: #e2e8f0; cursor: pointer;"
                                        title="Restart service"
                                        on:click=move |_| trigger_action(name_restart.clone(), ServiceAction::Restart)
                                    >
                                        "Restart"
                                    </button>
                                    {if is_active {
                                        view! {
                                            <button
                                                style="background: var(--danger-fill); border: 1px solid var(--danger-line); border-radius: 4px; padding: 3px 8px; font-size: 10px; color: var(--danger); cursor: pointer;"
                                                title="Stop service"
                                                on:click=move |_| trigger_action(name_stop.clone(), ServiceAction::Stop)
                                            >
                                                "Stop"
                                            </button>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <button
                                                style="background: var(--ok-fill); border: 1px solid var(--ok-line); border-radius: 4px; padding: 3px 8px; font-size: 10px; color: var(--ok); cursor: pointer;"
                                                title="Start service"
                                                on:click=move |_| trigger_action(name_stop.clone(), ServiceAction::Start)
                                            >
                                                "Start"
                                            </button>
                                        }.into_any()
                                    }}
                                    <button
                                        style="background: var(--accent-fill); border: 1px solid var(--accent-line); border-radius: 4px; padding: 3px 6px; font-size: 10px; color: var(--accent-light); cursor: pointer; display: flex; align-items: center; gap: 4px;"
                                        title="Inspect in Universal Inspector"
                                        on:click=move |_| inspect_service(name_inspect.clone())
                                    >
                                        <IconLayers size=11 />
                                    </button>
                                </div>
                            </div>
                        }
                    }
                />

                {move || if filtered_services().is_empty() {
                    Some(view! {
                        <div style="text-align: center; color: var(--text-faint); padding: 32px 16px; font-size: 12px;">
                            "No services matching filter."
                        </div>
                    })
                } else {
                    None
                }}
            </div>
        </div>
    }
}
