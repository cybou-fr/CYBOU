// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! File Manager tool card component for sandboxed filesystem exploration.

use cybou_web_contracts::SessionMode;
use leptos::prelude::*;
use leptos::task::spawn_local;
use web_sys::PointerEvent;

use crate::{
    CardId, DesktopLayout, GatewayMindClient, MindClient,
    components::{
        card_controls::{CardControls, CardResizeHandle},
        icons::{IconArrowLeft, IconCopy, IconFile, IconFolder, IconHome, IconRefresh, IconShield},
    },
    interaction::{DragState, ResizeState, card_style, start_drag},
    state::RuntimeState,
};

/// Bounded File Manager tool card component.
#[component]
pub fn FileManagerCard(
    layout: RwSignal<DesktopLayout>,
    selected: ReadSignal<&'static str>,
    set_selected: WriteSignal<&'static str>,
    dragging: RwSignal<Option<DragState>>,
    resizing: RwSignal<Option<ResizeState>>,
    auth_modal_open: RwSignal<bool>,
    runtime: RwSignal<RuntimeState>,
) -> impl IntoView {
    let card_id = CardId::FileManager(0);
    let card_open =
        move || layout.get().contains_card(card_id) && !layout.get().is_in_deck(card_id);
    let is_collapsed = move || layout.get().presentation(card_id).collapsed;
    let is_public_preview = move || match runtime.get() {
        RuntimeState::Ready { mode, .. } => mode == SessionMode::PublicPreview,
        _ => false,
    };

    let (current_path, set_current_path) = signal("/".to_string());
    let (entries, set_entries) = signal(Vec::<(String, bool, u64)>::new());
    let (selected_file, set_selected_file) = signal(Option::<String>::None);
    let (file_content, set_file_content) = signal(String::new());
    let (loading, set_loading) = signal(false);
    let (error_msg, set_error_msg) = signal(Option::<String>::None);

    let load_dir = move |path: String| {
        set_loading.set(true);
        set_error_msg.set(None);
        set_selected_file.set(None);
        let target_p = path.clone();
        set_current_path.set(path);
        spawn_local(async move {
            let client = GatewayMindClient;
            let cmd = if target_p == "/" || target_p.is_empty() {
                "ls -la".to_string()
            } else {
                format!("ls -la {target_p}")
            };
            match client.execute_shell(&cmd).await {
                Ok(resp) => {
                    set_loading.set(false);
                    if resp.exit_code == 0 {
                        let mut list = Vec::new();
                        for line in resp.stdout.lines() {
                            let trimmed = line.trim();
                            if trimmed.is_empty() || trimmed.starts_with("total") {
                                continue;
                            }
                            let parts: Vec<&str> = trimmed.split_whitespace().collect();
                            if parts.len() >= 9 {
                                let is_dir = parts[0].starts_with('d');
                                let size: u64 = parts[4].parse().unwrap_or(0);
                                let name = parts[8..].join(" ");
                                if name != "." && name != ".." {
                                    list.push((name, is_dir, size));
                                }
                            } else if parts.len() == 1 {
                                let is_dir = parts[0].ends_with('/');
                                let name = parts[0].trim_end_matches('/').to_string();
                                if name != "." && name != ".." {
                                    list.push((name, is_dir, 0));
                                }
                            }
                        }
                        list.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
                        set_entries.set(list);
                    } else {
                        set_error_msg.set(Some(if !resp.stderr.is_empty() {
                            resp.stderr
                        } else {
                            "Directory read error".to_string()
                        }));
                    }
                }
                Err(err) => {
                    set_loading.set(false);
                    set_error_msg.set(Some(format!("Network error: {err}")));
                }
            }
        });
    };

    let view_file = move |name: String| {
        set_selected_file.set(Some(name.clone()));
        set_loading.set(true);
        let p = if current_path.get() == "/" {
            format!("/{name}")
        } else {
            format!("{}/{name}", current_path.get())
        };
        spawn_local(async move {
            let client = GatewayMindClient;
            match client.execute_shell(&format!("cat {p}")).await {
                Ok(resp) => {
                    set_loading.set(false);
                    if resp.exit_code == 0 {
                        set_file_content.set(resp.stdout);
                    } else {
                        set_file_content.set(if !resp.stderr.is_empty() {
                            resp.stderr
                        } else {
                            "Could not read file".to_string()
                        });
                    }
                }
                Err(err) => {
                    set_loading.set(false);
                    set_file_content.set(format!("Error: {err}"));
                }
            }
        });
    };

    let go_up = move || {
        let cur = current_path.get();
        if cur != "/" && !cur.is_empty() {
            if let Some(pos) = cur.rfind('/') {
                let parent = if pos == 0 {
                    "/".to_string()
                } else {
                    cur[..pos].to_string()
                };
                load_dir(parent);
            }
        }
    };

    view! {
        <Show when=card_open>
            <div
                class:selected=move || selected.get() == "files"
                class:collapsed=is_collapsed
                class:pinned=move || layout.get().presentation(card_id).pinned
                class="object file-manager-card"
                style=move || card_style(layout.get(), card_id)
                tabindex="0"
                role="region"
                aria-label="Bounded File Manager"
                on:click=move |_| {
                    set_selected.set("files");
                    layout.update(|current| current.bring_forward(card_id));
                }
            >
                <header
                    class="object-header"
                    on:pointerdown=move |event: PointerEvent| {
                        start_drag(event, card_id, layout, dragging);
                    }
                >
                    <span class="card-title-group">
                        <IconFolder size=13 />
                        <strong class="card-title">"File Manager"</strong>
                        <small class="card-badge">"Zone 3 Read-Only"</small>
                    </span>
                    <CardControls card=card_id layout=layout />
                </header>

                <Show when=move || !is_collapsed()>
                    <Show
                        when=move || !is_public_preview()
                        fallback=move || view! {
                            <div class="card-auth-gate">
                                <IconShield size=26 />
                                <strong>"File Manager Locked"</strong>
                                <p>"Public preview does not permit sandboxed storage browsing. Sign in with Linux PAM credentials to unlock."</p>
                                <button class="primary-btn" on:click=move |_| auth_modal_open.set(true)>"Sign in"</button>
                            </div>
                        }
                    >
                        <div class="fm-body" on:pointerdown=move |e: PointerEvent| e.stop_propagation()>
                            <div class="fm-path-bar">
                                <div class="fm-crumbs">
                                    <span>{move || current_path.get()}</span>
                                </div>
                                <div class="fm-toolbar">
                                    <button class="fm-btn" title="Root" on:click=move |_| load_dir("/".to_string())>
                                        <IconHome size=12 />
                                        <span>"Root"</span>
                                    </button>
                                    <button class="fm-btn" title="Up one level" on:click=move |_| go_up()>
                                        <IconArrowLeft size=12 />
                                        <span>"Up"</span>
                                    </button>
                                    <button class="fm-btn" title="Refresh folder" on:click=move |_| load_dir(current_path.get())>
                                        <IconRefresh size=12 />
                                        <span>"Refresh"</span>
                                    </button>
                                </div>
                            </div>

                            <Show when=move || error_msg.get().is_some()>
                                <div class="auth-error">
                                    {move || error_msg.get().unwrap_or_default()}
                                </div>
                            </Show>

                            <div class="fm-content">
                                <div class="fm-grid">
                                    <Show when=move || loading.get()>
                                        <div class="fm-empty">"Loading directory…"</div>
                                    </Show>
                                    <Show when=move || !loading.get() && entries.get().is_empty() && error_msg.get().is_none()>
                                        <div class="fm-empty">"Empty directory"</div>
                                    </Show>
                                    <For
                                        each=move || entries.get()
                                        key=|(name, is_dir, _)| format!("{name}-{is_dir}")
                                        children=move |(name, is_dir, size)| {
                                            let n = name.clone();
                                            let n_click = name.clone();
                                            let p = current_path.get();
                                            view! {
                                                <div
                                                    class="fm-item"
                                                    class:is-dir=is_dir
                                                    class:is-file=!is_dir
                                                    on:click=move |_| {
                                                        if is_dir {
                                                            let new_p = if p == "/" {
                                                                format!("/{}", n_click)
                                                            } else {
                                                                format!("{}/{}", p, n_click)
                                                            };
                                                            load_dir(new_p);
                                                        } else {
                                                            view_file(n_click.clone());
                                                        }
                                                    }
                                                >
                                                    {if is_dir {
                                                        view! { <IconFolder size=24 /> }.into_any()
                                                    } else {
                                                        view! { <IconFile size=24 /> }.into_any()
                                                    }}
                                                    <span class="fm-item-name">{n}</span>
                                                    <span class="fm-item-size">{if is_dir { "dir".to_string() } else { format!("{size} B") }}</span>
                                                </div>
                                            }
                                        }
                                    />
                                </div>

                                <Show when=move || selected_file.get().is_some()>
                                    <aside class="fm-preview">
                                        <header class="fm-preview-header">
                                             <span><IconFile size=12 /> " " {move || selected_file.get().unwrap_or_default()}</span>
                                            <div class="fm-preview-actions">
                                                <button
                                                    class="fm-btn"
                                                    title="Copy file text to clipboard"
                                                    on:click=move |_| {
                                                        let text = file_content.get();
                                                        if let Some(window) = web_sys::window() {
                                                            let _ = window.navigator().clipboard().write_text(&text);
                                                        }
                                                    }
                                                >
                                                    <IconCopy size=12 />
                                                    <span>"Copy"</span>
                                                </button>
                                                <small class="fm-readonly-pill">"Read-only"</small>
                                                <button class="fm-btn" title="Close preview" on:click=move |_| set_selected_file.set(None)>"×"</button>
                                            </div>
                                        </header>
                                        <pre class="fm-preview-text">{move || file_content.get()}</pre>
                                    </aside>
                                </Show>
                            </div>
                        </div>
                    </Show>
                </Show>
                <CardResizeHandle card=card_id layout=layout resizing=resizing />
            </div>
        </Show>
    }
}
