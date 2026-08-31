// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Process Manager card component for monitoring and controlling OS processes.

use cybou_protocol::SubjectRef;
use cybou_protocol::system::ProcessSignal;
use leptos::prelude::*;

use crate::{
    CardId, MindClient,
    components::{freshness::FreshnessControls, icons::IconLayers},
    refresh::Freshness,
    tool_state::ToolCardStates,
};

/// How often the process table is re-read while the panel is open and visible.
///
/// Five seconds, like the monitor. A process table is read to find what is consuming the machine
/// right now, and one that lags behind sends a person to kill something that already exited.
const PROCESSES_INTERVAL_MS: u32 = 5_000;

#[component]
pub fn ProcessesContent(card: CardId) -> impl IntoView {
    let client = crate::GatewayMindClient;
    let tool_states = expect_context::<ToolCardStates>();
    let signals = tool_states.processes(card);

    let freshness = Freshness::new();

    let load_processes = move || {
        signals.loading.set(true);
        leptos::task::spawn_local(async move {
            match client.list_processes().await {
                Ok(projection) => {
                    signals.processes.set(projection.processes);
                    signals.status_msg.set(None);
                    freshness.arrived();
                }
                Err(err) => {
                    signals
                        .status_msg
                        .set(Some(format!("Failed to load processes: {err}")));
                }
            }
            signals.loading.set(false);
        });
    };

    let send_signal = move |pid: u32, sig: ProcessSignal| {
        leptos::task::spawn_local(async move {
            match client.send_process_signal(pid, sig).await {
                Ok(record) => {
                    // The boundary's own words, the way the Services panel reports them. This
                    // panel used to print "Signal delivered" whatever came back, which was a
                    // sentence about what the button meant rather than about what happened.
                    let said = match record.verdict.as_str() {
                        "granted-on-confirmation" | "granted" => {
                            record.attempt.as_ref().map_or_else(
                                || format!("Process {pid}: authorized, waiting to be carried out."),
                                |attempt| format!("Process {pid}: {}.", attempt.report),
                            )
                        }
                        _ => record.verdict_reason.clone().unwrap_or_else(|| {
                            format!("Process {pid}: refused, and no reason was given.")
                        }),
                    };
                    signals.status_msg.set(Some(said));
                    load_processes();
                }
                Err(err) => {
                    signals
                        .status_msg
                        .set(Some(format!("Signal failed: {err}")));
                }
            }
        });
    };

    let inspect_process = move |pid: u32, name: String| {
        let inspector_signals = tool_states.inspector(CardId::Inspector(0));
        inspector_signals
            .target_subject
            .set(Some(SubjectRef::Process { pid, name }));
    };

    // Trigger initial load
    Effect::new(move |_| {
        load_processes();
    });

    crate::refresh::keep_reading(
        PROCESSES_INTERVAL_MS,
        signals.auto_refresh,
        signals.loading,
        load_processes,
    );

    let sorted_and_filtered = move || {
        let mut list = signals.processes.get();
        let search = signals.search_query.get().to_lowercase();
        let sort = signals.sort_by.get();

        list.retain(|p| {
            if search.is_empty() {
                true
            } else {
                p.name.to_lowercase().contains(&search)
                    || p.user.to_lowercase().contains(&search)
                    || p.pid.to_string().contains(&search)
            }
        });

        match sort.as_str() {
            "cpu" => list.sort_by(|a, b| {
                b.cpu_percent
                    .partial_cmp(&a.cpu_percent)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            "memory" => list.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes)),
            "pid" => list.sort_by_key(|p| p.pid),
            "name" => list.sort_by_key(|a| a.name.to_lowercase()),
            _ => {}
        }
        list
    };

    let total_cpu = move || {
        let sum: f32 = signals.processes.get().iter().map(|p| p.cpu_percent).sum();
        format!("{sum:.1}%")
    };
    let total_mem_mb = move || {
        let bytes: u64 = signals.processes.get().iter().map(|p| p.memory_bytes).sum();
        format!("{} MB", bytes / (1024 * 1024))
    };

    view! {
        <div class="processes-panel" style="display: flex; flex-direction: column; height: 100%; width: 100%; ">
            // Header summary
            <div style="display: flex; flex-direction: column; gap: 8px; padding: 10px 12px; background: var(--bg-sunken); border-bottom: 1px solid var(--line);">
                <div style="display: flex; align-items: center; justify-content: space-between;">
                    <div style="display: flex; align-items: center; gap: 10px;">
                        <span style="font-weight: 600; font-size: 13px;">"Process Manager"</span>
                        <span style="font-size: 11px; background: var(--fill-subtle); padding: 2px 6px; border-radius: 8px;">
                            {move || format!("{} Processes", signals.processes.get().len())}
                        </span>
                        <span style="font-size: 11px; color: var(--accent-light); font-weight: 600;">
                            {move || format!("CPU: {}", total_cpu())}
                        </span>
                        <span style="font-size: 11px; color: #38bdf8; font-weight: 600;">
                            {move || format!("RAM: {}", total_mem_mb())}
                        </span>
                    </div>

                    <FreshnessControls
                        freshness=freshness
                        auto_refresh=signals.auto_refresh
                        loading=signals.loading
                        refresh_now=move |()| load_processes()
                    />
                </div>

                // Search & Sort bar
                <div style="display: flex; align-items: center; justify-content: space-between; gap: 8px;">
                    <div style="display: flex; gap: 4px; align-items: center;">
                        <span style="font-size: 10px; color: var(--text-dim);">"Sort:"</span>
                        <button
                            style=move || format!(
                                "background: {}; border: none; font-size: 10px; padding: 2px 6px; border-radius: 4px; color: inherit; cursor: pointer;",
                                if signals.sort_by.get() == "cpu" { "var(--accent-solid)" } else { "var(--fill-subtle)" }
                            )
                            on:click=move |_| signals.sort_by.set("cpu".to_owned())
                        >
                            "CPU"
                        </button>
                        <button
                            style=move || format!(
                                "background: {}; border: none; font-size: 10px; padding: 2px 6px; border-radius: 4px; color: inherit; cursor: pointer;",
                                if signals.sort_by.get() == "memory" { "var(--accent-solid)" } else { "var(--fill-subtle)" }
                            )
                            on:click=move |_| signals.sort_by.set("memory".to_owned())
                        >
                            "Memory"
                        </button>
                        <button
                            style=move || format!(
                                "background: {}; border: none; font-size: 10px; padding: 2px 6px; border-radius: 4px; color: inherit; cursor: pointer;",
                                if signals.sort_by.get() == "pid" { "var(--accent-solid)" } else { "var(--fill-subtle)" }
                            )
                            on:click=move |_| signals.sort_by.set("pid".to_owned())
                        >
                            "PID"
                        </button>
                        <button
                            style=move || format!(
                                "background: {}; border: none; font-size: 10px; padding: 2px 6px; border-radius: 4px; color: inherit; cursor: pointer;",
                                if signals.sort_by.get() == "name" { "var(--accent-solid)" } else { "var(--fill-subtle)" }
                            )
                            on:click=move |_| signals.sort_by.set("name".to_owned())
                        >
                            "Name"
                        </button>
                    </div>

                    <input
                        type="text"
                        placeholder="Filter processes..."
                        prop:value=move || signals.search_query.get()
                        on:input=move |e| signals.search_query.set(event_target_value(&e))
                        style="background: var(--bg-sunken-strong); border: 1px solid var(--fill-hover); border-radius: 4px; padding: 3px 8px; font-size: 11px; color: inherit; width: 130px;"
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

            // Process Table Header
            <div style="display: grid; grid-template-columns: 60px 1fr 70px 75px 65px 90px; padding: 6px 12px; font-size: 10px; font-weight: 700; color: var(--text-faint); border-bottom: 1px solid var(--fill-faintest); text-transform: uppercase;">
                <span>"PID"</span>
                <span>"Process"</span>
                <span>"User"</span>
                <span style="text-align: right;">"CPU"</span>
                <span style="text-align: right;">"RAM"</span>
                <span style="text-align: right;">"Actions"</span>
            </div>

            // Process Table Rows
            <div style="flex: 1; overflow-y: auto; display: flex; flex-direction: column;">
                <For
                    each=sorted_and_filtered
                    key=|p| p.pid
                    children=move |p| {
                        let pid = p.pid;
                        let name = p.name.clone();
                        let name_inspect = p.name.clone();
                        let mem_mb = p.memory_bytes / (1024 * 1024);

                        view! {
                            <div
                                style="display: grid; grid-template-columns: 60px 1fr 70px 75px 65px 90px; align-items: center; padding: 6px 12px; font-size: 11px; border-bottom: 1px solid var(--fill-faint); transition: background 0.1s ease;"
                                class="proc-row"
                            >
                                <span style="font-family: monospace; color: var(--text-dim);">{pid}</span>
                                <div style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 600; color: var(--text-bright);" title=p.cmdline.clone()>
                                    {name}
                                </div>
                                <span style="color: var(--text-second); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">{p.user}</span>
                                <span style={if p.cpu_percent > 2.0 { "text-align: right; font-family: monospace; color: var(--accent-light); font-weight: 600;" } else { "text-align: right; font-family: monospace; color: inherit; font-weight: normal;" }}>
                                    {format!("{:.1}%", p.cpu_percent)}
                                </span>
                                <span style="text-align: right; font-family: monospace; color: rgba(255,255,255,0.8);">
                                    {format!("{mem_mb} MB")}
                                </span>
                                <div style="display: flex; justify-content: flex-end; gap: 4px;">
                                    <button
                                        style="background: var(--danger-fill); border: 1px solid var(--danger-line); border-radius: 3px; padding: 2px 5px; font-size: 9px; color: var(--danger); cursor: pointer;"
                                        title="Send SIGTERM"
                                        on:click=move |_| send_signal(pid, ProcessSignal::Terminate)
                                    >
                                        "Term"
                                    </button>
                                    <button
                                        style="background: var(--accent-fill); border: 1px solid var(--accent-line); border-radius: 3px; padding: 2px 4px; font-size: 9px; color: var(--accent-light); cursor: pointer;"
                                        title="Inspect in Universal Inspector"
                                        on:click=move |_| inspect_process(pid, name_inspect.clone())
                                    >
                                        <IconLayers size=10 />
                                    </button>
                                </div>
                            </div>
                        }
                    }
                />

                {move || if sorted_and_filtered().is_empty() {
                    Some(view! {
                        <div style="text-align: center; color: var(--text-faint); padding: 32px 16px; font-size: 12px;">
                            "No processes matching filter."
                        </div>
                    })
                } else {
                    None
                }}
            </div>
        </div>
    }
}
