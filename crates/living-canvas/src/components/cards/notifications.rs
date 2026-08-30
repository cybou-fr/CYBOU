// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Notifications Center card component for desktop attention, evidence, system, and agent feeds.

use cybou_protocol::notification::{NotificationCategory, NotificationSeverity};
use leptos::prelude::*;
use uuid::Uuid;

use crate::{
    CardId, MindClient,
    components::icons::{
        IconAlertCircle, IconAlertTriangle, IconBell, IconClose, IconInfo, IconRefresh,
    },
    tool_state::ToolCardStates,
};

#[component]
pub fn NotificationsContent(card: CardId) -> impl IntoView {
    let client = crate::GatewayMindClient;
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
                    signals
                        .status_msg
                        .set(Some(format!("Failed to load notifications: {err}")));
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
                    signals
                        .status_msg
                        .set(Some(format!("Dismiss failed: {err}")));
                }
            }
        });
    };

    let trigger_action = move |notif_id: Uuid, action_id: String| {
        leptos::task::spawn_local(async move {
            match client
                .execute_notification_action(notif_id, &action_id)
                .await
            {
                Ok(outcome) => {
                    signals.status_msg.set(Some(outcome));
                    load_notifications();
                }
                Err(err) => {
                    signals
                        .status_msg
                        .set(Some(format!("Action failed: {err}")));
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
                    n.title.to_lowercase().contains(&search)
                        || n.body.to_lowercase().contains(&search)
                }
            })
            .collect::<Vec<_>>()
    };

    view! {
        <div class="notifications-panel" style="display: flex; flex-direction: column; height: 100%; width: 100%; background: var(--bg-card); color: var(--text-main); font-family: system-ui, -apple-system, sans-serif;">
            // Toolbar
            <div style="display: flex; flex-direction: column; gap: 8px; padding: 10px 12px; background: var(--bg-sunken); border-bottom: 1px solid var(--line);">
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
                                    <span style="background: var(--danger-fill-strong); color: var(--danger); font-size: 11px; padding: 2px 6px; border-radius: 10px; font-weight: 600;">
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
                            style="background: var(--fill-subtle); border: none; border-radius: 4px; padding: 4px 8px; color: var(--text-strong); font-size: 11px; cursor: pointer;"
                            on:click=move |_| dismiss_notif(None, true)
                        >
                            "Dismiss All"
                        </button>
                        <button
                            style="background: var(--fill-subtle); border: none; border-radius: 4px; padding: 4px 6px; color: inherit; cursor: pointer;"
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
                                if signals.selected_category.get().is_none() { "var(--accent-line)" } else { "var(--fill-faintest)" }
                            )
                            on:click=move |_| signals.selected_category.set(None)
                        >
                            "All"
                        </button>
                        <button
                            style=move || format!(
                                "background: {}; border: none; font-size: 11px; padding: 3px 8px; border-radius: 12px; color: var(--danger); cursor: pointer;",
                                if signals.selected_category.get() == Some(NotificationCategory::Attention) { "var(--danger-line)" } else { "var(--fill-faintest)" }
                            )
                            on:click=move |_| signals.selected_category.set(Some(NotificationCategory::Attention))
                        >
                            "Attention"
                        </button>
                        <button
                            style=move || format!(
                                "background: {}; border: none; font-size: 11px; padding: 3px 8px; border-radius: 12px; color: var(--info); cursor: pointer;",
                                if signals.selected_category.get() == Some(NotificationCategory::Evidence) { "var(--info-line)" } else { "var(--fill-faintest)" }
                            )
                            on:click=move |_| signals.selected_category.set(Some(NotificationCategory::Evidence))
                        >
                            "Evidence"
                        </button>
                        <button
                            style=move || format!(
                                "background: {}; border: none; font-size: 11px; padding: 3px 8px; border-radius: 12px; color: var(--caution); cursor: pointer;",
                                if signals.selected_category.get() == Some(NotificationCategory::System) { "var(--caution-line)" } else { "var(--fill-faintest)" }
                            )
                            on:click=move |_| signals.selected_category.set(Some(NotificationCategory::System))
                        >
                            "System"
                        </button>
                        <button
                            style=move || format!(
                                "background: {}; border: none; font-size: 11px; padding: 3px 8px; border-radius: 12px; color: #a78bfa; cursor: pointer;",
                                if signals.selected_category.get() == Some(NotificationCategory::Agent) { "rgba(167, 139, 250, 0.3)" } else { "var(--fill-faintest)" }
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
                            style="background: var(--bg-sunken-strong); border: 1px solid var(--fill-hover); border-radius: 4px; padding: 3px 8px; font-size: 11px; color: inherit; width: 110px;"
                        />
                    </div>
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

            // Notifications Feed
            <div style="flex: 1; overflow-y: auto; padding: 10px; display: flex; flex-direction: column; gap: 8px;">
                <For
                    each=filtered_items
                    key=|n| n.id
                    children=move |notif| {
                        let notif_id = notif.id;
                        let category_label = notif.category.label();

                        let (cat_bg, cat_color) = match notif.category {
                            NotificationCategory::Attention => ("var(--danger-fill-strong)", "var(--danger)"),
                            NotificationCategory::Evidence => ("var(--info-fill-strong)", "var(--info)"),
                            NotificationCategory::System => ("var(--caution-fill-strong)", "var(--caution)"),
                            NotificationCategory::Agent => ("rgba(167, 139, 250, 0.2)", "#a78bfa"),
                            NotificationCategory::Operation => ("var(--ok-fill-strong)", "var(--ok)"),
                        };

                        let (sev_border, _sev_icon) = match notif.severity {
                            NotificationSeverity::Critical => ("#ef4444", view! { <IconAlertCircle size=14 /> }.into_any()),
                            NotificationSeverity::Warning => ("#f59e0b", view! { <IconAlertTriangle size=14 /> }.into_any()),
                            NotificationSeverity::Notice => ("#3b82f6", view! { <IconInfo size=14 /> }.into_any()),
                            NotificationSeverity::Info => ("var(--fill-hover)", view! { <IconInfo size=14 /> }.into_any()),
                        };

                        view! {
                            <div
                                class="notif-card"
                                style=format!(
                                    "background: var(--fill-faint); border: 1px solid var(--line); border-left: 3px solid {sev_border}; border-radius: 6px; padding: 10px 12px; display: flex; flex-direction: column; gap: 6px;"
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
                                        style="background: none; border: none; color: var(--text-faint); cursor: pointer; padding: 2px;"
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
                                        <div style="display: flex; align-items: center; gap: 4px; font-size: 10px; color: var(--text-dim); margin-top: 2px;">
                                            <span>"Subject: "</span>
                                            <span style="background: var(--fill-subtle); padding: 1px 5px; border-radius: 3px; color: #cbd5e1; font-family: monospace;">
                                                {format!("{kind} » {title}")}
                                            </span>
                                        </div>
                                    }
                                })}

                                // Action Buttons
                                {if !notif.actions.is_empty() {
                                    Some(view! {
                                        <div style="display: flex; align-items: center; gap: 6px; margin-top: 6px; padding-top: 6px; border-top: 1px solid var(--fill-faintest);">
                                            {notif.actions.into_iter().map(|action| {
                                                let action_id = action.id.clone();
                                                let is_primary = action.primary;
                                                let btn_style = if is_primary {
                                                    "background: var(--accent-solid); color: var(--text-pure); border: none; font-weight: 600;"
                                                } else {
                                                    "background: var(--line); color: rgba(255,255,255,0.85); border: 1px solid var(--fill-hover);"
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
                        <div style="text-align: center; color: var(--text-faint); padding: 40px 16px; font-size: 12px;">
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
