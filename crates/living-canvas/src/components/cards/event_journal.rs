// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Canonical Event1 Journal timeline & replay card component.

use crate::{
    CardId, MindClient,
    components::icons::{IconLayers, IconRefresh},
    tool_state::ToolCardStates,
};
use leptos::prelude::*;

#[component]
pub fn EventJournalContent(card: CardId) -> impl IntoView {
    let client = crate::GatewayMindClient;
    let tool_states = expect_context::<ToolCardStates>();
    let signals = tool_states.event_journal(card);

    let load_journal = move || {
        signals.loading.set(true);
        leptos::task::spawn_local(async move {
            match client.get_event_journal(Some(100), None).await {
                Ok(proj) => {
                    signals.journal.set(Some(proj));
                    signals.status_msg.set(None);
                }
                Err(err) => {
                    signals
                        .status_msg
                        .set(Some(format!("Failed to load journal: {err}")));
                }
            }
            signals.loading.set(false);
        });
    };

    // Trigger initial load
    Effect::new(move |_| {
        load_journal();
    });

    view! {
        <div class="event-journal-panel" style="display: flex; flex-direction: column; height: 100%; width: 100%; background: var(--bg-card); color: var(--text-main); font-family: system-ui, -apple-system, sans-serif; overflow: hidden;">
            // Header
            <div style="display: flex; align-items: center; justify-content: space-between; padding: 10px 12px; background: var(--bg-sunken); border-bottom: 1px solid var(--line);">
                <div style="display: flex; align-items: center; gap: 8px;">
                    <IconLayers size=16 />
                    <span style="font-weight: 600; font-size: 13px;">"Canonical Event1 Journal"</span>
                </div>
                <div style="display: flex; align-items: center; gap: 6px;">
                    <input
                        type="text"
                        placeholder="Search event log..."
                        prop:value=move || signals.search_query.get()
                        on:input=move |e| signals.search_query.set(event_target_value(&e))
                        style="background: var(--bg-sunken-strong); border: 1px solid var(--fill-hover); border-radius: 4px; padding: 4px 8px; font-size: 11px; color: inherit; width: 140px;"
                    />
                    <button
                        style="background: var(--fill-subtle); border: none; border-radius: 4px; padding: 4px 6px; color: inherit; cursor: pointer;"
                        title="Refresh journal"
                        on:click=move |_| load_journal()
                    >
                        <IconRefresh size=13 />
                    </button>
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

            // Journal Timeline List
            <div style="flex: 1; overflow-y: auto; padding: 12px; display: flex; flex-direction: column; gap: 8px;">
                {move || signals.journal.get().map(|jnl| {
                    let q = signals.search_query.get().to_lowercase();
                    jnl.entries.into_iter()
                        .filter(|e| q.is_empty() || e.summary.to_lowercase().contains(&q) || e.event_type.to_lowercase().contains(&q) || e.origin_organ.to_lowercase().contains(&q))
                        .map(|entry| {
                            let (badge_bg, badge_fg) = match entry.origin_organ.as_str() {
                                "actiond" => ("var(--accent-fill-strong)", "var(--accent-light)"),
                                "agentd" => ("rgba(236, 72, 153, 0.2)", "#f472b6"),
                                "securityd" => ("var(--danger-fill-strong)", "var(--danger)"),
                                _ => ("rgba(16, 185, 129, 0.2)", "#34d399"),
                            };

                            view! {
                                <div style="background: var(--bg-sunken); border: 1px solid var(--fill-subtle); border-radius: 6px; padding: 10px 12px; display: flex; flex-direction: column; gap: 4px;">
                                    <div style="display: flex; align-items: center; justify-content: space-between; font-size: 11px;">
                                        <div style="display: flex; align-items: center; gap: 6px;">
                                            <span style=format!("font-size: 9px; font-weight: 700; padding: 2px 6px; border-radius: 3px; background: {}; color: {}; text-transform: uppercase;", badge_bg, badge_fg)>
                                                {entry.origin_organ}
                                            </span>
                                            <span style="font-weight: 700; color: var(--text-bright);">{entry.event_type}</span>
                                        </div>
                                        <span style="font-size: 10px; color: var(--text-faint); font-family: monospace;">
                                            {entry.timestamp}
                                        </span>
                                    </div>
                                    <div style="font-size: 12px; color: var(--text-main); margin-top: 2px;">
                                        {entry.summary}
                                    </div>
                                    <div style="font-size: 10px; font-family: monospace; color: var(--text-dim); background: rgba(0,0,0,0.25); padding: 4px 6px; border-radius: 3px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                                        {entry.payload_preview}
                                    </div>
                                </div>
                            }
                        }).collect::<Vec<_>>()
                })}
            </div>
        </div>
    }
}
