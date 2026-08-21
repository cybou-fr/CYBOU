// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Host authentication dialog component for logging into the Mind gateway.

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::{GatewayMindClient, components::icons::IconShield};

/// Modal dialog for user PAM authentication against `/api/v1/auth/login`.
#[component]
pub fn AuthModal(open: RwSignal<bool>) -> impl IntoView {
    let (username, set_username) = signal(String::new());
    let (password, set_password) = signal(String::new());
    let (error_msg, set_error_msg) = signal(Option::<String>::None);
    let (submitting, set_submitting) = signal(false);

    let do_login = move || {
        let u = username.get();
        let p = password.get();
        if u.is_empty() {
            set_error_msg.set(Some("Please enter a username".to_string()));
            return;
        }
        set_submitting.set(true);
        set_error_msg.set(None);
        spawn_local(async move {
            let client = GatewayMindClient;
            match client.login(&u, &p).await {
                Ok(true) => {
                    set_submitting.set(false);
                    open.set(false);
                    if let Some(w) = web_sys::window() {
                        let _ = w.location().reload();
                    }
                }
                Ok(false) => {
                    set_submitting.set(false);
                    set_error_msg.set(Some(
                        "Authentication failed. Ensure account is in 'cybou-access' group."
                            .to_string(),
                    ));
                }
                Err(err) => {
                    set_submitting.set(false);
                    set_error_msg.set(Some(format!("Login error: {err}")));
                }
            }
        });
    };

    view! {
        <Show when=move || open.get()>
            <div class="modal-overlay" on:click=move |_| open.set(false)>
                <div class="auth-modal" on:click=move |e: web_sys::MouseEvent| e.stop_propagation()>
                    <header class="auth-header">
                        <div class="auth-title">
                            <IconShield size=18 />
                            <h3>"Authenticate CYBOU Desktop"</h3>
                        </div>
                        <button class="modal-close-btn" on:click=move |_| open.set(false)>"×"</button>
                    </header>
                    <div class="auth-body">
                        <p class="auth-desc">"Sign in with a Linux host account belonging to the " <code>"cybou-access"</code> " group to unlock Zone 3 Body capabilities."</p>

                        <Show when=move || error_msg.get().is_some()>
                            <div class="auth-error">
                                {move || error_msg.get().unwrap_or_default()}
                            </div>
                        </Show>

                        <form on:submit=move |e: web_sys::SubmitEvent| {
                            e.prevent_default();
                            do_login();
                        }>
                            <label class="auth-label">
                                <span>"Username"</span>
                                <input
                                    type="text"
                                    class="auth-input"
                                    placeholder="Username (e.g. demo)"
                                    prop:value=username
                                    on:input=move |e| set_username.set(event_target_value(&e))
                                />
                            </label>

                            <label class="auth-label">
                                <span>"Password"</span>
                                <input
                                    type="password"
                                    class="auth-input"
                                    placeholder="Password"
                                    prop:value=password
                                    on:input=move |e| set_password.set(event_target_value(&e))
                                />
                            </label>

                            <footer class="auth-footer">
                                <button type="button" class="btn-secondary" on:click=move |_| open.set(false)>"Cancel"</button>
                                <button type="submit" class="btn-primary" disabled=move || submitting.get()>
                                    {move || if submitting.get() { "Signing in…" } else { "Sign in" }}
                                </button>
                            </footer>
                        </form>
                    </div>
                </div>
            </div>
        </Show>
    }
}
