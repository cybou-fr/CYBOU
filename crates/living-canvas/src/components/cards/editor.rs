// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Text and configuration editor tool card component (ADR-0045).

use cybou_web_contracts::SessionMode;
use leptos::prelude::*;
use std::sync::Arc;
use web_sys::{MouseEvent, PointerEvent};

use crate::{
    CardId, DesktopItemId, DesktopLayout,
    components::{
        card_frame::CardFrame,
        icons::{IconFile, IconShield},
    },
    interaction::{DragState, ResizeState},
    state::RuntimeState,
    tool_state::{EditorTab, ToolCardStates},
};

/// Text Editor content component with multi-tab buffers and Action1 authority gating.
#[component]
pub fn EditorContent(
    runtime: RwSignal<RuntimeState>,
    auth_modal_open: RwSignal<bool>,
    /// Instance identifier.
    #[prop(optional)]
    instance: u32,
) -> impl IntoView {
    let is_public_preview = move || match runtime.get() {
        RuntimeState::Ready { mode, .. } => mode == SessionMode::PublicPreview,
        _ => false,
    };

    let state = expect_context::<ToolCardStates>().editor(CardId::Editor(instance));
    let tabs = state.tabs;
    let active_tab_index = state.active_tab_index;
    let markdown_preview = state.markdown_preview;
    let save_proposal_open = state.save_proposal_open;
    let status_msg = state.status_msg;

    let active_tab = move || {
        let all_tabs = tabs.get();
        let idx = active_tab_index.get();
        all_tabs
            .get(idx)
            .cloned()
            .unwrap_or_else(EditorTab::untitled)
    };

    let update_current_content = move |new_content: String| {
        let idx = active_tab_index.get();
        tabs.update(|all| {
            if let Some(tab) = all.get_mut(idx) {
                tab.dirty = tab.content != new_content;
                tab.content = new_content;
            }
        });
    };

    let add_tab = move || {
        tabs.update(|all| {
            all.push(EditorTab::untitled());
        });
        active_tab_index.set(tabs.get().len().saturating_sub(1));
    };

    let close_tab = move |idx: usize| {
        tabs.update(|all| {
            if all.len() > 1 {
                all.remove(idx);
            }
        });
        if active_tab_index.get() >= tabs.get().len() {
            active_tab_index.set(tabs.get().len().saturating_sub(1));
        }
    };

    let trigger_save = move || {
        let tab = active_tab();
        if tab.location.requires_action_authorization() {
            save_proposal_open.set(true);
        } else {
            tabs.update(|all| {
                let idx = active_tab_index.get();
                if let Some(t) = all.get_mut(idx) {
                    t.original_content = t.content.clone();
                    t.dirty = false;
                }
            });
            status_msg.set(Some("Saved file successfully.".to_string()));
        }
    };

    view! {
        <Show
            when=move || !is_public_preview()
            fallback=move || view! {
                <div class="card-auth-gate">
                    <IconShield size=26 />
                    <strong>"Editor Locked"</strong>
                    <p>"Public preview does not permit workspace editing. Sign in to unlock."</p>
                    <button class="primary-btn" on:click=move |_| auth_modal_open.set(true)>"Sign in"</button>
                </div>
            }
        >
            <div class="editor-body" on:pointerdown=move |e: PointerEvent| e.stop_propagation()>
                <div class="editor-tab-bar">
                    <div class="editor-tabs">
                        <For
                            each=move || {
                                tabs.get()
                                    .into_iter()
                                    .enumerate()
                                    .map(|(i, t)| (i, t.name, t.dirty, t.location.display_path()))
                                    .collect::<Vec<_>>()
                            }
                            key=|(idx, name, dirty, path)| format!("{}-{}-{}-{}", idx, name, dirty, path)
                            children=move |(idx, name, dirty, _)| {
                                let is_active = move || active_tab_index.get() == idx;
                                view! {
                                    <div
                                        class="editor-tab"
                                        class:active=is_active
                                        on:click=move |_| active_tab_index.set(idx)
                                    >
                                        <span class="editor-tab-name">{name}</span>
                                        {if dirty {
                                            view! { <span class="editor-dirty-dot" title="Unsaved changes">"●"</span> }.into_any()
                                        } else {
                                            view! { <span/> }.into_any()
                                        }}
                                        <button
                                            class="editor-tab-close"
                                            title="Close tab"
                                            on:click=move |e: MouseEvent| {
                                                e.stop_propagation();
                                                close_tab(idx);
                                            }
                                        >
                                            "×"
                                        </button>
                                    </div>
                                }
                            }
                        />
                        <button class="editor-btn-new-tab" title="New file" on:click=move |_| add_tab()>
                            "+"
                        </button>
                    </div>

                    <div class="editor-actions">
                        {move || {
                            let tab = active_tab();
                            if tab.language == "markdown" {
                                view! {
                                    <button
                                        class="editor-action-btn"
                                        class:active=move || markdown_preview.get()
                                        on:click=move |_| markdown_preview.update(|v| *v = !*v)
                                    >
                                        "MD Preview"
                                    </button>
                                }.into_any()
                            } else {
                                view! { <span/> }.into_any()
                            }
                        }}
                        <button
                            class="editor-action-btn primary"
                            disabled=move || !active_tab().dirty
                            on:click=move |_| trigger_save()
                        >
                            "Save"
                        </button>
                    </div>
                </div>

                {move || {
                    let tab = active_tab();
                    if tab.location.requires_action_authorization() {
                        view! {
                            <div class="editor-authority-banner system">
                                <span class="badge">"System Config"</span>
                                <span>"Privileged location: direct write disabled. Saves route through Action1 proposal."</span>
                            </div>
                        }.into_any()
                    } else {
                        view! { <span/> }.into_any()
                    }
                }}

                <div class="editor-workspace" class:split-view=move || markdown_preview.get()>
                    <div class="editor-code-container">
                        <textarea
                            class="editor-textarea"
                            spellcheck="false"
                            prop:value=move || active_tab().content
                            on:input=move |e| {
                                update_current_content(event_target_value(&e));
                            }
                        />
                    </div>

                    <Show when=move || markdown_preview.get()>
                        <div class="editor-preview-container">
                            <div class="editor-preview-header">"Markdown Preview"</div>
                            <div class="editor-preview-content">
                                <pre class="editor-preview-raw">{move || active_tab().content}</pre>
                            </div>
                        </div>
                    </Show>
                </div>

                <div class="editor-status-bar">
                    <div class="editor-status-left">
                        <span>"Ln 1, Col 1"</span>
                        <span>"UTF-8"</span>
                        <span>"LF"</span>
                        <span class="editor-lang-badge">{move || active_tab().language}</span>
                    </div>
                    <div class="editor-status-right">
                        <span class="editor-path-label">{move || active_tab().location.display_path()}</span>
                    </div>
                </div>

                <Show when=move || save_proposal_open.get()>
                    <div class="editor-modal-overlay">
                        <div class="editor-modal-card">
                            <h3>"Save System Configuration"</h3>
                            <p>
                                "This modifies a privileged configuration file: "
                                <code>{move || active_tab().location.display_path()}</code>
                            </p>
                            <p class="modal-notice">
                                "CYBOU requires an Action1 FileWrite proposal with operator authorization before committing this change to disk."
                            </p>
                            <div class="editor-modal-actions">
                                <button class="editor-btn-secondary" on:click=move |_| save_proposal_open.set(false)>
                                    "Cancel"
                                </button>
                                <button
                                    class="editor-btn-primary"
                                    on:click=move |_| {
                                        save_proposal_open.set(false);
                                        status_msg.set(Some("Action1 proposal submitted for authorization.".to_string()));
                                    }
                                >
                                    "Request Action1 Save"
                                </button>
                            </div>
                        </div>
                    </div>
                </Show>
            </div>
        </Show>
    }
}

/// Text Editor standalone tool card component.
#[component]
pub fn EditorCard(
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
    let card_id = CardId::Editor(instance);

    let collapsed = move || {
        view! {
            <div class="card-collapsed-summary">
                <b>"Text Editor"</b>
                <span>"Ready"</span>
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
            kicker_title="Text Editor"
            kicker_icon=Arc::new(|| view! { <IconFile size=14 /> }.into_any())
            collapsed_summary=Arc::new(collapsed)
        >
            <EditorContent runtime=runtime auth_modal_open=auth_modal_open instance=instance />
        </CardFrame>
    }
}
