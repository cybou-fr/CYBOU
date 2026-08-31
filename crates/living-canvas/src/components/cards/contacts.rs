// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Personal Contacts & Cognitive Subject Directory card component.

use crate::{CardId, MindClient, components::icons::IconRefresh, tool_state::ToolCardStates};
use leptos::prelude::*;

#[component]
pub fn ContactsContent(card: CardId) -> impl IntoView {
    let client = crate::GatewayMindClient;
    let tool_states = expect_context::<ToolCardStates>();
    let signals = tool_states.contacts(card);

    let load_contacts = move || {
        signals.loading.set(true);
        leptos::task::spawn_local(async move {
            match client.get_contacts().await {
                Ok(proj) => {
                    signals.contacts.set(proj.contacts);
                    signals.status_msg.set(None);
                }
                Err(err) => {
                    signals
                        .status_msg
                        .set(Some(format!("Failed to load contacts: {err}")));
                }
            }
            signals.loading.set(false);
        });
    };

    let trigger_create_contact = move || {
        let name = signals.new_name.get();
        let email = signals.new_email.get();
        let role = signals.new_role.get();
        let org = signals.new_org.get();
        let tags: Vec<String> = signals
            .new_tags
            .get()
            .split(',')
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .collect();
        let notes = signals.new_notes.get();

        if name.trim().is_empty() || email.trim().is_empty() {
            signals
                .status_msg
                .set(Some("Name and email are required".to_owned()));
            return;
        }

        let req = cybou_web_contracts::CreateContactRequest {
            name,
            email,
            role,
            organization: org,
            phone: None,
            tags,
            notes,
            referenced_subject: None,
        };

        signals.loading.set(true);
        leptos::task::spawn_local(async move {
            match client.create_contact(req).await {
                Ok(cnt) => {
                    signals
                        .status_msg
                        .set(Some(format!("Added contact '{}'", cnt.name)));
                    signals.new_name.set(String::new());
                    signals.new_email.set(String::new());
                    signals.new_role.set(String::new());
                    signals.new_org.set(String::new());
                    signals.new_tags.set(String::new());
                    signals.new_notes.set(String::new());
                    signals.is_creating.set(false);
                    load_contacts();
                }
                Err(err) => {
                    signals
                        .status_msg
                        .set(Some(format!("Failed to add contact: {err}")));
                }
            }
            signals.loading.set(false);
        });
    };

    // Trigger initial load
    Effect::new(move |_| {
        load_contacts();
    });

    view! {
        <div class="contacts-panel" style="display: flex; flex-direction: column; height: 100%; width: 100%; overflow-y: auto;">
            // Header
            <div style="display: flex; align-items: center; justify-content: space-between; padding: 10px 12px; background: var(--bg-sunken); border-bottom: 1px solid var(--line);">
                <div style="display: flex; align-items: center; gap: 8px;">
                    <span style="font-weight: 600; font-size: 13px;">"Contacts & Cognitive Subjects"</span>
                </div>
                <div style="display: flex; align-items: center; gap: 6px;">
                    <button
                        style="background: linear-gradient(135deg, #ec4899, #d946ef); border: none; border-radius: 4px; padding: 4px 10px; font-size: 11px; font-weight: 700; color: #fff; cursor: pointer;"
                        on:click=move |_| signals.is_creating.update(|c| *c = !*c)
                    >
                        {move || if signals.is_creating.get() { "Cancel" } else { "+ Contact" }}
                    </button>
                    <button
                        style="background: var(--fill-subtle); border: none; border-radius: 4px; padding: 4px 6px; color: inherit; cursor: pointer;"
                        title="Refresh contacts"
                        on:click=move |_| load_contacts()
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
                // Create Contact Form
                <Show when=move || signals.is_creating.get()>
                    <div style="background: rgba(0,0,0,0.25); border: 1px solid var(--fill-hover); border-radius: 6px; padding: 10px; display: flex; flex-direction: column; gap: 6px;">
                        <div style="display: flex; gap: 6px;">
                            <input
                                type="text"
                                placeholder="Full Name..."
                                prop:value=move || signals.new_name.get()
                                on:input=move |e| signals.new_name.set(event_target_value(&e))
                                style="background: var(--bg-sunken-strong); border: 1px solid var(--fill-hover); border-radius: 4px; padding: 4px 8px; font-size: 11px; color: inherit; flex: 1;"
                            />
                            <input
                                type="text"
                                placeholder="Email..."
                                prop:value=move || signals.new_email.get()
                                on:input=move |e| signals.new_email.set(event_target_value(&e))
                                style="background: var(--bg-sunken-strong); border: 1px solid var(--fill-hover); border-radius: 4px; padding: 4px 8px; font-size: 11px; color: inherit; flex: 1;"
                            />
                        </div>
                        <div style="display: flex; gap: 6px;">
                            <input
                                type="text"
                                placeholder="Role..."
                                prop:value=move || signals.new_role.get()
                                on:input=move |e| signals.new_role.set(event_target_value(&e))
                                style="background: var(--bg-sunken-strong); border: 1px solid var(--fill-hover); border-radius: 4px; padding: 4px 8px; font-size: 11px; color: inherit; flex: 1;"
                            />
                            <input
                                type="text"
                                placeholder="Organization..."
                                prop:value=move || signals.new_org.get()
                                on:input=move |e| signals.new_org.set(event_target_value(&e))
                                style="background: var(--bg-sunken-strong); border: 1px solid var(--fill-hover); border-radius: 4px; padding: 4px 8px; font-size: 11px; color: inherit; flex: 1;"
                            />
                        </div>
                        <input
                            type="text"
                            placeholder="Tags (comma-separated)..."
                            prop:value=move || signals.new_tags.get()
                            on:input=move |e| signals.new_tags.set(event_target_value(&e))
                            style="background: var(--bg-sunken-strong); border: 1px solid var(--fill-hover); border-radius: 4px; padding: 4px 8px; font-size: 10px; color: inherit;"
                        />
                        <button
                            style="align-self: flex-end; background: #ec4899; border: none; border-radius: 4px; padding: 4px 12px; font-size: 11px; font-weight: 700; color: #fff; cursor: pointer;"
                            on:click=move |_| trigger_create_contact()
                        >
                            "Save Contact"
                        </button>
                    </div>
                </Show>

                // Contacts Cards Grid
                <div style="display: grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: 8px;">
                    {move || signals.contacts.get().into_iter().map(|c| {
                        view! {
                            <div style="background: var(--bg-sunken); border: 1px solid var(--fill-subtle); border-radius: 6px; padding: 10px 12px; display: flex; flex-direction: column; gap: 4px;">
                                <div style="font-weight: 700; font-size: 12px; color: var(--text-bright);">{c.name}</div>
                                <div style="font-size: 10px; color: #c084fc;">
                                    {format!("{} • {}", c.role, c.organization)}
                                </div>
                                <div style="font-size: 10px; color: var(--text-second); font-family: monospace;">
                                    {c.email}
                                </div>
                                {if c.tags.is_empty() {
                                    view! { <div></div> }.into_any()
                                } else {
                                    view! {
                                        <div style="display: flex; flex-wrap: wrap; gap: 4px; margin-top: 4px;">
                                            {c.tags.into_iter().map(|t| view! {
                                                <span style="font-size: 9px; padding: 1px 5px; border-radius: 3px; background: var(--fill-subtle); color: var(--text-strong);">
                                                    {format!("#{t}")}
                                                </span>
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    }.into_any()
                                }}
                            </div>
                        }
                    }).collect::<Vec<_>>()}
                </div>
            </div>
        </div>
    }
}
