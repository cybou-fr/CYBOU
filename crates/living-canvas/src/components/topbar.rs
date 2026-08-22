// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Topbar workspace navigation and system status header component.

use cybou_web_contracts::SessionMode;
use leptos::prelude::*;
use lucide_leptos::{Ellipsis, FolderOpen, Link, ListChecks, Sparkles};

use crate::{
    DesktopLayout, LayoutHistory,
    components::icons::{IconLayers, IconPin, IconRedo, IconRefresh, IconUndo},
    interaction::{apply_redo, apply_undo},
    state::RuntimeState,
};

/// Desktop Topbar component displaying status, auth state, and quick navigation.
#[component]
pub fn Topbar(
    runtime: RwSignal<RuntimeState>,
    auth_modal_open: RwSignal<bool>,
    selected: ReadSignal<&'static str>,
    set_selected: WriteSignal<&'static str>,
    runtime_menu_open: ReadSignal<bool>,
    set_runtime_menu_open: WriteSignal<bool>,
    layout: RwSignal<DesktopLayout>,
    history: RwSignal<LayoutHistory>,
) -> impl IntoView {
    let runtime_label = move || match runtime.get() {
        RuntimeState::Loading => "Connecting".to_owned(),
        RuntimeState::SignInRequired => "Not signed in".to_owned(),
        RuntimeState::Ready { mode, .. } => match mode {
            SessionMode::LocalDesktop => "This machine".to_owned(),
            SessionMode::PublicPreview => "Open".to_owned(),
            SessionMode::RemoteBrowser => "Signed in".to_owned(),
            SessionMode::SignInRequired => "Not signed in".to_owned(),
        },
        RuntimeState::Error(_) => "Unavailable".to_owned(),
    };

    // What a person needs to see, and nothing else. The projection version, the stream cursor and
    // an RFC 3339 instant with nanoseconds are facts about the plumbing; printing them across the
    // top of the screen told everybody who opened this page that it was not for them.
    let status_detail = move || match runtime.get() {
        RuntimeState::Loading => "Connecting to this machine…".to_owned(),
        RuntimeState::SignInRequired => "Nothing is shown until you sign in".to_owned(),
        RuntimeState::Ready { mode, .. } => match mode {
            SessionMode::LocalDesktop => "The surface is on this machine".to_owned(),
            SessionMode::PublicPreview => "Anyone with the address can see this".to_owned(),
            SessionMode::RemoteBrowser => "With an account on this machine".to_owned(),
            SessionMode::SignInRequired => "Nothing is shown until you sign in".to_owned(),
        },
        RuntimeState::Error(_) => "Cannot reach this machine".to_owned(),
    };

    // The plumbing is still reachable, on hover, by whoever wants it. Removing it from sight is not
    // the same as removing it.
    let status_tooltip = move || match runtime.get() {
        RuntimeState::Loading => "Awaiting server-established session".to_owned(),
        RuntimeState::SignInRequired => "No session has been established".to_owned(),
        RuntimeState::Ready {
            snapshot, session, ..
        } => format!(
            "Projection v{} · Cursor {} · Session expires {}",
            snapshot.projection_version, snapshot.cursor, session.expires_at
        ),
        RuntimeState::Error(message) => message,
    };

    let navigate_from_menu = move |panel: &'static str| {
        set_selected.set(panel);
        set_runtime_menu_open.set(false);
    };

    view! {
        <header class="topbar">
            <a class="brand" href="#canvas" aria-label="Cybou home">
                <img class="brand-mark" src="/cybou-mark.svg" alt="" />
                <span class="brand-text">
                    <span class="brand-title">"CYBOU"</span>
                </span>
            </a>

            <div class="topbar-center">
                <span
                    class="status-pill"
                    class:online=move || matches!(runtime.get(), RuntimeState::Ready { .. })
                    title=status_tooltip
                >
                    <span class="status-dot"></span>
                    <span class="status-mode">{runtime_label}</span>
                    <span class="status-meta">{status_detail}</span>
                </span>
            </div>

            <div class="topbar-actions">
                <div class="history-controls" aria-label="Layout history">
                    <button
                        class="history-btn"
                        title="Undo layout change (Ctrl+Z)"
                        aria-label="Undo layout change"
                        disabled=move || !history.get().can_undo()
                        on:click=move |_| apply_undo(history, layout)
                    >
                        <IconUndo size=13 />
                        <span>"Undo"</span>
                    </button>
                    <button
                        class="history-btn"
                        title="Redo layout change (Ctrl+Shift+Z / Ctrl+Y)"
                        aria-label="Redo layout change"
                        disabled=move || !history.get().can_redo()
                        on:click=move |_| apply_redo(history, layout)
                    >
                        <IconRedo size=13 />
                        <span>"Redo"</span>
                    </button>
                </div>

                <div class="runtime-menu-anchor">
                    <button
                        class="runtime-trigger"
                        class:active=runtime_menu_open
                        aria-expanded=move || runtime_menu_open.get().to_string()
                        aria-haspopup="menu"
                        on:click=move |_| set_runtime_menu_open.update(|open| *open = !*open)
                    >
                        <Ellipsis size=16 />
                        <span>"Menu"</span>
                    </button>
                    <Show when=move || runtime_menu_open.get()>
                        <div class="runtime-popover" role="menu">
                            <span class="popover-heading">"Open"</span>
                            <button
                                class:active=move || selected.get() == "capabilities"
                                on:click=move |_| navigate_from_menu("capabilities")
                             title="Composed by Health1"><Sparkles size=14 /><span>"Capabilities"</span></button>
                            <button
                                class:active=move || selected.get() == "identity"
                                on:click=move |_| navigate_from_menu("identity")
                             title="Composed by Identity1"><IconPin size=14 /><span>"Identity"</span></button>
                            <button
                                class:active=move || selected.get() == "session"
                                on:click=move |_| navigate_from_menu("session")
                             title="Composed by Trust"><IconPin size=14 /><span>"Session"</span></button>
                            <button
                                class:active=move || selected.get() == "journal"
                                on:click=move |_| navigate_from_menu("journal")
                             title="Composed by Event1"><Link size=14 /><span>"Journal"</span></button>
                            <button
                                class:active=move || selected.get() == "lifecycle"
                                on:click=move |_| navigate_from_menu("lifecycle")
                             title="Composed by Lifecycle1"><Sparkles size=14 /><span>"Lifecycle"</span></button>
                            <button
                                class:active=move || selected.get() == "commitments"
                                on:click=move |_| navigate_from_menu("commitments")
                             title="Composed by Intention1"><ListChecks size=14 /><span>"Commitments"</span></button>
                            <button
                                class:active=move || selected.get() == "self"
                                on:click=move |_| navigate_from_menu("self")
                             title="Composed by Self1"><Sparkles size=14 /><span>"Self-Model"</span></button>
                            <button
                                class:active=move || selected.get() == "attention"
                                on:click=move |_| navigate_from_menu("attention")
                             title="Composed by Workspace1"><Sparkles size=14 /><span>"Attention"</span></button>
                            <button
                                class:active=move || selected.get() == "beliefs"
                                on:click=move |_| navigate_from_menu("beliefs")
                             title="Composed by Epistemic1"><Sparkles size=14 /><span>"Beliefs"</span></button>
                            <button
                                class:active=move || selected.get() == "perception"
                                on:click=move |_| navigate_from_menu("perception")
                             title="Composed by Perception1"><Link size=14 /><span>"Perception"</span></button>
                            <button
                                class:active=move || selected.get() == "context"
                                on:click=move |_| navigate_from_menu("context")
                             title="Composed by Context1"><Link size=14 /><span>"Context"</span></button>
                            <div class="popover-divider"></div>
                            <span class="popover-heading">"Desktop"</span>
                            <button on:click=move |_| {
                                history.update(|h| h.push(layout.get_untracked()));
                                layout.update(|l| l.reset_desktop(None));
                                layout.get_untracked().save();
                                set_runtime_menu_open.set(false);
                            }><IconRefresh size=14 /><span>"Reset layout"</span></button>
                            <button on:click=move |_| {
                                history.update(|h| h.push(layout.get_untracked()));
                                layout.update(|l| {
                                    let _ = l.create_deck("Mind Core", vec![crate::CardId::Identity, crate::CardId::Session], 70.0, 50.0);
                                });
                                layout.get_untracked().save();
                                set_runtime_menu_open.set(false);
                            }><IconLayers size=14 /><span>"Group Identity and Session"</span></button>
                        </div>
                    </Show>
                </div>

                <button
                    class="auth-trigger-btn"
                    title="Sign in with an account on this machine"
                    aria-label="Sign in"
                    on:click=move |_| auth_modal_open.set(true)
                >
                    <FolderOpen size=14 />
                    <span>"Sign in"</span>
                </button>
            </div>
        </header>
    }
}
