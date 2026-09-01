// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! System and Journald Log Viewer card component.

use cybou_web_contracts::SystemLogsQueryRequest;
use leptos::prelude::*;

use crate::{
    CardId, MindClient, components::freshness::FreshnessControls, refresh::Freshness,
    tool_state::ToolCardStates,
};

/// How often the log view is re-read while the panel is open and visible.
///
/// Ten seconds. A log is the one panel where being behind is normal — lines arrive when they
/// arrive — but a viewer that never came back is a viewer showing the last thing that happened
/// before you looked away.
const LOGS_INTERVAL_MS: u32 = 10_000;

#[component]
pub fn SystemLogsContent(card: CardId) -> impl IntoView {
    let client = crate::GatewayMindClient;
    let tool_states = expect_context::<ToolCardStates>();
    let signals = tool_states.system_logs(card);

    let freshness = Freshness::new();

    let load_logs = move || {
        signals.loading.set(true);
        leptos::task::spawn_local(async move {
            let req = SystemLogsQueryRequest {
                unit: signals.selected_unit.get(),
                severity: signals.selected_severity.get(),
                search: if signals.search_query.get().is_empty() {
                    None
                } else {
                    Some(signals.search_query.get())
                },
                limit: Some(200),
            };
            match client.get_system_logs(&req).await {
                Ok(proj) => {
                    signals.logs.set(proj.logs);
                    signals.unavailable.set(proj.unavailable);
                    signals
                        .system_journal_readable
                        .set(proj.system_journal_readable);
                    signals.status_msg.set(None);
                    freshness.arrived();
                }
                Err(err) => {
                    signals
                        .status_msg
                        .set(Some(format!("Failed to load logs: {err}")));
                }
            }
            signals.loading.set(false);
        });
    };

    // Trigger initial load
    Effect::new(move |_| {
        load_logs();
    });

    crate::refresh::keep_reading(
        LOGS_INTERVAL_MS,
        signals.auto_refresh,
        signals.loading,
        load_logs,
    );

    view! {
        <div class="system-logs-panel" style="display: flex; flex-direction: column; height: 100%; width: 100%; background: var(--surface); font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;">
            // Control Toolbar
            <div style="display: flex; flex-direction: column; gap: 8px; padding: 10px 12px; background: var(--bg-sunken-strong); border-bottom: 1px solid var(--line); ">
                <div style="display: flex; align-items: center; justify-content: space-between;">
                    <div style="display: flex; align-items: center; gap: 8px;">
                        <span style="font-weight: 600; font-size: 13px;">"System Logs & Journal"</span>
                        <span style="font-size: 10px; background: var(--fill-subtle); padding: 2px 6px; border-radius: 8px; font-family: monospace;">
                            {move || format!("{} entries", signals.logs.get().len())}
                        </span>
                    </div>

                    <FreshnessControls
                        freshness=freshness
                        auto_refresh=signals.auto_refresh
                        loading=signals.loading
                        refresh_now=move |()| load_logs()
                    />
                </div>

                // Filter row
                <div style="display: flex; align-items: center; justify-content: space-between; gap: 8px; flex-wrap: wrap;">
                    <div style="display: flex; gap: 4px; align-items: center;">
                        <button
                            style=move || format!(
                                "background: {}; border: none; font-size: 10px; padding: 2px 6px; border-radius: 4px; color: inherit; cursor: pointer;",
                                if signals.selected_severity.get().is_none() { "var(--accent-solid)" } else { "var(--fill-subtle)" }
                            )
                            on:click=move |_| {
                                signals.selected_severity.set(None);
                                load_logs();
                            }
                        >
                            "All"
                        </button>
                        <button
                            style=move || format!(
                                "background: {}; border: none; font-size: 10px; padding: 2px 6px; border-radius: 4px; color: var(--danger); cursor: pointer;",
                                if signals.selected_severity.get().as_deref() == Some("err") { "var(--danger-line)" } else { "var(--fill-subtle)" }
                            )
                            on:click=move |_| {
                                signals.selected_severity.set(Some("err".to_owned()));
                                load_logs();
                            }
                        >
                            "Errors"
                        </button>
                        <button
                            style=move || format!(
                                "background: {}; border: none; font-size: 10px; padding: 2px 6px; border-radius: 4px; color: var(--caution); cursor: pointer;",
                                if signals.selected_severity.get().as_deref() == Some("warning") { "var(--caution-line)" } else { "var(--fill-subtle)" }
                            )
                            on:click=move |_| {
                                signals.selected_severity.set(Some("warning".to_owned()));
                                load_logs();
                            }
                        >
                            "Warnings"
                        </button>
                        <button
                            style=move || format!(
                                "background: {}; border: none; font-size: 10px; padding: 2px 6px; border-radius: 4px; color: var(--info); cursor: pointer;",
                                if signals.selected_severity.get().as_deref() == Some("info") { "var(--info-line)" } else { "var(--fill-subtle)" }
                            )
                            on:click=move |_| {
                                signals.selected_severity.set(Some("info".to_owned()));
                                load_logs();
                            }
                        >
                            "Info"
                        </button>
                    </div>

                    <input
                        type="text"
                        placeholder="Search logs..."
                        prop:value=move || signals.search_query.get()
                        on:input=move |e| {
                            signals.search_query.set(event_target_value(&e));
                            load_logs();
                        }
                        style="background: var(--bg-sunken-strong); border: 1px solid var(--fill-hover); border-radius: 4px; padding: 3px 8px; font-size: 11px; color: inherit; width: 140px;"
                    />
                </div>
            </div>

            // A reader outside the systemd-journal group is not refused; it is narrowed to its
            // own account. Without this line the feed would look complete and be one service's
            // half of the host.
            {move || (!signals.system_journal_readable.get() && signals.unavailable.get().is_none()).then(|| {
                view! {
                    <div style="background: var(--caution-fill); color: var(--caution); font-size: 11px; padding: 6px 12px; border-bottom: 1px solid var(--caution-line); font-family: system-ui;">
                        "Only this account's own journal is visible. Add the gateway account to the systemd-journal group to read the whole host."
                    </div>
                }
            })}

            // Status message toast
            {move || signals.status_msg.get().map(|msg| {
                view! {
                    <div class="card-status-line" role="status" aria-live="polite">
                        <span>{msg}</span>
                        <button class="card-status-dismiss" title="Dismiss" on:click=move |_| signals.status_msg.set(None)>"×"</button>
                    </div>
                }
            })}

            // Log Feed
            <div style="flex: 1; overflow-y: auto; padding: 8px 12px; display: flex; flex-direction: column; gap: 4px; font-size: 11px; line-height: 1.4;">
                <For
                    each=move || signals.logs.get()
                    key=|l| format!("{}-{}", l.timestamp, l.message)
                    children=move |entry| {
                        let (sev_color, sev_bg) = match entry.severity.as_str() {
                            "emerg" | "alert" | "crit" | "err" => ("var(--danger)", "var(--danger-fill)"),
                            "warning" => ("var(--caution)", "var(--caution-fill)"),
                            "notice" => ("var(--ok)", "var(--ok-fill)"),
                            _ => ("var(--text-muted)", "var(--text-dim)"),
                        };

                        view! {
                            <div style="display: flex; gap: 8px; align-items: flex-start; word-break: break-all;">
                                <span style="color: var(--text-faint); flex-shrink: 0; font-size: 10px;">
                                    {entry.timestamp}
                                </span>
                                <span style=format!("color: {sev_color}; background: {sev_bg}; padding: 0 4px; border-radius: 2px; font-size: 9px; font-weight: 700; text-transform: uppercase; flex-shrink: 0;")>
                                    {entry.severity}
                                </span>
                                {entry.unit.map(|u| view! {
                                    <span style="color: var(--accent-light); flex-shrink: 0; font-size: 10px;">
                                        {format!("[{u}]")}
                                    </span>
                                })}
                                <span style="color: var(--text-bright); flex: 1;">
                                    {entry.message}
                                </span>
                            </div>
                        }
                    }
                />

                // An empty feed is two different facts. A journal that answered and matched
                // nothing is a quiet host; a journal this reader cannot hear is an unknown one,
                // and drawing the first for the second is how a viewer reports silence on a
                // machine that is talking.
                {move || if signals.logs.get().is_empty() {
                    let text = signals.unavailable.get().map_or_else(
                        || "No log entries matching query.".to_owned(),
                        |reason| format!("The journal was not read: {}.", reason.explain()),
                    );
                    Some(view! {
                        <div style="text-align: center; color: var(--text-faint); padding: 32px 16px; font-size: 12px; font-family: system-ui;">
                            {text}
                        </div>
                    })
                } else {
                    None
                }}
            </div>
        </div>
    }
}
