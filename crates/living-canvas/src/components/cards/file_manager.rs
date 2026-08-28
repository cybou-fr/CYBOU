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
    let current_path = state.current_path;
    let entries = state.entries;
    let selected_file = state.selected_file;
    let file_content = state.file_content;
    let selected_location = state.selected_location;
    let selected_sha256 = state.selected_sha256;
    let loading = state.loading;
    let error_msg = state.error_msg;
    let was_read = state.read;
    let filter_query = state.filter_query;
    let sort_by = state.sort_by;
    let sort_ascending = state.sort_ascending;
    let create_modal_open = state.create_modal_open;
    let create_name = state.create_name;
    let create_error = state.create_error;

    let load_dir = move |path: String| {
        loading.set(true);
        error_msg.set(None);
        selected_file.set(None);
        selected_location.set(None);
        selected_sha256.set(None);
        let target_p = path.clone();
        current_path.set(path);
        spawn_local(async move {
            match GatewayMindClient.list_directory(&target_p).await {
                Ok(listing) => {
                    loading.set(false);
                    was_read.set(true);
                    if listing.truncated {
                        // A bounded listing says so. Showing the first five hundred entries as if
                        // they were all of them would be a smaller directory, not a partial answer.
                        error_msg.set(Some(format!(
                            "Showing {} of {} entries",
                            listing.entries.len(),
                            listing.total_entries
                        )));
                    }
                    entries.set(
                        listing
                            .entries
                            .into_iter()
                            .map(|entry| (entry.name, entry.is_dir, entry.size_bytes))
                            .collect(),
                    );
                }
                Err(err) => {
                    loading.set(false);
                    // Not read, so the panel keeps saying it has not read rather than reporting an
                    // empty directory it never saw.
                    entries.set(Vec::new());
                    error_msg.set(Some(err.to_string()));
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
        selected_file.set(Some(name.clone()));
        loading.set(true);
        let p = if current_path.get() == "/" {
            format!("/{name}")
        } else {
            format!("{}/{name}", current_path.get())
        };
        spawn_local(async move {
            match GatewayMindClient.read_text_file(&p).await {
                Ok(content) => {
                    loading.set(false);
                    selected_location.set(Some(content.location));
                    selected_sha256.set(Some(content.content_sha256));
                    file_content.set(content.text);
                }
                Err(err) => {
                    loading.set(false);
                    selected_location.set(None);
                    selected_sha256.set(None);
                    file_content.set(err.to_string());
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

    let display_entries = move || {
        let all = entries.get();
        let q = filter_query.get().trim().to_lowercase();
        let mut filtered: Vec<_> = if q.is_empty() {
            all
        } else {
            all.into_iter()
                .filter(|(name, _, _)| name.to_lowercase().contains(&q))
                .collect()
        };
        crate::tool_state::sort_directory_entries(
            &mut filtered,
            sort_by.get(),
            sort_ascending.get(),
        );
        filtered
    };

    let handle_create_file = move || {
        let name = create_name.get().trim().to_string();
        if name.is_empty() {
            create_error.set(Some("File name cannot be empty".to_string()));
            return;
        }
        if name.contains('/') || name.contains('\\') || name.contains("..") {
            create_error.set(Some("Invalid file name".to_string()));
            return;
        }
        let cur = current_path.get();
        let full_path = if cur == "/" {
            format!("/{name}")
        } else {
            format!("{cur}/{name}")
        };
        loading.set(true);
        create_error.set(None);
        let reload_dir = cur.clone();
        let select_new_name = name.clone();
        spawn_local(async move {
            let req = cybou_web_contracts::FileCreateRequest {
                path: full_path,
                text: String::new(),
            };
            match GatewayMindClient.create_file(&req).await {
                Ok(_) => {
                    create_modal_open.set(false);
                    create_name.set(String::new());
                    load_dir(reload_dir);
                    view_file(select_new_name);
                }
                Err(err) => {
                    loading.set(false);
                    create_error.set(Some(match err {
                        crate::ClientError::FileAlreadyExists => "File already exists".to_string(),
                        _ => err.to_string(),
                    }));
                }
            }
        });
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
                // Interactive Path Breadcrumbs
                <div class="fm-path-bar">
                    <div class="fm-crumbs">
                        {move || {
                            let p = current_path.get();
                            let crumbs = crate::tool_state::parse_path_breadcrumbs(&p);
                            crumbs.into_iter().enumerate().map(|(idx, (label, target))| {
                                let is_last = idx == p.split('/').filter(|s| !s.is_empty()).count();
                                let target_path = target.clone();
                                view! {
                                    <button
                                        class="fm-crumb-btn"
                                        class:active=is_last
                                        title=format!("Jump to {target}")
                                        on:click=move |_| load_dir(target_path.clone())
                                    >
                                        {if label == "root" {
                                            view! { <IconHome size=11 /> }.into_any()
                                        } else {
                                            view! { <span>{label}</span> }.into_any()
                                        }}
                                    </button>
                                    {if !is_last {
                                        view! { <span class="fm-crumb-sep">"/"</span> }.into_any()
                                    } else {
                                        view! { <span/> }.into_any()
                                    }}
                                }
                            }).collect::<Vec<_>>()
                        }}
                    </div>
                    <div class="fm-nav-actions">
                        <button class="fm-btn" title="Up one level" on:click=move |_| go_up()>
                            <IconArrowLeft size=12 />
                            <span>"Up"</span>
                        </button>
                        <button class="fm-btn" title="Refresh folder" on:click=move |_| load_dir(current_path.get())>
                            <IconRefresh size=12 />
                            <span>"Refresh"</span>
                        </button>
                        <button class="fm-btn primary" title="Create new file in folder" on:click=move |_| {
                            create_modal_open.set(true);
                            create_error.set(None);
                        }>
                            <span>"+ New"</span>
                        </button>
                    </div>
                </div>

                // Toolbar with Filter & Sorting Controls
                <div class="fm-controls-bar">
                    <div class="fm-filter-container">
                        <input
                            type="text"
                            class="fm-filter-input"
                            placeholder="Filter entries…"
                            prop:value=move || filter_query.get()
                            on:input=move |e| filter_query.set(event_target_value(&e))
                        />
                        {move || if !filter_query.get().is_empty() {
                            view! {
                                <button
                                    class="fm-filter-clear"
                                    title="Clear filter"
                                    on:click=move |_| filter_query.set(String::new())
                                >
                                    "×"
                                </button>
                            }.into_any()
                        } else {
                            view! { <span/> }.into_any()
                        }}
                    </div>

                    <div class="fm-sort-container">
                        <span class="fm-sort-label">"Sort:"</span>
                        <button
                            class="fm-sort-btn"
                            class:active=move || sort_by.get() == crate::tool_state::FileSortMode::Name
                            on:click=move |_| {
                                if sort_by.get() == crate::tool_state::FileSortMode::Name {
                                    sort_ascending.update(|a| *a = !*a);
                                } else {
                                    sort_by.set(crate::tool_state::FileSortMode::Name);
                                    sort_ascending.set(true);
                                }
                            }
                        >
                            "Name"
                        </button>
                        <button
                            class="fm-sort-btn"
                            class:active=move || sort_by.get() == crate::tool_state::FileSortMode::Size
                            on:click=move |_| {
                                if sort_by.get() == crate::tool_state::FileSortMode::Size {
                                    sort_ascending.update(|a| *a = !*a);
                                } else {
                                    sort_by.set(crate::tool_state::FileSortMode::Size);
                                    sort_ascending.set(true);
                                }
                            }
                        >
                            "Size"
                        </button>
                        <button
                            class="fm-sort-btn"
                            class:active=move || sort_by.get() == crate::tool_state::FileSortMode::Kind
                            on:click=move |_| {
                                if sort_by.get() == crate::tool_state::FileSortMode::Kind {
                                    sort_ascending.update(|a| *a = !*a);
                                } else {
                                    sort_by.set(crate::tool_state::FileSortMode::Kind);
                                    sort_ascending.set(true);
                                }
                            }
                        >
                            "Type"
                        </button>
                        <button
                            class="fm-sort-dir"
                            title="Toggle sort direction"
                            on:click=move |_| sort_ascending.update(|a| *a = !*a)
                        >
                            {move || if sort_ascending.get() { "↑" } else { "↓" }}
                        </button>
                    </div>

                    <div class="fm-entry-counter">
                        {move || {
                            let total = entries.get().len();
                            let shown = display_entries().len();
                            if !filter_query.get().is_empty() {
                                format!("{shown} of {total}")
                            } else {
                                format!("{total} item(s)")
                            }
                        }}
                    </div>
                </div>

                <Show when=move || error_msg.get().is_some()>
                    <div class="auth-error">
                        {move || error_msg.get().unwrap_or_default()}
                    </div>
                </Show>

                // New File Creation Modal
                <Show when=move || create_modal_open.get()>
                    <div class="fm-modal-backdrop" on:click=move |_| create_modal_open.set(false)>
                        <div class="fm-modal-card" on:click=move |e: web_sys::MouseEvent| e.stop_propagation()>
                            <header class="fm-modal-header">
                                <strong>"Create New File"</strong>
                                <button class="fm-modal-close" on:click=move |_| create_modal_open.set(false)>"×"</button>
                            </header>
                            <div class="fm-modal-body">
                                <p class="fm-modal-desc">{move || format!("Create an empty file in {}", current_path.get())}</p>
                                <input
                                    type="text"
                                    class="fm-modal-input"
                                    placeholder="e.g. document.txt, config.json"
                                    prop:value=move || create_name.get()
                                    on:input=move |e| create_name.set(event_target_value(&e))
                                    on:keydown=move |e: web_sys::KeyboardEvent| {
                                        if e.key() == "Enter" {
                                            handle_create_file();
                                        } else if e.key() == "Escape" {
                                            create_modal_open.set(false);
                                        }
                                    }
                                />
                                <Show when=move || create_error.get().is_some()>
                                    <div class="auth-error">
                                        {move || create_error.get().unwrap_or_default()}
                                    </div>
                                </Show>
                            </div>
                            <footer class="fm-modal-footer">
                                <button class="fm-btn" on:click=move |_| create_modal_open.set(false)>
                                    "Cancel"
                                </button>
                                <button class="fm-btn primary" on:click=move |_| handle_create_file()>
                                    "Create File"
                                </button>
                            </footer>
                        </div>
                    </div>
                </Show>

                <div class="fm-content">
                    <div class="fm-grid">
                        <Show when=move || loading.get()>
                            <div class="fm-empty">"Loading directory…"</div>
                        </Show>
                        <Show when=move || !loading.get() && display_entries().is_empty() && error_msg.get().is_none() && was_read.get()>
                            <div class="fm-empty">
                                {move || if filter_query.get().is_empty() { "Empty directory" } else { "No matching entries" }}
                            </div>
                        </Show>
                        <Show when=move || !loading.get() && !was_read.get() && error_msg.get().is_none()>
                            <div class="fm-empty">"Not read yet"</div>
                        </Show>
                        <For
                            each=display_entries
                            key=|(name, is_dir, _)| format!("{name}-{is_dir}")
                            children=move |(name, is_dir, size)| {
                                let n = name.clone();
                                let n_click = name.clone();
                                let p = current_path.get();
                                let is_selected = {
                                    let cur_sel = selected_file.get();
                                    cur_sel.as_ref() == Some(&name)
                                };
                                view! {
                                    <div
                                        class="fm-item"
                                        class:is-dir=is_dir
                                        class:is-file=!is_dir
                                        class:selected=is_selected
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
                                        <span class="fm-item-size">{if is_dir { "dir".to_string() } else { crate::tool_state::format_bytes(size) }}</span>
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
                                        class="fm-btn primary"
                                        title="Open file in Text Editor"
                                        on:click=move |_| {
                                            let tool_states = expect_context::<ToolCardStates>();
                                            let editor_state = tool_states.editor(CardId::Editor(0));
                                            let filename = selected_file.get().unwrap_or_default();
                                            let lang = if filename.ends_with(".rs") {
                                                "rust"
                                            } else if filename.ends_with(".toml") {
                                                "toml"
                                            } else if filename.ends_with(".json") {
                                                "json"
                                            } else if filename.ends_with(".md") {
                                                "markdown"
                                            } else {
                                                "text"
                                            };
                                            let text = file_content.get();
                                            let Some(location) = selected_location.get() else {
                                                error_msg.set(Some("Editor open refused — the gateway supplied no authority-domain reference for this file.".to_string()));
                                                return;
                                            };
                                            let Some(expected_sha256) = selected_sha256.get() else {
                                                error_msg.set(Some("Editor open refused — the gateway supplied no content version for this file.".to_string()));
                                                return;
                                            };
                                            let mut tab = crate::tool_state::EditorTab::from_location(location, text, expected_sha256);
                                            tab.name = filename;
                                            tab.language = lang.to_string();
                                            let admission = editor_state.admit_file(tab);
                                            editor_state.status_msg.set(Some(match admission {
                                                crate::tool_state::EditorTabAdmission::FocusedExisting =>
                                                    "Existing editor buffer focused; the File Manager preview did not replace its local contents.".to_string(),
                                                crate::tool_state::EditorTabAdmission::ReplacedPristineDraft =>
                                                    "File opened in the initial editor tab.".to_string(),
                                                crate::tool_state::EditorTabAdmission::Added =>
                                                    "File opened in a new editor tab.".to_string(),
                                            }));
                                            if let Some(l) = use_context::<RwSignal<DesktopLayout>>() {
                                                l.update(|layout| layout.open_card(CardId::Editor(0), 400.0, 180.0));
                                                l.get_untracked().save();
                                            }
                                            if let Some(select) = use_context::<WriteSignal<Option<DesktopItemId>>>() {
                                                select.set(Some(DesktopItemId::Card(CardId::Editor(0))));
                                            }
                                        }
                                    >
                                        <IconFile size=12 />
                                        <span>"Edit"</span>
                                    </button>
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
                                    <button class="fm-btn" title="Close preview" on:click=move |_| selected_file.set(None)>"×"</button>
                                </div>
                            </header>
                            <div class="fm-preview-meta">
                                <span class="fm-meta-badge">{move || format!("Size: {}", crate::tool_state::format_bytes(file_content.get().len() as u64))}</span>
                                {move || {
                                    if let Some(sha) = selected_sha256.get() {
                                        let short_sha = sha.chars().take(8).collect::<String>();
                                        view! {
                                            <span class="fm-meta-badge" title=format!("SHA-256: {sha}")>
                                                {format!("SHA: {short_sha}…")}
                                            </span>
                                        }.into_any()
                                    } else {
                                        view! { <span/> }.into_any()
                                    }
                                }}
                                <span class="fm-meta-badge system">"SafeShellJail"</span>
                            </div>
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
