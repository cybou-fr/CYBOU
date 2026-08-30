// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Process Manager card component for monitoring and controlling OS processes.

use cybou_protocol::SubjectRef;
use cybou_protocol::system::ProcessSignal;
use leptos::prelude::*;

use crate::{
    CardId, MindClient,
    components::icons::{IconLayers, IconRefresh},
    tool_state::ToolCardStates,
};

#[component]
pub fn ProcessesContent(card: CardId) -> impl IntoView {
    let client = crate::GatewayMindClient;
    let tool_states = expect_context::<ToolCardStates>();
    let signals = tool_states.processes(card);

    let load_processes = move || {
        signals.loading.set(true);
        leptos::task::spawn_local(async move {
            match client.list_processes().await {
                Ok(projection) => {
                    signals.processes.set(projection.processes);
                    signals.status_msg.set(None);
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
                Ok(outcome) => {
                    signals.status_msg.set(Some(outcome));
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
            "name" => list.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
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
        <div class="processes-panel" style="display: flex; flex-direction: column; height: 100%; width: 100%; background: var(--bg-card, #1e1e24); color: var(--text-main, #e0e0e0); font-family: system-ui, -apple-system, sans-serif;">
            // Header summary
            <div style="display: flex; flex-direction: column; gap: 8px; padding: 10px 12px; background: rgba(0,0,0,0.2); border-bottom: 1px solid rgba(255,255,255,0.08);">
                <div style="display: flex; align-items: center; justify-content: space-between;">
                    <div style="display: flex; align-items: center; gap: 10px;">
                        <span style="font-weight: 600; font-size: 13px;">"Process Manager"</span>
                        <span style="font-size: 11px; background: rgba(255,255,255,0.06); padding: 2px 6px; border-radius: 8px;">
                            {move || format!("{} Processes", signals.processes.get().len())}
                        </span>
                        <span style="font-size: 11px; color: #818cf8; font-weight: 600;">
                            {move || format!("CPU: {}", total_cpu())}
                        </span>
                        <span style="font-size: 11px; color: #38bdf8; font-weight: 600;">
                            {move || format!("RAM: {}", total_mem_mb())}
                        </span>
                    </div>

                    <button
                        style="background: rgba(255,255,255,0.06); border: none; border-radius: 4px; padding: 4px 6px; color: inherit; cursor: pointer;"
                        title="Refresh processes"
                        on:click=move |_| load_processes()
                    >
                        <IconRefresh size=13 />
                    </button>
                </div>

                // Search & Sort bar
                <div style="display: flex; align-items: center; justify-content: space-between; gap: 8px;">
                    <div style="display: flex; gap: 4px; align-items: center;">
                        <span style="font-size: 10px; color: rgba(255,255,255,0.5);">"Sort:"</span>
                        <button
                            style=move || format!(
                                "background: {}; border: none; font-size: 10px; padding: 2px 6px; border-radius: 4px; color: inherit; cursor: pointer;",
                                if signals.sort_by.get() == "cpu" { "#6366f1" } else { "rgba(255,255,255,0.06)" }
                            )
                            on:click=move |_| signals.sort_by.set("cpu".to_owned())
                        >
                            "CPU"
                        </button>
                        <button
                            style=move || format!(
                                "background: {}; border: none; font-size: 10px; padding: 2px 6px; border-radius: 4px; color: inherit; cursor: pointer;",
                                if signals.sort_by.get() == "memory" { "#6366f1" } else { "rgba(255,255,255,0.06)" }
                            )
                            on:click=move |_| signals.sort_by.set("memory".to_owned())
                        >
                            "Memory"
                        </button>
                        <button
                            style=move || format!(
                                "background: {}; border: none; font-size: 10px; padding: 2px 6px; border-radius: 4px; color: inherit; cursor: pointer;",
                                if signals.sort_by.get() == "pid" { "#6366f1" } else { "rgba(255,255,255,0.06)" }
                            )
                            on:click=move |_| signals.sort_by.set("pid".to_owned())
                        >
                            "PID"
                        </button>
                        <button
                            style=move || format!(
                                "background: {}; border: none; font-size: 10px; padding: 2px 6px; border-radius: 4px; color: inherit; cursor: pointer;",
                                if signals.sort_by.get() == "name" { "#6366f1" } else { "rgba(255,255,255,0.06)" }
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
                        style="background: rgba(0,0,0,0.3); border: 1px solid rgba(255,255,255,0.1); border-radius: 4px; padding: 3px 8px; font-size: 11px; color: inherit; width: 130px;"
                    />
                </div>
            </div>

            // Status message toast
            {move || signals.status_msg.get().map(|msg| {
                view! {
                    <div style="background: rgba(99, 102, 241, 0.15); color: #c7d2fe; font-size: 11px; padding: 6px 12px; border-bottom: 1px solid rgba(99, 102, 241, 0.3); display: flex; justify-content: space-between;">
                        <span>{msg}</span>
                        <button style="background: none; border: none; color: inherit; cursor: pointer;" on:click=move |_| signals.status_msg.set(None)>"×"</button>
                    </div>
                }
            })}

            // Process Table Header
            <div style="display: grid; grid-template-columns: 60px 1fr 70px 75px 65px 90px; padding: 6px 12px; font-size: 10px; font-weight: 700; color: rgba(255,255,255,0.4); border-bottom: 1px solid rgba(255,255,255,0.05); text-transform: uppercase;">
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
                                style="display: grid; grid-template-columns: 60px 1fr 70px 75px 65px 90px; align-items: center; padding: 6px 12px; font-size: 11px; border-bottom: 1px solid rgba(255,255,255,0.03); transition: background 0.1s ease;"
                                class="proc-row"
                            >
                                <span style="font-family: monospace; color: rgba(255,255,255,0.5);">{pid}</span>
                                <div style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 600; color: #f3f4f6;" title=p.cmdline.clone()>
                                    {name}
                                </div>
                                <span style="color: rgba(255,255,255,0.6); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">{p.user}</span>
                                <span style={if p.cpu_percent > 2.0 { "text-align: right; font-family: monospace; color: #818cf8; font-weight: 600;" } else { "text-align: right; font-family: monospace; color: inherit; font-weight: normal;" }}>
                                    {format!("{:.1}%", p.cpu_percent)}
                                </span>
                                <span style="text-align: right; font-family: monospace; color: rgba(255,255,255,0.8);">
                                    {format!("{mem_mb} MB")}
                                </span>
                                <div style="display: flex; justify-content: flex-end; gap: 4px;">
                                    <button
                                        style="background: rgba(239, 68, 68, 0.15); border: 1px solid rgba(239, 68, 68, 0.3); border-radius: 3px; padding: 2px 5px; font-size: 9px; color: #f87171; cursor: pointer;"
                                        title="Send SIGTERM"
                                        on:click=move |_| send_signal(pid, ProcessSignal::Terminate)
                                    >
                                        "Term"
                                    </button>
                                    <button
                                        style="background: rgba(99, 102, 241, 0.15); border: 1px solid rgba(99, 102, 241, 0.3); border-radius: 3px; padding: 2px 4px; font-size: 9px; color: #818cf8; cursor: pointer;"
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
                        <div style="text-align: center; color: rgba(255,255,255,0.4); padding: 32px 16px; font-size: 12px;">
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
