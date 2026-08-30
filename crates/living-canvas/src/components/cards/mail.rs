// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Personal Mail & Messages card component.

use crate::{CardId, MindClient, components::icons::IconRefresh, tool_state::ToolCardStates};
use leptos::prelude::*;

#[component]
pub fn MailContent(card: CardId) -> impl IntoView {
    let client = crate::GatewayMindClient;
    let tool_states = expect_context::<ToolCardStates>();
    let signals = tool_states.mail(card);

    let load_mail = move || {
        signals.loading.set(true);
        leptos::task::spawn_local(async move {
            match client.get_mail(None, None).await {
                Ok(proj) => {
                    signals.mail.set(Some(proj));
                    signals.status_msg.set(None);
                }
                Err(err) => {
                    signals
                        .status_msg
                        .set(Some(format!("Failed to load mail: {err}")));
                }
            }
            signals.loading.set(false);
        });
    };

    let trigger_send = move || {
        let to_str = signals.compose_to.get();
        let subject = signals.compose_subject.get();
        let body = signals.compose_body.get();
        if to_str.trim().is_empty() || subject.trim().is_empty() {
            signals
                .status_msg
                .set(Some("Recipient and subject are required".to_owned()));
            return;
        }
        let to = to_str.split(',').map(|s| s.trim().to_owned()).collect();
        let req = cybou_web_contracts::SendMailRequest {
            account_id: "acc-cybou-primary".to_owned(),
            to,
            subject,
            body,
            referenced_subject: None,
        };
        signals.loading.set(true);
        leptos::task::spawn_local(async move {
            match client.send_mail(req).await {
                Ok(msg) => {
                    signals
                        .status_msg
                        .set(Some(format!("Sent email '{}'", msg.subject)));
                    signals.compose_to.set(String::new());
                    signals.compose_subject.set(String::new());
                    signals.compose_body.set(String::new());
                    signals.is_composing.set(false);
                    load_mail();
                }
                Err(err) => {
                    signals.status_msg.set(Some(format!("Send failed: {err}")));
                }
            }
            signals.loading.set(false);
        });
    };

    // Trigger initial load
    Effect::new(move |_| {
        load_mail();
    });

    view! {
        <div class="mail-panel" style="display: flex; flex-direction: column; height: 100%; width: 100%; background: var(--bg-card); color: var(--text-main); font-family: system-ui, -apple-system, sans-serif; overflow: hidden;">
            // Header
            <div style="display: flex; align-items: center; justify-content: space-between; padding: 10px 12px; background: var(--bg-sunken); border-bottom: 1px solid var(--line);">
                <div style="display: flex; align-items: center; gap: 8px;">
                    <span style="font-weight: 600; font-size: 13px;">"Personal Mail & Messages"</span>
                </div>
                <div style="display: flex; align-items: center; gap: 6px;">
                    <button
                        style="background: linear-gradient(135deg, var(--accent-solid), #8b5cf6); border: none; border-radius: 4px; padding: 4px 10px; font-size: 11px; font-weight: 700; color: #fff; cursor: pointer;"
                        on:click=move |_| signals.is_composing.update(|c| *c = !*c)
                    >
                        {move || if signals.is_composing.get() { "Cancel" } else { "Compose" }}
                    </button>
                    <button
                        style="background: var(--fill-subtle); border: none; border-radius: 4px; padding: 4px 6px; color: inherit; cursor: pointer;"
                        title="Refresh mail"
                        on:click=move |_| load_mail()
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

            // Compose form or message list
            {move || if signals.is_composing.get() {
                view! {
                    <div style="padding: 12px; display: flex; flex-direction: column; gap: 8px; flex: 1; overflow-y: auto;">
                        <input
                            type="text"
                            placeholder="To (comma separated emails)..."
                            prop:value=move || signals.compose_to.get()
                            on:input=move |e| signals.compose_to.set(event_target_value(&e))
                            style="background: var(--bg-sunken-strong); border: 1px solid var(--fill-hover); border-radius: 4px; padding: 6px 8px; font-size: 11px; color: inherit;"
                        />
                        <input
                            type="text"
                            placeholder="Subject..."
                            prop:value=move || signals.compose_subject.get()
                            on:input=move |e| signals.compose_subject.set(event_target_value(&e))
                            style="background: var(--bg-sunken-strong); border: 1px solid var(--fill-hover); border-radius: 4px; padding: 6px 8px; font-size: 11px; color: inherit;"
                        />
                        <textarea
                            placeholder="Message body (markdown supported)..."
                            prop:value=move || signals.compose_body.get()
                            on:input=move |e| signals.compose_body.set(event_target_value(&e))
                            rows="8"
                            style="background: var(--bg-sunken-strong); border: 1px solid var(--fill-hover); border-radius: 4px; padding: 8px; font-size: 11px; font-family: inherit; color: inherit; resize: vertical; flex: 1;"
                        />
                        <button
                            style="align-self: flex-end; background: linear-gradient(135deg, var(--accent-solid), #8b5cf6); border: none; border-radius: 4px; padding: 6px 14px; font-size: 11px; font-weight: 700; color: #fff; cursor: pointer;"
                            on:click=move |_| trigger_send()
                        >
                            "Send Email"
                        </button>
                    </div>
                }.into_any()
            } else {
                view! {
                    <div style="display: flex; height: 100%; overflow: hidden;">
                        // Mail Messages List
                        <div style="flex: 1; overflow-y: auto; padding: 8px; display: flex; flex-direction: column; gap: 6px;">
                            {move || signals.mail.get().map(|m| {
                                m.messages.into_iter().map(|msg| {
                                    let msg_clone = msg.clone();
                                    view! {
                                        <div
                                            style="background: var(--bg-sunken); border: 1px solid var(--fill-subtle); border-radius: 4px; padding: 8px 10px; cursor: pointer; transition: background 0.15s ease;"
                                            on:click=move |_| signals.selected_message.set(Some(msg_clone.clone()))
                                        >
                                            <div style="display: flex; align-items: center; justify-content: space-between; font-size: 11px;">
                                                <span style="font-weight: 600; color: var(--text-bright);">{msg.from}</span>
                                                <span style="font-size: 10px; color: var(--text-faint); font-family: monospace;">{msg.timestamp}</span>
                                            </div>
                                            <div style="font-weight: 600; font-size: 12px; color: var(--accent-text); margin-top: 2px;">{msg.subject}</div>
                                            <div style="font-size: 10px; color: var(--text-dim); margin-top: 2px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                                                {msg.preview}
                                            </div>
                                        </div>
                                    }
                                }).collect::<Vec<_>>()
                            })}
                        </div>

                        // Selected Message Detail Pane
                        {move || signals.selected_message.get().map(|msg| {
                            view! {
                                <div style="flex: 1; border-left: 1px solid var(--line); padding: 12px; overflow-y: auto; display: flex; flex-direction: column; gap: 8px; background: rgba(0,0,0,0.1);">
                                    <div style="display: flex; justify-content: space-between; align-items: flex-start;">
                                        <div>
                                            <div style="font-weight: 700; font-size: 13px; color: var(--text-bright);">{msg.subject}</div>
                                            <div style="font-size: 11px; color: var(--text-second); margin-top: 2px;">
                                                {format!("From: {} • To: {}", msg.from, msg.to.join(", "))}
                                            </div>
                                        </div>
                                        <button
                                            style="background: none; border: none; color: var(--text-dim); cursor: pointer; font-size: 12px;"
                                            on:click=move |_| signals.selected_message.set(None)
                                        >
                                            "✕"
                                        </button>
                                    </div>
                                    <div style="border-top: 1px solid var(--fill-subtle); padding-top: 8px; font-size: 11px; line-height: 1.5; white-space: pre-wrap; color: var(--text-main);">
                                        {msg.body}
                                    </div>
                                </div>
                            }
                        })}
                    </div>
                }.into_any()
            }}
        </div>
    }
}
