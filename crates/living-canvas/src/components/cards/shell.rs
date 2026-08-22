// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! CYBOU Shell tool card and content component for bounded execution in the Body sandbox.

use cybou_web_contracts::SessionMode;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::sync::Arc;
use web_sys::{KeyboardEvent, PointerEvent};

use crate::{
    CardId, DesktopLayout, GatewayMindClient, MindClient,
    components::{
        card_frame::CardFrame,
        icons::{IconShield, IconTerminal},
    },
    interaction::{DragState, ResizeState},
    state::RuntimeState,
};

const SHELL_AUTOCOMPLETE: &[&str] = &[
    "cat", "cd", "clear", "echo", "grep", "head", "help", "ls", "pwd", "stat", "tail", "uname",
    "whoami",
];

/// Interactive Shell domain content presentation.
#[component]
pub fn ShellContent(
    runtime: RwSignal<RuntimeState>,
    auth_modal_open: RwSignal<bool>,
) -> impl IntoView {
    let is_public_preview = move || match runtime.get() {
        RuntimeState::Ready { mode, .. } => mode == SessionMode::PublicPreview,
        _ => false,
    };

    let (history, set_history) = signal(vec![(
        String::new(),
        "CYBOU Bounded Body Shell (ADR-0040 DemoReadOnly)\nType 'help' for available capabilities.\n"
            .to_string(),
        0,
    )]);
    let (cmd_history, set_cmd_history) = signal(Vec::<String>::new());
    let (history_idx, set_history_idx) = signal(Option::<usize>::None);
    let (temp_draft, set_temp_draft) = signal(String::new());

    let (input_val, set_input_val) = signal(String::new());
    let (cwd, set_cwd) = signal("/".to_string());
    let (running, set_running) = signal(false);
    let output_ref = NodeRef::<leptos::html::Div>::new();
    let input_ref = NodeRef::<leptos::html::Input>::new();

    let scroll_output_to_bottom = move || {
        if let Some(el) = output_ref.get() {
            el.set_scroll_top(el.scroll_height());
        }
    };

    let submit_command = move || {
        let cmd = input_val.get();
        let trimmed = cmd.trim();
        if trimmed.is_empty() {
            return;
        }
        if trimmed == "clear" {
            set_history.set(Vec::new());
            set_input_val.set(String::new());
            set_history_idx.set(None);
            set_temp_draft.set(String::new());
            return;
        }
        let cmd_str = trimmed.to_string();
        set_cmd_history.update(|h| {
            if h.last() != Some(&cmd_str) {
                h.push(cmd_str.clone());
            }
        });
        set_history_idx.set(None);
        set_temp_draft.set(String::new());
        set_running.set(true);
        set_input_val.set(String::new());
        spawn_local(async move {
            let client = GatewayMindClient;
            match client.execute_shell(&cmd_str).await {
                Ok(resp) => {
                    let text = if !resp.stdout.is_empty() {
                        resp.stdout
                    } else if !resp.stderr.is_empty() {
                        resp.stderr
                    } else {
                        String::new()
                    };
                    set_cwd.set(resp.cwd);
                    set_history.update(|h| h.push((cmd_str, text, resp.exit_code)));
                }
                Err(e) => {
                    set_history.update(|h| h.push((cmd_str, format!("Error: {e}\n"), 1)));
                }
            }
            set_running.set(false);
            scroll_output_to_bottom();
        });
    };

    view! {
        <Show
            when=move || !is_public_preview()
            fallback=move || view! {
                <div class="card-auth-gate">
                    <IconShield size=26 />
                    <strong>"CYBOU Shell Locked"</strong>
                    <p>"Public preview does not permit Body capabilities execution. Sign in with Linux PAM credentials to unlock."</p>
                    <button class="primary-btn" on:click=move |_| auth_modal_open.set(true)>"Sign in"</button>
                </div>
            }
        >
            <div
                class="shell-body"
                on:pointerdown=move |e: PointerEvent| e.stop_propagation()
                on:click=move |_| {
                    if let Some(inp) = input_ref.get() {
                        let _ = inp.focus();
                    }
                }
            >
                <div class="shell-output" node_ref=output_ref>
                    <For
                        each=move || history.get()
                        key=|(cmd, out, code)| format!("{cmd}-{out}-{code}")
                        children=move |(cmd, out, code)| {
                            view! {
                                <div class="shell-entry">
                                    {if !cmd.is_empty() {
                                        view! { <div class="shell-cmd-echo"><span class="shell-prompt-char">"›"</span>" "{cmd}</div> }.into_any()
                                    } else {
                                        ().into_any()
                                    }}
                                    <pre class="shell-out-text" class:error=move || code != 0>{out}</pre>
                                </div>
                            }
                        }
                    />
                </div>
                <div class="shell-input-line" on:pointerdown=move |e: PointerEvent| e.stop_propagation()>
                    <span class="shell-prompt">{move || format!("cybou:{} ›", cwd.get())}</span>
                    <input
                        node_ref=input_ref
                        type="text"
                        class="shell-input"
                        placeholder=move || if running.get() { "running…" } else { "type a command ('help')…" }
                        disabled=move || running.get()
                        prop:value=move || input_val.get()
                        on:pointerdown=move |e: PointerEvent| e.stop_propagation()
                        on:input=move |e| {
                            set_input_val.set(event_target_value(&e));
                            set_history_idx.set(None);
                        }
                        on:keydown=move |e: KeyboardEvent| {
                            let key = e.key();
                            if key == "Enter" {
                                e.prevent_default();
                                submit_command();
                            } else if key == "ArrowUp" {
                                e.prevent_default();
                                let cmds = cmd_history.get();
                                if cmds.is_empty() {
                                    return;
                                }
                                let current_idx = history_idx.get();
                                let next_idx = match current_idx {
                                    None => {
                                        set_temp_draft.set(input_val.get());
                                        cmds.len().saturating_sub(1)
                                    }
                                    Some(idx) => idx.saturating_sub(1),
                                };
                                set_history_idx.set(Some(next_idx));
                                if let Some(cmd) = cmds.get(next_idx) {
                                    set_input_val.set(cmd.clone());
                                }
                            } else if key == "ArrowDown" {
                                e.prevent_default();
                                let cmds = cmd_history.get();
                                if let Some(idx) = history_idx.get() {
                                    if idx + 1 < cmds.len() {
                                        let next_idx = idx + 1;
                                        set_history_idx.set(Some(next_idx));
                                        if let Some(cmd) = cmds.get(next_idx) {
                                            set_input_val.set(cmd.clone());
                                        }
                                    } else {
                                        set_history_idx.set(None);
                                        set_input_val.set(temp_draft.get());
                                    }
                                }
                            } else if key == "Tab" {
                                e.prevent_default();
                                let current = input_val.get();
                                let trimmed = current.trim();
                                if !trimmed.is_empty() && !trimmed.contains(' ') {
                                    let matches: Vec<&&str> = SHELL_AUTOCOMPLETE
                                        .iter()
                                        .filter(|c| c.starts_with(trimmed))
                                        .collect();
                                    if matches.len() == 1 {
                                        set_input_val.set(format!("{} ", matches[0]));
                                    }
                                }
                            }
                        }
                    />
                </div>
            </div>
        </Show>
    }
}

/// CYBOU Shell interactive tool card component.
#[component]
pub fn ShellCard(
    layout: RwSignal<DesktopLayout>,
    selected: ReadSignal<&'static str>,
    set_selected: WriteSignal<&'static str>,
    dragging: RwSignal<Option<DragState>>,
    resizing: RwSignal<Option<ResizeState>>,
    auth_modal_open: RwSignal<bool>,
    runtime: RwSignal<RuntimeState>,
) -> impl IntoView {
    let card_id = CardId::Shell(0);

    let collapsed = move || {
        view! {
            <div class="card-collapsed-summary">
                <b>"Shell"</b>
                <span>"Zone 3 Body"</span>
            </div>
        }
        .into_any()
    };

    view! {
        <CardFrame
            card=card_id
            layout=layout
            selected=selected
            set_selected=set_selected
            dragging=dragging
            resizing=resizing
            kicker_title="CYBOU Shell · Zone 3 Body"
            kicker_icon=Arc::new(|| view! { <IconTerminal size=14 /> }.into_any())
            collapsed_summary=Arc::new(collapsed)
        >
            <ShellContent runtime=runtime auth_modal_open=auth_modal_open />
        </CardFrame>
    }
}
