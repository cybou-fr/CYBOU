// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Personal Notes & Knowledge base card component.

use crate::{CardId, MindClient, components::icons::IconRefresh, tool_state::ToolCardStates};
use leptos::prelude::*;

#[component]
pub fn NotesContent(card: CardId) -> impl IntoView {
    let client = crate::GatewayMindClient;
    let tool_states = expect_context::<ToolCardStates>();
    let signals = tool_states.notes(card);

    let load_notes = move || {
        signals.loading.set(true);
        leptos::task::spawn_local(async move {
            match client.get_notes().await {
                Ok(proj) => {
                    signals.notes.set(proj.notes);
                    signals.status_msg.set(None);
                }
                Err(err) => {
                    signals
                        .status_msg
                        .set(Some(format!("Failed to load notes: {err}")));
                }
            }
            signals.loading.set(false);
        });
    };

    let trigger_save = move || {
        let title = signals.edit_title.get();
        let content = signals.edit_content.get();
        let tags: Vec<String> = signals
            .edit_tags
            .get()
            .split(',')
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .collect();
        let is_pinned = signals.edit_pinned.get();

        if title.trim().is_empty() {
            signals
                .status_msg
                .set(Some("Please enter a note title".to_owned()));
            return;
        }

        signals.loading.set(true);
        leptos::task::spawn_local(async move {
            if let Some(id) = signals.selected_note_id.get() {
                let req = cybou_web_contracts::UpdateNoteRequest {
                    id,
                    title,
                    content_markdown: content,
                    tags,
                    is_pinned,
                };
                match client.update_note(req).await {
                    Ok(note) => {
                        signals
                            .status_msg
                            .set(Some(format!("Updated note '{}'", note.title)));
                        load_notes();
                    }
                    Err(err) => {
                        signals
                            .status_msg
                            .set(Some(format!("Update failed: {err}")));
                    }
                }
            } else {
                let req = cybou_web_contracts::CreateNoteRequest {
                    title,
                    content_markdown: content,
                    tags,
                    is_pinned,
                    referenced_subject: None,
                };
                match client.create_note(req).await {
                    Ok(note) => {
                        signals
                            .status_msg
                            .set(Some(format!("Created note '{}'", note.title)));
                        signals.selected_note_id.set(Some(note.id));
                        load_notes();
                    }
                    Err(err) => {
                        signals
                            .status_msg
                            .set(Some(format!("Create failed: {err}")));
                    }
                }
            }
            signals.loading.set(false);
        });
    };

    let select_note = move |note: cybou_protocol::personal::NoteRecord| {
        signals.selected_note_id.set(Some(note.id));
        signals.edit_title.set(note.title);
        signals.edit_content.set(note.content_markdown);
        signals.edit_tags.set(note.tags.join(", "));
        signals.edit_pinned.set(note.is_pinned);
    };

    let new_note = move || {
        signals.selected_note_id.set(None);
        signals.edit_title.set(String::new());
        signals.edit_content.set(String::new());
        signals.edit_tags.set(String::new());
        signals.edit_pinned.set(false);
    };

    // Trigger initial load
    Effect::new(move |_| {
        load_notes();
    });

    view! {
        <div class="notes-panel" style="display: flex; flex-direction: column; height: 100%; width: 100%; overflow: hidden;">
            // Header
            <div style="display: flex; align-items: center; justify-content: space-between; padding: 10px 12px; background: var(--bg-sunken); border-bottom: 1px solid var(--line);">
                <div style="display: flex; align-items: center; gap: 8px;">
                    <span style="font-weight: 600; font-size: 13px;">"Notes & Knowledge Snippets"</span>
                </div>
                <div style="display: flex; align-items: center; gap: 6px;">
                    <button
                        style="background: linear-gradient(135deg, var(--accent-solid), var(--accent-solid)); border: none; border-radius: 4px; padding: 4px 10px; font-size: 11px; font-weight: 700; color: var(--text-bright); cursor: pointer;"
                        on:click=move |_| new_note()
                    >
                        "+ New Note"
                    </button>
                    <button
                        style="background: var(--fill-subtle); border: none; border-radius: 4px; padding: 4px 6px; color: inherit; cursor: pointer;"
                        title="Refresh notes"
                        on:click=move |_| load_notes()
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

            <div style="display: flex; flex: 1; overflow: hidden;">
                // Notes Sidebar List
                <div style="width: 200px; border-right: 1px solid var(--line); overflow-y: auto; padding: 8px; display: flex; flex-direction: column; gap: 4px; background: var(--bg-sunken);">
                    {move || signals.notes.get().into_iter().map(|n| {
                        let n_sel = n.clone();
                        let n_click = n.clone();
                        let is_sel = move || signals.selected_note_id.get().as_ref() == Some(&n_sel.id);
                        view! {
                            <div
                                style=move || format!("padding: 8px 10px; border-radius: 4px; cursor: pointer; font-size: 11px; background: {}; border: 1px solid {};", if is_sel() { "var(--accent-fill-strong)" } else { "transparent" }, if is_sel() { "var(--accent-line-strong)" } else { "transparent" })
                                on:click=move |_| select_note(n_click.clone())
                            >
                                <div style="font-weight: 600; color: var(--text-bright); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                                    {if n.is_pinned { "📌 " } else { "" }}
                                    {n.title}
                                </div>
                                <div style="font-size: 9px; color: var(--text-faint); margin-top: 2px;">
                                    {n.updated_at}
                                </div>
                            </div>
                        }
                    }).collect::<Vec<_>>()}
                </div>

                // Note Editor / Detail
                <div style="flex: 1; padding: 12px; display: flex; flex-direction: column; gap: 8px; overflow-y: auto;">
                    <div style="display: flex; gap: 8px; align-items: center;">
                        <input
                            type="text"
                            placeholder="Note Title..."
                            prop:value=move || signals.edit_title.get()
                            on:input=move |e| signals.edit_title.set(event_target_value(&e))
                            style="flex: 1; background: var(--bg-sunken-strong); border: 1px solid var(--fill-hover); border-radius: 4px; padding: 6px 8px; font-size: 12px; font-weight: 600; color: inherit;"
                        />
                        <label style="display: flex; align-items: center; gap: 4px; font-size: 11px; cursor: pointer;">
                            <input
                                type="checkbox"
                                prop:checked=move || signals.edit_pinned.get()
                                on:change=move |e| signals.edit_pinned.set(event_target_checked(&e))
                            />
                            "Pinned"
                        </label>
                    </div>

                    <input
                        type="text"
                        placeholder="Tags (comma-separated, e.g. architecture, canvas)..."
                        prop:value=move || signals.edit_tags.get()
                        on:input=move |e| signals.edit_tags.set(event_target_value(&e))
                        style="background: var(--bg-sunken-strong); border: 1px solid var(--fill-hover); border-radius: 4px; padding: 4px 8px; font-size: 10px; color: var(--text-second);"
                    />

                    <textarea
                        placeholder="Write Markdown notes here..."
                        prop:value=move || signals.edit_content.get()
                        on:input=move |e| signals.edit_content.set(event_target_value(&e))
                        style="flex: 1; background: var(--bg-sunken-strong); border: 1px solid var(--fill-hover); border-radius: 4px; padding: 8px; font-size: 11px; font-family: monospace; color: inherit; resize: none;"
                    />

                    <div style="display: flex; justify-content: flex-end;">
                        <button
                            style="background: linear-gradient(135deg, var(--accent-solid), var(--accent-solid)); border: none; border-radius: 4px; padding: 6px 14px; font-size: 11px; font-weight: 700; color: var(--text-bright); cursor: pointer;"
                            on:click=move |_| trigger_save()
                        >
                            "Save Note"
                        </button>
                    </div>
                </div>
            </div>
        </div>
    }
}
