// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Operations Manager card component for monitoring server-owned background tasks.

use cybou_protocol::operation::{CancelOutcome, ObservationState, OperationState};
use leptos::prelude::*;
use uuid::Uuid;

use crate::{
    CardId, MindClient,
    components::{
        freshness::FreshnessControls,
        icons::{IconActivity, IconStop},
    },
    refresh::Freshness,
    tool_state::ToolCardStates,
};

/// Agent1 reconciliation runs every two seconds, so polling faster would only repeat an owner
/// projection that cannot yet have changed.
const OPERATIONS_INTERVAL_MS: u32 = 2_000;

fn retained_selection(
    selected: Option<Uuid>,
    operations: &[cybou_protocol::operation::OperationRecord],
) -> Option<Uuid> {
    selected.filter(|id| operations.iter().any(|operation| operation.id == *id))
}

#[component]
pub fn OperationsContent(card: CardId) -> impl IntoView {
    let client = crate::GatewayMindClient;
    let tool_states = expect_context::<ToolCardStates>();
    let signals = tool_states.operations(card);
    let freshness = Freshness::new();

    let load_logs = move |id: Uuid| {
        leptos::task::spawn_local(async move {
            match client.get_operation_logs(id).await {
                Ok(projection) => signals.selected_logs.set(projection.logs),
                Err(error) => {
                    signals.selected_logs.set(Vec::new());
                    signals
                        .status_msg
                        .set(Some(format!("Failed to restore operation logs: {error}")));
                }
            }
        });
    };

    let load_operations = move || {
        if signals.loading.get_untracked() {
            return;
        }
        signals.loading.set(true);
        leptos::task::spawn_local(async move {
            match client.list_operations().await {
                Ok(projection) => {
                    let selected = retained_selection(
                        signals.selected_op_id.get_untracked(),
                        &projection.operations,
                    );
                    signals.selected_op_id.set(selected);
                    if let Some(id) = selected {
                        load_logs(id);
                    } else {
                        signals.selected_logs.set(Vec::new());
                    }
                    signals.operations.set(projection.operations);
                    signals.status_msg.set(None);
                    freshness.arrived();
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

    let cancel_op = move |id: Uuid| {
        leptos::task::spawn_local(async move {
            match client
                .cancel_operation(id, Some("Cancelled by user".to_owned()))
                .await
            {
                // The owner distinguishes a recorded request from an observed teardown, and so
                // does the desktop: a signalled worker may still be running.
                Ok(CancelOutcome::CancellationConfirmed) => {
                    signals
                        .status_msg
                        .set(Some("Operation cancelled".to_owned()));
                    load_operations();
                }
                Ok(_) => {
                    signals.status_msg.set(Some(
                        "Cancellation requested; waiting for the worker to stop".to_owned(),
                    ));
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

    // A gateway restart is a transient read failure, not a reason to freeze the last operation
    // state forever. The next tick reconnects through the stateless HTTP adapter and rehydrates the
    // selected operation and its owner-held logs by stable ID.
    crate::refresh::keep_reading(
        OPERATIONS_INTERVAL_MS,
        signals.auto_refresh,
        signals.loading,
        load_operations,
    );

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
                        OperationState::Failed { .. }
                            | OperationState::Cancelled
                            | OperationState::Refused { .. }
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
        <div class="operations-panel" style="display: flex; flex-direction: column; height: 100%; width: 100%; ">
            // Toolbar
            <div class="ops-toolbar" style="display: flex; align-items: center; justify-content: space-between; padding: 8px 12px; background: var(--bg-sunken); border-bottom: 1px solid var(--line);">
                <div style="display: flex; align-items: center; gap: 8px;">
                    <span style="font-weight: 600; font-size: 13px; display: flex; align-items: center; gap: 6px;">
                        <IconActivity size=15 />
                        "Operations Monitor"
                    </span>
                    <span style="background: var(--accent-fill-strong); color: var(--accent-light); font-size: 11px; padding: 2px 6px; border-radius: 10px; font-weight: 600;">
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

                    <FreshnessControls
                        freshness=freshness
                        auto_refresh=signals.auto_refresh
                        loading=signals.loading
                        refresh_now=move |()| load_operations()
                    />
                </div>
            </div>

            // Status message / toast
            {move || signals.status_msg.get().map(|msg| {
                view! {
                    <div class="card-alert-line" role="alert">
                        <span>{msg}</span>
                        <button style="background: none; border: none; color: inherit; cursor: pointer;" on:click=move |_| signals.status_msg.set(None)>"×"</button>
                    </div>
                }
            })}

            // Main Split Layout: Operations list & Log Inspector
            <div style="display: flex; flex: 1; min-height: 0;">
                // Left: Operations list
                <div style="flex: 1; overflow-y: auto; padding: 8px; display: flex; flex-direction: column; gap: 8px; border-right: 1px solid var(--fill-subtle);">
                    <For
                        each=filtered_ops
                        key=|op| op.id
                        children=move |op| {
                            let op_id = op.id;
                            let is_selected = move || signals.selected_op_id.get() == Some(op_id);
                            // An owner that cannot measure progress reports None. Painting a 0%
                            // bar would read as "nothing done"; unknown is not zero.
                            let percent = op.progress.percent;
                            let observation = op.observation;
                            let is_terminal = op.state.is_terminal();
                            let cancellable = op.cancellable && !is_terminal;

                            let (badge_bg, badge_color, badge_text) = match &op.state {
                                OperationState::Running if op.cancellation_requested => ("var(--caution-fill-strong)", "var(--caution)", "Cancelling"),
                                OperationState::Running => ("var(--info-fill-strong)", "var(--info)", "Running"),
                                OperationState::Queued => ("var(--caution-fill-strong)", "var(--caution)", "Queued"),
                                OperationState::Completed => ("var(--ok-fill-strong)", "var(--ok)", "Completed"),
                                OperationState::Failed { .. } => ("var(--danger-fill-strong)", "var(--danger)", "Failed"),
                                OperationState::Cancelled => ("var(--text-dim)", "var(--text-dim)", "Cancelled"),
                                // Nothing ran, so this is not a failure and must not be painted as one.
                                OperationState::Refused { .. } => ("var(--caution-fill-strong)", "var(--caution)", "Refused"),
                            };

                            view! {
                                <div
                                    class=move || if is_selected() { "op-card selected" } else { "op-card" }
                                    style=move || format!(
                                        "background: {}; border: 1px solid {}; border-radius: 6px; padding: 10px; cursor: pointer; transition: all 0.15s ease;",
                                        if is_selected() { "var(--accent-fill)" } else { "var(--fill-faint)" },
                                        if is_selected() { "var(--accent-line-strong)" } else { "var(--line)" }
                                    )
                                    on:click=move |_| {
                                        signals.selected_op_id.set(Some(op_id));
                                        load_logs(op_id);
                                    }
                                >
                                    <div style="display: flex; align-items: flex-start; justify-content: space-between; margin-bottom: 6px;">
                                        <div>
                                            <div style="font-weight: 600; font-size: 12px; color: var(--text-main);">
                                                {op.label.clone()}
                                            </div>
                                            <div style="font-size: 10px; color: var(--text-dim); margin-top: 2px;">
                                                {op.kind.display_label()}
                                            </div>
                                        </div>
                                        <span style=format!("background: {badge_bg}; color: {badge_color}; font-size: 10px; font-weight: 600; padding: 2px 6px; border-radius: 4px; text-transform: uppercase;")>
                                            {badge_text}
                                        </span>
                                    </div>

                                    // Progress bar; indeterminate when the owner cannot measure it.
                                    <div
                                        class=move || if percent.is_none() && !is_terminal { "op-progress indeterminate" } else { "op-progress" }
                                        style="width: 100%; height: 4px; background: var(--line); border-radius: 2px; overflow: hidden; margin: 6px 0;"
                                    >
                                        <div
                                            style=format!(
                                                "width: {}; height: 100%; background: {}; transition: width 0.3s ease;",
                                                percent.map_or_else(
                                                    || if is_terminal { "0%".to_owned() } else { "40%".to_owned() },
                                                    |value| format!("{value}%"),
                                                ),
                                                if matches!(op.state, OperationState::Completed) { "var(--ok)" } else { "var(--accent-solid)" }
                                            )
                                        ></div>
                                    </div>

                                    // Step info
                                    <div style="display: flex; justify-content: space-between; align-items: center; font-size: 11px; color: var(--text-strong);">
                                        <span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 220px;">
                                            {op.progress.step.clone()}
                                        </span>
                                        {percent.map_or_else(
                                            || view! { <span style="font-weight: 600; font-size: 10px; color: var(--text-dim);">{if is_terminal { "" } else { "in progress" }}</span> }.into_any(),
                                            |p| view! { <span style="font-weight: 600; font-size: 10px;">{format!("{p:.0}%")}</span> }.into_any(),
                                        )}
                                    </div>

                                    // Observation line: whether the owner can still see the work.
                                    <div style="font-size: 10px; margin-top: 4px; color: var(--text-dim);">
                                        {match observation {
                                            ObservationState::Known => String::new(),
                                            ObservationState::Stale => "Not confirmed by the executing authority in the last reconciliation".to_owned(),
                                            ObservationState::Detached => "Detached: the executing authority no longer establishes this operation".to_owned(),
                                            ObservationState::Unavailable => "The executing authority cannot be read right now".to_owned(),
                                        }}
                                    </div>

                                    // Footer actions
                                    <div style="display: flex; justify-content: flex-end; gap: 6px; margin-top: 8px;">
                                        {if cancellable {
                                            Some(view! {
                                                <button
                                                    style="background: var(--danger-fill); color: var(--danger); border: 1px solid var(--danger-line); font-size: 10px; padding: 2px 6px; border-radius: 4px; cursor: pointer; display: flex; align-items: center; gap: 4px;"
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
                            <div style="text-align: center; color: var(--text-faint); padding: 32px 16px; font-size: 12px;">
                                "No operations matching filter."
                            </div>
                        })
                    } else {
                        None
                    }}
                </div>

                // Right: Execution Log stream
                <div style="flex: 1.2; display: flex; flex-direction: column; background: var(--bg-sunken-strong); font-family: monospace; font-size: 11px;">
                    <div style="padding: 6px 12px; background: var(--bg-sunken); border-bottom: 1px solid var(--fill-subtle); display: flex; justify-content: space-between; align-items: center;">
                        <span style="font-weight: 600; color: var(--text-strong);">"Execution Logs"</span>
                        {move || signals.selected_op_id.get().map(|id| view! {
                            <span style="font-size: 10px; color: var(--text-faint);">{format!("id: {}", &id.to_string()[..8])}</span>
                        })}
                    </div>

                    <div style="flex: 1; overflow-y: auto; padding: 8px; display: flex; flex-direction: column; gap: 4px;">
                        <For
                            each=move || signals.selected_logs.get()
                            key=|log| format!("{}-{}", log.timestamp, log.text)
                            children=move |log| {
                                let stream_color = match log.stream.as_str() {
                                    "stderr" => "var(--danger)",
                                    "system" => "var(--caution)",
                                    _ => "var(--text-dim)",
                                };
                                view! {
                                    <div style="display: flex; gap: 8px; line-height: 1.4;">
                                        <span style=format!("color: {stream_color}; font-size: 10px; min-width: 48px; user-select: none;")>
                                            {format!("[{}]", log.stream)}
                                        </span>
                                        <span style="color: var(--text-main); word-break: break-all;">
                                            {log.text}
                                        </span>
                                    </div>
                                }
                            }
                        />

                        {move || if signals.selected_logs.get().is_empty() {
                            Some(view! {
                                <div style="text-align: center; color: var(--fill-hover); padding: 32px 16px;">
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

#[cfg(test)]
mod tests {
    use super::retained_selection;
    use cybou_protocol::{
        action::Proposer,
        operation::{OperationKind, OperationProgress, OperationRecord, OperationState},
    };
    use time::OffsetDateTime;
    use uuid::Uuid;

    fn operation(id: Uuid) -> OperationRecord {
        OperationRecord {
            id,
            kind: OperationKind::AgentTask,
            state: OperationState::Running,
            label: "Agent task".to_owned(),
            initiator: Proposer::Mind,
            subject: None,
            progress: OperationProgress {
                percent: None,
                step: "Running".to_owned(),
                total_steps: None,
                current_step: None,
                detail: None,
            },
            cancellable: true,
            establisher: None,
            cancellation_requested: false,
            observation: cybou_protocol::operation::ObservationState::Known,
            last_observed_at: None,
            started_at: OffsetDateTime::UNIX_EPOCH,
            updated_at: OffsetDateTime::UNIX_EPOCH,
            finished_at: None,
        }
    }

    #[test]
    fn reconnect_keeps_only_a_selection_the_owner_still_has() {
        let selected = Uuid::new_v4();
        assert_eq!(
            retained_selection(Some(selected), &[operation(selected)]),
            Some(selected)
        );
        assert_eq!(retained_selection(Some(selected), &[]), None);
        assert_eq!(retained_selection(None, &[operation(selected)]), None);
    }
}
