// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Personal Calendar & Event Schedule card component.

use crate::{CardId, MindClient, components::icons::IconRefresh, tool_state::ToolCardStates};
use leptos::prelude::*;

#[component]
pub fn CalendarContent(card: CardId) -> impl IntoView {
    let client = crate::GatewayMindClient;
    let tool_states = expect_context::<ToolCardStates>();
    let signals = tool_states.calendar(card);

    let load_calendar = move || {
        signals.loading.set(true);
        leptos::task::spawn_local(async move {
            match client.get_calendar().await {
                Ok(proj) => {
                    signals.calendar.set(Some(proj));
                    signals.status_msg.set(None);
                }
                Err(err) => {
                    signals
                        .status_msg
                        .set(Some(format!("Failed to load calendar: {err}")));
                }
            }
            signals.loading.set(false);
        });
    };

    let trigger_create_event = move || {
        let title = signals.new_title.get();
        let desc = signals.new_desc.get();
        let start = signals.new_start.get();
        let end = signals.new_end.get();
        let color = signals.new_color.get();
        if title.trim().is_empty() {
            signals
                .status_msg
                .set(Some("Please enter an event title".to_owned()));
            return;
        }
        let req = cybou_web_contracts::CreateCalendarEventRequest {
            title,
            description: desc,
            start_time: start,
            end_time: end,
            is_all_day: false,
            location: None,
            attendees: vec!["operator@cybou.local".to_owned()],
            color_category: color,
            referenced_subject: None,
        };
        signals.loading.set(true);
        leptos::task::spawn_local(async move {
            match client.create_calendar_event(req).await {
                Ok(event) => {
                    signals
                        .status_msg
                        .set(Some(format!("Created event '{}'", event.title)));
                    signals.new_title.set(String::new());
                    signals.new_desc.set(String::new());
                    signals.is_creating.set(false);
                    load_calendar();
                }
                Err(err) => {
                    signals
                        .status_msg
                        .set(Some(format!("Create failed: {err}")));
                }
            }
            signals.loading.set(false);
        });
    };

    // Trigger initial load
    Effect::new(move |_| {
        load_calendar();
    });

    view! {
        <div class="calendar-panel" style="display: flex; flex-direction: column; height: 100%; width: 100%; background: var(--bg-card); color: var(--text-main); font-family: system-ui, -apple-system, sans-serif; overflow-y: auto;">
            // Header
            <div style="display: flex; align-items: center; justify-content: space-between; padding: 10px 12px; background: var(--bg-sunken); border-bottom: 1px solid var(--line);">
                <div style="display: flex; align-items: center; gap: 8px;">
                    <span style="font-weight: 600; font-size: 13px;">"Calendar & Cognitive Schedules"</span>
                </div>
                <div style="display: flex; align-items: center; gap: 6px;">
                    <button
                        style="background: linear-gradient(135deg, #10b981, #059669); border: none; border-radius: 4px; padding: 4px 10px; font-size: 11px; font-weight: 700; color: #fff; cursor: pointer;"
                        on:click=move |_| signals.is_creating.update(|c| *c = !*c)
                    >
                        {move || if signals.is_creating.get() { "Cancel" } else { "+ Event" }}
                    </button>
                    <button
                        style="background: var(--fill-subtle); border: none; border-radius: 4px; padding: 4px 6px; color: inherit; cursor: pointer;"
                        title="Refresh calendar"
                        on:click=move |_| load_calendar()
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

            <div style="padding: 12px; display: flex; flex-direction: column; gap: 12px;">
                // Create Event Inline Form
                <Show when=move || signals.is_creating.get()>
                    <div style="background: rgba(0,0,0,0.25); border: 1px solid var(--fill-hover); border-radius: 6px; padding: 10px; display: flex; flex-direction: column; gap: 6px;">
                        <input
                            type="text"
                            placeholder="Event Title..."
                            prop:value=move || signals.new_title.get()
                            on:input=move |e| signals.new_title.set(event_target_value(&e))
                            style="background: var(--bg-sunken-strong); border: 1px solid var(--fill-hover); border-radius: 4px; padding: 4px 8px; font-size: 11px; color: inherit;"
                        />
                        <input
                            type="text"
                            placeholder="Description..."
                            prop:value=move || signals.new_desc.get()
                            on:input=move |e| signals.new_desc.set(event_target_value(&e))
                            style="background: var(--bg-sunken-strong); border: 1px solid var(--fill-hover); border-radius: 4px; padding: 4px 8px; font-size: 11px; color: inherit;"
                        />
                        <div style="display: flex; gap: 6px;">
                            <input
                                type="text"
                                placeholder="Start Time (ISO)..."
                                prop:value=move || signals.new_start.get()
                                on:input=move |e| signals.new_start.set(event_target_value(&e))
                                style="background: var(--bg-sunken-strong); border: 1px solid var(--fill-hover); border-radius: 4px; padding: 4px 8px; font-size: 10px; font-family: monospace; color: inherit; flex: 1;"
                            />
                            <input
                                type="text"
                                placeholder="End Time (ISO)..."
                                prop:value=move || signals.new_end.get()
                                on:input=move |e| signals.new_end.set(event_target_value(&e))
                                style="background: var(--bg-sunken-strong); border: 1px solid var(--fill-hover); border-radius: 4px; padding: 4px 8px; font-size: 10px; font-family: monospace; color: inherit; flex: 1;"
                            />
                        </div>
                        <button
                            style="align-self: flex-end; background: #10b981; border: none; border-radius: 4px; padding: 4px 12px; font-size: 11px; font-weight: 700; color: #fff; cursor: pointer;"
                            on:click=move |_| trigger_create_event()
                        >
                            "Save Event"
                        </button>
                    </div>
                </Show>

                // Events List
                <div style="display: flex; flex-direction: column; gap: 8px;">
                    {move || signals.calendar.get().map(|c| {
                        c.events.into_iter().map(|evt| {
                            let (bg_color, border_color) = match evt.color_category.as_str() {
                                "emerald" => ("rgba(16, 185, 129, 0.1)", "rgba(16, 185, 129, 0.3)"),
                                "amber" => ("rgba(245, 158, 11, 0.1)", "var(--caution-line)"),
                                _ => ("rgba(99, 102, 241, 0.1)", "var(--accent-line)"),
                            };

                            view! {
                                <div style=format!("background: {}; border-left: 3px solid {}; border: 1px solid var(--fill-subtle); border-radius: 4px; padding: 10px 12px; display: flex; flex-direction: column; gap: 4px;", bg_color, border_color)>
                                    <div style="display: flex; align-items: center; justify-content: space-between;">
                                        <span style="font-weight: 700; font-size: 12px; color: var(--text-bright);">{evt.title}</span>
                                        <span style="font-size: 10px; color: var(--text-dim); font-family: monospace;">
                                            {format!("{} - {}", evt.start_time, evt.end_time)}
                                        </span>
                                    </div>
                                    <div style="font-size: 11px; color: var(--text-strong);">{evt.description}</div>
                                    {evt.location.map(|loc| view! {
                                        <div style="font-size: 10px; color: #a7f3d0; font-family: monospace;">
                                            {format!("📍 {loc}")}
                                        </div>
                                    })}
                                </div>
                            }
                        }).collect::<Vec<_>>()
                    })}
                </div>
            </div>
        </div>
    }
}
