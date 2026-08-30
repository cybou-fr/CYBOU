// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! User Accounts and SSH Authorized Keys settings card component.

use crate::{CardId, MindClient, components::icons::IconRefresh, tool_state::ToolCardStates};
use leptos::prelude::*;

#[component]
pub fn UserSettingsContent(card: CardId) -> impl IntoView {
    let client = crate::GatewayMindClient;
    let tool_states = expect_context::<ToolCardStates>();
    let signals = tool_states.user_settings(card);

    let load_users = move || {
        signals.loading.set(true);
        leptos::task::spawn_local(async move {
            match client.get_users_settings().await {
                Ok(proj) => {
                    signals.users.set(proj.users);
                    signals.ssh_keys.set(proj.ssh_keys);
                    signals.status_msg.set(None);
                }
                Err(err) => {
                    signals
                        .status_msg
                        .set(Some(format!("Failed to load users: {err}")));
                }
            }
            signals.loading.set(false);
        });
    };

    let trigger_create_user = move || {
        let username = signals.new_user_name.get();
        let full_name = signals.new_full_name.get();
        let is_admin = signals.new_is_admin.get();
        if username.trim().is_empty() {
            signals
                .status_msg
                .set(Some("Please enter a username".to_owned()));
            return;
        }
        signals.loading.set(true);
        leptos::task::spawn_local(async move {
            match client.create_user(&username, &full_name, is_admin).await {
                Ok(user) => {
                    signals.status_msg.set(Some(format!(
                        "Created user account '{}' (UID {})",
                        user.username, user.uid
                    )));
                    signals.new_user_name.set(String::new());
                    signals.new_full_name.set(String::new());
                    signals.new_is_admin.set(false);
                    load_users();
                }
                Err(err) => {
                    signals
                        .status_msg
                        .set(Some(format!("User creation failed: {err}")));
                }
            }
            signals.loading.set(false);
        });
    };

    let trigger_add_key = move || {
        let name = signals.new_key_name.get();
        let key = signals.new_public_key.get();
        if name.trim().is_empty() || key.trim().is_empty() {
            signals
                .status_msg
                .set(Some("Please enter a key label and public key".to_owned()));
            return;
        }
        signals.loading.set(true);
        leptos::task::spawn_local(async move {
            match client.add_ssh_key(&name, &key).await {
                Ok(k) => {
                    signals.status_msg.set(Some(format!(
                        "Added SSH key '{}' ({})",
                        k.name, k.fingerprint
                    )));
                    signals.new_key_name.set(String::new());
                    signals.new_public_key.set(String::new());
                    load_users();
                }
                Err(err) => {
                    signals
                        .status_msg
                        .set(Some(format!("Failed to add SSH key: {err}")));
                }
            }
            signals.loading.set(false);
        });
    };

    let trigger_delete_key = move |key_id: String| {
        leptos::task::spawn_local(async move {
            match client.delete_ssh_key(&key_id).await {
                Ok(outcome) => {
                    signals.status_msg.set(Some(outcome));
                    load_users();
                }
                Err(err) => {
                    signals
                        .status_msg
                        .set(Some(format!("Delete failed: {err}")));
                }
            }
        });
    };

    // Trigger initial load
    Effect::new(move |_| {
        load_users();
    });

    view! {
        <div class="user-settings-panel" style="display: flex; flex-direction: column; height: 100%; width: 100%; background: var(--bg-card, #1e1e24); color: var(--text-main, #e0e0e0); font-family: system-ui, -apple-system, sans-serif; overflow-y: auto;">
            // Header
            <div style="display: flex; align-items: center; justify-content: space-between; padding: 10px 12px; background: rgba(0,0,0,0.2); border-bottom: 1px solid rgba(255,255,255,0.08);">
                <div style="display: flex; align-items: center; gap: 8px;">
                    <span style="font-weight: 600; font-size: 13px;">"User Accounts & SSH Keys"</span>
                </div>
                <button
                    style="background: rgba(255,255,255,0.06); border: none; border-radius: 4px; padding: 4px 6px; color: inherit; cursor: pointer;"
                    title="Refresh user accounts"
                    on:click=move |_| load_users()
                >
                    <IconRefresh size=13 />
                </button>
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

            <div style="padding: 12px; display: flex; flex-direction: column; gap: 14px;">
                // User Accounts Section
                <div style="background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.06); border-radius: 6px; padding: 10px 12px;">
                    <div style="font-weight: 600; font-size: 11px; margin-bottom: 8px;">
                        {move || format!("Configured Accounts ({})", signals.users.get().len())}
                    </div>
                    <div style="display: flex; flex-direction: column; gap: 6px;">
                        {move || signals.users.get().into_iter().map(|u| {
                            view! {
                                <div style="background: rgba(0,0,0,0.2); border-radius: 4px; padding: 8px 10px; display: flex; align-items: center; justify-content: space-between; font-size: 11px;">
                                    <div>
                                        <div style="display: flex; align-items: center; gap: 8px;">
                                            <span style="font-weight: 600; color: #f3f4f6; font-family: monospace;">{u.username}</span>
                                            <span style="color: rgba(255,255,255,0.6);">{u.full_name}</span>
                                            {if u.is_admin {
                                                Some(view! {
                                                    <span style="background: rgba(245, 158, 11, 0.2); color: #fbbf24; font-size: 9px; font-weight: 700; padding: 1px 5px; border-radius: 3px;">
                                                        "ADMIN"
                                                    </span>
                                                })
                                            } else {
                                                None
                                            }}
                                        </div>
                                        <div style="font-size: 10px; color: rgba(255,255,255,0.4); font-family: monospace; margin-top: 2px;">
                                            {format!("UID {} • {} • groups: {}", u.uid, u.home_dir, u.groups.join(", "))}
                                        </div>
                                    </div>
                                </div>
                            }
                        }).collect::<Vec<_>>()}
                    </div>

                    // New User Inline Form
                    <div style="margin-top: 10px; display: flex; gap: 6px; align-items: center; flex-wrap: wrap; background: rgba(0,0,0,0.15); padding: 8px; border-radius: 4px;">
                        <input
                            type="text"
                            placeholder="Username..."
                            prop:value=move || signals.new_user_name.get()
                            on:input=move |e| signals.new_user_name.set(event_target_value(&e))
                            style="background: rgba(0,0,0,0.3); border: 1px solid rgba(255,255,255,0.1); border-radius: 4px; padding: 3px 6px; font-size: 11px; color: inherit; width: 100px;"
                        />
                        <input
                            type="text"
                            placeholder="Full name..."
                            prop:value=move || signals.new_full_name.get()
                            on:input=move |e| signals.new_full_name.set(event_target_value(&e))
                            style="background: rgba(0,0,0,0.3); border: 1px solid rgba(255,255,255,0.1); border-radius: 4px; padding: 3px 6px; font-size: 11px; color: inherit; flex: 1;"
                        />
                        <label style="display: flex; align-items: center; gap: 4px; font-size: 10px; cursor: pointer;">
                            <input
                                type="checkbox"
                                prop:checked=move || signals.new_is_admin.get()
                                on:change=move |e| signals.new_is_admin.set(event_target_checked(&e))
                            />
                            "Admin"
                        </label>
                        <button
                            style="background: rgba(99, 102, 241, 0.2); border: 1px solid rgba(99, 102, 241, 0.4); border-radius: 4px; padding: 3px 8px; font-size: 11px; color: #c7d2fe; font-weight: 600; cursor: pointer;"
                            on:click=move |_| trigger_create_user()
                        >
                            "Add User"
                        </button>
                    </div>
                </div>

                // Authorized SSH Keys Section
                <div style="background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.06); border-radius: 6px; padding: 10px 12px;">
                    <div style="font-weight: 600; font-size: 11px; margin-bottom: 8px;">
                        {move || format!("Authorized SSH Keys ({})", signals.ssh_keys.get().len())}
                    </div>
                    <div style="display: flex; flex-direction: column; gap: 6px;">
                        {move || signals.ssh_keys.get().into_iter().map(|key| {
                            let key_id = key.id.clone();
                            view! {
                                <div style="background: rgba(0,0,0,0.2); border-radius: 4px; padding: 8px 10px; display: flex; align-items: center; justify-content: space-between; font-size: 11px;">
                                    <div>
                                        <div style="display: flex; align-items: center; gap: 8px;">
                                            <span style="font-weight: 600; color: #f3f4f6;">{key.name}</span>
                                            <span style="background: rgba(255,255,255,0.06); font-size: 9px; padding: 1px 5px; border-radius: 3px; color: rgba(255,255,255,0.6); font-family: monospace;">
                                                {key.key_type}
                                            </span>
                                        </div>
                                        <div style="font-size: 10px; color: rgba(255,255,255,0.4); font-family: monospace; margin-top: 2px;">
                                            {format!("{} • Added {}", key.fingerprint, key.created_at)}
                                        </div>
                                    </div>
                                    <button
                                        style="background: rgba(239, 68, 68, 0.15); border: 1px solid rgba(239, 68, 68, 0.3); border-radius: 4px; padding: 2px 6px; font-size: 10px; color: #f87171; cursor: pointer;"
                                        title="Delete SSH key"
                                        on:click=move |_| trigger_delete_key(key_id.clone())
                                    >
                                        "Delete"
                                    </button>
                                </div>
                            }
                        }).collect::<Vec<_>>()}
                    </div>

                    // Add SSH Key Form
                    <div style="margin-top: 10px; display: flex; flex-direction: column; gap: 6px; background: rgba(0,0,0,0.15); padding: 8px; border-radius: 4px;">
                        <input
                            type="text"
                            placeholder="Key label (e.g. Workstation ED25519)..."
                            prop:value=move || signals.new_key_name.get()
                            on:input=move |e| signals.new_key_name.set(event_target_value(&e))
                            style="background: rgba(0,0,0,0.3); border: 1px solid rgba(255,255,255,0.1); border-radius: 4px; padding: 3px 6px; font-size: 11px; color: inherit;"
                        />
                        <textarea
                            placeholder="Paste public key (ssh-ed25519 AAAAC3...)..."
                            prop:value=move || signals.new_public_key.get()
                            on:input=move |e| signals.new_public_key.set(event_target_value(&e))
                            rows="2"
                            style="background: rgba(0,0,0,0.3); border: 1px solid rgba(255,255,255,0.1); border-radius: 4px; padding: 4px 6px; font-size: 10px; font-family: monospace; color: inherit; resize: vertical;"
                        />
                        <button
                            style="align-self: flex-end; background: rgba(99, 102, 241, 0.2); border: 1px solid rgba(99, 102, 241, 0.4); border-radius: 4px; padding: 4px 10px; font-size: 11px; color: #c7d2fe; font-weight: 600; cursor: pointer;"
                            on:click=move |_| trigger_add_key()
                        >
                            "Add Key"
                        </button>
                    </div>
                </div>
            </div>
        </div>
    }
}
