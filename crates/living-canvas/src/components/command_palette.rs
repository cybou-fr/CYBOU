// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Command palette action launcher and fuzzy navigation menu.

use leptos::prelude::*;
use lucide_leptos::{Link, ListChecks, Search, Sparkles};
use web_sys::KeyboardEvent;

use crate::interaction::usable_viewport;
use crate::{
    ArrangementMode, CardId, DesktopItemId, DesktopLayout, LayoutHistory,
    components::icons::{
        IconExternalLink, IconGrid, IconLayers, IconMaximize, IconMinimize, IconPin, IconRedo,
        IconRefresh, IconUndo,
    },
    interaction::{apply_redo, apply_undo},
    state::command_matches,
};

/// Command palette modal and shortcut launcher.
#[component]
pub fn CommandPalette(
    layout: RwSignal<DesktopLayout>,
    history: RwSignal<LayoutHistory>,
    set_selected: WriteSignal<Option<DesktopItemId>>,
    auth_modal_open: RwSignal<bool>,
    command_open: ReadSignal<bool>,
    set_command_open: WriteSignal<bool>,
    command_query: ReadSignal<String>,
    set_command_query: WriteSignal<String>,
    command_input: NodeRef<leptos::html::Input>,
    set_zoom: WriteSignal<f64>,
    set_pan: WriteSignal<(f64, f64)>,
    #[prop(default = RwSignal::new(crate::state::RuntimeState::Loading))]
    runtime: RwSignal<crate::state::RuntimeState>,
) -> impl IntoView {
    let select_from_command = move |panel: &'static str| {
        // A named panel is always a system card, and a system card is a singleton, so its key
        // does identify it. Tool cards are never reached this way.
        set_selected.set(CardId::from_key(panel).map(DesktopItemId::Card));
        set_command_open.set(false);
        set_command_query.set(String::new());
    };

    let ask_answer = move || crate::state::ask_cybou(&command_query.get(), &runtime.get());

    view! {
        <section class="command-palette" aria-label="Action launcher">
            <Show when=move || command_open.get()>
                <nav class="command-menu" aria-label="Command palette actions">
                    {move || {
                        ask_answer().map(|ans| {
                            let target_click = ans.target;
                            view! {
                                <div class="ask-cybou-card">
                                    <div class="ask-cybou-header">
                                        <Sparkles size=14 />
                                        <b>"Ask CYBOU"</b>
                                        <span class="ask-cybou-headline">{ans.headline}</span>
                                    </div>
                                    <p class="ask-cybou-detail">{ans.detail}</p>
                                    {target_click.map(|(label, card)| {
                                        view! {
                                            <button
                                                type="button"
                                                class="ask-cybou-action-btn"
                                                on:click=move |_| {
                                                    set_selected.set(Some(DesktopItemId::Card(card)));
                                                    if !layout.get().contains_card(card) {
                                                        layout.update(|l| l.open_card(card, 380.0, 480.0));
                                                    } else if layout.get().presentation(card).collapsed {
                                                        layout.update(|l| l.set_collapsed(card, false));
                                                    }
                                                    layout.update(|l| l.bring_forward(card));
                                                    set_command_open.set(false);
                                                    set_command_query.set(String::new());
                                                }
                                            >
                                                {label}
                                            </button>
                                        }
                                    })}
                                </div>
                            }
                        })
                    }}
                    <button
                        class:hidden=move || !command_matches(&command_query.get(), "insight telemetry machine health findings why status")
                        on:click=move |_| select_from_command("insight")
                    ><Sparkles size=15 /><span><b>"Open System Insight"</b><i>"Telemetry1 host health & self-healing"</i></span></button>
                    <button
                        class:hidden=move || !command_matches(&command_query.get(), "agents agent1 launch opencode task autonomous")
                        on:click=move |_| select_from_command("agents")
                    ><ListChecks size=15 /><span><b>"Open Agents"</b><i>"Agent1 bounded capsule runtime"</i></span></button>
                    <button
                        class:hidden=move || !command_matches(&command_query.get(), "capabilities health dependencies")
                        on:click=move |_| select_from_command("capabilities")
                    ><Sparkles size=15 /><span><b>"Open Capabilities"</b><i>"Health1 capability dependencies"</i></span></button>
                    <button
                        class:hidden=move || !command_matches(&command_query.get(), "identity subject continuity provenance")
                        on:click=move |_| select_from_command("identity")
                    ><IconPin size=15 /><span><b>"Open Identity"</b><i>"Identity1 subject continuity"</i></span></button>
                    <button
                        class:hidden=move || !command_matches(&command_query.get(), "session trust gateway authentication mode")
                        on:click=move |_| select_from_command("session")
                    ><IconPin size=15 /><span><b>"Open Session"</b><i>"Gateway trust and session mode"</i></span></button>
                    <button
                        class:hidden=move || !command_matches(&command_query.get(), "journal contributions causal integrity event1")
                        on:click=move |_| select_from_command("journal")
                    ><Link size=15 /><span><b>"Open Journal"</b><i>"Event1 canonical event log"</i></span></button>
                    <button
                        class:hidden=move || !command_matches(&command_query.get(), "lifecycle sleep wake consolidation")
                        on:click=move |_| select_from_command("lifecycle")
                    ><Sparkles size=15 /><span><b>"Open Lifecycle"</b><i>"Lifecycle1 sleep and wake state"</i></span></button>
                    <button
                        class:hidden=move || !command_matches(&command_query.get(), "commitments obligations intention1")
                        on:click=move |_| select_from_command("commitments")
                    ><ListChecks size=15 /><span><b>"Open Commitments"</b><i>"Intention1 open obligations"</i></span></button>
                    <button
                        class:hidden=move || !command_matches(&command_query.get(), "self assessment autobiographical narration self1")
                        on:click=move |_| select_from_command("self")
                    ><Sparkles size=15 /><span><b>"Open Self-Model"</b><i>"Self1 autobiographical narration"</i></span></button>
                    <button
                        class:hidden=move || !command_matches(&command_query.get(), "attention focus global workspace theory workspace1")
                        on:click=move |_| select_from_command("attention")
                    ><Sparkles size=15 /><span><b>"Open Attention"</b><i>"Workspace1 attention focus"</i></span></button>
                    <button
                        class:hidden=move || !command_matches(&command_query.get(), "beliefs epistemic1 validity propositions")
                        on:click=move |_| select_from_command("beliefs")
                    ><Sparkles size=15 /><span><b>"Open Beliefs"</b><i>"Epistemic1 derived propositions"</i></span></button>
                    <button
                        class:hidden=move || !command_matches(&command_query.get(), "perception observations host perception1")
                        on:click=move |_| select_from_command("perception")
                    ><Link size=15 /><span><b>"Open Perception"</b><i>"Perception1 host facts"</i></span></button>
                    <button
                        class:hidden=move || !command_matches(&command_query.get(), "context association concepts context1")
                        on:click=move |_| select_from_command("context")
                    ><Link size=15 /><span><b>"Open Context"</b><i>"Context1 associative graph"</i></span></button>
                    <button
                        class:hidden=move || !command_matches(&command_query.get(), "shell terminal bash command zone3 body")
                        on:click=move |_| {
                            layout.update(|l| l.open_card(CardId::Shell(0), 400.0, 160.0));
                            layout.get_untracked().save();
                            select_from_command("shell");
                        }
                    ><IconExternalLink size=15 /><span><b>"Open Shell"</b><i>"A bounded, read-only shell"</i></span></button>
                    <button
                        class:hidden=move || !command_matches(&command_query.get(), "files file manager storage browse read-only")
                        on:click=move |_| {
                            layout.update(|l| l.open_card(CardId::FileManager(0), 380.0, 120.0));
                            layout.get_untracked().save();
                            select_from_command("files");
                        }
                    ><IconExternalLink size=15 /><span><b>"Open File Manager"</b><i>"Browse files, read-only"</i></span></button>
                    <button
                        class:hidden=move || !command_matches(&command_query.get(), "events feed live stream sse journal")
                        on:click=move |_| {
                            layout.update(|l| l.open_card(CardId::JournalFeed(0), 420.0, 150.0));
                            layout.get_untracked().save();
                            select_from_command("journal-feed");
                        }
                    ><IconExternalLink size=15 /><span><b>"Open Event Stream"</b><i>"Real-time Journal SSE stream"</i></span></button>
                    <button
                        class:hidden=move || !command_matches(&command_query.get(), "auth sign in login pam")
                        on:click=move |_| {
                            auth_modal_open.set(true);
                            set_command_open.set(false);
                            set_command_query.set(String::new());
                        }
                    ><IconPin size=15 /><span><b>"Authenticate / Sign in"</b><i>"Linux PAM credentials"</i></span></button>
                    <button
                        class:hidden=move || !command_matches(&command_query.get(), "group mind deck cards")
                        on:click=move |_| {
                            history.update(|h| h.push(layout.get_untracked()));
                            layout.update(|l| {
                                let _ = l.create_deck("Mind Core", vec![CardId::Identity, CardId::Session], 70.0, 50.0);
                            });
                            layout.get_untracked().save();
                            set_command_open.set(false);
                            set_command_query.set(String::new());
                        }
                    ><IconLayers size=15 /><span><b>"Create Deck: Mind Core"</b><i>"Group Identity and Session"</i></span></button>
                    <button
                        class:hidden=move || !command_matches(&command_query.get(), "undo layout revert")
                        on:click=move |_| {
                            apply_undo(history, layout);
                            set_command_open.set(false);
                            set_command_query.set(String::new());
                        }
                    ><IconUndo size=15 /><span><b>"Undo Layout Change"</b><i>"Revert position or deck state"</i></span></button>
                    <button
                        class:hidden=move || !command_matches(&command_query.get(), "redo layout forward")
                        on:click=move |_| {
                            apply_redo(history, layout);
                            set_command_open.set(false);
                            set_command_query.set(String::new());
                        }
                    ><IconRedo size=15 /><span><b>"Redo Layout Change"</b><i>"Re-apply position or deck state"</i></span></button>
                    <button
                        class:hidden=move || !command_matches(&command_query.get(), "arrange home canonical default")
                        on:click=move |_| {
                            history.update(|h| h.push(layout.get_untracked()));
                            layout.update(|l| l.apply_arrangement(ArrangementMode::Home, Some(usable_viewport())));
                            layout.get_untracked().save();
                            set_command_open.set(false);
                            set_command_query.set(String::new());
                        }
                    ><IconRefresh size=15 /><span><b>"Arrange: Home"</b><i>"Canonical workspace overview"</i></span></button>
                    <button
                        class:hidden=move || !command_matches(&command_query.get(), "arrange grid structured columns")
                        on:click=move |_| {
                            history.update(|h| h.push(layout.get_untracked()));
                            layout.update(|l| l.apply_arrangement(ArrangementMode::Grid, Some(usable_viewport())));
                            layout.get_untracked().save();
                            set_command_open.set(false);
                            set_command_query.set(String::new());
                        }
                    ><IconGrid size=15 /><span><b>"Arrange: Grid"</b><i>"Structured multi-track lanes"</i></span></button>
                    <button
                        class:hidden=move || !command_matches(&command_query.get(), "arrange compact packing fit")
                        on:click=move |_| {
                            history.update(|h| h.push(layout.get_untracked()));
                            layout.update(|l| l.apply_arrangement(ArrangementMode::Compact, Some(usable_viewport())));
                            layout.get_untracked().save();
                            set_command_open.set(false);
                            set_command_query.set(String::new());
                        }
                    ><IconMinimize size=15 /><span><b>"Arrange: Compact"</b><i>"Dense packing"</i></span></button>
                    <button
                        class:hidden=move || !command_matches(&command_query.get(), "arrange relations causal")
                        on:click=move |_| {
                            history.update(|h| h.push(layout.get_untracked()));
                            layout.update(|l| l.apply_arrangement(ArrangementMode::Relations, Some(usable_viewport())));
                            layout.get_untracked().save();
                            set_command_open.set(false);
                            set_command_query.set(String::new());
                        }
                    ><Link size=15 /><span><b>"Arrange: Relations"</b><i>"Mind organ graph"</i></span></button>
                    <button
                        class:hidden=move || !command_matches(&command_query.get(), "fit all zoom viewport center")
                        on:click=move |_| {
                            if let Some(bbox) = layout.get_untracked().bounding_rect() {
                                let (w, h) = (
                                    web_sys::window().and_then(|w| w.inner_width().ok()).and_then(|v| v.as_f64()).unwrap_or(1440.0),
                                    web_sys::window().and_then(|w| w.inner_height().ok()).and_then(|v| v.as_f64()).unwrap_or(900.0),
                                );
                                let (z, (px, py)) = DesktopLayout::fit_to_viewport(bbox, w, h, 60.0);
                                set_zoom.set(z);
                                set_pan.set((px, py));
                            } else {
                                set_zoom.set(1.0);
                                set_pan.set((0.0, 0.0));
                            }
                            set_command_open.set(false);
                            set_command_query.set(String::new());
                        }
                    ><IconMaximize size=15 /><span><b>"Fit All to Viewport"</b><i>"Ctrl+0 · Center and scale canvas"</i></span></button>
                    <button
                        class:hidden=move || !command_matches(&command_query.get(), "reset layout")
                        on:click=move |_| {
                            history.update(|h| h.push(layout.get_untracked()));
                            layout.update(|l| l.reset_desktop(None));
                            layout.get_untracked().save();
                            set_command_open.set(false);
                            set_command_query.set(String::new());
                        }
                    ><IconRefresh size=15 /><span><b>"Reset Desktop Layout"</b><i>"Canonical Home coordinates"</i></span></button>
                </nav>
            </Show>

            <label class:open=move || command_open.get() class="command-bar" aria-label="Search or act">
                <Search size=19 />
                <input
                    node_ref=command_input
                    type="search"
                    placeholder="Search or act…"
                    prop:value=move || command_query.get()
                    on:focus=move |_| set_command_open.set(true)
                    on:input=move |event| set_command_query.set(event_target_value(&event))
                    on:keydown=move |event: KeyboardEvent| {
                        if event.key() == "Enter" {
                            let q = command_query.get();
                            if command_matches(&q, "undo") {
                                event.prevent_default();
                                apply_undo(history, layout);
                                set_command_open.set(false);
                                set_command_query.set(String::new());
                            } else if command_matches(&q, "redo") {
                                event.prevent_default();
                                apply_redo(history, layout);
                                set_command_open.set(false);
                                set_command_query.set(String::new());
                            } else if command_matches(&q, "arrange home") {
                                event.prevent_default();
                                history.update(|h| h.push(layout.get_untracked()));
                                layout.update(|l| l.apply_arrangement(ArrangementMode::Home, Some(usable_viewport())));
                                layout.get_untracked().save();
                                set_command_open.set(false);
                                set_command_query.set(String::new());
                            } else if command_matches(&q, "arrange grid") {
                                event.prevent_default();
                                history.update(|h| h.push(layout.get_untracked()));
                                layout.update(|l| l.apply_arrangement(ArrangementMode::Grid, Some(usable_viewport())));
                                layout.get_untracked().save();
                                set_command_open.set(false);
                                set_command_query.set(String::new());
                            } else if command_matches(&q, "arrange compact") {
                                event.prevent_default();
                                history.update(|h| h.push(layout.get_untracked()));
                                layout.update(|l| l.apply_arrangement(ArrangementMode::Compact, Some(usable_viewport())));
                                layout.get_untracked().save();
                                set_command_open.set(false);
                                set_command_query.set(String::new());
                            } else if command_matches(&q, "arrange relations") {
                                event.prevent_default();
                                history.update(|h| h.push(layout.get_untracked()));
                                layout.update(|l| l.apply_arrangement(ArrangementMode::Relations, Some(usable_viewport())));
                                layout.get_untracked().save();
                                set_command_open.set(false);
                                set_command_query.set(String::new());
                            } else if command_matches(&q, "fit all") {
                                event.prevent_default();
                                if let Some(bbox) = layout.get_untracked().bounding_rect() {
                                    let (w, h) = (
                                        web_sys::window().and_then(|w| w.inner_width().ok()).and_then(|v| v.as_f64()).unwrap_or(1440.0),
                                        web_sys::window().and_then(|w| w.inner_height().ok()).and_then(|v| v.as_f64()).unwrap_or(900.0),
                                    );
                                    let (z, (px, py)) = DesktopLayout::fit_to_viewport(bbox, w, h, 60.0);
                                    set_zoom.set(z);
                                    set_pan.set((px, py));
                                }
                                set_command_open.set(false);
                                set_command_query.set(String::new());
                            } else if command_matches(&q, "reset layout") || command_matches(&q, "reset desktop") {
                                event.prevent_default();
                                history.update(|h| h.push(layout.get_untracked()));
                                layout.update(|l| l.reset_desktop(None));
                                layout.get_untracked().save();
                                set_command_open.set(false);
                                set_command_query.set(String::new());
                            } else if command_matches(&q, "insight") || command_matches(&q, "status") || command_matches(&q, "health") || command_matches(&q, "why") {
                                event.prevent_default();
                                select_from_command("insight");
                            } else if command_matches(&q, "agents") || command_matches(&q, "launch") || command_matches(&q, "opencode") {
                                event.prevent_default();
                                select_from_command("agents");
                            }
                        } else if event.key() == "Escape" {
                            set_command_open.set(false);
                        }
                    }
                />
                <kbd class="shortcut">"Ctrl+K"</kbd>
            </label>
        </section>
    }
}
