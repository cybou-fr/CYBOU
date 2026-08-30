// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Operations Manager card component for monitoring server-owned background tasks.

use cybou_protocol::operation::OperationState;
use leptos::prelude::*;
use uuid::Uuid;

use crate::{
    CardId, MindClient,
    components::icons::{IconActivity, IconRefresh, IconStop},
    tool_state::ToolCardStates,
};

#[component]
pub fn OperationsContent(card: CardId) -> impl IntoView {
    let client = crate::GatewayMindClient;
    let tool_states = expect_context::<ToolCardStates>();
    let signals = tool_states.operations(card);

    let load_operations = move || {
        signals.loading.set(true);
        leptos::task::spawn_local(async move {
            match client.list_operations().await {
                Ok(projection) => {
                    signals.operations.set(projection.operations);
                    signals.status_msg.set(None);
                }
                Err(err) => {
                    signals
                        .status_msg
                        .set(Some(format!("Failed to load operations: {err}")));
                }
            }
            signals.loading.set(false);
        });
    };

    let load_logs = move |id: Uuid| {
        leptos::task::spawn_local(async move {
            if let Ok(projection) = client.get_operation_logs(id).await {
                signals.selected_logs.set(projection.logs);
            }
        });
    };

    let cancel_op = move |id: Uuid| {
        leptos::task::spawn_local(async move {
            match client
                .cancel_operation(id, Some("Cancelled by user".to_owned()))
                .await
            {
                Ok(()) => {
                    signals
                        .status_msg
                        .set(Some("Operation cancelled".to_owned()));
                    load_operations();
                }
                Err(err) => {
                    signals
                        .status_msg
                        .set(Some(format!("Cancel failed: {err}")));
                }
            }
        });
    };

    // Trigger initial load
    Effect::new(move |_| {
        load_operations();
    });

    let filtered_ops = move || {
        let all = signals.operations.get();
        let filter = signals.filter_status.get();
        match filter.as_deref() {
            Some("running") => all
                .into_iter()
                .filter(|op| matches!(op.state, OperationState::Running | OperationState::Queued))
                .collect(),
            Some("completed") => all
                .into_iter()
                .filter(|op| matches!(op.state, OperationState::Completed))
                .collect(),
            Some("failed") => all
                .into_iter()
                .filter(|op| {
                    matches!(
                        op.state,
                        OperationState::Failed { .. } | OperationState::Cancelled
                    )
                })
                .collect(),
            _ => all,
        }
    };

    let active_count = move || {
        signals
            .operations
            .get()
            .iter()
            .filter(|op| !op.state.is_terminal())
            .count()
    };

    view! {
        <div class="operations-panel" style="display: flex; flex-direction: column; height: 100%; width: 100%; background: var(--bg-card, #1e1e24); color: var(--text-main, #e0e0e0); font-family: system-ui, -apple-system, sans-serif;">
            // Toolbar
            <div class="ops-toolbar" style="display: flex; align-items: center; justify-content: space-between; padding: 8px 12px; background: rgba(0,0,0,0.2); border-bottom: 1px solid rgba(255,255,255,0.08);">
                <div style="display: flex; align-items: center; gap: 8px;">
                    <span style="font-weight: 600; font-size: 13px; display: flex; align-items: center; gap: 6px;">
                        <IconActivity size=15 />
                        "Operations Monitor"
                    </span>
                    <span style="background: rgba(99, 102, 241, 0.2); color: #818cf8; font-size: 11px; padding: 2px 6px; border-radius: 10px; font-weight: 600;">
                        {move || format!("{} active", active_count())}
                    </span>
                </div>

                <div style="display: flex; align-items: center; gap: 6px;">
                    // Filter tabs
                    <button
                        class=move || if signals.filter_status.get().is_none() { "filter-btn active" } else { "filter-btn" }
                        style="background: none; border: none; font-size: 11px; padding: 4px 8px; border-radius: 4px; color: inherit; cursor: pointer;"
                        on:click=move |_| signals.filter_status.set(None)
                    >
                        "All"
                    </button>
                    <button
                        class=move || if signals.filter_status.get().as_deref() == Some("running") { "filter-btn active" } else { "filter-btn" }
                        style="background: none; border: none; font-size: 11px; padding: 4px 8px; border-radius: 4px; color: inherit; cursor: pointer;"
                        on:click=move |_| signals.filter_status.set(Some("running".to_owned()))
                    >
                        "Running"
                    </button>
                    <button
                        class=move || if signals.filter_status.get().as_deref() == Some("completed") { "filter-btn active" } else { "filter-btn" }
                        style="background: none; border: none; font-size: 11px; padding: 4px 8px; border-radius: 4px; color: inherit; cursor: pointer;"
                        on:click=move |_| signals.filter_status.set(Some("completed".to_owned()))
                    >
                        "Completed"
                    </button>

                    <button
                        style="background: rgba(255,255,255,0.06); border: none; border-radius: 4px; padding: 4px 6px; color: inherit; cursor: pointer; display: flex; align-items: center;"
                        title="Refresh operations"
                        on:click=move |_| load_operations()
                    >
                        <IconRefresh size=13 />
                    </button>
                </div>
            </div>

            // Status message / toast
            {move || signals.status_msg.get().map(|msg| {
                view! {
                    <div style="background: rgba(239, 68, 68, 0.15); color: #fca5a5; font-size: 11px; padding: 6px 12px; border-bottom: 1px solid rgba(239, 68, 68, 0.3); display: flex; justify-content: space-between;">
                        <span>{msg}</span>
                        <button style="background: none; border: none; color: inherit; cursor: pointer;" on:click=move |_| signals.status_msg.set(None)>"×"</button>
                    </div>
                }
            })}

            // Main Split Layout: Operations list & Log Inspector
            <div style="display: flex; flex: 1; min-height: 0;">
                // Left: Operations list
                <div style="flex: 1; overflow-y: auto; padding: 8px; display: flex; flex-direction: column; gap: 8px; border-right: 1px solid rgba(255,255,255,0.06);">
                    <For
                        each=filtered_ops
                        key=|op| op.id
                        children=move |op| {
                            let op_id = op.id;
                            let is_selected = move || signals.selected_op_id.get() == Some(op_id);
                            let percent = op.progress.percent.unwrap_or(0.0);
                            let is_terminal = op.state.is_terminal();
                            let cancellable = op.cancellable && !is_terminal;

                            let (badge_bg, badge_color, badge_text) = match &op.state {
                                OperationState::Running => ("rgba(59, 130, 246, 0.2)", "#60a5fa", "Running"),
                                OperationState::Queued => ("rgba(245, 158, 11, 0.2)", "#fbbf24", "Queued"),
                                OperationState::Completed => ("rgba(34, 197, 94, 0.2)", "#4ade80", "Completed"),
                                OperationState::Failed { .. } => ("rgba(239, 68, 68, 0.2)", "#f87171", "Failed"),
                                OperationState::Cancelled => ("rgba(156, 163, 175, 0.2)", "#9ca3af", "Cancelled"),
                            };

                            view! {
                                <div
                                    class=move || if is_selected() { "op-card selected" } else { "op-card" }
                                    style=move || format!(
                                        "background: {}; border: 1px solid {}; border-radius: 6px; padding: 10px; cursor: pointer; transition: all 0.15s ease;",
                                        if is_selected() { "rgba(99, 102, 241, 0.12)" } else { "rgba(255,255,255,0.03)" },
                                        if is_selected() { "rgba(99, 102, 241, 0.4)" } else { "rgba(255,255,255,0.07)" }
                                    )
                                    on:click=move |_| {
                                        signals.selected_op_id.set(Some(op_id));
                                        load_logs(op_id);
                                    }
                                >
                                    <div style="display: flex; align-items: flex-start; justify-content: space-between; margin-bottom: 6px;">
                                        <div>
                                            <div style="font-weight: 600; font-size: 12px; color: var(--text-main, #f3f4f6);">
                                                {op.label.clone()}
                                            </div>
                                            <div style="font-size: 10px; color: rgba(255,255,255,0.5); margin-top: 2px;">
                                                {op.kind.display_label()}
                                            </div>
                                        </div>
                                        <span style=format!("background: {badge_bg}; color: {badge_color}; font-size: 10px; font-weight: 600; padding: 2px 6px; border-radius: 4px; text-transform: uppercase;")>
                                            {badge_text}
                                        </span>
                                    </div>

                                    // Progress bar
                                    <div style="width: 100%; height: 4px; background: rgba(255,255,255,0.08); border-radius: 2px; overflow: hidden; margin: 6px 0;">
                                        <div
                                            style=format!(
                                                "width: {}%; height: 100%; background: {}; transition: width 0.3s ease;",
                                                percent,
                                                if matches!(op.state, OperationState::Completed) { "#22c55e" } else { "#6366f1" }
                                            )
                                        ></div>
                                    </div>

                                    // Step info
                                    <div style="display: flex; justify-content: space-between; align-items: center; font-size: 11px; color: rgba(255,255,255,0.7);">
                                        <span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 220px;">
                                            {op.progress.step.clone()}
                                        </span>
                                        {op.progress.percent.map(|p| view! { <span style="font-weight: 600; font-size: 10px;">{format!("{:.0}%", p)}</span> })}
                                    </div>

                                    // Footer actions
                                    <div style="display: flex; justify-content: flex-end; gap: 6px; margin-top: 8px;">
                                        {if cancellable {
                                            Some(view! {
                                                <button
                                                    style="background: rgba(239, 68, 68, 0.15); color: #f87171; border: 1px solid rgba(239, 68, 68, 0.3); font-size: 10px; padding: 2px 6px; border-radius: 4px; cursor: pointer; display: flex; align-items: center; gap: 4px;"
                                                    on:click=move |e| {
                                                        e.stop_propagation();
                                                        cancel_op(op_id);
                                                    }
                                                >
                                                    <IconStop size=10 />
                                                    "Cancel"
                                                </button>
                                            })
                                        } else {
                                            None
                                        }}
                                    </div>
                                </div>
                            }
                        }
                    />

                    {move || if filtered_ops().is_empty() {
                        Some(view! {
                            <div style="text-align: center; color: rgba(255,255,255,0.4); padding: 32px 16px; font-size: 12px;">
                                "No operations matching filter."
                            </div>
                        })
                    } else {
                        None
                    }}
                </div>

                // Right: Execution Log stream
                <div style="flex: 1.2; display: flex; flex-direction: column; background: rgba(0,0,0,0.3); font-family: monospace; font-size: 11px;">
                    <div style="padding: 6px 12px; background: rgba(0,0,0,0.2); border-bottom: 1px solid rgba(255,255,255,0.06); display: flex; justify-content: space-between; align-items: center;">
                        <span style="font-weight: 600; color: rgba(255,255,255,0.7);">"Execution Logs"</span>
                        {move || signals.selected_op_id.get().map(|id| view! {
                            <span style="font-size: 10px; color: rgba(255,255,255,0.4);">{format!("id: {}", &id.to_string()[..8])}</span>
                        })}
                    </div>

                    <div style="flex: 1; overflow-y: auto; padding: 8px; display: flex; flex-direction: column; gap: 4px;">
                        <For
                            each=move || signals.selected_logs.get()
                            key=|log| format!("{}-{}", log.timestamp, log.text)
                            children=move |log| {
                                let stream_color = match log.stream.as_str() {
                                    "stderr" => "#f87171",
                                    "system" => "#fbbf24",
                                    _ => "#9ca3af",
                                };
                                view! {
                                    <div style="display: flex; gap: 8px; line-height: 1.4;">
                                        <span style=format!("color: {stream_color}; font-size: 10px; min-width: 48px; user-select: none;")>
                                            {format!("[{}]", log.stream)}
                                        </span>
                                        <span style="color: #e5e7eb; word-break: break-all;">
                                            {log.text}
                                        </span>
                                    </div>
                                }
                            }
                        />

                        {move || if signals.selected_logs.get().is_empty() {
                            Some(view! {
                                <div style="text-align: center; color: rgba(255,255,255,0.3); padding: 32px 16px;">
                                    "Select an operation to inspect live execution output."
                                </div>
                            })
                        } else {
                            None
                        }}
                    </div>
                </div>
            </div>
        </div>
    }
}
