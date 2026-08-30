// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! File Manager 1.0 tool card and content component for typed workspace and home filesystem exploration.

use cybou_protocol::{LocationRef, SubjectRef};
use cybou_web_contracts::{
    HostDirectoryCreateRequest, HostFileCreateRequest, HostPathDeleteRequest,
    HostPathRenameRequest, LocationCategory, SessionMode,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::sync::Arc;
use web_sys::PointerEvent;

use crate::{
    CardId, DesktopItemId, DesktopLayout, GatewayMindClient, MindClient,
    components::{
        card_frame::CardFrame,
        icons::{
            IconArrowLeft, IconBot, IconCopy, IconDownload, IconEdit, IconFile, IconFolder,
            IconFolderPlus, IconHome, IconPlus, IconRefresh, IconSearch, IconShield, IconTrash,
            IconUpload,
        },
    },
    interaction::{DragState, ResizeState},
    state::RuntimeState,
    tool_state::ToolCardStates,
};

/// The bytes of a file the person picked.
///
/// `File` inherits `arrayBuffer` from `Blob`, and that promise is the only way to see the bytes:
/// the browser hands out a handle, not the contents.
async fn read_picked_file(file: &web_sys::File) -> Result<Vec<u8>, String> {
    let buffer = wasm_bindgen_futures::JsFuture::from(file.array_buffer())
        .await
        .map_err(|_| "the browser would not read it".to_owned())?;

    Ok(js_sys::Uint8Array::new(&buffer).to_vec())
}

/// Hand bytes to the browser to save, under a name.
///
/// The object URL is revoked immediately after the click. It is a reference into this document's
/// memory, and one left behind holds the whole file for as long as the desktop is open — which,
/// for a desktop, is until the person closes the tab.
fn save_bytes_to_disk(file_name: &str, bytes: &[u8]) -> Result<(), String> {
    use wasm_bindgen::JsCast as _;

    let parts = js_sys::Array::new();
    parts.push(&js_sys::Uint8Array::from(bytes).buffer());

    let blob = web_sys::Blob::new_with_u8_array_sequence(&parts)
        .map_err(|_| "the browser refused the contents".to_owned())?;
    let url = web_sys::Url::create_object_url_with_blob(&blob)
        .map_err(|_| "the browser refused to name the contents".to_owned())?;

    let result = (|| {
        let document = web_sys::window()
            .and_then(|window| window.document())
            .ok_or_else(|| "there is no document to save from".to_owned())?;
        let anchor: web_sys::HtmlAnchorElement = document
            .create_element("a")
            .map_err(|_| "the browser refused a link".to_owned())?
            .dyn_into()
            .map_err(|_| "the browser refused a link".to_owned())?;
        anchor.set_href(&url);
        anchor.set_download(file_name);
        anchor.click();
        Ok(())
    })();

    let _ = web_sys::Url::revoke_object_url(&url);
    result
}

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

    let user_home_dir = move || match runtime.get() {
        RuntimeState::Ready { session, .. } if !session.consumer_id.is_empty() => {
            format!("/home/{}", session.consumer_id)
        }
        _ => "/home/user".to_string(),
    };

    let state = expect_context::<ToolCardStates>().file_manager(CardId::FileManager(instance));
    let active_category = state.active_category;
    let current_path = state.current_path;
    let entries = state.entries;
    let selected_file = state.selected_file;
    let file_content = state.file_content;
    let selected_location = state.selected_location;
    let selected_sha256 = state.selected_sha256;
    let loading = state.loading;
    let directory_request_generation = state.directory_request_generation;
    let file_request_generation = state.file_request_generation;
    let error_msg = state.error_msg;
    let action_message = state.action_message;
    let was_read = state.read;
    let filter_query = state.filter_query;
    let sort_by = state.sort_by;
    let sort_ascending = state.sort_ascending;
    let create_modal_open = state.create_modal_open;
    let create_name = state.create_name;
    let create_error = state.create_error;
    let create_dir_modal_open = state.create_dir_modal_open;
    let create_dir_name = state.create_dir_name;
    let rename_modal_open = state.rename_modal_open;
    let rename_target = state.rename_target;
    let rename_new_name = state.rename_new_name;
    let delete_modal_open = state.delete_modal_open;
    let delete_target = state.delete_target;

    let default_root_for_category = move |cat: LocationCategory| -> String {
        match cat {
            LocationCategory::Home => user_home_dir(),
            LocationCategory::AgentWorkspace => "/var/lib/cybou/workspaces".to_string(),
            LocationCategory::Sandbox => "/".to_string(),
            LocationCategory::Backup => "/snapshots".to_string(),
            LocationCategory::System => "/etc".to_string(),
        }
    };

    let load_dir = move |path: String| {
        directory_request_generation
            .update(|generation| *generation = generation.saturating_add(1));
        let generation = directory_request_generation.get_untracked();
        file_request_generation.update(|generation| *generation = generation.saturating_add(1));
        loading.set(true);
        error_msg.set(None);
        selected_file.set(None);
        selected_location.set(None);
        selected_sha256.set(None);
        let target_p = path.clone();
        current_path.set(path);
        let cat = active_category.get_untracked();

        spawn_local(async move {
            if cat == LocationCategory::Home {
                match GatewayMindClient.host_list_directory(&target_p).await {
                    Ok(listing) => {
                        if directory_request_generation.get_untracked() != generation {
                            return;
                        }
                        loading.set(false);
                        was_read.set(true);
                        if listing.truncated {
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
                        if directory_request_generation.get_untracked() != generation {
                            return;
                        }
                        loading.set(false);
                        entries.set(Vec::new());
                        error_msg.set(Some(err.to_string()));
                    }
                }
            } else {
                match GatewayMindClient.list_directory(&target_p).await {
                    Ok(listing) => {
                        if directory_request_generation.get_untracked() != generation {
                            return;
                        }
                        loading.set(false);
                        was_read.set(true);
                        if listing.truncated {
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
                        if directory_request_generation.get_untracked() != generation {
                            return;
                        }
                        loading.set(false);
                        entries.set(Vec::new());
                        error_msg.set(Some(err.to_string()));
                    }
                }
            }
        });
    };

    // Initial read on mount or path change
    Effect::new(move |_| {
        let cat = active_category.get();
        let cur = current_path.get_untracked();
        let expected_prefix = match cat {
            LocationCategory::Home => user_home_dir(),
            LocationCategory::AgentWorkspace => "/var/lib/cybou/workspaces".to_string(),
            LocationCategory::Sandbox => "/".to_string(),
            LocationCategory::Backup => "/snapshots".to_string(),
            LocationCategory::System => "/etc".to_string(),
        };
        let target = if cur.is_empty() || cur == "/" && cat == LocationCategory::Home {
            expected_prefix
        } else {
            cur
        };
        if !was_read.get_untracked() && !loading.get_untracked() {
            load_dir(target);
        }
    });

    let switch_category = move |cat: LocationCategory| {
        active_category.set(cat);
        let new_root = default_root_for_category(cat);
        load_dir(new_root);
    };

    let view_file = move |name: String| {
        file_request_generation.update(|generation| *generation = generation.saturating_add(1));
        let generation = file_request_generation.get_untracked();
        selected_file.set(Some(name.clone()));
        loading.set(true);
        let cur = current_path.get();
        let p = if cur == "/" {
            format!("/{name}")
        } else {
            format!("{cur}/{name}")
        };
        let cat = active_category.get_untracked();

        spawn_local(async move {
            if cat == LocationCategory::Home {
                match GatewayMindClient.host_read_file(&p).await {
                    Ok(content) => {
                        if file_request_generation.get_untracked() != generation {
                            return;
                        }
                        loading.set(false);
                        selected_location.set(Some(content.location));
                        selected_sha256.set(Some(content.content_sha256));
                        file_content.set(content.text);
                    }
                    Err(err) => {
                        if file_request_generation.get_untracked() != generation {
                            return;
                        }
                        loading.set(false);
                        selected_location.set(None);
                        selected_sha256.set(None);
                        file_content.set(err.to_string());
                    }
                }
            } else {
                match GatewayMindClient.read_text_file(&p).await {
                    Ok(content) => {
                        if file_request_generation.get_untracked() != generation {
                            return;
                        }
                        loading.set(false);
                        selected_location.set(Some(content.location));
                        selected_sha256.set(Some(content.content_sha256));
                        file_content.set(content.text);
                    }
                    Err(err) => {
                        if file_request_generation.get_untracked() != generation {
                            return;
                        }
                        loading.set(false);
                        selected_location.set(None);
                        selected_sha256.set(None);
                        file_content.set(err.to_string());
                    }
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

    // Transfers exist for the sandbox only. The home and agent-workspace domains are served by
    // their own owner, which carries bounded UTF-8 reads and no byte transfer, so a Download button
    // there would be a button that always fails. It is absent rather than disabled-and-unexplained.
    let transfers_available = move || active_category.get() == LocationCategory::Sandbox;

    let upload_input: NodeRef<leptos::html::Input> = NodeRef::new();

    let download_file = move |name: String| {
        let cur = current_path.get_untracked();
        let full_path = if cur == "/" {
            format!("/{name}")
        } else {
            format!("{cur}/{name}")
        };
        action_message.set(Some(format!("Downloading {name}…")));

        spawn_local(async move {
            match GatewayMindClient.download_file(&full_path).await {
                Ok(bytes) => match save_bytes_to_disk(&name, &bytes) {
                    Ok(()) => action_message.set(Some(format!("Downloaded {name}"))),
                    // The bytes arrived and the browser would not take them. Saying "downloaded"
                    // here would be reporting the half that worked as the whole.
                    Err(reason) => {
                        error_msg.set(Some(format!("{name} could not be saved: {reason}")))
                    }
                },
                Err(err) => error_msg.set(Some(format!("{name} could not be read: {err}"))),
            }
        });
    };

    let upload_files = move |files: web_sys::FileList| {
        let cur = current_path.get_untracked();

        for index in 0..files.length() {
            let Some(file) = files.get(index) else {
                continue;
            };
            let name = file.name();
            let full_path = if cur == "/" {
                format!("/{name}")
            } else {
                format!("{cur}/{name}")
            };
            let reload_dir = cur.clone();

            spawn_local(async move {
                let bytes = match read_picked_file(&file).await {
                    Ok(bytes) => bytes,
                    Err(reason) => {
                        error_msg.set(Some(format!("{name} could not be read: {reason}")));
                        return;
                    }
                };

                let request = cybou_web_contracts::FileUploadRequest {
                    path: full_path,
                    content_base64: {
                        use base64::Engine as _;
                        base64::engine::general_purpose::STANDARD.encode(&bytes)
                    },
                };
                match GatewayMindClient.upload_file(&request).await {
                    Ok(_) => {
                        action_message.set(Some(format!("Uploaded {name}")));
                        load_dir(reload_dir);
                    }
                    Err(crate::ClientError::FileAlreadyExists) => {
                        error_msg.set(Some(format!(
                            "{name} already exists here and was not replaced"
                        )));
                    }
                    Err(err) => error_msg.set(Some(format!("{name} was not uploaded: {err}"))),
                }
            });
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
        let cat = active_category.get_untracked();

        spawn_local(async move {
            let res = if cat == LocationCategory::Home {
                let req = HostFileCreateRequest {
                    path: full_path,
                    text: String::new(),
                    exclusive: true,
                };
                GatewayMindClient.host_create_file(&req).await
            } else {
                let req = cybou_web_contracts::FileCreateRequest {
                    path: full_path,
                    text: String::new(),
                };
                GatewayMindClient.create_file(&req).await
            };

            match res {
                Ok(_) => {
                    create_modal_open.set(false);
                    create_name.set(String::new());
                    action_message.set(Some(format!("Created file {select_new_name}")));
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

    let handle_create_dir = move || {
        let name = create_dir_name.get().trim().to_string();
        if name.is_empty() {
            create_error.set(Some("Directory name cannot be empty".to_string()));
            return;
        }
        if name.contains('/') || name.contains('\\') || name.contains("..") {
            create_error.set(Some("Invalid directory name".to_string()));
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
        let select_name = name.clone();
        let cat = active_category.get_untracked();

        spawn_local(async move {
            let res = if cat == LocationCategory::Home {
                let req = HostDirectoryCreateRequest {
                    path: full_path,
                    recursive: true,
                };
                GatewayMindClient.host_create_directory(&req).await
            } else {
                Err(crate::ClientError::GatewayRequest(
                    "Directory creation in sandbox is restricted".to_string(),
                ))
            };

            match res {
                Ok(()) => {
                    create_dir_modal_open.set(false);
                    create_dir_name.set(String::new());
                    action_message.set(Some(format!("Created directory {select_name}")));
                    load_dir(reload_dir);
                }
                Err(err) => {
                    loading.set(false);
                    create_error.set(Some(err.to_string()));
                }
            }
        });
    };

    let handle_rename = move || {
        let Some(target) = rename_target.get() else {
            return;
        };
        let new_name = rename_new_name.get().trim().to_string();
        if new_name.is_empty() || new_name == target {
            rename_modal_open.set(false);
            return;
        }
        let cur = current_path.get();
        let from_path = if cur == "/" {
            format!("/{target}")
        } else {
            format!("{cur}/{target}")
        };
        let to_path = if cur == "/" {
            format!("/{new_name}")
        } else {
            format!("{cur}/{new_name}")
        };
        loading.set(true);
        let reload_dir = cur.clone();
        let cat = active_category.get_untracked();

        spawn_local(async move {
            let res = if cat == LocationCategory::Home {
                let req = HostPathRenameRequest { from_path, to_path };
                GatewayMindClient.host_rename_path(&req).await
            } else {
                Err(crate::ClientError::GatewayRequest(
                    "Rename in sandbox is restricted".to_string(),
                ))
            };

            match res {
                Ok(()) => {
                    rename_modal_open.set(false);
                    action_message.set(Some(format!("Renamed {target} to {new_name}")));
                    load_dir(reload_dir);
                }
                Err(err) => {
                    loading.set(false);
                    error_msg.set(Some(err.to_string()));
                }
            }
        });
    };

    let handle_delete = move || {
        let Some((target, is_dir)) = delete_target.get() else {
            return;
        };
        let cur = current_path.get();
        let path = if cur == "/" {
            format!("/{target}")
        } else {
            format!("{cur}/{target}")
        };
        loading.set(true);
        let reload_dir = cur.clone();
        let cat = active_category.get_untracked();

        spawn_local(async move {
            let res = if cat == LocationCategory::Home {
                let req = HostPathDeleteRequest {
                    path,
                    recursive: is_dir,
                };
                GatewayMindClient.host_delete_path(&req).await
            } else {
                Err(crate::ClientError::GatewayRequest(
                    "Deletion in sandbox is restricted".to_string(),
                ))
            };

            match res {
                Ok(()) => {
                    delete_modal_open.set(false);
                    delete_target.set(None);
                    if selected_file.get().as_ref() == Some(&target) {
                        selected_file.set(None);
                    }
                    action_message.set(Some(format!("Deleted {target}")));
                    load_dir(reload_dir);
                }
                Err(err) => {
                    loading.set(false);
                    error_msg.set(Some(err.to_string()));
                }
            }
        });
    };

    // Helper to open in Editor
    let open_in_editor = move |filename: String| {
        let tool_states = expect_context::<ToolCardStates>();
        let editor_state = tool_states.editor(CardId::Editor(0));
        let lang = if filename.ends_with(".rs") {
            "rust"
        } else if filename.ends_with(".py") {
            "python"
        } else if filename.ends_with(".sh") {
            "shell"
        } else if filename.ends_with(".toml") {
            "toml"
        } else if filename.ends_with(".yaml") || filename.ends_with(".yml") {
            "yaml"
        } else if filename.ends_with(".json") {
            "json"
        } else if filename.ends_with(".md") {
            "markdown"
        } else {
            "text"
        };
        let text = file_content.get();
        let Some(location) = selected_location.get() else {
            error_msg.set(Some(
                "Editor open refused: no authority reference".to_string(),
            ));
            return;
        };
        let Some(expected_sha256) = selected_sha256.get() else {
            error_msg.set(Some("Editor open refused: no content version".to_string()));
            return;
        };
        let mut tab = crate::tool_state::EditorTab::from_location(location, text, expected_sha256);
        tab.name = filename;
        tab.language = lang.to_string();
        let admission = editor_state.admit_file(tab);
        editor_state.status_msg.set(Some(match admission {
            crate::tool_state::EditorTabAdmission::FocusedExisting => {
                "Existing editor buffer focused.".to_string()
            }
            crate::tool_state::EditorTabAdmission::ReplacedPristineDraft => {
                "File opened in the initial editor tab.".to_string()
            }
            crate::tool_state::EditorTabAdmission::Added => {
                "File opened in a new editor tab.".to_string()
            }
        }));
        if let Some(l) = use_context::<RwSignal<DesktopLayout>>() {
            l.update(|layout| layout.open_card(CardId::Editor(0), 420.0, 160.0));
            l.get_untracked().save();
        }
        if let Some(select) = use_context::<WriteSignal<Option<DesktopItemId>>>() {
            select.set(Some(DesktopItemId::Card(CardId::Editor(0))));
        }
    };

    // Helper to inspect in Universal Inspector
    let open_in_inspector = move |_filename: String| {
        let Some(location) = selected_location.get() else {
            return;
        };
        let tool_states = expect_context::<ToolCardStates>();
        let inspector_state = tool_states.inspector(CardId::Inspector(0));
        inspector_state
            .target_subject
            .set(Some(SubjectRef::File { location }));
        if let Some(l) = use_context::<RwSignal<DesktopLayout>>() {
            l.update(|layout| layout.open_card(CardId::Inspector(0), 480.0, 180.0));
            l.get_untracked().save();
        }
        if let Some(select) = use_context::<WriteSignal<Option<DesktopItemId>>>() {
            select.set(Some(DesktopItemId::Card(CardId::Inspector(0))));
        }
    };

    // Helper to give to Agent
    let give_to_agent = move |filename: String| {
        if let Some(l) = use_context::<RwSignal<DesktopLayout>>() {
            l.update(|layout| layout.open_card(CardId::Agents, 460.0, 200.0));
            l.get_untracked().save();
        }
        action_message.set(Some(format!(
            "File {filename} attached to Agent workspace context"
        )));
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
                // Top level Typed Locations sidebar + main workspace
                <div class="fm-shell-layout">
                    <aside class="fm-sidebar">
                        <div class="fm-sidebar-title">"Locations"</div>
                        <button
                            class="fm-nav-item"
                            class:active=move || active_category.get() == LocationCategory::Home
                            on:click=move |_| switch_category(LocationCategory::Home)
                        >
                            <IconHome size=14 />
                            <span>"Home (~)"</span>
                        </button>
                        <button
                            class="fm-nav-item"
                            class:active=move || active_category.get() == LocationCategory::AgentWorkspace
                            on:click=move |_| switch_category(LocationCategory::AgentWorkspace)
                        >
                            <IconBot size=14 />
                            <span>"Agent Workspaces"</span>
                        </button>
                        <button
                            class="fm-nav-item"
                            class:active=move || active_category.get() == LocationCategory::Sandbox
                            on:click=move |_| switch_category(LocationCategory::Sandbox)
                        >
                            <IconShield size=14 />
                            <span>"Safe Sandbox"</span>
                        </button>
                        <button
                            class="fm-nav-item"
                            class:active=move || active_category.get() == LocationCategory::Backup
                            on:click=move |_| switch_category(LocationCategory::Backup)
                        >
                            <IconCopy size=14 />
                            <span>"Backups"</span>
                        </button>
                        <button
                            class="fm-nav-item"
                            class:active=move || active_category.get() == LocationCategory::System
                            on:click=move |_| switch_category(LocationCategory::System)
                        >
                            <IconFolder size=14 />
                            <span>"System (/etc)"</span>
                        </button>
                    </aside>

                    <div class="fm-main-area">
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
                                    create_name.set(String::new());
                                }>
                                    <IconPlus size=12 />
                                    <span>"New File"</span>
                                </button>
                                <button class="fm-btn" title="Create new folder" on:click=move |_| {
                                    create_dir_modal_open.set(true);
                                    create_error.set(None);
                                    create_dir_name.set(String::new());
                                }>
                                    <IconFolderPlus size=12 />
                                    <span>"New Folder"</span>
                                </button>
                                <Show when=transfers_available>
                                    <button
                                        class="fm-btn"
                                        title="Upload files into this folder"
                                        on:click=move |_| {
                                            if let Some(input) = upload_input.get() {
                                                input.click();
                                            }
                                        }
                                    >
                                        <IconUpload size=12 />
                                        <span>"Upload"</span>
                                    </button>
                                    <input
                                        type="file"
                                        multiple
                                        class="fm-upload-input"
                                        node_ref=upload_input
                                        on:change=move |event| {
                                            let input: web_sys::HtmlInputElement = event_target(&event);
                                            if let Some(files) = input.files() {
                                                upload_files(files);
                                            }
                                            // Cleared so picking the same file twice fires change
                                            // again. Without this a failed upload cannot be retried
                                            // from the picker.
                                            input.set_value("");
                                        }
                                    />
                                </Show>
                            </div>
                        </div>

                        // Toolbar with Filter & Sorting Controls
                        <div class="fm-controls-bar">
                            <div class="fm-filter-container">
                                <IconSearch size=12 />
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

                        <Show when=move || action_message.get().is_some()>
                            <div class="fm-action-toast">
                                <span>{move || action_message.get().unwrap_or_default()}</span>
                                <button class="fm-toast-close" on:click=move |_| action_message.set(None)>"×"</button>
                            </div>
                        </Show>

                        <Show when=move || error_msg.get().is_some()>
                            <div class="auth-error">
                                {move || error_msg.get().unwrap_or_default()}
                            </div>
                        </Show>

                        // Modals
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
                                            placeholder="e.g. main.rs, config.json, notes.md"
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
                                        <button class="fm-btn" on:click=move |_| create_modal_open.set(false)>"Cancel"</button>
                                        <button class="fm-btn primary" on:click=move |_| handle_create_file()>"Create File"</button>
                                    </footer>
                                </div>
                            </div>
                        </Show>

                        <Show when=move || create_dir_modal_open.get()>
                            <div class="fm-modal-backdrop" on:click=move |_| create_dir_modal_open.set(false)>
                                <div class="fm-modal-card" on:click=move |e: web_sys::MouseEvent| e.stop_propagation()>
                                    <header class="fm-modal-header">
                                        <strong>"Create New Folder"</strong>
                                        <button class="fm-modal-close" on:click=move |_| create_dir_modal_open.set(false)>"×"</button>
                                    </header>
                                    <div class="fm-modal-body">
                                        <p class="fm-modal-desc">{move || format!("Create a folder in {}", current_path.get())}</p>
                                        <input
                                            type="text"
                                            class="fm-modal-input"
                                            placeholder="e.g. projects, documents, src"
                                            prop:value=move || create_dir_name.get()
                                            on:input=move |e| create_dir_name.set(event_target_value(&e))
                                            on:keydown=move |e: web_sys::KeyboardEvent| {
                                                if e.key() == "Enter" {
                                                    handle_create_dir();
                                                } else if e.key() == "Escape" {
                                                    create_dir_modal_open.set(false);
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
                                        <button class="fm-btn" on:click=move |_| create_dir_modal_open.set(false)>"Cancel"</button>
                                        <button class="fm-btn primary" on:click=move |_| handle_create_dir()>"Create Folder"</button>
                                    </footer>
                                </div>
                            </div>
                        </Show>

                        <Show when=move || rename_modal_open.get()>
                            <div class="fm-modal-backdrop" on:click=move |_| rename_modal_open.set(false)>
                                <div class="fm-modal-card" on:click=move |e: web_sys::MouseEvent| e.stop_propagation()>
                                    <header class="fm-modal-header">
                                        <strong>"Rename Item"</strong>
                                        <button class="fm-modal-close" on:click=move |_| rename_modal_open.set(false)>"×"</button>
                                    </header>
                                    <div class="fm-modal-body">
                                        <p class="fm-modal-desc">{move || format!("Rename {}", rename_target.get().unwrap_or_default())}</p>
                                        <input
                                            type="text"
                                            class="fm-modal-input"
                                            prop:value=move || rename_new_name.get()
                                            on:input=move |e| rename_new_name.set(event_target_value(&e))
                                            on:keydown=move |e: web_sys::KeyboardEvent| {
                                                if e.key() == "Enter" {
                                                    handle_rename();
                                                } else if e.key() == "Escape" {
                                                    rename_modal_open.set(false);
                                                }
                                            }
                                        />
                                    </div>
                                    <footer class="fm-modal-footer">
                                        <button class="fm-btn" on:click=move |_| rename_modal_open.set(false)>"Cancel"</button>
                                        <button class="fm-btn primary" on:click=move |_| handle_rename()>"Rename"</button>
                                    </footer>
                                </div>
                            </div>
                        </Show>

                        <Show when=move || delete_modal_open.get()>
                            <div class="fm-modal-backdrop" on:click=move |_| delete_modal_open.set(false)>
                                <div class="fm-modal-card" on:click=move |e: web_sys::MouseEvent| e.stop_propagation()>
                                    <header class="fm-modal-header">
                                        <strong>"Confirm Deletion"</strong>
                                        <button class="fm-modal-close" on:click=move |_| delete_modal_open.set(false)>"×"</button>
                                    </header>
                                    <div class="fm-modal-body">
                                        <p class="fm-modal-desc">
                                            {move || {
                                                if let Some((name, is_dir)) = delete_target.get() {
                                                    if is_dir {
                                                        format!("Are you sure you want to delete folder '{name}' and all its contents?")
                                                    } else {
                                                        format!("Are you sure you want to delete file '{name}'?")
                                                    }
                                                } else {
                                                    String::new()
                                                }
                                            }}
                                        </p>
                                    </div>
                                    <footer class="fm-modal-footer">
                                        <button class="fm-btn" on:click=move |_| delete_modal_open.set(false)>"Cancel"</button>
                                        <button class="fm-btn danger" on:click=move |_| handle_delete()>"Delete"</button>
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
                                        let n_menu = name.clone();
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
                                                <div class="fm-item-quick-actions" on:click=move |e: web_sys::MouseEvent| e.stop_propagation()>
                                                    {
                                                        let n_rename = n_menu.clone();
                                                        let n_delete = n_menu.clone();
                                                        let n_download = n_menu.clone();
                                                        view! {
                                                            <Show when=move || !is_dir && transfers_available()>
                                                                {
                                                                    let n_download = n_download.clone();
                                                                    view! {
                                                                        <button
                                                                            class="fm-item-action-btn"
                                                                            title="Download"
                                                                            on:click=move |_| download_file(n_download.clone())
                                                                        >
                                                                            <IconDownload size=11 />
                                                                        </button>
                                                                    }
                                                                }
                                                            </Show>
                                                            <button
                                                                class="fm-item-action-btn"
                                                                title="Rename"
                                                                on:click=move |_| {
                                                                    rename_target.set(Some(n_rename.clone()));
                                                                    rename_new_name.set(n_rename.clone());
                                                                    rename_modal_open.set(true);
                                                                }
                                                            >
                                                                <IconEdit size=11 />
                                                            </button>
                                                            <button
                                                                class="fm-item-action-btn danger"
                                                                title="Delete"
                                                                on:click=move |_| {
                                                                    delete_target.set(Some((n_delete.clone(), is_dir)));
                                                                    delete_modal_open.set(true);
                                                                }
                                                            >
                                                                <IconTrash size=11 />
                                                            </button>
                                                        }
                                                    }
                                                </div>
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
                                                title="Open in Text Editor"
                                                on:click=move |_| {
                                                    let filename = selected_file.get().unwrap_or_default();
                                                    open_in_editor(filename);
                                                }
                                            >
                                                <IconEdit size=12 />
                                                <span>"Edit"</span>
                                            </button>
                                            <button
                                                class="fm-btn"
                                                title="Universal Inspector"
                                                on:click=move |_| {
                                                    let filename = selected_file.get().unwrap_or_default();
                                                    open_in_inspector(filename);
                                                }
                                            >
                                                <IconSearch size=12 />
                                                <span>"Inspect"</span>
                                            </button>
                                            <button
                                                class="fm-btn"
                                                title="Give to Agent"
                                                on:click=move |_| {
                                                    let filename = selected_file.get().unwrap_or_default();
                                                    give_to_agent(filename);
                                                }
                                            >
                                                <IconBot size=12 />
                                                <span>"Agent"</span>
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
                                        <span class="fm-meta-badge system">
                                            {move || match selected_location.get() {
                                                Some(LocationRef::HostUserPath(_)) => "HostUserPath",
                                                Some(LocationRef::AgentWorkspace { .. }) => "AgentWorkspace",
                                                Some(LocationRef::SystemConfigPath(_)) => "SystemConfigPath",
                                                Some(LocationRef::SafeShellJail { .. }) => "SafeShellJail",
                                                Some(LocationRef::BackupSnapshot { .. }) => "BackupSnapshot",
                                                _ => "LocationRef",
                                            }}
                                        </span>
                                    </div>
                                    <pre class="fm-preview-text">{move || file_content.get()}</pre>
                                </aside>
                            </Show>
                        </div>
                    </div>
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
                <span>"Workspace & Home"</span>
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
