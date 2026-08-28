// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Text and configuration editor tool card component (ADR-0045).

use cybou_web_contracts::{SessionMode, UserDraftSaveRequest};
use gloo_timers::future::TimeoutFuture;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::sync::Arc;
use web_sys::{MouseEvent, PointerEvent};

use crate::{
    CardId, ClientError, DesktopItemId, DesktopLayout, GatewayMindClient, MindClient,
    components::{
        card_frame::CardFrame,
        icons::{IconFile, IconShield},
    },
    interaction::{DragState, ResizeState},
    state::RuntimeState,
    tool_state::{EditorTab, FileConflict, ToolCardStates},
};

fn open_conflict_diff(
    tab: EditorTab,
    tool_states: ToolCardStates,
    layout: RwSignal<DesktopLayout>,
    set_selected: WriteSignal<Option<DesktopItemId>>,
) {
    let Some(conflict) = tab.conflict else {
        return;
    };
    let diff_card = CardId::Diff(0);
    let diff = tool_states.diff(diff_card);
    diff.title.set(format!(
        "Concurrent changes — {}",
        tab.location.display_path()
    ));
    diff.original_label
        .set("Current server version".to_string());
    diff.proposed_label.set("Unsaved editor buffer".to_string());
    diff.original_content.set(conflict.server_content);
    diff.proposed_content.set(tab.content);
    diff.status_msg.set(Some(
        "Read-only comparison. Resolve the conflict explicitly in the editor; no file has been changed."
            .to_string(),
    ));
    layout.update(|desktop| desktop.open_card(diff_card, 860.0, 180.0));
    layout.get_untracked().save();
    set_selected.set(Some(DesktopItemId::Card(diff_card)));
}

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
    let conflict_discard_open = state.conflict_discard_open;
    let pending_close_tab = state.pending_close_tab;
    let next_draft_number = state.next_draft_number;
    let card_close_open = state.card_close_open;
    let status_msg = state.status_msg;
    let autosave_generation = state.autosave_generation;
    let save_as_open = state.save_as_open;
    let save_as_path = state.save_as_path;
    let layout = expect_context::<RwSignal<DesktopLayout>>();
    let set_selected = expect_context::<WriteSignal<Option<DesktopItemId>>>();
    let tool_states = expect_context::<ToolCardStates>();

    if instance == 0 {
        spawn_local(async move {
            if let Ok(recovered) = GatewayMindClient.drafts().await
                && !recovered.drafts.is_empty()
            {
                let mut restored = Vec::with_capacity(recovered.drafts.len());
                for draft in recovered.drafts {
                    let current_path = match &draft.base_location {
                        Some(cybou_protocol::LocationRef::SafeShellJail { path, .. }) => {
                            Some(path.clone())
                        }
                        _ => None,
                    };
                    let tab = if let Some(path) = current_path {
                        match GatewayMindClient.read_text_file(&path).await {
                            Ok(current) => EditorTab::from_recovery_against_file(draft, current),
                            Err(_) => EditorTab::from_recovery(draft),
                        }
                    } else {
                        EditorTab::from_recovery(draft)
                    };
                    restored.push(tab);
                }
                let conflicts = restored.iter().filter(|tab| tab.conflict.is_some()).count();
                tabs.update(|all| {
                    let pristine = all.len() == 1 && !all[0].dirty && all[0].content.is_empty();
                    let restored = restored
                        .into_iter()
                        .filter(|draft| !all.iter().any(|tab| tab.recovery_id == draft.recovery_id))
                        .collect::<Vec<_>>();
                    if pristine && !restored.is_empty() {
                        *all = restored;
                    } else {
                        all.extend(restored);
                    }
                });
                status_msg.set(Some(if conflicts == 0 {
                    "Recovered durable editor drafts with current authority.".to_string()
                } else {
                    format!(
                        "Recovered durable editor drafts; {conflicts} changed on the server and require conflict review."
                    )
                }));
            }
        });
    }

    let active_tab = move || {
        let all_tabs = tabs.get();
        let idx = active_tab_index.get();
        all_tabs
            .get(idx)
            .cloned()
            .unwrap_or_else(EditorTab::untitled)
    };

    let show_conflict_diff = move || {
        open_conflict_diff(active_tab(), tool_states, layout, set_selected);
    };

    let update_current_content = move |new_content: String| {
        let idx = active_tab_index.get();
        tabs.update(|all| {
            if let Some(tab) = all.get_mut(idx) {
                tab.content = new_content;
                tab.dirty = tab.recovered_unsaved || tab.content != tab.original_content;
            }
        });
        autosave_generation.update(|generation| *generation = generation.saturating_add(1));
        let generation = autosave_generation.get_untracked();
        let snapshot = tabs.get_untracked().get(idx).cloned();
        spawn_local(async move {
            TimeoutFuture::new(750).await;
            if autosave_generation.get_untracked() != generation {
                return;
            }
            let Some(tab) = snapshot.filter(|tab| tab.dirty) else {
                return;
            };
            let base_location =
                (!matches!(tab.location, cybou_protocol::LocationRef::Draft { .. }))
                    .then_some(tab.location);
            let request = UserDraftSaveRequest {
                draft_id: tab.recovery_id,
                title: tab.name,
                content: tab.content,
                base_location,
                base_sha256: tab.expected_sha256,
            };
            if let Err(error) = GatewayMindClient.save_draft(&request).await {
                status_msg.set(Some(format!("Draft recovery autosave failed — {error}.")));
            }
        });
    };

    let add_tab = move || {
        let number = next_draft_number.get_untracked();
        next_draft_number.update(|next| *next = next.saturating_add(1));
        tabs.update(|all| {
            all.push(EditorTab::draft(number));
        });
        active_tab_index.set(tabs.get().len().saturating_sub(1));
    };

    let discard_tab = move |idx: usize| {
        if let Some(recovery_id) = tabs
            .get_untracked()
            .get(idx)
            .map(|tab| tab.recovery_id.clone())
        {
            spawn_local(async move {
                let _ = GatewayMindClient.delete_draft(&recovery_id).await;
            });
        }
        let previous_active = active_tab_index.get_untracked();
        let replacement_number = next_draft_number.get_untracked();
        tabs.update(|all| {
            if all.len() > 1 {
                all.remove(idx);
            } else {
                all[0] = EditorTab::draft(replacement_number);
                next_draft_number.update(|next| *next = next.saturating_add(1));
            }
        });
        let remaining = tabs.get_untracked().len();
        let next_active = if remaining == 1 {
            0
        } else if idx < previous_active {
            previous_active - 1
        } else if idx == previous_active {
            idx.min(remaining - 1)
        } else {
            previous_active
        };
        active_tab_index.set(next_active);
        pending_close_tab.set(None);
    };

    let close_tab = move |idx: usize| {
        let requires_confirmation = tabs
            .get_untracked()
            .get(idx)
            .is_some_and(|tab| tab.dirty || tab.conflict.is_some());
        if requires_confirmation {
            pending_close_tab.set(Some(idx));
        } else {
            discard_tab(idx);
        }
    };

    let trigger_save = move || {
        let tab = active_tab();
        if tab.location.requires_action_authorization() {
            save_proposal_open.set(true);
        } else if matches!(
            tab.location,
            cybou_protocol::LocationRef::SafeShellJail { .. }
        ) {
            let Some(expected_sha256) = tab.expected_sha256.clone() else {
                status_msg.set(Some(
                    "Save refused — this buffer has no server-established content version."
                        .to_string(),
                ));
                return;
            };
            let idx = active_tab_index.get();
            let request = cybou_web_contracts::FileWriteRequest {
                location: tab.location,
                expected_sha256,
                text: tab.content.clone(),
            };
            let recovery_id = tab.recovery_id;
            status_msg.set(Some("Saving…".to_string()));
            spawn_local(async move {
                match GatewayMindClient.write_text_file(&request).await {
                    Ok(saved) => {
                        let written_text = request.text;
                        tabs.update(|all| {
                            if let Some(current) = all.get_mut(idx) {
                                current.original_content = written_text.clone();
                                current.recovered_unsaved = false;
                                current.dirty = current.content != written_text;
                                current.expected_sha256 = Some(saved.content_sha256);
                                current.conflict = None;
                            }
                        });
                        let changed_while_saving = tabs
                            .get_untracked()
                            .get(idx)
                            .is_some_and(|current| current.dirty);
                        status_msg.set(Some(if changed_while_saving {
                            format!(
                                "Saved and verified ({} bytes); newer buffer changes remain unsaved.",
                                saved.size_bytes
                            )
                        } else {
                            format!("Saved and verified ({} bytes).", saved.size_bytes)
                        }));
                        let _ = GatewayMindClient.delete_draft(&recovery_id).await;
                    }
                    Err(ClientError::FileChangedSinceRead) => {
                        let cybou_protocol::LocationRef::SafeShellJail { path, .. } =
                            &request.location
                        else {
                            unreachable!("this save branch accepts only SafeShellJail locations");
                        };
                        match GatewayMindClient.read_text_file(path).await {
                            Ok(fresh) if fresh.location == request.location => {
                                tabs.update(|all| {
                                    if let Some(current) = all.get_mut(idx) {
                                        current.conflict = Some(FileConflict {
                                            server_content: fresh.text,
                                            server_sha256: fresh.content_sha256,
                                        });
                                    }
                                });
                                if let Some(conflicted_tab) =
                                    tabs.get_untracked().get(idx).cloned()
                                {
                                    open_conflict_diff(
                                        conflicted_tab,
                                        tool_states,
                                        layout,
                                        set_selected,
                                    );
                                }
                                status_msg.set(Some(
                                    "Save stopped because the file changed on the server. The unsaved buffer is preserved and a verified comparison is open."
                                        .to_string(),
                                ));
                            }
                            Ok(_) => status_msg.set(Some(
                                "Save stopped because the file changed. The follow-up read returned a different authority reference, so no comparison was opened; the editor buffer remains unsaved."
                                    .to_string(),
                            )),
                            Err(error) => status_msg.set(Some(format!(
                                "Save stopped because the file changed, and its current version could not be read: {error}. The editor buffer remains unsaved."
                            ))),
                        }
                    }
                    Err(error) => status_msg.set(Some(format!(
                        "Save failed — {error}. The editor buffer remains unsaved."
                    ))),
                }
            });
        } else {
            status_msg.set(Some(
                "Save unavailable for this location domain. Changes remain only in this open editor buffer."
                    .to_string(),
            ));
        }
    };

    let trigger_save_as = move || {
        let path = save_as_path.get_untracked();
        if path.trim().is_empty() {
            status_msg.set(Some("Save As requires a relative jail path.".to_string()));
            return;
        }
        let idx = active_tab_index.get_untracked();
        let tab = active_tab();
        let recovery_id = tab.recovery_id.clone();
        let request = cybou_web_contracts::FileCreateRequest {
            path: path.clone(),
            text: tab.content.clone(),
        };
        status_msg.set(Some("Creating file exclusively…".to_string()));
        spawn_local(async move {
            match GatewayMindClient.create_text_file(&request).await {
                Ok(created) => {
                    tabs.update(|all| {
                        if let Some(current) = all.get_mut(idx)
                            && current.recovery_id == recovery_id
                        {
                            current.name = path.rsplit('/').next().unwrap_or(&path).to_string();
                            current.location = created.location;
                            current.original_content = request.text.clone();
                            current.expected_sha256 = Some(created.content_sha256);
                            current.recovered_unsaved = false;
                            current.dirty = current.content != request.text;
                            current.conflict = None;
                        }
                    });
                    save_as_open.set(false);
                    save_as_path.set(String::new());
                    status_msg.set(Some(format!(
                        "Created and verified ({} bytes).",
                        created.size_bytes
                    )));
                    let _ = GatewayMindClient.delete_draft(&recovery_id).await;
                }
                Err(ClientError::FileAlreadyExists) => status_msg.set(Some(
                    "Save As stopped — that file already exists. Choose another path.".to_string(),
                )),
                Err(error) => status_msg.set(Some(format!(
                    "Save As failed — {error}. The editor buffer remains recoverable."
                ))),
            }
        });
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
                            disabled=move || !active_tab().dirty || active_tab().conflict.is_some()
                            on:click=move |_| {
                                if matches!(active_tab().location, cybou_protocol::LocationRef::Draft { .. }) {
                                    save_as_open.set(true);
                                } else {
                                    trigger_save();
                                }
                            }
                        >
                            {move || if matches!(active_tab().location, cybou_protocol::LocationRef::Draft { .. }) { "Save As" } else { "Save" }}
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

                <Show when=move || active_tab().conflict.is_some()>
                    <div class="editor-authority-banner system">
                        <span class="badge">"Write conflict"</span>
                        <span>"The server changed after this tab was opened. Save is paused until you choose a base version."</span>
                        <button class="editor-action-btn" on:click=move |_| show_conflict_diff()>
                            "Refresh comparison"
                        </button>
                        <button
                            class="editor-action-btn"
                            on:click=move |_| {
                                let idx = active_tab_index.get();
                                tabs.update(|all| {
                                    if let Some(tab) = all.get_mut(idx)
                                        && let Some(conflict) = tab.conflict.take()
                                    {
                                        tab.expected_sha256 = Some(conflict.server_sha256);
                                        tab.dirty = tab.content != conflict.server_content;
                                    }
                                });
                                status_msg.set(Some(
                                    "Reviewed server version accepted as the next save base. The local buffer is still unsaved; Save will use a new conditional write."
                                        .to_string(),
                                ));
                            }
                        >
                            "Keep buffer · use reviewed base"
                        </button>
                        <button
                            class="editor-action-btn"
                            on:click=move |_| conflict_discard_open.set(true)
                        >
                            "Use server version"
                        </button>
                    </div>
                </Show>

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
                                "CYBOU requires an Action1 FileWrite proposal with operator authorization before committing this change to disk. This proposal path is not connected yet."
                            </p>
                            <div class="editor-modal-actions">
                                <button class="editor-btn-secondary" on:click=move |_| save_proposal_open.set(false)>
                                    "Cancel"
                                </button>
                                <button
                                    class="editor-btn-primary"
                                    on:click=move |_| {
                                        save_proposal_open.set(false);
                                        status_msg.set(Some("Action1 save unavailable — no proposal was submitted. Changes remain only in this open editor buffer.".to_string()));
                                    }
                                >
                                    "Acknowledge"
                                </button>
                            </div>
                        </div>
                    </div>
                </Show>

                <Show when=move || save_as_open.get()>
                    <div class="editor-modal-overlay">
                        <div class="editor-modal-card">
                            <h3>"Save As"</h3>
                            <p>"Create a new file inside your authenticated workspace jail. Existing files are never overwritten."</p>
                            <input
                                class="editor-save-as-path"
                                type="text"
                                placeholder="notes/new-file.txt"
                                prop:value=move || save_as_path.get()
                                on:input=move |event| save_as_path.set(event_target_value(&event))
                            />
                            <div class="editor-modal-actions">
                                <button class="editor-btn-secondary" on:click=move |_| save_as_open.set(false)>
                                    "Cancel"
                                </button>
                                <button class="editor-btn-primary" on:click=move |_| trigger_save_as()>
                                    "Create File"
                                </button>
                            </div>
                        </div>
                    </div>
                </Show>

                <Show when=move || conflict_discard_open.get()>
                    <div class="editor-modal-overlay">
                        <div class="editor-modal-card">
                            <h3>"Discard Local Buffer?"</h3>
                            <p>"Replace this tab with the verified server version. Unsaved local changes in this tab cannot be recovered by CYBOU."</p>
                            <div class="editor-modal-actions">
                                <button class="editor-btn-secondary" on:click=move |_| conflict_discard_open.set(false)>
                                    "Cancel"
                                </button>
                                <button
                                    class="editor-btn-primary"
                                    on:click=move |_| {
                                        let idx = active_tab_index.get();
                                        tabs.update(|all| {
                                            if let Some(tab) = all.get_mut(idx)
                                                && let Some(conflict) = tab.conflict.take()
                                            {
                                                tab.content = conflict.server_content.clone();
                                                tab.original_content = conflict.server_content;
                                                tab.expected_sha256 = Some(conflict.server_sha256);
                                                tab.dirty = false;
                                            }
                                        });
                                        conflict_discard_open.set(false);
                                        status_msg.set(Some("Verified server version loaded; local unsaved changes were discarded by explicit confirmation.".to_string()));
                                    }
                                >
                                    "Discard Local Changes"
                                </button>
                            </div>
                        </div>
                    </div>
                </Show>

                <Show when=move || pending_close_tab.get().is_some()>
                    <div class="editor-modal-overlay">
                        <div class="editor-modal-card">
                            <h3>"Close Unsaved Tab?"</h3>
                            <p>
                                "Closing "
                                <code>{move || pending_close_tab.get().and_then(|idx| tabs.get().get(idx).map(|tab| tab.name.clone())).unwrap_or_else(|| "this tab".to_string())}</code>
                                " will permanently discard its local buffer. No file will be changed."
                            </p>
                            <div class="editor-modal-actions">
                                <button class="editor-btn-secondary" on:click=move |_| pending_close_tab.set(None)>
                                    "Keep Editing"
                                </button>
                                <button
                                    class="editor-btn-primary"
                                    on:click=move |_| {
                                        if let Some(idx) = pending_close_tab.get_untracked() {
                                            discard_tab(idx);
                                            status_msg.set(Some("Unsaved tab closed by explicit confirmation; no file was changed.".to_string()));
                                        }
                                    }
                                >
                                    "Discard and Close"
                                </button>
                            </div>
                        </div>
                    </div>
                </Show>

                <Show when=move || card_close_open.get()>
                    <div class="editor-modal-overlay">
                        <div class="editor-modal-card">
                            <h3>"Close Editor With Unsaved Buffers?"</h3>
                            <p>"This editor contains unsaved or unresolved tabs. Closing the panel permanently discards every local buffer in it; no file will be changed."</p>
                            <div class="editor-modal-actions">
                                <button class="editor-btn-secondary" on:click=move |_| card_close_open.set(false)>
                                    "Keep Editor Open"
                                </button>
                                <button
                                    class="editor-btn-primary"
                                    on:click=move |_| {
                                        let recovery_ids = tabs
                                            .get_untracked()
                                            .into_iter()
                                            .map(|tab| tab.recovery_id)
                                            .collect::<Vec<_>>();
                                        spawn_local(async move {
                                            for recovery_id in recovery_ids {
                                                let _ = GatewayMindClient.delete_draft(&recovery_id).await;
                                            }
                                        });
                                        let card = CardId::Editor(instance);
                                        layout.update(|desktop| desktop.close_card(card));
                                        layout.get_untracked().save();
                                        tool_states.forget(card);
                                    }
                                >
                                    "Discard All and Close"
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
                <span>"Draft only"</span>
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
