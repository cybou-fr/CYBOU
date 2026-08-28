// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Notifications Center card component for desktop attention, evidence, system, and agent feeds.

use leptos::prelude::*;
use cybou_protocol::notification::{NotificationActionKind, NotificationCategory, NotificationSeverity};
use uuid::Uuid;

use crate::{
    CardId,
    components::icons::{
        IconAlertCircle, IconAlertTriangle, IconBell, IconBot, IconCheckCircle, IconClose,
        IconInfo, IconRefresh, IconSearch,
    },
    state::RuntimeState,
    tool_state::ToolCardStates,
};

#[component]
pub fn NotificationsContent(card: CardId) -> impl IntoView {
    let state = expect_context::<RuntimeState>();
    let client = state.client;
    let tool_states = expect_context::<ToolCardStates>();
    let signals = tool_states.notifications(card);

    let load_notifications = move || {
        signals.loading.set(true);
        leptos::task::spawn_local(async move {
            match client.list_notifications().await {
                Ok(projection) => {
                    signals.notifications.set(projection.notifications);
                    signals.unread_count.set(projection.unread_count);
                    signals.attention_count.set(projection.attention_count);
                    signals.status_msg.set(None);
                }
                Err(err) => {
                    signals.status_msg.set(Some(format!("Failed to load notifications: {err}")));
                }
            }
            signals.loading.set(false);
        });
    };

    let dismiss_notif = move |id: Option<Uuid>, all: bool| {
        leptos::task::spawn_local(async move {
            match client.dismiss_notifications(id, all).await {
                Ok(()) => {
                    load_notifications();
                }
                Err(err) => {
                    signals.status_msg.set(Some(format!("Dismiss failed: {err}")));
                }
            }
        });
    };

    let trigger_action = move |notif_id: Uuid, action_id: String| {
        leptos::task::spawn_local(async move {
            match client.execute_notification_action(notif_id, &action_id).await {
                Ok(outcome) => {
                    signals.status_msg.set(Some(outcome));
                    load_notifications();
                }
                Err(err) => {
                    signals.status_msg.set(Some(format!("Action failed: {err}")));
                }
            }
        });
    };

    // Trigger initial load
    Effect::new(move |_| {
        load_notifications();
    });

    let filtered_items = move || {
        let all = signals.notifications.get();
        let cat = signals.selected_category.get();
        let search = signals.search_query.get().to_lowercase();

        all.into_iter()
            .filter(|n| !n.dismissed)
            .filter(|n| {
                if let Some(category) = cat {
                    n.category == category
                } else {
                    true
                }
            })
            .filter(|n| {
                if search.is_empty() {
                    true
                } else {
                    n.title.to_lowercase().contains(&search) || n.body.to_lowercase().contains(&search)
                }
            })
            .collect::<Vec<_>>()
    };

    view! {
        <div class="notifications-panel" style="display: flex; flex-direction: column; height: 100%; width: 100%; background: var(--bg-card, #1e1e24); color: var(--text-main, #e0e0e0); font-family: system-ui, -apple-system, sans-serif;">
            // Toolbar
            <div style="display: flex; flex-direction: column; gap: 8px; padding: 10px 12px; background: rgba(0,0,0,0.2); border-bottom: 1px solid rgba(255,255,255,0.08);">
                <div style="display: flex; align-items: center; justify-content: space-between;">
                    <div style="display: flex; align-items: center; gap: 8px;">
                        <span style="font-weight: 600; font-size: 13px; display: flex; align-items: center; gap: 6px;">
                            <IconBell size=15 />
                            "Notifications Center"
                        </span>
                        {move || {
                            let att = signals.attention_count.get();
                            if att > 0 {
                                Some(view! {
                                    <span style="background: rgba(239, 68, 68, 0.2); color: #f87171; font-size: 11px; padding: 2px 6px; border-radius: 10px; font-weight: 600;">
                                        {format!("{} Attention", att)}
                                    </span>
                                })
                            } else {
                                None
                            }
                        }}
                    </div>

                    <div style="display: flex; align-items: center; gap: 6px;">
                        <button
                            style="background: rgba(255,255,255,0.06); border: none; border-radius: 4px; padding: 4px 8px; color: rgba(255,255,255,0.7); font-size: 11px; cursor: pointer;"
                            on:click=move |_| dismiss_notif(None, true)
                        >
                            "Dismiss All"
                        </button>
                        <button
                            style="background: rgba(255,255,255,0.06); border: none; border-radius: 4px; padding: 4px 6px; color: inherit; cursor: pointer;"
                            title="Refresh"
                            on:click=move |_| load_notifications()
                        >
                            <IconRefresh size=13 />
                        </button>
                    </div>
                </div>

                // Category filter chips & search input
                <div style="display: flex; align-items: center; justify-content: space-between; gap: 8px;">
                    <div style="display: flex; gap: 4px; flex-wrap: wrap;">
                        <button
                            style=move || format!(
                                "background: {}; border: none; font-size: 11px; padding: 3px 8px; border-radius: 12px; color: inherit; cursor: pointer;",
                                if signals.selected_category.get().is_none() { "rgba(99, 102, 241, 0.3)" } else { "rgba(255,255,255,0.05)" }
                            )
                            on:click=move |_| signals.selected_category.set(None)
                        >
                            "All"
                        </button>
                        <button
                            style=move || format!(
                                "background: {}; border: none; font-size: 11px; padding: 3px 8px; border-radius: 12px; color: #f87171; cursor: pointer;",
                                if signals.selected_category.get() == Some(NotificationCategory::Attention) { "rgba(239, 68, 68, 0.3)" } else { "rgba(255,255,255,0.05)" }
                            )
                            on:click=move |_| signals.selected_category.set(Some(NotificationCategory::Attention))
                        >
                            "Attention"
                        </button>
                        <button
                            style=move || format!(
                                "background: {}; border: none; font-size: 11px; padding: 3px 8px; border-radius: 12px; color: #60a5fa; cursor: pointer;",
                                if signals.selected_category.get() == Some(NotificationCategory::Evidence) { "rgba(59, 130, 246, 0.3)" } else { "rgba(255,255,255,0.05)" }
                            )
                            on:click=move |_| signals.selected_category.set(Some(NotificationCategory::Evidence))
                        >
                            "Evidence"
                        </button>
                        <button
                            style=move || format!(
                                "background: {}; border: none; font-size: 11px; padding: 3px 8px; border-radius: 12px; color: #fbbf24; cursor: pointer;",
                                if signals.selected_category.get() == Some(NotificationCategory::System) { "rgba(245, 158, 11, 0.3)" } else { "rgba(255,255,255,0.05)" }
                            )
                            on:click=move |_| signals.selected_category.set(Some(NotificationCategory::System))
                        >
                            "System"
                        </button>
                        <button
                            style=move || format!(
                                "background: {}; border: none; font-size: 11px; padding: 3px 8px; border-radius: 12px; color: #a78bfa; cursor: pointer;",
                                if signals.selected_category.get() == Some(NotificationCategory::Agent) { "rgba(167, 139, 250, 0.3)" } else { "rgba(255,255,255,0.05)" }
                            )
                            on:click=move |_| signals.selected_category.set(Some(NotificationCategory::Agent))
                        >
                            "Agent"
                        </button>
                    </div>

                    // Quick search filter
                    <div style="position: relative; display: flex; align-items: center;">
                        <input
                            type="text"
                            placeholder="Filter..."
                            prop:value=move || signals.search_query.get()
                            on:input=move |e| signals.search_query.set(event_target_value(&e))
                            style="background: rgba(0,0,0,0.3); border: 1px solid rgba(255,255,255,0.1); border-radius: 4px; padding: 3px 8px; font-size: 11px; color: inherit; width: 110px;"
                        />
                    </div>
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

            // Notifications Feed
            <div style="flex: 1; overflow-y: auto; padding: 10px; display: flex; flex-direction: column; gap: 8px;">
                <For
                    each=filtered_items
                    key=|n| n.id
                    children=move |notif| {
                        let notif_id = notif.id;
                        let category_label = notif.category.label();
                        
                        let (cat_bg, cat_color) = match notif.category {
                            NotificationCategory::Attention => ("rgba(239, 68, 68, 0.2)", "#f87171"),
                            NotificationCategory::Evidence => ("rgba(59, 130, 246, 0.2)", "#60a5fa"),
                            NotificationCategory::System => ("rgba(245, 158, 11, 0.2)", "#fbbf24"),
                            NotificationCategory::Agent => ("rgba(167, 139, 250, 0.2)", "#a78bfa"),
                            NotificationCategory::Operation => ("rgba(34, 197, 94, 0.2)", "#4ade80"),
                        };

                        let (sev_border, sev_icon) = match notif.severity {
                            NotificationSeverity::Critical => ("#ef4444", view! { <IconAlertCircle size=14 /> }.into_any()),
                            NotificationSeverity::Warning => ("#f59e0b", view! { <IconAlertTriangle size=14 /> }.into_any()),
                            NotificationSeverity::Notice => ("#3b82f6", view! { <IconInfo size=14 /> }.into_any()),
                            NotificationSeverity::Info => ("rgba(255,255,255,0.1)", view! { <IconInfo size=14 /> }.into_any()),
                        };

                        view! {
                            <div
                                class="notif-card"
                                style=format!(
                                    "background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.08); border-left: 3px solid {sev_border}; border-radius: 6px; padding: 10px 12px; display: flex; flex-direction: column; gap: 6px;"
                                )
                            >
                                <div style="display: flex; align-items: center; justify-content: space-between;">
                                    <div style="display: flex; align-items: center; gap: 6px;">
                                        <span style=format!("background: {cat_bg}; color: {cat_color}; font-size: 9px; font-weight: 700; padding: 1px 5px; border-radius: 3px; text-transform: uppercase;")>
                                            {category_label}
                                        </span>
                                        <span style="font-weight: 600; font-size: 12px; color: var(--text-main, #f9fafb);">
                                            {notif.title.clone()}
                                        </span>
                                    </div>

                                    <button
                                        style="background: none; border: none; color: rgba(255,255,255,0.4); cursor: pointer; padding: 2px;"
                                        title="Dismiss notification"
                                        on:click=move |_| dismiss_notif(Some(notif_id), false)
                                    >
                                        <IconClose size=11 />
                                    </button>
                                </div>

                                <div style="font-size: 11px; line-height: 1.4; color: rgba(255,255,255,0.75);">
                                    {notif.body.clone()}
                                </div>

                                // Subject reference chip if present
                                {notif.subject.as_ref().map(|s| {
                                    let title = s.display_title();
                                    let kind = s.kind_name();
                                    view! {
                                        <div style="display: flex; align-items: center; gap: 4px; font-size: 10px; color: rgba(255,255,255,0.5); margin-top: 2px;">
                                            <span>"Subject: "</span>
                                            <span style="background: rgba(255,255,255,0.06); padding: 1px 5px; border-radius: 3px; color: #cbd5e1; font-family: monospace;">
                                                {format!("{kind} » {title}")}
                                            </span>
                                        </div>
                                    }
                                })}

                                // Action Buttons
                                {if !notif.actions.is_empty() {
                                    Some(view! {
                                        <div style="display: flex; align-items: center; gap: 6px; margin-top: 6px; padding-top: 6px; border-top: 1px solid rgba(255,255,255,0.05);">
                                            {notif.actions.into_iter().map(|action| {
                                                let action_id = action.id.clone();
                                                let is_primary = action.primary;
                                                let btn_style = if is_primary {
                                                    "background: #6366f1; color: #ffffff; border: none; font-weight: 600;"
                                                } else {
                                                    "background: rgba(255,255,255,0.08); color: rgba(255,255,255,0.85); border: 1px solid rgba(255,255,255,0.1);"
                                                };
                                                view! {
                                                    <button
                                                        style=format!(
                                                            "{btn_style} font-size: 11px; padding: 4px 10px; border-radius: 4px; cursor: pointer; transition: background 0.15s ease;"
                                                        )
                                                        on:click=move |_| trigger_action(notif_id, action_id.clone())
                                                    >
                                                        {action.label}
                                                    </button>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    })
                                } else {
                                    None
                                }}
                            </div>
                        }
                    }
                />

                {move || if filtered_items().is_empty() {
                    Some(view! {
                        <div style="text-align: center; color: rgba(255,255,255,0.4); padding: 40px 16px; font-size: 12px;">
                            "No active notifications."
                        </div>
                    })
                } else {
                    None
                }}
            </div>
        </div>
    }
}
