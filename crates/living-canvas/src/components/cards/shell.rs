// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! CYBOU Shell tool card and content component for bounded execution in the Body sandbox.

use cybou_web_contracts::SessionMode;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::sync::Arc;
use web_sys::{KeyboardEvent, PointerEvent};

use crate::{
    CardId, DesktopItemId, DesktopLayout, GatewayMindClient, MindClient,
    ansi::AnsiOutput,
    components::{
        card_frame::CardFrame,
        icons::{IconCopy, IconShield, IconTerminal},
    },
    interaction::{DragState, ResizeState},
    state::RuntimeState,
    tool_state::ToolCardStates,
};

const SHELL_AUTOCOMPLETE: &[&str] = &[
    "cat", "cd", "clear", "date", "du", "echo", "file", "find", "grep", "head", "help", "ls",
    "pwd", "stat", "tail", "wc",
];

const SHELL_QUICK_COMMANDS: &[&str] = &["help", "ls -la", "pwd", "date", "du -h", "stat ."];

/// Interactive Shell domain content presentation.
#[component]
pub fn ShellContent(
    runtime: RwSignal<RuntimeState>,
    auth_modal_open: RwSignal<bool>,
    /// Which Shell card this is, taken from `CardId::Shell(n)`.
    ///
    /// Sent with every command so the gateway drives this card's shell and not another card's.
    #[prop(optional)]
    instance: u32,
) -> impl IntoView {
    let is_public_preview = move || match runtime.get() {
        RuntimeState::Ready { mode, .. } => mode == SessionMode::PublicPreview,
        _ => false,
    };

    // Looked up, not created. These signals are owned by the root, so collapsing this card,
    // switching away from its deck tab, or docking it does not destroy what was typed into it.
    let state = expect_context::<ToolCardStates>().shell(CardId::Shell(instance));
    let (history, set_history) = (state.history, state.history);
    let (cmd_history, set_cmd_history) = (state.cmd_history, state.cmd_history);
    let (history_idx, set_history_idx) = (state.history_idx, state.history_idx);
    let (temp_draft, set_temp_draft) = (state.temp_draft, state.temp_draft);
    let (input_val, set_input_val) = (state.input, state.input);
    let (cwd, set_cwd) = (state.cwd, state.cwd);
    let (running, set_running) = (state.running, state.running);

    // These stay component-local on purpose: they point at DOM nodes, which really do belong to
    // one mount.
    let output_ref = NodeRef::<leptos::html::Div>::new();
    let input_ref = NodeRef::<leptos::html::Input>::new();

    let scroll_output_to_bottom = move || {
        if let Some(el) = output_ref.get() {
            el.set_scroll_top(el.scroll_height());
        }
    };

    let run_command_str = move |cmd_str: String| {
        let trimmed = cmd_str.trim().to_string();
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
        set_cmd_history.update(|h| {
            if h.last() != Some(&trimmed) {
                h.push(trimmed.clone());
            }
        });
        set_history_idx.set(None);
        set_temp_draft.set(String::new());
        set_running.set(true);
        set_input_val.set(String::new());
        let to_exec = trimmed.clone();
        spawn_local(async move {
            let client = GatewayMindClient;
            match client.execute_shell(&to_exec, instance).await {
                Ok(resp) => {
                    let text = crate::tool_state::merge_shell_output(&resp.stdout, &resp.stderr);
                    set_cwd.set(resp.cwd);
                    set_history.update(|h| h.push((to_exec, text, resp.exit_code)));
                }
                Err(e) => {
                    set_history.update(|h| h.push((to_exec, format!("Error: {e}\n"), 1)));
                }
            }
            set_running.set(false);
            scroll_output_to_bottom();
        });
    };

    let submit_command = move || {
        let cmd = input_val.get();
        run_command_str(cmd);
    };

    let copy_entire_session = move || {
        let entries = history.get();
        let mut full_log = String::new();
        for (cmd, out, _) in entries {
            full_log.push_str("› ");
            full_log.push_str(&cmd);
            full_log.push('\n');
            full_log.push_str(&out);
            if !out.ends_with('\n') {
                full_log.push('\n');
            }
        }
        if let Some(window) = web_sys::window() {
            let _ = window.navigator().clipboard().write_text(&full_log);
        }
    };

    view! {
        <Show
            when=move || !is_public_preview()
            fallback=move || view! {
                <div class="card-auth-gate">
                    <IconShield size=26 />
                    <strong>"Shell locked"</strong>
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
                // Terminal Top Toolbar
                <div class="shell-topbar">
                    <div class="shell-topbar-left">
                        <span class="shell-cwd-chip" title="Current working directory">
                            {move || format!("cwd: {}", cwd.get())}
                        </span>
                        <span
                            class="shell-status-badge"
                            class:running=move || running.get()
                        >
                            {move || if running.get() { "running" } else { "ready" }}
                        </span>
                    </div>
                    <div class="shell-topbar-actions">
                        <button
                            class="shell-action-btn"
                            title="Copy entire session to clipboard"
                            disabled=move || history.get().is_empty()
                            on:click=move |e: web_sys::MouseEvent| {
                                e.stop_propagation();
                                copy_entire_session();
                            }
                        >
                            <IconCopy size=11 />
                            <span>"Copy All"</span>
                        </button>
                        <button
                            class="shell-action-btn"
                            title="Clear terminal history"
                            disabled=move || history.get().is_empty()
                            on:click=move |e: web_sys::MouseEvent| {
                                e.stop_propagation();
                                set_history.set(Vec::new());
                                set_history_idx.set(None);
                            }
                        >
                            <span>"Clear"</span>
                        </button>
                    </div>
                </div>

                <div class="shell-output" node_ref=output_ref>
                    <Show when=move || history.get().is_empty()>
                        <div class="shell-welcome-banner">
                            <div class="shell-welcome-title">"CYBOU Safe Shell"</div>
                            <div class="shell-welcome-desc">"Bounded execution sandbox. Click a quick command or type below:"</div>
                            <div class="shell-chips-container">
                                <For
                                    each=move || SHELL_QUICK_COMMANDS.to_vec()
                                    key=|cmd| cmd.to_string()
                                    children=move |cmd| {
                                        let cmd_str = cmd.to_string();
                                        let cmd_click = cmd.to_string();
                                        view! {
                                            <button
                                                class="shell-chip"
                                                on:click=move |e: web_sys::MouseEvent| {
                                                    e.stop_propagation();
                                                    run_command_str(cmd_click.clone());
                                                }
                                            >
                                                {cmd_str}
                                            </button>
                                        }
                                    }
                                />
                            </div>
                        </div>
                    </Show>

                    <For
                        each=move || history.get()
                        key=|(cmd, out, code)| format!("{cmd}-{out}-{code}")
                        children=move |(cmd, out, code)| {
                            let out_copy = out.clone();
                            view! {
                                <div class="shell-entry">
                                    {if cmd.is_empty() {
                                        view! { <span/> }.into_any()
                                    } else {
                                        view! {
                                            <div class="shell-entry-header">
                                                <div class="shell-cmd-echo">
                                                    <span class="shell-prompt-char">"›"</span>" "{cmd}
                                                </div>
                                                <div class="shell-entry-meta">
                                                    <span
                                                        class="shell-exit-badge"
                                                        class:error=move || code != 0
                                                        title=if code == 0 { "Success".to_string() } else { format!("Exit status {code}") }
                                                    >
                                                        {if code == 0 { "0".to_string() } else { format!("exit {code}") }}
                                                    </span>
                                                    <button
                                                        class="shell-entry-copy-btn"
                                                        title="Copy command output"
                                                        on:click=move |e: web_sys::MouseEvent| {
                                                            e.stop_propagation();
                                                            if let Some(window) = web_sys::window() {
                                                                let _ = window.navigator().clipboard().write_text(&out_copy);
                                                            }
                                                        }
                                                    >
                                                        <IconCopy size=10 />
                                                    </button>
                                                </div>
                                            </div>
                                        }.into_any()
                                    }}
                                    <AnsiOutput content=Signal::derive(move || out.clone()) is_error=code != 0 />
                                </div>
                            }
                        }
                    />
                </div>
                <div class="shell-input-line" on:pointerdown=move |e: PointerEvent| e.stop_propagation()>
                    <span class="shell-prompt">{move || format!("{} ›", cwd.get())}</span>
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
    selected: ReadSignal<Option<DesktopItemId>>,
    set_selected: WriteSignal<Option<DesktopItemId>>,
    dragging: RwSignal<Option<DragState>>,
    resizing: RwSignal<Option<ResizeState>>,
    auth_modal_open: RwSignal<bool>,
    runtime: RwSignal<RuntimeState>,
    /// Which Shell card this is.
    #[prop(optional)]
    instance: u32,
) -> impl IntoView {
    let card_id = CardId::Shell(instance);

    let collapsed = move || {
        view! {
            <div class="card-collapsed-summary">
                <b>"Shell"</b>
                <span>"Bounded, read-only"</span>
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
            kicker_title="Shell"
            kicker_icon=Arc::new(|| view! { <IconTerminal size=14 /> }.into_any())
            collapsed_summary=Arc::new(collapsed)
        >
            <ShellContent runtime=runtime auth_modal_open=auth_modal_open instance=instance />
        </CardFrame>
    }
}
