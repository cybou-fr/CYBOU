// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! System and Journald Log Viewer card component.

use leptos::prelude::*;
use cybou_web_contracts::SystemLogsQueryRequest;

use crate::{
    MindClient,
    CardId,
    components::icons::IconRefresh,
    state::RuntimeState,
    tool_state::ToolCardStates,
};

#[component]
pub fn SystemLogsContent(card: CardId) -> impl IntoView {
    let client = crate::GatewayMindClient;
    let tool_states = expect_context::<ToolCardStates>();
    let signals = tool_states.system_logs(card);

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
                    signals.status_msg.set(None);
                }
                Err(err) => {
                    signals.status_msg.set(Some(format!("Failed to load logs: {err}")));
                }
            }
            signals.loading.set(false);
        });
    };

    // Trigger initial load
    Effect::new(move |_| {
        load_logs();
    });

    view! {
        <div class="system-logs-panel" style="display: flex; flex-direction: column; height: 100%; width: 100%; background: #131418; color: var(--text-main, #e0e0e0); font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;">
            // Control Toolbar
            <div style="display: flex; flex-direction: column; gap: 8px; padding: 10px 12px; background: rgba(0,0,0,0.3); border-bottom: 1px solid rgba(255,255,255,0.08); font-family: system-ui, -apple-system, sans-serif;">
                <div style="display: flex; align-items: center; justify-content: space-between;">
                    <div style="display: flex; align-items: center; gap: 8px;">
                        <span style="font-weight: 600; font-size: 13px;">"System Logs & Journal"</span>
                        <span style="font-size: 10px; background: rgba(255,255,255,0.06); padding: 2px 6px; border-radius: 8px; font-family: monospace;">
                            {move || format!("{} entries", signals.logs.get().len())}
                        </span>
                    </div>

                    <button
                        style="background: rgba(255,255,255,0.06); border: none; border-radius: 4px; padding: 4px 6px; color: inherit; cursor: pointer;"
                        title="Reload logs"
                        on:click=move |_| load_logs()
                    >
                        <IconRefresh size=13 />
                    </button>
                </div>

                // Filter row
                <div style="display: flex; align-items: center; justify-content: space-between; gap: 8px; flex-wrap: wrap;">
                    <div style="display: flex; gap: 4px; align-items: center;">
                        <button
                            style=move || format!(
                                "background: {}; border: none; font-size: 10px; padding: 2px 6px; border-radius: 4px; color: inherit; cursor: pointer;",
                                if signals.selected_severity.get().is_none() { "#6366f1" } else { "rgba(255,255,255,0.06)" }
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
                                "background: {}; border: none; font-size: 10px; padding: 2px 6px; border-radius: 4px; color: #f87171; cursor: pointer;",
                                if signals.selected_severity.get().as_deref() == Some("err") { "rgba(239,68,68,0.3)" } else { "rgba(255,255,255,0.06)" }
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
                                "background: {}; border: none; font-size: 10px; padding: 2px 6px; border-radius: 4px; color: #fbbf24; cursor: pointer;",
                                if signals.selected_severity.get().as_deref() == Some("warning") { "rgba(245,158,11,0.3)" } else { "rgba(255,255,255,0.06)" }
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
                                "background: {}; border: none; font-size: 10px; padding: 2px 6px; border-radius: 4px; color: #60a5fa; cursor: pointer;",
                                if signals.selected_severity.get().as_deref() == Some("info") { "rgba(59,130,246,0.3)" } else { "rgba(255,255,255,0.06)" }
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
                        style="background: rgba(0,0,0,0.3); border: 1px solid rgba(255,255,255,0.1); border-radius: 4px; padding: 3px 8px; font-size: 11px; color: inherit; width: 140px;"
                    />
                </div>
            </div>

            // Status message toast
            {move || signals.status_msg.get().map(|msg| {
                view! {
                    <div style="background: rgba(99, 102, 241, 0.15); color: #c7d2fe; font-size: 11px; padding: 6px 12px; border-bottom: 1px solid rgba(99, 102, 241, 0.3); display: flex; justify-content: space-between; font-family: system-ui;">
                        <span>{msg}</span>
                        <button style="background: none; border: none; color: inherit; cursor: pointer;" on:click=move |_| signals.status_msg.set(None)>"×"</button>
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
                            "emerg" | "alert" | "crit" | "err" => ("#f87171", "rgba(239,68,68,0.15)"),
                            "warning" => ("#fbbf24", "rgba(245,158,11,0.15)"),
                            "notice" => ("#34d399", "rgba(52,211,153,0.15)"),
                            _ => ("#94a3b8", "rgba(148,163,184,0.1)"),
                        };

                        view! {
                            <div style="display: flex; gap: 8px; align-items: flex-start; word-break: break-all;">
                                <span style="color: rgba(255,255,255,0.4); flex-shrink: 0; font-size: 10px;">
                                    {entry.timestamp}
                                </span>
                                <span style=format!("color: {sev_color}; background: {sev_bg}; padding: 0 4px; border-radius: 2px; font-size: 9px; font-weight: 700; text-transform: uppercase; flex-shrink: 0;")>
                                    {entry.severity}
                                </span>
                                {entry.unit.map(|u| view! {
                                    <span style="color: #818cf8; flex-shrink: 0; font-size: 10px;">
                                        {format!("[{u}]")}
                                    </span>
                                })}
                                <span style="color: #f1f5f9; flex: 1;">
                                    {entry.message}
                                </span>
                            </div>
                        }
                    }
                />

                {move || if signals.logs.get().is_empty() {
                    Some(view! {
                        <div style="text-align: center; color: rgba(255,255,255,0.4); padding: 32px 16px; font-size: 12px; font-family: system-ui;">
                            "No log entries matching query."
                        </div>
                    })
                } else {
                    None
                }}
            </div>
        </div>
    }
}
