// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! File Manager tool card and content component for sandboxed filesystem exploration.

use cybou_web_contracts::SessionMode;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::sync::Arc;
use web_sys::PointerEvent;

use crate::{
    CardId, DesktopItemId, DesktopLayout, GatewayMindClient, MindClient,
    components::{
        card_frame::CardFrame,
        icons::{IconArrowLeft, IconCopy, IconFile, IconFolder, IconHome, IconRefresh, IconShield},
    },
    interaction::{DragState, ResizeState},
    state::RuntimeState,
    tool_state::ToolCardStates,
};

/// File Manager domain content presentation.
#[component]
pub fn FileManagerContent(
    runtime: RwSignal<RuntimeState>,
    auth_modal_open: RwSignal<bool>,
    /// Which File Manager card this is, taken from `CardId::FileManager(n)`.
    #[prop(optional)]
    instance: u32,
) -> impl IntoView {
    let is_public_preview = move || match runtime.get() {
        RuntimeState::Ready { mode, .. } => mode == SessionMode::PublicPreview,
        _ => false,
    };

    // Looked up, not created. The directory a person navigated to is something they did, and
    // collapsing the card or switching a deck tab is not them undoing it.
    let state = expect_context::<ToolCardStates>().file_manager(CardId::FileManager(instance));
    let (current_path, set_current_path) = (state.current_path, state.current_path);
    let (entries, set_entries) = (state.entries, state.entries);
    let (selected_file, set_selected_file) = (state.selected_file, state.selected_file);
    let (file_content, set_file_content) = (state.file_content, state.file_content);
    let (loading, set_loading) = (state.loading, state.loading);
    let (error_msg, set_error_msg) = (state.error_msg, state.error_msg);
    let (was_read, set_was_read) = (state.read, state.read);

    let load_dir = move |path: String| {
        set_loading.set(true);
        set_error_msg.set(None);
        set_selected_file.set(None);
        let target_p = path.clone();
        set_current_path.set(path);
        spawn_local(async move {
            match GatewayMindClient.list_directory(&target_p).await {
                Ok(listing) => {
                    set_loading.set(false);
                    set_was_read.set(true);
                    if listing.truncated {
                        // A bounded listing says so. Showing the first five hundred entries as if
                        // they were all of them would be a smaller directory, not a partial answer.
                        set_error_msg.set(Some(format!(
                            "Showing {} of {} entries",
                            listing.entries.len(),
                            listing.total_entries
                        )));
                    }
                    set_entries.set(
                        listing
                            .entries
                            .into_iter()
                            .map(|entry| (entry.name, entry.is_dir, entry.size_bytes))
                            .collect(),
                    );
                }
                Err(err) => {
                    set_loading.set(false);
                    // Not read, so the panel keeps saying it has not read rather than reporting an
                    // empty directory it never saw.
                    set_entries.set(Vec::new());
                    set_error_msg.set(Some(err.to_string()));
                }
            }
        });
    };

    // Read once, the first time this card is shown. Without it the panel opened on an assertion
    // about a directory it had never asked about, and a person had to press Refresh to find out
    // whether the first screen had been true.
    Effect::new(move |_| {
        if !was_read.get_untracked() && !loading.get_untracked() {
            load_dir(current_path.get_untracked());
        }
    });

    let view_file = move |name: String| {
        set_selected_file.set(Some(name.clone()));
        set_loading.set(true);
        let p = if current_path.get() == "/" {
            format!("/{name}")
        } else {
            format!("{}/{name}", current_path.get())
        };
        spawn_local(async move {
            match GatewayMindClient.read_text_file(&p).await {
                Ok(content) => {
                    set_loading.set(false);
                    set_file_content.set(content.text);
                }
                Err(err) => {
                    set_loading.set(false);
                    set_file_content.set(err.to_string());
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
                        <Show when=move || !loading.get() && entries.get().is_empty() && error_msg.get().is_none() && was_read.get()>
                            <div class="fm-empty">"Empty directory"</div>
                        </Show>
                        <Show when=move || !loading.get() && !was_read.get() && error_msg.get().is_none()>
                            <div class="fm-empty">"Not read yet"</div>
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
    }
}

/// Bounded File Manager tool card component.
#[component]
pub fn FileManagerCard(
    layout: RwSignal<DesktopLayout>,
    selected: ReadSignal<Option<DesktopItemId>>,
    set_selected: WriteSignal<Option<DesktopItemId>>,
    dragging: RwSignal<Option<DragState>>,
    resizing: RwSignal<Option<ResizeState>>,
    auth_modal_open: RwSignal<bool>,
    runtime: RwSignal<RuntimeState>,
    /// Which instance of this tool card this is.
    #[prop(optional)]
    instance: u32,
) -> impl IntoView {
    let card_id = CardId::FileManager(instance);

    let collapsed = move || {
        view! {
            <div class="card-collapsed-summary">
                <b>"File Manager"</b>
                <span>"Read-only"</span>
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
            kicker_title="File Manager"
            kicker_icon=Arc::new(|| view! { <IconFolder size=14 /> }.into_any())
            collapsed_summary=Arc::new(collapsed)
        >
            <FileManagerContent runtime=runtime auth_modal_open=auth_modal_open instance=instance />
        </CardFrame>
    }
}
