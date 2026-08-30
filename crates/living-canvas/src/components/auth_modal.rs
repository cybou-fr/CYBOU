// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Signing in to this machine.
//!
//! Two surfaces, one form. `SignInView` is the whole screen and cannot be dismissed, because on a
//! deployment that serves nothing until somebody signs in there is nothing behind it to dismiss it
//! to. `AuthModal` is the dialog a person already looking at their own desktop opens to become
//! somebody, and it closes.

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::{GatewayMindClient, components::icons::IconShield};

/// The sign-in form and everything it needs to do its job.
///
/// Shared so the dialog and the full-screen gate cannot drift into saying different things about
/// the same act.
#[component]
fn SignInForm(
    /// Set to `false` when a session is established, if there is anything to close.
    #[prop(optional)]
    open: Option<RwSignal<bool>>,
) -> impl IntoView {
    let (username, set_username) = signal(String::new());
    let (password, set_password) = signal(String::new());
    let (error_msg, set_error_msg) = signal(Option::<String>::None);
    let (submitting, set_submitting) = signal(false);

    let do_login = move || {
        let u = username.get();
        let p = password.get();
        if u.is_empty() {
            set_error_msg.set(Some("Enter your username".to_string()));
            return;
        }
        set_submitting.set(true);
        set_error_msg.set(None);
        spawn_local(async move {
            match GatewayMindClient.login(&u, &p).await {
                Ok(true) => {
                    set_submitting.set(false);
                    if let Some(open) = open {
                        open.set(false);
                    }
                    if let Some(w) = web_sys::window() {
                        let _ = w.location().reload();
                    }
                }
                Ok(false) => {
                    // What the gateway said, which is one bit. It does not say whether the account
                    // exists, and neither does this: a sign-in screen that distinguished the two
                    // would be a way to find out who has an account on somebody else's machine.
                    set_submitting.set(false);
                    set_error_msg.set(Some("That username and password were not accepted.".into()));
                }
                Err(err) => {
                    set_submitting.set(false);
                    set_error_msg.set(Some(format!("Could not reach this machine: {err}")));
                }
            }
        });
    };

    view! {
        <div class="auth-body">
            <Show when=move || error_msg.get().is_some()>
                <div class="auth-error" role="alert">{move || error_msg.get().unwrap_or_default()}</div>
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
                        autocomplete="username"
                        prop:value=username
                        on:input=move |e| set_username.set(event_target_value(&e))
                    />
                </label>

                <label class="auth-label">
                    <span>"Password"</span>
                    <input
                        type="password"
                        class="auth-input"
                        autocomplete="current-password"
                        prop:value=password
                        on:input=move |e| set_password.set(event_target_value(&e))
                    />
                </label>

                <footer class="auth-footer">
                    {open.map(|open| view! {
                        <button type="button" class="btn-secondary" on:click=move |_| open.set(false)>
                            "Cancel"
                        </button>
                    })}
                    <button type="submit" class="btn-primary" disabled=move || submitting.get()>
                        {move || if submitting.get() { "Signing in…" } else { "Sign in" }}
                    </button>
                </footer>
            </form>
        </div>
    }
}

/// The whole screen, when nothing is served until somebody signs in.
///
/// Not dismissable and deliberately empty behind: a gate a person can close is not a gate, and a
/// desktop drawn underneath one would be showing what the gate exists to withhold.
#[component]
pub fn SignInView() -> impl IntoView {
    view! {
        <main class="sign-in-screen">
            <div class="sign-in-card">
                <img class="sign-in-mark" src="/cybou-mark.svg" alt="" />
                <h1>"CYBOU"</h1>
                <p class="auth-desc">"Sign in with an account on this machine."</p>
                <SignInForm />
            </div>
        </main>
    }
}

/// The sign-in dialog, for somebody already looking at their own desktop.
#[component]
pub fn AuthModal(open: RwSignal<bool>) -> impl IntoView {
    view! {
        <Show when=move || open.get()>
            <div class="modal-overlay" on:click=move |_| open.set(false)>
                <div class="auth-modal" role="dialog" aria-modal="true" aria-label="Sign in" on:click=move |e: web_sys::MouseEvent| e.stop_propagation()>
                    <header class="auth-header">
                        <div class="auth-title">
                            <IconShield size=18 />
                            <h3>"Sign in"</h3>
                        </div>
                        <button class="modal-close-btn" on:click=move |_| open.set(false)>"×"</button>
                    </header>
                    <p class="auth-desc">"Sign in with an account on this machine."</p>
                    <SignInForm open=open />
                </div>
            </div>
        </Show>
    }
}
