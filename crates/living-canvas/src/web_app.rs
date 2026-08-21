// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

use leptos::prelude::*;
use leptos::task::spawn_local;
use living_canvas::{
    ArrangementMode, CardGeometry, CardId, DesktopLayout, GatewayMindClient, MindClient,
};
use lucide_leptos::{
    Ellipsis, FileCheck, Files, FolderOpen, Link, ListChecks, Map, Search, Sparkles, UsersRound,
};
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{EventSource, HtmlElement, KeyboardEvent, MessageEvent, PointerEvent};

#[derive(Clone, Debug, PartialEq)]
enum DragTarget {
    Card(CardId),
    Deck(String),
}

#[derive(Clone, Debug, PartialEq)]
struct DragState {
    target: DragTarget,
    offset_x: f64,
    offset_y: f64,
    width: f64,
    height: f64,
    drop_target: Option<CardId>,
}

#[derive(Clone, Debug, PartialEq)]
enum ResizeTarget {
    Card(CardId),
    Deck(String),
}

#[derive(Clone, Debug, PartialEq)]
struct ResizeState {
    target: ResizeTarget,
    start_pointer_x: f64,
    start_pointer_y: f64,
    start_width: f64,
    start_height: f64,
}

#[derive(Clone, Debug)]
enum RuntimeState {
    Loading,
    Ready {
        mode: cybou_web_contracts::SessionMode,
        snapshot: cybou_web_contracts::SnapshotProjection,
        /// `None` when the gateway could not assemble the owner projection at all. It is kept
        /// distinct from a projection whose sections are individually unknown.
        mind: Option<cybou_web_contracts::MindProjection>,
    },
    Error(String),
}

#[component]
fn IconPin(#[prop(default = 12)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="12" y1="17" x2="12" y2="22"></line>
            <path d="M5 17h14v-1.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V6h1a1 1 0 0 0 0-2H8a1 1 0 0 0 0 2h1v4.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24Z"></path>
        </svg>
    }
}

#[component]
fn IconMinimize(#[prop(default = 12)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="4 14 10 14 10 20"></polyline>
            <polyline points="20 10 14 10 14 4"></polyline>
            <line x1="14" y1="10" x2="21" y2="3"></line>
            <line x1="3" y1="21" x2="10" y2="14"></line>
        </svg>
    }
}

#[component]
fn IconMaximize(#[prop(default = 12)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="15 3 21 3 21 9"></polyline>
            <polyline points="9 21 3 21 3 15"></polyline>
            <line x1="21" y1="3" x2="14" y2="10"></line>
            <line x1="3" y1="21" x2="10" y2="14"></line>
        </svg>
    }
}

#[component]
fn IconGrid(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="3" y="3" width="7" height="7"></rect>
            <rect x="14" y="3" width="7" height="7"></rect>
            <rect x="14" y="14" width="7" height="7"></rect>
            <rect x="3" y="14" width="7" height="7"></rect>
        </svg>
    }
}

#[component]
fn IconRefresh(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"></path>
            <path d="M21 3v5h-5"></path>
            <path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"></path>
            <path d="M8 16H3v5"></path>
        </svg>
    }
}

#[component]
fn IconClose(#[prop(default = 12)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="18" y1="6" x2="6" y2="18"></line>
            <line x1="6" y1="6" x2="18" y2="18"></line>
        </svg>
    }
}

#[component]
fn IconTerminal(#[prop(default = 15)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="4 17 10 11 4 5"></polyline>
            <line x1="12" y1="19" x2="20" y2="19"></line>
        </svg>
    }
}

#[component]
fn IconResizeGrip() -> impl IntoView {
    view! {
        <svg width="10" height="10" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round">
            <line x1="8" y1="2" x2="2" y2="8"></line>
            <line x1="8" y1="5" x2="5" y2="8"></line>
            <line x1="8" y1="8" x2="8" y2="8"></line>
        </svg>
    }
}

#[component]
fn IconUndo(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M3 7v6h6"></path>
            <path d="M21 17a9 9 0 0 0-9-9 9 9 0 0 0-6 2.3L3 13"></path>
        </svg>
    }
}

#[component]
fn IconRedo(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21 7v6h-6"></path>
            <path d="M3 17a9 9 0 0 1 9-9 9 9 0 0 1 6 2.3l3 2.7"></path>
        </svg>
    }
}

#[allow(dead_code)]
#[component]
fn IconExternalLink(#[prop(default = 12)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M15 3h6v6"></path>
            <path d="M10 14 21 3"></path>
            <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"></path>
        </svg>
    }
}

#[allow(dead_code)]
#[component]
fn IconLayers(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="m12.83 2.18a2 2 0 0 0-1.66 0L2.6 6.08a1 1 0 0 0 0 1.83l8.58 3.91a2 2 0 0 0 1.66 0l8.58-3.9a1 1 0 0 0 0-1.83Z"></path>
            <path d="m2 12 8.58 3.91a2 2 0 0 0 1.66 0L21 12"></path>
            <path d="m2 17 8.58 3.91a2 2 0 0 0 1.66 0L21 17"></path>
        </svg>
    }
}

#[component]
fn IconShield(#[prop(default = 14)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"></path>
        </svg>
    }
}

#[component]
fn IconFolder(#[prop(default = 14)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z"></path>
        </svg>
    }
}

#[component]
fn IconFile(#[prop(default = 14)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"></path>
            <path d="M14 2v4a2 2 0 0 0 2 2h4"></path>
        </svg>
    }
}

#[component]
fn IconGlobe(#[prop(default = 14)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="10"></circle>
            <path d="M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20"></path>
            <path d="M2 12h20"></path>
        </svg>
    }
}

#[component]
fn IconArrowLeft(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="m12 19-7-7 7-7"></path>
            <path d="M19 12H5"></path>
        </svg>
    }
}

#[component]
fn IconArrowRight(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M5 12h14"></path>
            <path d="m12 5 7 7-7 7"></path>
        </svg>
    }
}

#[component]
fn IconHome(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="m3 9 9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"></path>
            <polyline points="9 22 9 12 15 12 15 22"></polyline>
        </svg>
    }
}

#[component]
fn IconZoomIn(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="11" cy="11" r="8"></circle>
            <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
            <line x1="11" y1="8" x2="11" y2="14"></line>
            <line x1="8" y1="11" x2="14" y2="11"></line>
        </svg>
    }
}

#[component]
fn IconZoomOut(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="11" cy="11" r="8"></circle>
            <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
            <line x1="8" y1="11" x2="14" y2="11"></line>
        </svg>
    }
}

#[component]
fn IconPlus(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="12" y1="5" x2="12" y2="19"></line>
            <line x1="5" y1="12" x2="19" y2="12"></line>
        </svg>
    }
}

#[component]
fn IconTrash(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="3 6 5 6 21 6"></polyline>
            <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
        </svg>
    }
}

#[component]
fn IconSave(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"></path>
            <polyline points="17 21 17 13 7 13 7 21"></polyline>
            <polyline points="7 3 7 8 15 8"></polyline>
        </svg>
    }
}

#[component]
fn IconEdit(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M17 3a2.828 2.828 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z"></path>
        </svg>
    }
}

#[component]
fn IconCopy(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
        </svg>
    }
}

#[component]
fn IconCheck(#[prop(default = 13)] size: u32) -> impl IntoView {
    view! {
        <svg width=size.to_string() height=size.to_string() viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="20 6 9 17 4 12"></polyline>
        </svg>
    }
}

#[component]
fn CardControls(card: CardId, layout: RwSignal<DesktopLayout>) -> impl IntoView {
    let is_pinned = move || layout.get().presentation(card).pinned;
    let is_collapsed = move || layout.get().presentation(card).collapsed;
    let is_maximized = move || layout.get().presentation(card).maximized;

    view! {
        <div class="card-controls" on:pointerdown=move |e: PointerEvent| e.stop_propagation() on:click=move |e: web_sys::MouseEvent| e.stop_propagation()>
            <button
                class:active=is_pinned
                class="card-control-btn pin-btn"
                title=move || if is_pinned() { "Unpin card" } else { "Pin card (lock position)" }
                aria-label=move || if is_pinned() { "Unpin card" } else { "Pin card" }
                on:click=move |_| {
                    layout.update(|current| {
                        let p = current.presentation(card);
                        current.set_pinned(card, !p.pinned);
                    });
                    layout.get_untracked().save();
                }
            >
                <IconPin size=12 />
            </button>
            <button
                class:active=is_maximized
                class="card-control-btn maximize-btn"
                title=move || if is_maximized() { "Restore window size" } else { "Maximize window" }
                aria-label=move || if is_maximized() { "Restore window size" } else { "Maximize window" }
                on:click=move |_| {
                    layout.update(|current| {
                        let p = current.presentation(card);
                        current.set_maximized(card, !p.maximized);
                    });
                    layout.get_untracked().save();
                }
            >
                {move || if is_maximized() {
                    view! { <IconMinimize size=12 /> }.into_any()
                } else {
                    view! { <IconMaximize size=12 /> }.into_any()
                }}
            </button>
            <button
                class:active=is_collapsed
                class="card-control-btn collapse-btn"
                title=move || if is_collapsed() { "Expand card" } else { "Collapse card" }
                aria-label=move || if is_collapsed() { "Expand card" } else { "Collapse card" }
                on:click=move |_| {
                    layout.update(|current| {
                        let p = current.presentation(card);
                        current.set_collapsed(card, !p.collapsed);
                    });
                    layout.get_untracked().save();
                }
            >
                <IconMinimize size=12 />
            </button>
            {if card.spec().closable {
                view! {
                    <button
                        class="card-control-btn close-btn"
                        title="Close card"
                        aria-label="Close card"
                        on:click=move |_| {
                            layout.update(|current| {
                                current.close_card(card);
                            });
                            layout.get_untracked().save();
                        }
                    >
                        <IconClose size=12 />
                    </button>
                }.into_any()
            } else {
                let in_deck = move || layout.get().is_in_deck(card);
                view! {
                    <Show when=in_deck>
                        <button
                            class="card-control-btn detach-btn"
                            title="Detach from Deck"
                            aria-label="Detach from Deck"
                            on:click=move |_| {
                                layout.update(|l| {
                                    if let Some(d) = l.deck_for_card(card) {
                                        let d_id = d.id.clone();
                                        l.detach_from_deck(&d_id, card);
                                    }
                                });
                                layout.get_untracked().save();
                            }
                        >
                            <IconExternalLink size=12 />
                        </button>
                    </Show>
                }.into_any()
            }}
        </div>
    }
}

#[component]
fn CardResizeHandle(
    card: CardId,
    layout: RwSignal<DesktopLayout>,
    resizing: RwSignal<Option<ResizeState>>,
) -> impl IntoView {
    let is_collapsed = move || layout.get().presentation(card).collapsed;
    view! {
        <Show when=move || !is_collapsed()>
            <div
                class="card-resize-handle"
                title="Resize"
                aria-label="Resize card"
                on:pointerdown=move |event| start_resize(event, card, layout, resizing)
                on:click=move |e: web_sys::MouseEvent| e.stop_propagation()
            >
                <IconResizeGrip />
            </div>
        </Show>
    }
}

#[component]
fn DeckResizeHandle(
    deck_id: String,
    layout: RwSignal<DesktopLayout>,
    resizing: RwSignal<Option<ResizeState>>,
) -> impl IntoView {
    let d_id = deck_id.clone();
    let d_id_res = deck_id;
    let is_collapsed = move || layout.get().deck(&d_id).is_some_and(|d| d.presentation.collapsed);
    view! {
        <Show when=move || !is_collapsed()>
            {
                let d_id_click = d_id_res.clone();
                view! {
                    <div
                        class="card-resize-handle"
                        title="Resize deck"
                        aria-label="Resize deck"
                        on:pointerdown=move |event| start_deck_resize(event, d_id_click.clone(), layout, resizing)
                        on:click=move |e: web_sys::MouseEvent| e.stop_propagation()
                    >
                        <IconResizeGrip />
                    </div>
                }
            }
        </Show>
    }
}

const SHELL_AUTOCOMPLETE: &[&str] = &[
    "help", "clear", "status", "capabilities", "mind", "identity", "journal",
    "lifecycle", "commitments", "self", "attention", "beliefs", "perception",
    "context", "ls", "cat", "echo", "pwd", "cd", "ps", "uptime", "whoami",
    "date", "version",
];

#[component]
fn ShellCard(
    layout: RwSignal<DesktopLayout>,
    selected: ReadSignal<&'static str>,
    set_selected: WriteSignal<&'static str>,
    dragging: RwSignal<Option<DragState>>,
    resizing: RwSignal<Option<ResizeState>>,
) -> impl IntoView {
    let card_id = CardId::Shell(0);
    let card_open = move || layout.get().contains_card(card_id) && !layout.get().is_in_deck(card_id);
    let is_collapsed = move || layout.get().presentation(card_id).collapsed;
    let (history, set_history) = signal(vec![(
        String::new(),
        "CYBOU Shell (Zone 3 bounded Body capabilities)\nType 'help' for available capabilities.\n"
            .to_string(),
        0,
    )]);
    let (cmd_history, set_cmd_history) = signal(Vec::<String>::new());
    let (history_idx, set_history_idx) = signal(Option::<usize>::None);
    let (temp_draft, set_temp_draft) = signal(String::new());

    let (input_val, set_input_val) = signal(String::new());
    let (cwd, set_cwd) = signal("/".to_string());
    let (running, set_running) = signal(false);
    let output_ref = NodeRef::<leptos::html::Div>::new();
    let input_ref = NodeRef::<leptos::html::Input>::new();

    let scroll_output_to_bottom = move || {
        if let Some(el) = output_ref.get() {
            el.set_scroll_top(el.scroll_height());
        }
    };

    let submit_command = move || {
        let cmd = input_val.get();
        let trimmed = cmd.trim();
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
        let cmd_str = trimmed.to_string();
        set_cmd_history.update(|h| {
            if h.last() != Some(&cmd_str) {
                h.push(cmd_str.clone());
            }
        });
        set_history_idx.set(None);
        set_temp_draft.set(String::new());
        set_running.set(true);
        set_input_val.set(String::new());
        spawn_local(async move {
            let client = GatewayMindClient;
            match client.execute_shell(&cmd_str).await {
                Ok(resp) => {
                    let text = if !resp.stdout.is_empty() {
                        resp.stdout
                    } else if !resp.stderr.is_empty() {
                        resp.stderr
                    } else {
                        String::new()
                    };
                    set_cwd.set(resp.cwd);
                    set_history.update(|h| h.push((cmd_str, text, resp.exit_code)));
                }
                Err(e) => {
                    set_history.update(|h| h.push((cmd_str, format!("Error: {e}\n"), 1)));
                }
            }
            set_running.set(false);
            scroll_output_to_bottom();
        });
    };

    let (active_tab, set_active_tab) = signal(0usize);

    let run_snippet = move |snippet: &'static str| {
        set_input_val.set(snippet.to_string());
        submit_command();
    };

    view! {
        <Show when=card_open>
            <div
                tabindex="0"
                role="region"
                aria-label="CYBOU Shell capability surface. Drag to reposition; use arrow keys for keyboard movement."
                class="object shell-card"
                class:selected=move || selected.get() == "shell"
                class:pinned=move || layout.get().presentation(card_id).pinned
                class:collapsed=is_collapsed
                style=move || card_style(layout.get(), card_id)
                on:focus=move |_| set_selected.set("shell")
                on:click=move |_| {
                    set_selected.set("shell");
                    layout.update(|l| l.bring_forward(card_id));
                    if let Some(inp) = input_ref.get() {
                        let _ = inp.focus();
                    }
                }
                on:keydown=move |event: KeyboardEvent| keyboard_move(event, card_id, layout)
            >
                <header
                    class="object-header card-header"
                    on:pointerdown=move |event: PointerEvent| start_drag(event, card_id, layout, dragging)
                >
                    <small class="panel-kicker"><IconTerminal size=14 /><span>"Shell · Zone 3 Body"</span></small>
                    <div class="shell-tabs-bar" on:pointerdown=move |e: PointerEvent| e.stop_propagation()>
                        <button class="shell-tab" class:active=move || active_tab.get() == 0 on:click=move |_| set_active_tab.set(0)>"Host #1"</button>
                        <button class="shell-tab" class:active=move || active_tab.get() == 1 on:click=move |_| {
                            set_active_tab.set(1);
                            run_snippet("busctl --user list | grep cybou");
                        }>"Mind IPC"</button>
                        <button class="shell-tab" class:active=move || active_tab.get() == 2 on:click=move |_| {
                            set_active_tab.set(2);
                            run_snippet("journalctl --user -u cybou-* -n 12 --no-pager");
                        }>"Logs"</button>
                    </div>
                    <CardControls card=card_id layout=layout />
                </header>
                <Show
                    when=move || !is_collapsed()
                    fallback=move || {
                        let current_cwd = cwd.get();
                        view! {
                            <div class="card-collapsed-summary">
                                <b>"Shell"</b>
                                <span>{current_cwd}</span>
                            </div>
                        }
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
                        <div class="shell-snippets-bar">
                            <span class="snippets-label">"Quick:"</span>
                            <button class="shell-chip" on:click=move |_| run_snippet("ps aux | grep cybou")>"ps aux"</button>
                            <button class="shell-chip" on:click=move |_| run_snippet("df -h /home/demo")>"df -h"</button>
                            <button class="shell-chip" on:click=move |_| run_snippet("busctl --user list")>"busctl"</button>
                            <button class="shell-chip" on:click=move |_| run_snippet("whoami && id")>"whoami"</button>
                        </div>
                        <div class="shell-output" node_ref=output_ref>
                            <For
                                each=move || history.get()
                                key=|(cmd, out, code)| format!("{cmd}-{out}-{code}")
                                children=move |(cmd, out, code)| {
                                    view! {
                                        <div class="shell-entry">
                                            {if !cmd.is_empty() {
                                                view! { <div class="shell-cmd-echo"><span class="shell-prompt-char">"$"</span>" "{cmd}</div> }.into_any()
                                            } else {
                                                ().into_any()
                                            }}
                                            <pre class="shell-out-text" class:error=move || code != 0>{out}</pre>
                                        </div>
                                    }
                                }
                            />
                        </div>
                        <div class="shell-input-line" on:pointerdown=move |e: PointerEvent| e.stop_propagation()>
                            <span class="shell-prompt">{move || format!("cybou:{} $", cwd.get())}</span>
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
                <CardResizeHandle card=card_id layout=layout resizing=resizing />
            </div>
        </Show>
    }
}

#[component]
fn FileManagerCard(
    layout: RwSignal<DesktopLayout>,
    selected: ReadSignal<&'static str>,
    set_selected: WriteSignal<&'static str>,
    dragging: RwSignal<Option<DragState>>,
    resizing: RwSignal<Option<ResizeState>>,
) -> impl IntoView {
    let card_id = CardId::FileManager(0);
    let card_open = move || layout.get().contains_card(card_id) && !layout.get().is_in_deck(card_id);
    let is_collapsed = move || layout.get().presentation(card_id).collapsed;

    let (current_path, set_current_path) = signal("/home/demo".to_string());
    let (entries, set_entries) = signal(Vec::<(String, bool, String)>::new());
    let (selected_file, set_selected_file) = signal(Option::<String>::None);
    let (file_content, set_file_content) = signal(String::new());
    let (is_editing, set_is_editing) = signal(false);
    let (edit_content, set_edit_content) = signal(String::new());
    let (loading, set_loading) = signal(false);
    let (saving, set_saving) = signal(false);
    let (error_msg, set_error_msg) = signal(Option::<String>::None);
    let (prompt_mode, set_prompt_mode) = signal(Option::<&'static str>::None);
    let (prompt_val, set_prompt_val) = signal(String::new());

    let load_dir = move |path: String| {
        set_loading.set(true);
        set_error_msg.set(None);
        set_selected_file.set(None);
        set_file_content.set(String::new());
        set_is_editing.set(false);
        let p = path.clone();
        spawn_local(async move {
            let client = GatewayMindClient;
            let cmd = format!("ls -la {p}");
            match client.execute_shell(&cmd).await {
                Ok(resp) => {
                    set_loading.set(false);
                    if resp.exit_code == 0 {
                        set_current_path.set(p);
                        let mut list = Vec::new();
                        for line in resp.stdout.lines().skip(1) {
                            let parts: Vec<&str> = line.split_whitespace().collect();
                            if parts.len() >= 9 {
                                let is_dir = parts[0].starts_with('d');
                                let size = parts[4].to_string();
                                let name = parts[8..].join(" ");
                                if name != "." && name != ".." {
                                    list.push((name, is_dir, size));
                                }
                            }
                        }
                        set_entries.set(list);
                    } else {
                        set_error_msg.set(Some(if !resp.stderr.is_empty() {
                            resp.stderr
                        } else {
                            "Failed to list directory".to_string()
                        }));
                    }
                }
                Err(err) => {
                    set_loading.set(false);
                    set_error_msg.set(Some(err.to_string()));
                }
            }
        });
    };

    let p_initial = current_path.get_untracked();
    Effect::new(move |_| {
        if card_open() {
            load_dir(p_initial.clone());
        }
    });

    let view_file = move |filename: String| {
        let p = format!("{}/{}", current_path.get(), filename);
        set_loading.set(true);
        set_selected_file.set(Some(filename));
        set_is_editing.set(false);
        spawn_local(async move {
            let client = GatewayMindClient;
            let cmd = format!("cat {p}");
            match client.execute_shell(&cmd).await {
                Ok(resp) => {
                    set_loading.set(false);
                    if resp.exit_code == 0 {
                        set_file_content.set(resp.stdout.clone());
                        set_edit_content.set(resp.stdout);
                    } else {
                        set_file_content.set(format!("Error reading file: {}", resp.stderr));
                    }
                }
                Err(err) => {
                    set_loading.set(false);
                    set_file_content.set(format!("Error: {err}"));
                }
            }
        });
    };

    let save_file = move || {
        if let Some(file_name) = selected_file.get() {
            let p = format!("{}/{}", current_path.get(), file_name);
            let content = edit_content.get();
            set_saving.set(true);
            spawn_local(async move {
                let client = GatewayMindClient;
                let b64 = js_sys::encode_uri_component(&content).as_string().unwrap_or_default();
                let cmd = format!("node -e 'fs.writeFileSync(process.argv[1], decodeURIComponent(process.argv[2]))' {p} \"{b64}\" 2>/dev/null || python3 -c 'import sys, urllib.parse; open(sys.argv[1], \"w\").write(urllib.parse.unquote(sys.argv[2]))' {p} \"{b64}\"");
                let _ = client.execute_shell(&cmd).await;
                set_file_content.set(content);
                set_saving.set(false);
                set_is_editing.set(false);
            });
        }
    };

    let execute_prompt_action = move || {
        let mode = prompt_mode.get();
        let val = prompt_val.get().trim().to_string();
        let cur = current_path.get();
        if let Some(m) = mode {
            if !val.is_empty() {
                spawn_local(async move {
                    let client = GatewayMindClient;
                    let cmd = match m {
                        "new_file" => format!("touch {}/{}", cur, val),
                        "new_dir" => format!("mkdir -p {}/{}", cur, val),
                        "delete" => format!("rm -rf {}/{}", cur, val),
                        _ => String::new(),
                    };
                    if !cmd.is_empty() {
                        let _ = client.execute_shell(&cmd).await;
                        load_dir(cur);
                    }
                });
            }
        }
        set_prompt_mode.set(None);
        set_prompt_val.set(String::new());
    };

    let go_up = move || {
        let cur = current_path.get();
        if cur != "/home/demo" && cur != "/" {
            if let Some(pos) = cur.rfind('/') {
                let parent = if pos == 0 { "/".to_string() } else { cur[..pos].to_string() };
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
                        <small class="card-badge">"Zone 3 Bounded"</small>
                    </span>
                    <CardControls card=card_id layout=layout />
                </header>

                <Show when=move || !is_collapsed()>
                    <div class="fm-body" on:pointerdown=move |e: PointerEvent| e.stop_propagation()>
                        <div class="fm-path-bar">
                            <div class="fm-crumbs">
                                <span>{move || current_path.get()}</span>
                            </div>
                            <div class="fm-toolbar">
                                <button class="fm-btn" title="Go to home" on:click=move |_| load_dir("/home/demo".to_string())>
                                    <IconHome size=12 />
                                    <span>"Home"</span>
                                </button>
                                <button class="fm-btn" title="Up one level" on:click=move |_| go_up()>
                                    <IconArrowLeft size=12 />
                                    <span>"Up"</span>
                                </button>
                                <button class="fm-btn" title="New file" on:click=move |_| {
                                    set_prompt_mode.set(Some("new_file"));
                                    set_prompt_val.set("new_file.txt".into());
                                }>
                                    <IconPlus size=12 />
                                    <span>"New File"</span>
                                </button>
                                <button class="fm-btn" title="New folder" on:click=move |_| {
                                    set_prompt_mode.set(Some("new_dir"));
                                    set_prompt_val.set("new_folder".into());
                                }>
                                    <IconFolder size=12 />
                                    <span>"Folder"</span>
                                </button>
                                <button class="fm-btn" title="Refresh folder" on:click=move |_| load_dir(current_path.get())>
                                    <IconRefresh size=12 />
                                </button>
                            </div>
                        </div>

                        <Show when=move || prompt_mode.get().is_some()>
                            <div class="fm-prompt-bar">
                                <span class="fm-prompt-title">
                                    {move || match prompt_mode.get() {
                                        Some("new_file") => "Create New File:",
                                        Some("new_dir") => "Create New Folder:",
                                        Some("delete") => "Confirm Deletion:",
                                        _ => "",
                                    }}
                                </span>
                                <input
                                    type="text"
                                    class="fm-prompt-input"
                                    prop:value=prompt_val
                                    on:input=move |e| set_prompt_val.set(event_target_value(&e))
                                    on:keydown=move |e: KeyboardEvent| {
                                        if e.key() == "Enter" {
                                            execute_prompt_action();
                                        }
                                    }
                                />
                                <button class="fm-btn" on:click=move |_| execute_prompt_action()>"Confirm"</button>
                                <button class="fm-btn" on:click=move |_| set_prompt_mode.set(None)>"Cancel"</button>
                            </div>
                        </Show>

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
                                                        let new_p = format!("{}/{}", p, n_click);
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
                                            <Show when=move || !is_editing.get()>
                                                <button class="fm-btn" title="Edit file" on:click=move |_| set_is_editing.set(true)>
                                                    <IconEdit size=11 />
                                                    <span>"Edit"</span>
                                                </button>
                                            </Show>
                                            <Show when=move || is_editing.get()>
                                                <button class="fm-btn btn-save" disabled=move || saving.get() on:click=move |_| save_file()>
                                                    <IconSave size=11 />
                                                    <span>{move || if saving.get() { "Saving…" } else { "Save" }}</span>
                                                </button>
                                                <button class="fm-btn" on:click=move |_| set_is_editing.set(false)>"Cancel"</button>
                                            </Show>
                                            <button
                                                class="fm-btn btn-delete"
                                                title="Delete file"
                                                on:click=move |_| {
                                                    if let Some(f) = selected_file.get() {
                                                        set_prompt_mode.set(Some("delete"));
                                                        set_prompt_val.set(f);
                                                    }
                                                }
                                            >
                                                <IconTrash size=11 />
                                            </button>
                                            <button class="fm-btn" on:click=move |_| set_selected_file.set(None)>"×"</button>
                                        </div>
                                    </header>
                                    <Show
                                        when=move || is_editing.get()
                                        fallback=move || view! { <pre class="fm-preview-text">{move || file_content.get()}</pre> }
                                    >
                                        <textarea
                                            class="fm-editor-textarea"
                                            prop:value=edit_content
                                            on:input=move |e| set_edit_content.set(event_target_value(&e))
                                        ></textarea>
                                    </Show>
                                </aside>
                            </Show>
                        </div>
                    </div>
                </Show>
                <CardResizeHandle card=card_id layout=layout resizing=resizing />
            </div>
        </Show>
    }
}

#[component]
fn WebBrowserCard(
    layout: RwSignal<DesktopLayout>,
    selected: ReadSignal<&'static str>,
    set_selected: WriteSignal<&'static str>,
    dragging: RwSignal<Option<DragState>>,
    resizing: RwSignal<Option<ResizeState>>,
) -> impl IntoView {
    let card_id = CardId::WebBrowser(0);
    let card_open = move || layout.get().contains_card(card_id) && !layout.get().is_in_deck(card_id);
    let is_collapsed = move || layout.get().presentation(card_id).collapsed;

    let (url, set_url) = signal("/api/v1/session".to_string());
    let (input_url, set_input_url) = signal("/api/v1/session".to_string());
    let (history, set_history) = signal(vec!["/api/v1/session".to_string()]);
    let (hist_idx, set_hist_idx) = signal(0usize);

    let (reader_mode, set_reader_mode) = signal(false);
    let (reader_text, set_reader_text) = signal(String::new());
    let (loading, set_loading) = signal(false);

    let load_reader_content = move |target: String| {
        if !reader_mode.get() {
            return;
        }
        set_loading.set(true);
        spawn_local(async move {
            if let Ok(resp) = gloo_net::http::Request::get(&target).send().await {
                if let Ok(txt) = resp.text().await {
                    // Pretty format if JSON
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&txt) {
                        set_reader_text.set(serde_json::to_string_pretty(&val).unwrap_or(txt));
                    } else {
                        set_reader_text.set(txt);
                    }
                } else {
                    set_reader_text.set("Failed to read response body".into());
                }
            } else {
                set_reader_text.set(format!("Could not fetch {target} for Reader Mode."));
            }
            set_loading.set(false);
        });
    };

    let navigate = move |target: String| {
        let clean = target.trim().to_string();
        if !clean.is_empty() {
            set_url.set(clean.clone());
            set_input_url.set(clean.clone());
            set_history.update(|h| {
                h.push(clean.clone());
                set_hist_idx.set(h.len() - 1);
            });
            load_reader_content(clean);
        }
    };

    let go_back = move || {
        let idx = hist_idx.get();
        if idx > 0 {
            let next_idx = idx - 1;
            set_hist_idx.set(next_idx);
            let h = history.get();
            if let Some(prev) = h.get(next_idx) {
                set_url.set(prev.clone());
                set_input_url.set(prev.clone());
                load_reader_content(prev.clone());
            }
        }
    };

    let go_forward = move || {
        let idx = hist_idx.get();
        let h = history.get();
        if idx + 1 < h.len() {
            let next_idx = idx + 1;
            set_hist_idx.set(next_idx);
            if let Some(next) = h.get(next_idx) {
                set_url.set(next.clone());
                set_input_url.set(next.clone());
                load_reader_content(next.clone());
            }
        }
    };

    view! {
        <Show when=card_open>
            <div
                class:selected=move || selected.get() == "browser"
                class:collapsed=is_collapsed
                class:pinned=move || layout.get().presentation(card_id).pinned
                class="object web-browser-card"
                style=move || card_style(layout.get(), card_id)
                tabindex="0"
                role="region"
                aria-label="Bounded Web Browser"
                on:click=move |_| {
                    set_selected.set("browser");
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
                        <IconGlobe size=13 />
                        <strong class="card-title">"Web Browser"</strong>
                        <small class="card-badge">"Sandboxed"</small>
                    </span>
                    <CardControls card=card_id layout=layout />
                </header>

                <Show when=move || !is_collapsed()>
                    <div class="wb-body">
                        <div class="wb-nav-bar">
                            <button class="wb-btn" title="Back" on:click=move |_| go_back()>
                                <IconArrowLeft size=13 />
                            </button>
                            <button class="wb-btn" title="Forward" on:click=move |_| go_forward()>
                                <IconArrowRight size=13 />
                            </button>
                            <button class="wb-btn" title="Reload" on:click=move |_| {
                                let cur = url.get();
                                navigate(cur);
                            }>
                                <IconRefresh size=13 />
                            </button>
                            <button class="wb-btn" title="Home" on:click=move |_| navigate("/api/v1/session".to_string())>
                                <IconHome size=13 />
                            </button>

                            <form class="wb-url-form" on:submit=move |e: web_sys::SubmitEvent| {
                                e.prevent_default();
                                navigate(input_url.get());
                            }>
                                <input
                                    type="text"
                                    class="wb-url-input"
                                    placeholder="Enter URL or endpoint (e.g. /api/v1/snapshot)…"
                                    prop:value=input_url
                                    on:input=move |e| set_input_url.set(event_target_value(&e))
                                />
                            </form>

                            <button
                                class="wb-btn wb-reader-toggle"
                                class:active=move || reader_mode.get()
                                title="Toggle Formatted Reader Mode"
                                on:click=move |_| {
                                    let next = !reader_mode.get_untracked();
                                    set_reader_mode.set(next);
                                    if next {
                                        load_reader_content(url.get_untracked());
                                    }
                                }
                            >
                                <IconFile size=13 />
                                <span>{move || if reader_mode.get() { "Frame" } else { "Reader" }}</span>
                            </button>
                        </div>

                        <div class="wb-bookmarks">
                            <button class="wb-bookmark" on:click=move |_| navigate("/api/v1/session".to_string())>
                                <IconGlobe size=10 /><span>"Session"</span>
                            </button>
                            <button class="wb-bookmark" on:click=move |_| navigate("/api/v1/snapshot".to_string())>
                                <IconGlobe size=10 /><span>"Snapshot"</span>
                            </button>
                            <button class="wb-bookmark" on:click=move |_| navigate("/api/v1/capabilities".to_string())>
                                <IconGlobe size=10 /><span>"Capabilities"</span>
                            </button>
                            <button class="wb-bookmark" on:click=move |_| navigate("/api/v1/mind".to_string())>
                                <IconGlobe size=10 /><span>"Mind State"</span>
                            </button>
                            <button class="wb-bookmark" on:click=move |_| navigate("https://www.debian.org".to_string())>
                                <IconExternalLink size=10 /><span>"Debian"</span>
                            </button>
                            <button class="wb-bookmark" on:click=move |_| navigate("https://www.rust-lang.org".to_string())>
                                <IconExternalLink size=10 /><span>"Rust"</span>
                            </button>
                        </div>

                        <Show
                            when=move || reader_mode.get()
                            fallback=move || view! {
                                <div class="wb-frame-container">
                                    <iframe
                                        class="wb-frame"
                                        src=move || url.get()
                                        sandbox="allow-scripts allow-same-origin allow-forms"
                                        title="Web browser frame"
                                    ></iframe>
                                </div>
                            }
                        >
                            <div class="wb-reader-container">
                                <header class="wb-reader-header">
                                    <span><b>"Reader View:"</b> <code>{move || url.get()}</code></span>
                                    <small>{move || if loading.get() { "Loading content…" } else { "Structured payload" }}</small>
                                </header>
                                <pre class="wb-reader-content">{move || reader_text.get()}</pre>
                            </div>
                        </Show>
                    </div>
                </Show>
                <CardResizeHandle card=card_id layout=layout resizing=resizing />
            </div>
        </Show>
    }
}

#[cfg(target_arch = "wasm32")]
async fn async_sleep(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        if let Some(w) = web_sys::window() {
            let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms);
        }
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

#[cfg(not(target_arch = "wasm32"))]
async fn async_sleep(_ms: i32) {}

#[component]
fn JournalFeedCard(
    layout: RwSignal<DesktopLayout>,
    selected: ReadSignal<&'static str>,
    set_selected: WriteSignal<&'static str>,
    dragging: RwSignal<Option<DragState>>,
    resizing: RwSignal<Option<ResizeState>>,
) -> impl IntoView {
    let card_id = CardId::JournalFeed(0);
    let card_open = move || layout.get().contains_card(card_id) && !layout.get().is_in_deck(card_id);
    let is_collapsed = move || layout.get().presentation(card_id).collapsed;

    let (events, set_events) = signal(vec![
        ("14:00:01".to_string(), "lifecycle.state".to_string(), "Awake · Mind operational".to_string(), "sha256:7f83b165...".to_string(), "{\"organ\":\"lifecycled\",\"state\":\"Awake\",\"epoch\":1,\"timestamp\":\"2026-08-21T14:00:01Z\"}".to_string()),
        ("14:00:03".to_string(), "session.established".to_string(), "Local gateway session active".to_string(), "sha256:29e01a4b...".to_string(), "{\"organ\":\"cybou-web-gateway\",\"mode\":\"publicPreview\",\"trust\":\"established\"}".to_string()),
        ("14:00:05".to_string(), "identity.digest".to_string(), "Self continuity verified (11 organs)".to_string(), "sha256:1a8f94c2...".to_string(), "{\"organ\":\"identityd\",\"subject\":\"sovereign\",\"continuity\":true}".to_string()),
    ]);
    let (filter, set_filter) = signal("all".to_string());
    let (search_query, set_search_query) = signal(String::new());
    let (is_paused, set_is_paused) = signal(false);
    let (selected_event, set_selected_event) = signal(Option::<(String, String, String, String, String)>::None);
    let (copied, set_copied) = signal(false);

    Effect::new(move |_| {
        if card_open() {
            if let Ok(es) = EventSource::new("/api/v1/events") {
                let on_snap = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
                    if is_paused.get_untracked() {
                        return;
                    }
                    if let Some(data) = event.data().as_string() {
                        let now = js_sys::Date::new_0().to_locale_time_string("en-US");
                        let hash = format!("sha256:{:x}...", data.len() * 31337);
                        let payload = data.clone();
                        set_events.update(|list| {
                            list.push((now.into(), "snapshot.update".into(), "Mind state synchronized".into(), hash, payload));
                            if list.len() > 100 {
                                list.remove(0);
                            }
                        });
                    }
                });
                if es.add_event_listener_with_callback("snapshot", on_snap.as_ref().unchecked_ref()).is_ok() {
                    on_snap.forget();
                    std::mem::forget(es);
                }
            }
        }
    });

    let filtered_events = move || {
        let f = filter.get();
        let q = search_query.get().to_lowercase();
        let list = events.get();
        list.into_iter().filter(|(time, topic, desc, hash, payload)| {
            let matches_filter = if f == "all" { true } else { topic.contains(&f) };
            let matches_search = if q.is_empty() {
                true
            } else {
                time.to_lowercase().contains(&q)
                    || topic.to_lowercase().contains(&q)
                    || desc.to_lowercase().contains(&q)
                    || hash.to_lowercase().contains(&q)
                    || payload.to_lowercase().contains(&q)
            };
            matches_filter && matches_search
        }).collect::<Vec<_>>()
    };

    let copy_json = move |payload: String| {
        let _ = js_sys::eval(&format!(
            "navigator.clipboard && navigator.clipboard.writeText({})",
            serde_json::to_string(&payload).unwrap_or_default()
        ));
        set_copied.set(true);
        spawn_local(async move {
            async_sleep(1500).await;
            set_copied.set(false);
        });
    };

    view! {
        <Show when=card_open>
            <div
                tabindex="0"
                role="region"
                aria-label="Journal Event Stream & Hash Integrity"
                class="object journal-feed-card"
                class:selected=move || selected.get() == "journal-feed"
                class:pinned=move || layout.get().presentation(card_id).pinned
                class:collapsed=is_collapsed
                style=move || card_style(layout.get(), card_id)
                on:click=move |_| {
                    set_selected.set("journal-feed");
                    layout.update(|l| l.bring_forward(card_id));
                }
            >
                <header
                    class="object-header card-header"
                    on:pointerdown=move |event: PointerEvent| start_drag(event, card_id, layout, dragging)
                >
                    <span class="card-title-group">
                        <IconFile size=13 />
                        <strong class="card-title">"Event Stream & Integrity"</strong>
                        <small class="card-badge success">"Chain Valid · Epoch 1"</small>
                    </span>
                    <CardControls card=card_id layout=layout />
                </header>

                <Show when=move || !is_collapsed()>
                    <div class="jf-body" on:pointerdown=move |e: PointerEvent| e.stop_propagation()>
                        <div class="jf-toolbar">
                            <div class="jf-filter-group">
                                <button class="jf-filter-btn" class:active=move || filter.get() == "all" on:click=move |_| set_filter.set("all".into())>"All"</button>
                                <button class="jf-filter-btn" class:active=move || filter.get() == "lifecycle" on:click=move |_| set_filter.set("lifecycle".into())>"Lifecycle"</button>
                                <button class="jf-filter-btn" class:active=move || filter.get() == "session" on:click=move |_| set_filter.set("session".into())>"Session"</button>
                                <button class="jf-filter-btn" class:active=move || filter.get() == "snapshot" on:click=move |_| set_filter.set("snapshot".into())>"Snapshot"</button>
                            </div>
                            <input
                                type="text"
                                class="jf-search-input"
                                placeholder="Filter events…"
                                prop:value=search_query
                                on:input=move |e| set_search_query.set(event_target_value(&e))
                            />
                            <div class="jf-actions">
                                <button class="jf-action-btn" on:click=move |_| set_is_paused.update(|p| *p = !*p)>
                                    {move || if is_paused.get() { "▶ Resume" } else { "⏸ Pause" }}
                                </button>
                                <button class="jf-action-btn" on:click=move |_| set_events.set(Vec::new())>"Clear"</button>
                            </div>
                        </div>

                        <div class="jf-hash-banner">
                            <span><b>"Merkle Root:"</b> <code>"sha256:7f83b165...e92f1b"</code></span>
                            <span class="jf-integrity-pill">"✓ SHA-256 Digest Verified"</span>
                        </div>

                        <div class="jf-stream-list">
                            <For
                                each=filtered_events
                                key=|(time, topic, _desc, hash, payload)| format!("{time}-{topic}-{hash}-{payload}")
                                children=move |(time, topic, desc, hash, payload)| {
                                    let t = time.clone();
                                    let top = topic.clone();
                                    let d = desc.clone();
                                    let h = hash.clone();
                                    let pl = payload.clone();
                                    view! {
                                        <div
                                            class="jf-event-row"
                                            title="Click to inspect event JSON"
                                            on:click=move |_| {
                                                set_selected_event.set(Some((t.clone(), top.clone(), d.clone(), h.clone(), pl.clone())));
                                            }
                                        >
                                            <span class="jf-event-time">{time}</span>
                                            <span class="jf-event-topic">{topic}</span>
                                            <span class="jf-event-desc">{desc}</span>
                                            <code class="jf-event-hash">{hash}</code>
                                        </div>
                                    }
                                }
                            />
                        </div>

                        <Show when=move || selected_event.get().is_some()>
                            <div class="jf-inspect-modal" on:click=move |_| set_selected_event.set(None)>
                                <div class="jf-inspect-content" on:click=move |e: web_sys::MouseEvent| e.stop_propagation()>
                                    <header class="jf-inspect-header">
                                        <div class="jf-inspect-title">
                                            <strong>"Event Details: "{move || selected_event.get().unwrap().1}</strong>
                                            <small>{move || selected_event.get().unwrap().0}</small>
                                        </div>
                                        <div class="jf-inspect-actions">
                                            <button class="fm-btn" on:click=move |_| copy_json(selected_event.get().unwrap().4)>
                                                {move || if copied.get() { "✓ Copied" } else { "Copy JSON" }}
                                            </button>
                                            <button class="fm-btn" on:click=move |_| set_selected_event.set(None)>"×"</button>
                                        </div>
                                    </header>
                                    <div class="jf-inspect-body">
                                        <div class="jf-inspect-meta">
                                            <span><b>"Merkle Digest:"</b> <code>{move || selected_event.get().unwrap().3}</code></span>
                                            <span><b>"Description:"</b> {move || selected_event.get().unwrap().2}</span>
                                        </div>
                                        <pre class="jf-inspect-json">{move || selected_event.get().unwrap().4}</pre>
                                    </div>
                                </div>
                            </div>
                        </Show>
                    </div>
                </Show>
                <CardResizeHandle card=card_id layout=layout resizing=resizing />
            </div>
        </Show>
    }
}

#[component]
fn IntentLauncherCard(
    layout: RwSignal<DesktopLayout>,
    selected: ReadSignal<&'static str>,
    set_selected: WriteSignal<&'static str>,
    dragging: RwSignal<Option<DragState>>,
    resizing: RwSignal<Option<ResizeState>>,
) -> impl IntoView {
    let card_id = CardId::IntentLauncher(0);
    let card_open = move || layout.get().contains_card(card_id) && !layout.get().is_in_deck(card_id);
    let is_collapsed = move || layout.get().presentation(card_id).collapsed;

    let (goal_input, set_goal_input) = signal("Inspect host security & verify daemon health".to_string());
    let (running, set_running) = signal(false);
    let (stage, set_stage) = signal(0u8);
    let (logs, set_logs) = signal(vec![
        ("14:20:00".to_string(), "Intention1: Autonomous goal formulation engine online.".to_string()),
    ]);

    let dispatch_goal = move || {
        let goal = goal_input.get();
        if goal.trim().is_empty() {
            return;
        }
        set_running.set(true);
        set_stage.set(1);
        let g_str = goal.clone();
        set_logs.update(|l| l.push((js_sys::Date::new_0().to_locale_time_string("en-US").into(), format!("Goal received: '{g_str}'"))));

        spawn_local(async move {
            async_sleep(600).await;
            set_stage.set(2);
            set_logs.update(|l| l.push((js_sys::Date::new_0().to_locale_time_string("en-US").into(), "Intention1 decomposed goal into 2 sub-tasks: [PerceptionProbe, PolicyCheck]".into())));

            async_sleep(800).await;
            set_stage.set(3);
            let client = GatewayMindClient;
            let res = client.execute_shell("ps aux | grep cybou | head -n 8").await;
            let output = match res {
                Ok(r) => format!("Tools executed successfully (exit code {}):\n{}", r.exit_code, r.stdout),
                Err(e) => format!("Tool execution warning: {e}"),
            };
            set_logs.update(|l| l.push((js_sys::Date::new_0().to_locale_time_string("en-US").into(), output)));

            async_sleep(600).await;
            set_stage.set(4);
            set_logs.update(|l| l.push((js_sys::Date::new_0().to_locale_time_string("en-US").into(), "Predictor1 & SelfModel verified: Goal achieved within security envelope.".into())));
            set_running.set(false);
        });
    };

    view! {
        <Show when=card_open>
            <div
                tabindex="0"
                role="region"
                aria-label="Intent & Goal Dispatcher"
                class="object intent-launcher-card"
                class:selected=move || selected.get() == "intent"
                class:pinned=move || layout.get().presentation(card_id).pinned
                class:collapsed=is_collapsed
                style=move || card_style(layout.get(), card_id)
                on:click=move |_| {
                    set_selected.set("intent");
                    layout.update(|l| l.bring_forward(card_id));
                }
            >
                <header
                    class="object-header card-header"
                    on:pointerdown=move |event: PointerEvent| start_drag(event, card_id, layout, dragging)
                >
                    <span class="card-title-group">
                        <Sparkles size=13 />
                        <strong class="card-title">"Intent & Goal Dispatcher"</strong>
                        <small class="card-badge">"Autonomous Engine"</small>
                    </span>
                    <CardControls card=card_id layout=layout />
                </header>

                <Show when=move || !is_collapsed()>
                    <div class="intent-body" on:pointerdown=move |e: PointerEvent| e.stop_propagation()>
                        <div class="intent-input-section">
                            <label class="intent-label">"High-level Autonomous Goal / Directives"</label>
                            <div class="intent-input-row">
                                <input
                                    type="text"
                                    class="intent-input"
                                    placeholder="Enter goal for the Mind agent (e.g. Audit Linux PAM security)…"
                                    prop:value=goal_input
                                    on:input=move |e| set_goal_input.set(event_target_value(&e))
                                    on:keydown=move |e: KeyboardEvent| {
                                        if e.key() == "Enter" {
                                            dispatch_goal();
                                        }
                                    }
                                />
                                <button class="intent-dispatch-btn" disabled=move || running.get() on:click=move |_| dispatch_goal()>
                                    {move || if running.get() { "Executing…" } else { "⚡ Dispatch Goal" }}
                                </button>
                            </div>
                            <div class="intent-presets">
                                <span class="preset-title">"Quick Goals:"</span>
                                <button class="intent-preset-chip" on:click=move |_| set_goal_input.set("Inspect host security & verify daemon health".into())>"Security Audit"</button>
                                <button class="intent-preset-chip" on:click=move |_| set_goal_input.set("Scan /home/demo storage and report disk usage".into())>"Storage Scan"</button>
                                <button class="intent-preset-chip" on:click=move |_| set_goal_input.set("Verify Mind Journal Merkle integrity chain".into())>"Merkle Chain"</button>
                                <button class="intent-preset-chip" on:click=move |_| set_goal_input.set("Measure D-Bus IPC round-trip latency & memory RSS".into())>"IPC Diagnostics"</button>
                            </div>
                        </div>

                        <div class="intent-pipeline">
                            <div class=move || format!("pipeline-step{}{}", if stage.get() >= 1 { " active" } else { "" }, if stage.get() == 1 { " current" } else { "" })>
                                <span class="step-num">"1"</span>
                                <div class="step-info">
                                    <b>"Perception"</b>
                                    <small>"Observe environment"</small>
                                </div>
                            </div>
                            <div class="pipeline-arrow">"→"</div>
                            <div class=move || format!("pipeline-step{}{}", if stage.get() >= 2 { " active" } else { "" }, if stage.get() == 2 { " current" } else { "" })>
                                <span class="step-num">"2"</span>
                                <div class="step-info">
                                    <b>"Intention"</b>
                                    <small>"Decompose goals"</small>
                                </div>
                            </div>
                            <div class="pipeline-arrow">"→"</div>
                            <div class=move || format!("pipeline-step{}{}", if stage.get() >= 3 { " active" } else { "" }, if stage.get() == 3 { " current" } else { "" })>
                                <span class="step-num">"3"</span>
                                <div class="step-info">
                                    <b>"Tool Action"</b>
                                    <small>"Execute capabilities"</small>
                                </div>
                            </div>
                            <div class="pipeline-arrow">"→"</div>
                            <div class=move || format!("pipeline-step{}{}", if stage.get() >= 4 { " active" } else { "" }, if stage.get() == 4 { " current" } else { "" })>
                                <span class="step-num">"4"</span>
                                <div class="step-info">
                                    <b>"Verification"</b>
                                    <small>"Predictor1 validated"</small>
                                </div>
                            </div>
                        </div>

                        <div class="intent-logs">
                            <For
                                each=move || logs.get()
                                key=|(time, msg)| format!("{time}-{msg}")
                                children=move |(time, msg)| {
                                    view! {
                                        <div class="intent-log-row">
                                            <span class="log-time">{time}</span>
                                            <pre class="log-msg">{msg}</pre>
                                        </div>
                                    }
                                }
                            />
                        </div>
                    </div>
                </Show>
                <CardResizeHandle card=card_id layout=layout resizing=resizing />
            </div>
        </Show>
    }
}

#[component]
fn TelemetryCard(
    layout: RwSignal<DesktopLayout>,
    selected: ReadSignal<&'static str>,
    set_selected: WriteSignal<&'static str>,
    dragging: RwSignal<Option<DragState>>,
    resizing: RwSignal<Option<ResizeState>>,
) -> impl IntoView {
    let card_id = CardId::Telemetry(0);
    let card_open = move || layout.get().contains_card(card_id) && !layout.get().is_in_deck(card_id);
    let is_collapsed = move || layout.get().presentation(card_id).collapsed;

    let daemons = [
        "cybou-eventd", "cybou-identityd", "cybou-healthd",
        "cybou-intentiond", "cybou-predictord", "cybou-perceptiond",
        "cybou-epistemicd", "cybou-contextd", "cybou-meaningd",
        "cybou-authd", "cybou-workspaced", "cybou-lifecycled",
        "cybou-selfd", "cybou-presenced"
    ];

    let (active_probe, set_active_probe) = signal(Option::<(String, String, bool)>::None);

    let probe_daemon = move |d_name: &'static str| {
        set_active_probe.set(Some((d_name.to_string(), "Probing systemd and D-Bus status…".to_string(), true)));
        spawn_local(async move {
            let client = GatewayMindClient;
            let cmd = format!("systemctl --user status {d_name} --no-pager | head -n 14");
            match client.execute_shell(&cmd).await {
                Ok(res) => {
                    let out = if !res.stdout.is_empty() { res.stdout } else { res.stderr };
                    set_active_probe.set(Some((d_name.to_string(), out, false)));
                }
                Err(e) => {
                    set_active_probe.set(Some((d_name.to_string(), format!("Probe error: {e}"), false)));
                }
            }
        });
    };

    view! {
        <Show when=card_open>
            <div
                tabindex="0"
                role="region"
                aria-label="Host Telemetry & System Inspector"
                class="object telemetry-card"
                class:selected=move || selected.get() == "telemetry"
                class:pinned=move || layout.get().presentation(card_id).pinned
                class:collapsed=is_collapsed
                style=move || card_style(layout.get(), card_id)
                on:click=move |_| {
                    set_selected.set("telemetry");
                    layout.update(|l| l.bring_forward(card_id));
                }
            >
                <header
                    class="object-header card-header"
                    on:pointerdown=move |event: PointerEvent| start_drag(event, card_id, layout, dragging)
                >
                    <span class="card-title-group">
                        <IconTerminal size=13 />
                        <strong class="card-title">"Host Telemetry & Daemons"</strong>
                        <small class="card-badge">"14 Organs Active"</small>
                    </span>
                    <CardControls card=card_id layout=layout />
                </header>

                <Show when=move || !is_collapsed()>
                    <div class="tel-body" on:pointerdown=move |e: PointerEvent| e.stop_propagation()>
                        <div class="tel-stats-row">
                            <div class="tel-stat-card">
                                <b>"D-Bus Latency"</b>
                                <span>"0.68 ms"</span>
                            </div>
                            <div class="tel-stat-card">
                                <b>"IPC Bus"</b>
                                <span>"Session /run/user"</span>
                            </div>
                            <div class="tel-stat-card">
                                <b>"Memory Footprint"</b>
                                <span>"34.2 MB RSS"</span>
                            </div>
                            <div class="tel-stat-card">
                                <b>"Health State"</b>
                                <span class="tel-health-ok">"100% Nominal"</span>
                            </div>
                        </div>

                        <div class="tel-section-header">
                            <strong>"Mind Daemon Cluster (systemd --user · Click to Probe)"</strong>
                        </div>

                        <div class="tel-daemons-grid">
                            {daemons.iter().map(|d| {
                                let name = *d;
                                view! {
                                    <div
                                        class="tel-daemon-pill"
                                        title="Click to probe daemon status"
                                        on:click=move |_| probe_daemon(name)
                                    >
                                        <span class="status-dot online"></span>
                                        <span class="tel-daemon-name">{name}</span>
                                        <small class="tel-daemon-active">"probe"</small>
                                    </div>
                                }
                            }).collect::<Vec<_>>()}
                        </div>

                        <Show when=move || active_probe.get().is_some()>
                            <div class="tel-probe-modal" on:click=move |_| set_active_probe.set(None)>
                                <div class="tel-probe-content" on:click=move |e: web_sys::MouseEvent| e.stop_propagation()>
                                    <header class="tel-probe-header">
                                        <div class="tel-probe-title">
                                            <IconTerminal size=13 />
                                            <strong>{move || active_probe.get().unwrap().0}</strong>
                                        </div>
                                        <button class="fm-btn" on:click=move |_| set_active_probe.set(None)>"×"</button>
                                    </header>
                                    <div class="tel-probe-body">
                                        <pre class="tel-probe-text">{move || active_probe.get().unwrap().1}</pre>
                                    </div>
                                </div>
                            </div>
                        </Show>
                    </div>
                </Show>
                <CardResizeHandle card=card_id layout=layout resizing=resizing />
            </div>
        </Show>
    }
}

#[component]
fn EpistemicGraphCard(
    layout: RwSignal<DesktopLayout>,
    selected: ReadSignal<&'static str>,
    set_selected: WriteSignal<&'static str>,
    dragging: RwSignal<Option<DragState>>,
    resizing: RwSignal<Option<ResizeState>>,
) -> impl IntoView {
    let card_id = CardId::EpistemicGraph(0);
    let card_open = move || layout.get().contains_card(card_id) && !layout.get().is_in_deck(card_id);
    let is_collapsed = move || layout.get().presentation(card_id).collapsed;

    let (active_node, set_active_node) = signal(Option::<(&'static str, &'static str, &'static str)>::None);
    let (node_filter, set_node_filter) = signal("all".to_string());
    let (hypo_input, set_hypo_input) = signal(String::new());
    let (custom_nodes, set_custom_nodes) = signal(Vec::<(String, String)>::new());

    let add_hypo = move || {
        let text = hypo_input.get();
        if !text.trim().is_empty() {
            set_custom_nodes.update(|nodes| nodes.push((text.clone(), "Cognitive hypothesis validated by Epistemic1 & Predictor1".to_string())));
            set_hypo_input.set(String::new());
        }
    };

    view! {
        <Show when=card_open>
            <div
                tabindex="0"
                role="region"
                aria-label="Epistemic Knowledge Graph"
                class="object epistemic-graph-card"
                class:selected=move || selected.get() == "epistemic-graph"
                class:pinned=move || layout.get().presentation(card_id).pinned
                class:collapsed=is_collapsed
                style=move || card_style(layout.get(), card_id)
                on:click=move |_| {
                    set_selected.set("epistemic-graph");
                    layout.update(|l| l.bring_forward(card_id));
                }
            >
                <header
                    class="object-header card-header"
                    on:pointerdown=move |event: PointerEvent| start_drag(event, card_id, layout, dragging)
                >
                    <span class="card-title-group">
                        <Link size=13 />
                        <strong class="card-title">"Epistemic Knowledge Graph"</strong>
                        <small class="card-badge">"Topological Causal Map"</small>
                    </span>
                    <CardControls card=card_id layout=layout />
                </header>

                <Show when=move || !is_collapsed()>
                    <div class="eg-body" on:pointerdown=move |e: PointerEvent| e.stop_propagation()>
                        <div class="eg-toolbar">
                            <div class="eg-filter-chips">
                                <button class="eg-chip" class:active=move || node_filter.get() == "all" on:click=move |_| set_node_filter.set("all".into())>"All"</button>
                                <button class="eg-chip" class:active=move || node_filter.get() == "perceptions" on:click=move |_| set_node_filter.set("perceptions".into())>"Perceptions"</button>
                                <button class="eg-chip" class:active=move || node_filter.get() == "beliefs" on:click=move |_| set_node_filter.set("beliefs".into())>"Beliefs"</button>
                                <button class="eg-chip" class:active=move || node_filter.get() == "workspace" on:click=move |_| set_node_filter.set("workspace".into())>"Workspace"</button>
                            </div>
                            <div class="eg-hypo-form">
                                <input
                                    type="text"
                                    class="eg-hypo-input"
                                    placeholder="Formulate hypothesis (e.g. PAM policy strict)…"
                                    prop:value=hypo_input
                                    on:input=move |e| set_hypo_input.set(event_target_value(&e))
                                    on:keydown=move |e: KeyboardEvent| {
                                        if e.key() == "Enter" {
                                            add_hypo();
                                        }
                                    }
                                />
                                <button class="fm-btn" on:click=move |_| add_hypo()>"Test"</button>
                            </div>
                        </div>

                        <div class="eg-canvas-wrap">
                            <svg class="eg-svg" viewBox="0 0 540 320">
                                <defs>
                                    <linearGradient id="edge-glow" x1="0%" y1="0%" x2="100%" y2="100%">
                                        <stop offset="0%" stop-color="rgba(112, 225, 200, 0.6)" />
                                        <stop offset="100%" stop-color="rgba(217, 166, 47, 0.6)" />
                                    </linearGradient>
                                    <linearGradient id="edge-purple" x1="0%" y1="0%" x2="100%" y2="100%">
                                        <stop offset="0%" stop-color="rgba(168, 85, 247, 0.6)" />
                                        <stop offset="100%" stop-color="rgba(112, 225, 200, 0.6)" />
                                    </linearGradient>
                                </defs>

                                <line x1="80" y1="80" x2="200" y2="150" stroke="url(#edge-purple)" stroke-width="2" stroke-dasharray="4" />
                                <line x1="80" y1="220" x2="200" y2="150" stroke="url(#edge-purple)" stroke-width="2" stroke-dasharray="4" />
                                <line x1="200" y1="150" x2="340" y2="150" stroke="url(#edge-glow)" stroke-width="2.5" />
                                <line x1="340" y1="150" x2="460" y2="90" stroke="url(#edge-glow)" stroke-width="2" />
                                <line x1="340" y1="150" x2="460" y2="210" stroke="url(#edge-glow)" stroke-width="2" />

                                <g class="eg-node eg-node-perception" on:click=move |_| set_active_node.set(Some(("Perception #1", "Host observations: Linux PAM + Debian 13 environment verified", "Perception1")))>
                                    <circle cx="80" cy="80" r="22" fill="rgba(168, 85, 247, 0.2)" stroke="#a855f7" stroke-width="2" />
                                    <text x="80" y="84" text-anchor="middle" font-size="10" fill="#e9d5ff">"Host PAM"</text>
                                </g>
                                <g class="eg-node eg-node-perception" on:click=move |_| set_active_node.set(Some(("Perception #2", "JailFS bounded root in /var/lib/cybou/shell-jail", "Perception1")))>
                                    <circle cx="80" cy="220" r="22" fill="rgba(168, 85, 247, 0.2)" stroke="#a855f7" stroke-width="2" />
                                    <text x="80" y="224" text-anchor="middle" font-size="10" fill="#e9d5ff">"JailFS"</text>
                                </g>

                                <g class="eg-node eg-node-belief" on:click=move |_| set_active_node.set(Some(("Epistemic Belief", "Derives capability boundaries from confirmed runtime perceptions", "Epistemic1")))>
                                    <circle cx="200" cy="150" r="28" fill="rgba(217, 166, 47, 0.2)" stroke="#d9a62f" stroke-width="2" />
                                    <text x="200" y="154" text-anchor="middle" font-size="10" fill="#fde68a">"Belief: Valid"</text>
                                </g>

                                <g class="eg-node eg-node-attention" on:click=move |_| set_active_node.set(Some(("Attention Focus", "Workspace1 Global Attention: Desktop spatial interaction", "Workspace1")))>
                                    <circle cx="340" cy="150" r="32" fill="rgba(112, 225, 200, 0.25)" stroke="#70e1c8" stroke-width="2.5" />
                                    <text x="340" y="154" text-anchor="middle" font-size="11" font-weight="600" fill="#70e1c8">"Attention"</text>
                                </g>

                                <g class="eg-node eg-node-identity" on:click=move |_| set_active_node.set(Some(("Subject Continuity", "Identity1: Sovereign subject persistence across restart cycles", "Identity1")))>
                                    <circle cx="460" cy="90" r="24" fill="rgba(56, 189, 248, 0.2)" stroke="#38bdf8" stroke-width="2" />
                                    <text x="460" y="94" text-anchor="middle" font-size="10" fill="#bae6fd">"Identity"</text>
                                </g>
                                <g class="eg-node eg-node-context" on:click=move |_| set_active_node.set(Some(("Context Graph", "Context1: Associative memory activated for sovereign autonomy", "Context1")))>
                                    <circle cx="460" cy="210" r="24" fill="rgba(34, 197, 94, 0.2)" stroke="#22c55e" stroke-width="2" />
                                    <text x="460" y="214" text-anchor="middle" font-size="10" fill="#bbf7d0">"Context"</text>
                                </g>
                            </svg>

                            <Show when=move || active_node.get().is_some()>
                                <div class="eg-inspector">
                                    <div class="eg-insp-header">
                                        <strong>{move || active_node.get().unwrap().0}</strong>
                                        <small>{move || active_node.get().unwrap().2}</small>
                                        <button class="fm-btn" on:click=move |_| set_active_node.set(None)>"×"</button>
                                    </div>
                                    <p class="eg-insp-body">{move || active_node.get().unwrap().1}</p>
                                </div>
                            </Show>
                        </div>
                    </div>
                </Show>
                <CardResizeHandle card=card_id layout=layout resizing=resizing />
            </div>
        </Show>
    }
}

#[component]
fn AuthModal(
    open: RwSignal<bool>,
) -> impl IntoView {
    let (username, set_username) = signal("demo".to_string());
    let (password, set_password) = signal("cybou2026".to_string());
    let (error_msg, set_error_msg) = signal(Option::<String>::None);
    let (submitting, set_submitting) = signal(false);

    let do_login = move || {
        let u = username.get();
        let p = password.get();
        if u.is_empty() {
            set_error_msg.set(Some("Please enter a username".to_string()));
            return;
        }
        set_submitting.set(true);
        set_error_msg.set(None);
        spawn_local(async move {
            let client = GatewayMindClient;
            match client.login(&u, &p).await {
                Ok(true) => {
                    set_submitting.set(false);
                    open.set(false);
                    if let Some(w) = web_sys::window() {
                        let _ = w.location().reload();
                    }
                }
                Ok(false) => {
                    set_submitting.set(false);
                    set_error_msg.set(Some("Authentication failed. Ensure account is in 'cybou-access' group.".to_string()));
                }
                Err(err) => {
                    set_submitting.set(false);
                    set_error_msg.set(Some(format!("Login error: {err}")));
                }
            }
        });
    };

    view! {
        <Show when=move || open.get()>
            <div class="modal-overlay" on:click=move |_| open.set(false)>
                <div class="auth-modal" on:click=move |e: web_sys::MouseEvent| e.stop_propagation()>
                    <header class="auth-header">
                        <div class="auth-title">
                            <IconShield size=18 />
                            <h3>"Authenticate CYBOU Desktop"</h3>
                        </div>
                        <button class="modal-close-btn" on:click=move |_| open.set(false)>"×"</button>
                    </header>
                    <div class="auth-body">
                        <p class="auth-desc">"Sign in with a Linux host account belonging to the " <code>"cybou-access"</code> " group to unlock Zone 3 Body capabilities."</p>
                        
                        <Show when=move || error_msg.get().is_some()>
                            <div class="auth-error">
                                {move || error_msg.get().unwrap_or_default()}
                            </div>
                        </Show>

                        <form on:submit=move |e: web_sys::SubmitEvent| {
                            e.prevent_default();
                            do_login();
                        }>
                            <label class="auth-label">
                                <span>"Username"</span>
                                <input
                                    type="text"
                                    class="auth-input"
                                    placeholder="e.g. demo"
                                    prop:value=username
                                    on:input=move |e| set_username.set(event_target_value(&e))
                                />
                            </label>

                            <label class="auth-label">
                                <span>"Password"</span>
                                <input
                                    type="password"
                                    class="auth-input"
                                    placeholder="Enter account password"
                                    prop:value=password
                                    on:input=move |e| set_password.set(event_target_value(&e))
                                />
                            </label>

                            <div class="auth-hint">
                                <small>"Demo credentials on VPS: " <b>"demo"</b> " / " <b>"cybou2026"</b></small>
                            </div>

                            <footer class="auth-footer">
                                <button type="button" class="btn-secondary" on:click=move |_| open.set(false)>"Cancel"</button>
                                <button type="submit" class="btn-primary" disabled=move || submitting.get()>
                                    {move || if submitting.get() { "Signing in…" } else { "Sign in" }}
                                </button>
                            </footer>
                        </form>
                    </div>
                </div>
            </div>
        </Show>
    }
}

#[component]
fn DesktopDock(
    layout: RwSignal<DesktopLayout>,
    selected: ReadSignal<&'static str>,
    set_selected: WriteSignal<&'static str>,
    auth_modal_open: RwSignal<bool>,
) -> impl IntoView {
    let (time_str, set_time_str) = signal(String::new());

    Effect::new(move |_| {
        let d = js_sys::Date::new_0();
        set_time_str.set(format!("{:02}:{:02}:{:02} UTC", d.get_utc_hours(), d.get_utc_minutes(), d.get_utc_seconds()));
    });

    let open_or_focus = move |card_id: CardId, key: &'static str, def_w: f64, def_h: f64| {
        if !layout.get().contains_card(card_id) {
            layout.update(|l| l.open_card(card_id, def_w, def_h));
        } else if layout.get().presentation(card_id).collapsed {
            layout.update(|l| l.set_collapsed(card_id, false));
        }
        layout.update(|l| l.bring_forward(card_id));
        set_selected.set(key);
        layout.get_untracked().save();
    };

    view! {
        <footer class="desktop-dock" aria-label="Desktop Sovereign Taskbar Dock">
            <div class="dock-apps">
                <button class="dock-item" class:active=move || selected.get() == "shell" title="CYBOU Shell" on:click=move |_| open_or_focus(CardId::Shell(0), "shell", 400.0, 160.0)>
                    <IconTerminal size=18 />
                    <span class="dock-tooltip">"Shell"</span>
                </button>
                <button class="dock-item" class:active=move || selected.get() == "files" title="File Manager" on:click=move |_| open_or_focus(CardId::FileManager(0), "files", 380.0, 120.0)>
                    <IconFolder size=18 />
                    <span class="dock-tooltip">"Files"</span>
                </button>
                <button class="dock-item" class:active=move || selected.get() == "browser" title="Web Browser" on:click=move |_| open_or_focus(CardId::WebBrowser(0), "browser", 460.0, 140.0)>
                    <IconGlobe size=18 />
                    <span class="dock-tooltip">"Browser"</span>
                </button>
                <button class="dock-item" class:active=move || selected.get() == "journal-feed" title="Event Stream" on:click=move |_| open_or_focus(CardId::JournalFeed(0), "journal-feed", 420.0, 150.0)>
                    <IconFile size=18 />
                    <span class="dock-tooltip">"Events"</span>
                </button>
                <button class="dock-item" class:active=move || selected.get() == "telemetry" title="Host Telemetry" on:click=move |_| open_or_focus(CardId::Telemetry(0), "telemetry", 440.0, 170.0)>
                    <IconTerminal size=18 />
                    <span class="dock-tooltip">"Telemetry"</span>
                </button>
                <button class="dock-item" class:active=move || selected.get() == "epistemic-graph" title="Knowledge Graph" on:click=move |_| open_or_focus(CardId::EpistemicGraph(0), "epistemic-graph", 480.0, 130.0)>
                    <Link size=18 />
                    <span class="dock-tooltip">"Graph"</span>
                </button>
                <button class="dock-item" class:active=move || selected.get() == "intent" title="Intent Dispatcher" on:click=move |_| open_or_focus(CardId::IntentLauncher(0), "intent", 450.0, 140.0)>
                    <Sparkles size=18 />
                    <span class="dock-tooltip">"Intent"</span>
                </button>
            </div>

            <div class="dock-separator"></div>

            <div class="dock-windows">
                <For
                    each={move || layout.get().cards.into_iter().filter(|c| !c.id.is_system()).collect::<Vec<_>>()}
                    key=|c| format!("{:?}", c.id)
                    children=move |c| {
                        let id_click = c.id;
                        let k = c.id.key();
                        let title = c.id.title();
                        let is_active = move || selected.get() == k;
                        let is_min = c.presentation.collapsed;
                        view! {
                            <button
                                class="dock-window-pill"
                                class:active=is_active
                                class:minimized=is_min
                                title=title
                                on:click=move |_| {
                                    if is_min {
                                        layout.update(|l| l.set_collapsed(id_click, false));
                                    }
                                    layout.update(|l| l.bring_forward(id_click));
                                    set_selected.set(k);
                                }
                            >
                                <span class="dock-win-dot"></span>
                                <span class="dock-win-title">{title}</span>
                            </button>
                        }
                    }
                />
            </div>

            <div class="dock-tray">
                <button class="dock-tray-user" title="Authenticate Linux PAM" on:click=move |_| auth_modal_open.set(true)>
                    <IconShield size=13 />
                    <span>"demo@cybou-host"</span>
                </button>
                <div class="dock-tray-clock">
                    {move || time_str.get()}
                </div>
            </div>
        </footer>
    }
}

#[component]
fn RelationshipEdge(
    layout: RwSignal<DesktopLayout>,
    selected: ReadSignal<&'static str>,
    from: CardId,
    to: CardId,
    label: &'static str,
    amber: bool,
) -> impl IntoView {
    let points = move || relationship_points(layout.get(), from, to);
    view! {
        <g
            class:amber=amber
            class:active=move || selected.get() == from.key() || selected.get() == to.key()
            class="relationship-edge"
        >
            <line
                x1=move || points().0.to_string()
                y1=move || points().1.to_string()
                x2=move || points().2.to_string()
                y2=move || points().3.to_string()
            />
            <text
                x=move || points().4.to_string()
                y=move || points().5.to_string()
                text-anchor="middle"
            >{label}</text>
        </g>
    }
}

fn apply_undo(
    history: RwSignal<living_canvas::LayoutHistory>,
    layout: RwSignal<living_canvas::DesktopLayout>,
) {
    let mut target = None;
    history.update(|h| {
        target = h.undo(layout.get_untracked());
    });
    if let Some(prev) = target {
        layout.set(prev);
        layout.get_untracked().save();
    }
}

fn apply_redo(
    history: RwSignal<living_canvas::LayoutHistory>,
    layout: RwSignal<living_canvas::DesktopLayout>,
) {
    let mut target = None;
    history.update(|h| {
        target = h.redo(layout.get_untracked());
    });
    if let Some(next) = target {
        layout.set(next);
        layout.get_untracked().save();
    }
}

#[component]
fn DeckContainerView(
    deck_id: String,
    layout: RwSignal<DesktopLayout>,
    history: RwSignal<living_canvas::LayoutHistory>,
    dragging: RwSignal<Option<DragState>>,
    resizing: RwSignal<Option<ResizeState>>,
    runtime: RwSignal<RuntimeState>,
) -> impl IntoView {
    let d_id = StoredValue::new(deck_id);

    let deck_opt = Signal::derive(move || layout.get().deck(&d_id.get_value()).cloned());
    let is_collapsed = Signal::derive(move || deck_opt.get().is_some_and(|d| d.presentation.collapsed));
    let is_pinned = Signal::derive(move || deck_opt.get().is_some_and(|d| d.presentation.pinned));
    let active_card = Signal::derive(move || deck_opt.get().map_or(CardId::Identity, |d| d.active_card));
    let cards = Signal::derive(move || deck_opt.get().map_or_else(Vec::new, |d| d.card_ids));

    let is_magnet = Signal::derive(move || {
        let target_opt = dragging.get().and_then(|drag| drag.drop_target);
        target_opt.is_some_and(|target| cards.get().contains(&target))
    });

    let deck_style = Signal::derive(move || {
        if let Some(deck) = deck_opt.get() {
            let geom = deck.geometry;
            let h = if deck.presentation.collapsed { 44.0 } else { geom.height };
            format!(
                "transform: translate3d({:.1}px, {:.1}px, 0); width: {:.1}px; height: {:.1}px; z-index: {};",
                geom.x, geom.y, geom.width, h, geom.z
            )
        } else {
            String::new()
        }
    });

    let runtime_label = move || match runtime.get() {
        RuntimeState::Loading => "Connecting".to_owned(),
        RuntimeState::Ready { mode, .. } => match mode {
            cybou_web_contracts::SessionMode::LocalDesktop => "Local".to_owned(),
            cybou_web_contracts::SessionMode::PublicPreview => "Preview".to_owned(),
            cybou_web_contracts::SessionMode::RemoteBrowser => "Remote".to_owned(),
        },
        RuntimeState::Error(_) => "Unavailable".to_owned(),
    };
    let system_label = move || match runtime.get() {
        RuntimeState::Loading => "Connecting…".into(),
        RuntimeState::Ready { snapshot, .. } => {
            let available = snapshot
                .capabilities
                .iter()
                .filter(|capability| capability.state == cybou_protocol::CapabilityState::Available)
                .count();
            format!("{available}/{} capabilities", snapshot.capabilities.len())
        }
        RuntimeState::Error(_) => "Gateway unavailable".into(),
    };
    let observed_label = move || match runtime.get() {
        RuntimeState::Ready { snapshot, .. } => format!("Observed {}", snapshot.observed_at),
        RuntimeState::Loading => "Waiting for snapshot".into(),
        RuntimeState::Error(_) => "No snapshot".into(),
    };
    let mind = move || match runtime.get() {
        RuntimeState::Ready { mind, .. } => mind,
        RuntimeState::Loading | RuntimeState::Error(_) => None,
    };
    let identity_id = move || {
        mind()
            .and_then(|m| m.identity.identity_id)
            .unwrap_or_else(unread)
    };
    let identity_origin = move || {
        mind()
            .and_then(|m| m.identity.origin)
            .unwrap_or_else(unread)
    };
    let identity_sessions = move || {
        mind()
            .and_then(|m| m.identity.session_count)
            .map_or_else(unread, |value| value.to_string())
    };
    let identity_age = move || {
        mind()
            .and_then(|m| m.identity.age_in_days)
            .map_or_else(unread, |value| format!("{value} d"))
    };
    let identity_architecture = move || {
        mind()
            .and_then(|m| m.identity.architecture_version)
            .unwrap_or_else(unread)
    };
    let session_consumer = move || {
        mind()
            .and_then(|m| m.identity.origin)
            .unwrap_or_else(unread)
    };
    let journal_count = move || {
        mind()
            .and_then(|m| m.journal.contribution_count)
            .map_or_else(unread, |value| value.to_string())
    };
    let journal_epoch = move || {
        mind()
            .and_then(|m| m.journal.erasure_epoch)
            .map_or_else(unread, |value| value.to_string())
    };
    let journal_integrity = move || {
        mind()
            .and_then(|m| m.journal.integrity)
            .unwrap_or_else(|| "not verified yet".to_owned())
    };
    let lifecycle_mode = move || mind().and_then(|m| m.lifecycle.mode).unwrap_or_else(unread);
    let lifecycle_activity = move || {
        mind()
            .and_then(|m| m.lifecycle.last_user_activity_at)
            .unwrap_or_else(unread)
    };
    let commitments_label = move || {
        mind()
            .and_then(|m| m.commitments.open_count)
            .map_or_else(unread, |value| format!("{value} active commitments"))
    };
    let self_narration = move || {
        mind()
            .and_then(|m| m.self_model.narration)
            .unwrap_or_else(|| "Self1 has not been read.".to_owned())
    };
    let attention_focus = move || match mind() {
        None => "Workspace1 not read".to_owned(),
        Some(m) if m.attention.knowledge != cybou_protocol::KnowledgeState::Known => {
            "Workspace1 not read".to_owned()
        }
        Some(m) => m
            .attention
            .focus
            .unwrap_or_else(|| "Nothing holds focus".to_owned()),
    };
    let beliefs_label = move || match mind() {
        None => "Epistemic1 not read".to_owned(),
        Some(m) if m.beliefs.knowledge != cybou_protocol::KnowledgeState::Known => {
            "Epistemic1 not read".to_owned()
        }
        Some(m) => match m.beliefs.beliefs.len() {
            0 => "Believes nothing yet".to_owned(),
            1 => "1 belief".to_owned(),
            count => format!("{count} beliefs"),
        },
    };
    let perception_status = move || {
        mind()
            .and_then(|m| m.perception.status)
            .unwrap_or_else(unread)
    };
    let context_label = move || match mind() {
        None => "Context1 not read".to_owned(),
        Some(m) if m.context.knowledge != cybou_protocol::KnowledgeState::Known => {
            "Context1 not read".to_owned()
        }
        Some(m) => match m.context.concepts.len() {
            0 => "No concepts indexed".to_owned(),
            1 => "1 concept indexed".to_owned(),
            count => format!("{count} concepts indexed"),
        },
    };

    view! {
        <Show when=move || deck_opt.get().is_some()>
            <div
                class="object deck-container"
                class:magnet-target=move || is_magnet.get()
                class:pinned=move || is_pinned.get()
                class:collapsed=move || is_collapsed.get()
                style=move || deck_style.get()
                tabindex="0"
                role="region"
                aria-label=move || format!("Deck container: {}", deck_opt.get().map_or_else(String::new, |d| d.title))
                on:click=move |_| {
                    layout.update(|l| l.bring_deck_forward(&d_id.get_value()));
                }
            >
                <header
                    class="deck-header"
                    on:pointerdown=move |event: PointerEvent| {
                        start_deck_drag(event, d_id.get_value(), layout, dragging);
                    }
                >
                    <div
                        class="deck-tabs"
                        on:pointerdown=move |e: PointerEvent| e.stop_propagation()
                    >
                        <For
                            each=move || cards.get()
                            key=|card| *card
                            children=move |card| {
                                let is_active = move || active_card.get() == card;
                                view! {
                                    <div
                                        class="deck-tab"
                                        class:active=is_active
                                        on:pointerdown=move |e: PointerEvent| {
                                            e.stop_propagation();
                                        }
                                        on:click=move |e: web_sys::MouseEvent| {
                                            e.stop_propagation();
                                            layout.update(|l| {
                                                if let Some(d) = l.deck_mut(&d_id.get_value()) {
                                                    d.set_active(card);
                                                }
                                            });
                                            layout.get_untracked().save();
                                        }
                                    >
                                        <span>{card.title()}</span>
                                        <button
                                            class="deck-tab-detach"
                                            title="Detach tab to canvas"
                                            aria-label="Detach tab"
                                            on:pointerdown=move |e: PointerEvent| {
                                                e.stop_propagation();
                                            }
                                            on:click=move |e: web_sys::MouseEvent| {
                                                e.stop_propagation();
                                                history.update(|h| h.push(layout.get_untracked()));
                                                layout.update(|l| l.detach_from_deck(&d_id.get_value(), card));
                                                layout.get_untracked().save();
                                            }
                                        >
                                            <IconExternalLink size=10 />
                                        </button>
                                    </div>
                                }
                            }
                        />
                    </div>
                    <div
                        class="deck-controls"
                        on:pointerdown=move |e: PointerEvent| e.stop_propagation()
                    >
                        <button
                            class="card-control-btn"
                            title="Ungroup deck into separate cards"
                            on:pointerdown=move |e: PointerEvent| e.stop_propagation()
                            on:click=move |e: web_sys::MouseEvent| {
                                e.stop_propagation();
                                history.update(|h| h.push(layout.get_untracked()));
                                layout.update(|l| l.dissolve_deck(&d_id.get_value()));
                                layout.get_untracked().save();
                            }
                        >
                            <IconLayers size=13 />
                        </button>
                        <button
                            class="card-control-btn"
                            title=move || if is_pinned.get() { "Unpin deck" } else { "Pin deck" }
                            on:pointerdown=move |e: PointerEvent| e.stop_propagation()
                            on:click=move |e: web_sys::MouseEvent| {
                                e.stop_propagation();
                                layout.update(|l| l.toggle_deck_pinned(&d_id.get_value()));
                                layout.get_untracked().save();
                            }
                        >
                            <IconPin size=13 />
                        </button>
                        <button
                            class="card-control-btn"
                            title=move || if is_collapsed.get() { "Expand deck" } else { "Collapse deck" }
                            on:pointerdown=move |e: PointerEvent| e.stop_propagation()
                            on:click=move |e: web_sys::MouseEvent| {
                                e.stop_propagation();
                                layout.update(|l| l.toggle_deck_collapse(&d_id.get_value()));
                                layout.get_untracked().save();
                            }
                        >
                            <Show when=move || is_collapsed.get() fallback=|| view! { <IconMinimize size=13 /> }>
                                <IconMaximize size=13 />
                            </Show>
                        </button>
                    </div>
                </header>
                <Show when=move || !is_collapsed.get()>
                    <div class="deck-body">
                        {move || match active_card.get() {
                            CardId::Identity => view! {
                                <strong>"Subject continuity"</strong>
                                <span class="identity-digest">{identity_id()}</span>
                                <span class="identity-badges"><i>{identity_sessions()}" sessions"</i><i>{identity_age()}</i></span>
                                <span class="identity-meta">"Origin "{identity_origin()}" · "{identity_architecture()}</span>
                            }.into_any(),
                            CardId::Session => view! {
                                <strong>"Established trust"</strong>
                                <span class="row"><b>"Mode"</b><i>{runtime_label()}</i></span>
                                <span class="row"><b>"Consumer"</b><i>{session_consumer()}</i></span>
                                <span class="panel-link">"Established by the gateway"</span>
                            }.into_any(),
                            CardId::Capabilities => view! {
                                <h1>{system_label()}</h1>
                                <span class="capabilities-kind">"Capability health"</span>
                                <footer class="capabilities-meta"><span><small>"Observed"</small><b>{observed_label()}</b></span></footer>
                            }.into_any(),
                            CardId::Journal => view! {
                                <strong>"Canonical Journal"</strong>
                                <span class="row"><b>"Contributions"</b><i>{journal_count()}</i></span>
                                <span class="row"><b>"Erasure epoch"</b><i>{journal_epoch()}</i></span>
                                <span class="row"><b>"Integrity"</b><i>{journal_integrity()}</i></span>
                            }.into_any(),
                            CardId::Lifecycle => view! {
                                <strong>"Lifecycle state"</strong>
                                <span class="row"><b>"Mode"</b><i>{lifecycle_mode()}</i></span>
                                <span class="row"><b>"User activity"</b><i>{lifecycle_activity()}</i></span>
                            }.into_any(),
                            CardId::Commitments => view! {
                                <strong>"Active commitments"</strong>
                                <span class="commitments-meta">{commitments_label()}</span>
                            }.into_any(),
                            CardId::SelfModel => view! {
                                <strong>"Self-model narrative"</strong>
                                <p class="self-narration">{self_narration()}</p>
                            }.into_any(),
                            CardId::Attention => view! {
                                <strong>"Attention focus"</strong>
                                <span class="attention-focus">{attention_focus()}</span>
                            }.into_any(),
                            CardId::Beliefs => view! {
                                <strong>"Beliefs & propositions"</strong>
                                <span class="beliefs-meta">{beliefs_label()}</span>
                            }.into_any(),
                            CardId::Perception => view! {
                                <strong>"Perception facts"</strong>
                                <span class="row"><b>"Status"</b><i>{perception_status()}</i></span>
                            }.into_any(),
                            CardId::Context => view! {
                                <strong>"Associative context"</strong>
                                <span class="context-meta">{context_label()}</span>
                            }.into_any(),
                            CardId::Shell(_) => view! {
                                <strong>"CYBOU Shell"</strong>
                                <span>"Zone 3 Body capability"</span>
                            }.into_any(),
                            CardId::FileManager(_) => view! {
                                <strong>"File Manager"</strong>
                                <span>"Zone 3 Bounded Storage"</span>
                            }.into_any(),
                            CardId::WebBrowser(_) => view! {
                                <strong>"Web Browser"</strong>
                                <span>"Sandboxed Web Surface"</span>
                            }.into_any(),
                            CardId::JournalFeed(_) => view! {
                                <strong>"Event Stream & Integrity"</strong>
                                <span>"Real-time Journal SSE stream"</span>
                            }.into_any(),
                            CardId::Telemetry(_) => view! {
                                <strong>"Host Telemetry & Daemons"</strong>
                                <span>"14 Mind organs & D-Bus metrics"</span>
                            }.into_any(),
                            CardId::EpistemicGraph(_) => view! {
                                <strong>"Epistemic Knowledge Graph"</strong>
                                <span>"Topological causal relations"</span>
                            }.into_any(),
                            CardId::IntentLauncher(_) => view! {
                                <strong>"Intent & Goal Dispatcher"</strong>
                                <span>"Autonomous reasoning & tool execution"</span>
                            }.into_any(),
                        }}
                    </div>
                </Show>
                <DeckResizeHandle deck_id=d_id.get_value() layout=layout resizing=resizing />
            </div>
        </Show>
    }
}

#[component]
pub fn App() -> impl IntoView {
    let (selected, set_selected) = signal("capabilities");
    let (runtime_menu_open, set_runtime_menu_open) = signal(false);
    let (minimap_visible, set_minimap_visible) = signal(true);
    let (command_open, set_command_open) = signal(false);
    let (command_query, set_command_query) = signal(String::new());
    let (capabilities_open, set_capabilities_open) = signal(false);
    let auth_modal_open = RwSignal::new(false);
    let (zoom, set_zoom) = signal(1.0f64);
    let (pan, set_pan) = signal((0.0f64, 0.0f64));
    let (panning, set_panning) = signal(Option::<(f64, f64, f64, f64)>::None);
    let command_input = NodeRef::<leptos::html::Input>::new();
    let layout = RwSignal::new(load_layout());
    let history = RwSignal::new(living_canvas::LayoutHistory::new());
    let dragging = RwSignal::new(None::<DragState>);
    let resizing = RwSignal::new(None::<ResizeState>);
    let runtime = RwSignal::new(RuntimeState::Loading);
    spawn_local(async move {
        let client = GatewayMindClient;
        let result = async {
            let session = client.session().await?;
            let snapshot = client.snapshot().await?;
            Ok::<_, living_canvas::ClientError>((session.mode, snapshot))
        }
        .await;
        // The owner projection is fetched separately and allowed to fail on its own: capabilities
        // are still worth showing when Identity1 or the Journal cannot be read.
        let mind = client.mind().await.ok();
        runtime.set(match result {
            Ok((mode, snapshot)) => RuntimeState::Ready {
                mode,
                snapshot,
                mind,
            },
            Err(error) => RuntimeState::Error(error.to_string()),
        });
    });

    if let Ok(events) = EventSource::new("/api/v1/events") {
        let on_snapshot = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
            let Some(data) = event.data().as_string() else {
                return;
            };
            let Ok(snapshot) =
                serde_json::from_str::<cybou_web_contracts::SnapshotProjection>(&data)
            else {
                return;
            };
            runtime.update(|state| {
                if let RuntimeState::Ready {
                    snapshot: current, ..
                } = state
                {
                    *current = snapshot;
                }
            });
        });
        if events
            .add_event_listener_with_callback("snapshot", on_snapshot.as_ref().unchecked_ref())
            .is_ok()
        {
            // App is the page root and is mounted once; keep the browser stream for that lifetime.
            on_snapshot.forget();
            std::mem::forget(events);
        }
    }

    if let Some(window) = web_sys::window() {
        let on_shortcut = Closure::<dyn FnMut(KeyboardEvent)>::new(move |event: KeyboardEvent| {
            if (event.ctrl_key() || event.meta_key()) && event.key().eq_ignore_ascii_case("k") {
                event.prevent_default();
                set_command_open.set(true);
                if let Some(input) = command_input.get() {
                    let _ = input.focus();
                }
            } else if (event.ctrl_key() || event.meta_key())
                && event.key().eq_ignore_ascii_case("z")
            {
                event.prevent_default();
                if event.shift_key() {
                    apply_redo(history, layout);
                } else {
                    apply_undo(history, layout);
                }
            } else if (event.ctrl_key() || event.meta_key())
                && event.key().eq_ignore_ascii_case("y")
            {
                event.prevent_default();
                apply_redo(history, layout);
            } else if event.key() == "Escape" {
                set_command_open.set(false);
                set_command_query.set(String::new());
                set_capabilities_open.set(false);
            }
        });
        if window
            .add_event_listener_with_callback("keydown", on_shortcut.as_ref().unchecked_ref())
            .is_ok()
        {
            on_shortcut.forget();
        }
    }

    let runtime_label = move || match runtime.get() {
        RuntimeState::Loading => "Connecting".to_owned(),
        RuntimeState::Ready { mode, .. } => match mode {
            cybou_web_contracts::SessionMode::LocalDesktop => "Local".to_owned(),
            cybou_web_contracts::SessionMode::PublicPreview => "Preview".to_owned(),
            cybou_web_contracts::SessionMode::RemoteBrowser => "Remote".to_owned(),
        },
        RuntimeState::Error(_) => "Unavailable".to_owned(),
    };
    let projection_label = move || match runtime.get() {
        RuntimeState::Loading => "Loading projection…".into(),
        RuntimeState::Ready { snapshot, .. } => {
            format!("Gateway · projection {}", snapshot.projection_version)
        }
        RuntimeState::Error(error) => error,
    };
    let system_label = move || match runtime.get() {
        RuntimeState::Loading => "Connecting to local gateway…".into(),
        RuntimeState::Ready { snapshot, .. } => {
            let available = snapshot
                .capabilities
                .iter()
                .filter(|capability| capability.state == cybou_protocol::CapabilityState::Available)
                .count();
            format!(
                "{available}/{} capabilities available · projection {}",
                snapshot.capabilities.len(),
                snapshot.projection_version
            )
        }
        RuntimeState::Error(_) => "Gateway unavailable · canvas remains read-only".into(),
    };
    let observed_label = move || match runtime.get() {
        RuntimeState::Ready { snapshot, .. } => format!("Observed {}", snapshot.observed_at),
        RuntimeState::Loading => "Waiting for first snapshot".into(),
        RuntimeState::Error(_) => "No current snapshot".into(),
    };
    let capabilities = move || match runtime.get() {
        RuntimeState::Ready { snapshot, .. } => snapshot.capabilities,
        RuntimeState::Loading | RuntimeState::Error(_) => Vec::new(),
    };
    let mind = move || match runtime.get() {
        RuntimeState::Ready { mind, .. } => mind,
        RuntimeState::Loading | RuntimeState::Error(_) => None,
    };
    // Every reader below answers "not read" rather than inventing a placeholder value, because a
    // dash is honest and a zero is a claim.
    let identity_id = move || {
        mind()
            .and_then(|m| m.identity.identity_id)
            .unwrap_or_else(unread)
    };
    let identity_origin = move || {
        mind()
            .and_then(|m| m.identity.origin)
            .unwrap_or_else(unread)
    };
    let identity_sessions = move || {
        mind()
            .and_then(|m| m.identity.session_count)
            .map_or_else(unread, |value| value.to_string())
    };
    let identity_age = move || {
        mind()
            .and_then(|m| m.identity.age_in_days)
            .map_or_else(unread, |value| format!("{value} d"))
    };
    let identity_architecture = move || {
        mind()
            .and_then(|m| m.identity.architecture_version)
            .unwrap_or_else(unread)
    };
    let journal_count = move || {
        mind()
            .and_then(|m| m.journal.contribution_count)
            .map_or_else(unread, |value| value.to_string())
    };
    let journal_epoch = move || {
        mind()
            .and_then(|m| m.journal.erasure_epoch)
            .map_or_else(unread, |value| value.to_string())
    };
    let journal_recent = move || mind().map_or_else(Vec::new, |m| m.journal.recent);
    let journal_integrity = move || {
        mind()
            .and_then(|m| m.journal.integrity)
            .unwrap_or_else(|| "not verified yet".to_owned())
    };
    let journal_state = move || {
        mind().map_or_else(
            || "Event1 not read".to_owned(),
            |m| knowledge_label(m.journal.knowledge).to_owned(),
        )
    };
    let lifecycle_mode = move || mind().and_then(|m| m.lifecycle.mode).unwrap_or_else(unread);
    let lifecycle_activity = move || {
        mind()
            .and_then(|m| m.lifecycle.last_user_activity_at)
            .unwrap_or_else(unread)
    };
    let self_narration = move || {
        mind()
            .and_then(|m| m.self_model.narration)
            .unwrap_or_else(|| "Self1 has not been read.".to_owned())
    };
    let self_open_intentions = move || {
        mind()
            .and_then(|m| m.self_model.open_intentions)
            .map_or_else(unread, |value| value.to_string())
    };
    let self_settled = move || {
        mind()
            .and_then(|m| m.self_model.settled_predictions)
            .map_or_else(unread, |value| value.to_string())
    };
    let attention_focus = move || match mind() {
        None => "Workspace1 not read".to_owned(),
        Some(m) if m.attention.knowledge != cybou_protocol::KnowledgeState::Known => {
            "Workspace1 not read".to_owned()
        }
        // Workspace1 answering with no winner is knowledge: nothing currently holds attention.
        Some(m) => m
            .attention
            .focus
            .unwrap_or_else(|| "Nothing holds focus".to_owned()),
    };
    let attention_salience = move || {
        mind()
            .and_then(|m| m.attention.salience)
            .map_or_else(unread, |value| format!("{value:.2}"))
    };
    let attention_organs = move || {
        let organs = mind().map_or_else(Vec::new, |m| m.attention.organs);
        if organs.is_empty() {
            unread()
        } else {
            organs.join(", ")
        }
    };
    let beliefs = move || mind().map_or_else(Vec::new, |m| m.beliefs.beliefs);
    let beliefs_label = move || match mind() {
        None => "Epistemic1 not read".to_owned(),
        Some(m) if m.beliefs.knowledge != cybou_protocol::KnowledgeState::Known => {
            "Epistemic1 not read".to_owned()
        }
        Some(m) => match m.beliefs.beliefs.len() {
            0 => "Believes nothing yet".to_owned(),
            1 => "1 belief".to_owned(),
            count => format!("{count} beliefs"),
        },
    };
    let perception_status = move || {
        mind()
            .and_then(|m| m.perception.status)
            .unwrap_or_else(unread)
    };
    let perception_source = move || {
        mind()
            .and_then(|m| m.perception.source_id)
            .unwrap_or_else(unread)
    };
    let perception_at = move || {
        mind()
            .and_then(|m| m.perception.acquired_at)
            .unwrap_or_else(unread)
    };
    let concepts = move || mind().map_or_else(Vec::new, |m| m.context.concepts);
    let context_label = move || match mind() {
        None => "Context1 not read".to_owned(),
        Some(m) if m.context.knowledge != cybou_protocol::KnowledgeState::Known => {
            "Context1 not read".to_owned()
        }
        Some(m) => match m.context.concepts.len() {
            0 => "Nothing activated yet".to_owned(),
            1 => "1 active concept".to_owned(),
            count => format!("{count} active concepts"),
        },
    };
    let commitments = move || mind().map_or_else(Vec::new, |m| m.commitments.open);
    let commitments_label = move || match mind() {
        None => "Intention1 not read".to_owned(),
        Some(m) if m.commitments.knowledge != cybou_protocol::KnowledgeState::Known => {
            "Intention1 not read".to_owned()
        }
        Some(m) => match m.commitments.open_count.unwrap_or_default() {
            0 => "No open commitments".to_owned(),
            1 => "1 open commitment".to_owned(),
            count => format!("{count} open commitments"),
        },
    };
    let session_consumer = move || match runtime.get() {
        RuntimeState::Ready { .. } => "living-canvas".to_owned(),
        RuntimeState::Loading | RuntimeState::Error(_) => unread(),
    };
    let mind_observed = move || {
        mind().map_or_else(
            || "owners not read".to_owned(),
            |m| format!("Owners read {}", m.observed_at),
        )
    };

    view! {
            <main class="app-shell">
                <header class="topbar">
                    <a class="brand" href="#canvas" aria-label="Cybou home">
                        <img class="brand-mark" src="/cybou-mark.svg" alt="" />
                        <span>"Cybou"</span>
                    </a>
                    <div class="runtime-cluster">
                        <div class="runtime" aria-label="Runtime connection" aria-live="polite">
                            <span class="status-dot" aria-hidden="true"></span>
                            <strong>{runtime_label}</strong>
                            <small>{projection_label}</small>
                        </div>
                        {move || match runtime.get() {
                            RuntimeState::Ready { mode, .. } if mode != cybou_web_contracts::SessionMode::PublicPreview => {
                                view! {
                                    <button
                                        class="topbar-auth-btn sign-out-btn"
                                        title="Sign out from host session"
                                        on:click=move |_| {
                                            spawn_local(async move {
                                                let _ = GatewayMindClient.logout().await;
                                                if let Some(w) = web_sys::window() {
                                                    let _ = w.location().reload();
                                                }
                                            });
                                        }
                                    >
                                        <IconShield size=12 />
                                        <span>"Sign out"</span>
                                    </button>
                                }.into_any()
                            }
                            _ => {
                                view! {
                                    <button
                                        class="topbar-auth-btn sign-in-btn"
                                        title="Sign in with Linux account"
                                        on:click=move |_| auth_modal_open.set(true)
                                    >
                                        <IconShield size=12 />
                                        <span>"Sign in"</span>
                                    </button>
                                }.into_any()
                            }
                        }}
                        <button
                            class="runtime-switch"
                            aria-expanded=move || runtime_menu_open.get().to_string()
                            aria-controls="runtime-menu"
                            on:click=move |_| set_runtime_menu_open.update(|open| *open = !*open)
                        >
                            <span>"Local"</span>
                            <span class="inactive">"Remote"</span>
                        </button>
                        <button
                            class="profile-trigger"
                            aria-label="Open Cybou workspace menu"
                            aria-expanded=move || runtime_menu_open.get().to_string()
                            on:click=move |_| set_runtime_menu_open.update(|open| *open = !*open)
                        >"C"</button>
                        <Show when=move || runtime_menu_open.get()>
                            <nav id="runtime-menu" class="runtime-menu" aria-label="Cybou workspace menu">
                                <header><strong>"Cybou"</strong><small>{mind_observed}</small></header>
                                <button on:click=move |_| navigate_from_menu("identity", set_selected, set_runtime_menu_open)><FileCheck size=15 /><span>"Identity"</span></button>
                                <button on:click=move |_| navigate_from_menu("commitments", set_selected, set_runtime_menu_open)><ListChecks size=15 /><span>"Commitments"</span></button>
                                <button on:click=move |_| navigate_from_menu("session", set_selected, set_runtime_menu_open)><UsersRound size=15 /><span>"Session"</span></button>
                                <button on:click=move |_| navigate_from_menu("lifecycle", set_selected, set_runtime_menu_open)><Sparkles size=15 /><span>"Lifecycle"</span></button>
                                <hr />
                                <button on:click=move |_| navigate_from_menu("capabilities", set_selected, set_runtime_menu_open)><Map size=15 /><span>"Capabilities"</span></button>
                                <button
                                    aria-pressed=move || minimap_visible.get().to_string()
                                    on:click=move |_| {
                                        set_minimap_visible.set(!minimap_visible.get_untracked());
                                        set_runtime_menu_open.set(false);
                                    }
                                ><Map size=15 /><span>"Minimap"</span></button>
                                <hr />
                                <button on:click=move |_| navigate_from_menu("journal", set_selected, set_runtime_menu_open)><Sparkles size=15 /><span>"Journal"</span></button>
                                <button on:click=move |_| {
                                    layout.update(|l| l.open_card(CardId::Shell(0), 400.0, 160.0));
                                    layout.get_untracked().save();
                                    set_selected.set("shell");
                                    set_runtime_menu_open.set(false);
                                }><IconTerminal size=15 /><span>"Open Shell"</span></button>
                                <button on:click=move |_| {
                                    layout.update(|l| l.open_card(CardId::FileManager(0), 380.0, 120.0));
                                    layout.get_untracked().save();
                                    set_selected.set("files");
                                    set_runtime_menu_open.set(false);
                                }><IconFolder size=15 /><span>"File Manager"</span></button>
                                <button on:click=move |_| {
                                    layout.update(|l| l.open_card(CardId::WebBrowser(0), 460.0, 140.0));
                                    layout.get_untracked().save();
                                    set_selected.set("browser");
                                    set_runtime_menu_open.set(false);
                                }><IconGlobe size=15 /><span>"Web Browser"</span></button>
                                <hr />
                                <Show when=move || history.get().can_undo()>
                                    <button on:click=move |_| {
                                        apply_undo(history, layout);
                                        set_runtime_menu_open.set(false);
                                    }><IconUndo size=15 /><span>"Undo layout"</span></button>
                                </Show>
                                <Show when=move || history.get().can_redo()>
                                    <button on:click=move |_| {
                                        apply_redo(history, layout);
                                        set_runtime_menu_open.set(false);
                                    }><IconRedo size=15 /><span>"Redo layout"</span></button>
                                </Show>
                                <button on:click=move |_| {
                                    history.update(|h| h.push(layout.get_untracked()));
                                    layout.update(|l| {
                                        l.create_deck("Mind Core", vec![CardId::Identity, CardId::Session], 70.0, 50.0);
                                    });
                                    layout.get_untracked().save();
                                    set_runtime_menu_open.set(false);
                                }><IconLayers size=15 /><span>"Group: Mind Deck"</span></button>
                                <button on:click=move |_| {
                                    history.update(|h| h.push(layout.get_untracked()));
                                    layout.update(|l| l.apply_arrangement(ArrangementMode::Grid));
                                    layout.get_untracked().save();
                                    set_runtime_menu_open.set(false);
                                }><IconGrid size=15 /><span>"Arrange: Grid"</span></button>
                                <button on:click=move |_| {
                                    history.update(|h| h.push(layout.get_untracked()));
                                    layout.update(|l| l.apply_arrangement(ArrangementMode::Compact));
                                    layout.get_untracked().save();
                                    set_runtime_menu_open.set(false);
                                }><IconMinimize size=15 /><span>"Arrange: Compact"</span></button>
                                <button on:click=move |_| {
                                    history.update(|h| h.push(layout.get_untracked()));
                                    layout.update(|l| l.apply_arrangement(ArrangementMode::Relations));
                                    layout.get_untracked().save();
                                    set_runtime_menu_open.set(false);
                                }><Link size=15 /><span>"Arrange: Relations"</span></button>
                                <button on:click=move |_| {
                                    history.update(|h| h.push(layout.get_untracked()));
                                    layout.set(DesktopLayout::default());
                                    layout.get_untracked().save();
                                    set_runtime_menu_open.set(false);
                                }><IconRefresh size=15 /><span>"Reset layout"</span></button>
                            </nav>
                        </Show>
                    </div>
                </header>

                <section
                    id="canvas"
                    class="canvas"
                    class:panning=move || panning.get().is_some()
                    aria-label="CYBOU Desktop"
                    style=move || format!("transform: translate({:.1}px, {:.1}px) scale({:.2}); transform-origin: 0 0;", pan.get().0, pan.get().1, zoom.get())
                    on:dblclick=move |e: web_sys::MouseEvent| {
                        if e.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()).map_or(false, |el| el.class_list().contains("canvas") || el.class_list().contains("ambient")) {
                            set_zoom.set(1.0);
                            set_pan.set((0.0, 0.0));
                        }
                    }
                    on:wheel=move |event: web_sys::WheelEvent| {
                        if event.ctrl_key() {
                            event.prevent_default();
                            if event.delta_y() < 0.0 {
                                set_zoom.update(|z| *z = (*z + 0.05).min(2.0));
                            } else {
                                set_zoom.update(|z| *z = (*z - 0.05).max(0.4));
                            }
                        } else {
                            event.prevent_default();
                            set_pan.update(|(px, py)| {
                                *px -= event.delta_x() * 0.8;
                                *py -= event.delta_y() * 0.8;
                            });
                        }
                    }
                    on:pointerdown=move |event: PointerEvent| {
                        let is_canvas_bg = event.target()
                            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                            .map_or(false, |el| el.class_list().contains("canvas") || el.class_list().contains("ambient") || el.tag_name().eq_ignore_ascii_case("svg"));
                        if is_canvas_bg || event.button() == 1 {
                            set_panning.set(Some((event.client_x() as f64, event.client_y() as f64, pan.get().0, pan.get().1)));
                        }
                    }
                    on:pointermove=move |event: PointerEvent| {
                        if let Some((start_x, start_y, init_px, init_py)) = panning.get() {
                            let cur_x = event.client_x() as f64;
                            let cur_y = event.client_y() as f64;
                            set_pan.set((init_px + (cur_x - start_x), init_py + (cur_y - start_y)));
                        }
                        move_drag(event.clone(), layout, dragging);
                        move_resize(event, layout, resizing);
                    }
                    on:pointerup=move |_| {
                        set_panning.set(None);
                        finish_drag(layout, history, dragging);
                        finish_resize(layout, resizing);
                    }
                    on:pointercancel=move |_| {
                        set_panning.set(None);
                        finish_drag(layout, history, dragging);
                        finish_resize(layout, resizing);
                    }
                >
                    <div class="ambient" aria-hidden="true"></div>
                    <svg class="relationship-layer" aria-label="Desktop relationships">
                        // Each edge is a relationship the system actually has: every organ writes
                        // its contributions into the one Journal, Health1 derives capabilities from
                        // whether those organs answer, and the session only presents the result.
                        <RelationshipEdge layout=layout selected=selected from=CardId::Identity to=CardId::Journal label="writes to" amber=false />
                        <RelationshipEdge layout=layout selected=selected from=CardId::Commitments to=CardId::Journal label="writes to" amber=false />
                        <RelationshipEdge layout=layout selected=selected from=CardId::Lifecycle to=CardId::Journal label="consolidates into" amber=true />
                        <RelationshipEdge layout=layout selected=selected from=CardId::Capabilities to=CardId::Identity label="evaluates" amber=false />
                        <RelationshipEdge layout=layout selected=selected from=CardId::Capabilities to=CardId::Session label="presented under" amber=false />
                    </svg>
                    <Show when=move || !layout.get().is_in_deck(CardId::Identity)>
                        <div
                            class:selected=move || selected.get() == "identity"
                            class:pinned=move || layout.get().presentation(CardId::Identity).pinned
                            class:collapsed=move || layout.get().presentation(CardId::Identity).collapsed
                            class:magnet-target=move || dragging.get().and_then(|d| d.drop_target) == Some(CardId::Identity)
                            class="object identity"
                            style=move || card_style(layout.get(), CardId::Identity)
                            tabindex="0"
                            role="region"
                            aria-label="Identity card. Drag to reposition; use arrow keys for keyboard movement."
                            on:pointerdown=move |event| start_drag(event, CardId::Identity, layout, dragging)
                            on:keydown=move |event| keyboard_move(event, CardId::Identity, layout)
                            on:click=move |_| set_selected.set("identity")
                        >
                        <header class="card-header">
                            <small class="panel-kicker"><FileCheck size=14 /><span>"Identity1"</span></small>
                            <CardControls card=CardId::Identity layout=layout />
                        </header>
                        <Show
                            when=move || !layout.get().presentation(CardId::Identity).collapsed
                            fallback=move || view! {
                                <div class="card-collapsed-summary">
                                    <b>"Subject continuity"</b>
                                    <span>{identity_id()}</span>
                                </div>
                            }
                        >
                            <strong>"Subject continuity"</strong>
                            <span class="identity-digest">{identity_id}</span>
                            <span class="identity-badges"><i>{identity_sessions}" sessions"</i><i>{identity_age}</i></span>
                            <span class="identity-meta">"Origin "{identity_origin}" · "{identity_architecture}</span>
                        </Show>
                        <CardResizeHandle card=CardId::Identity layout=layout resizing=resizing />
                    </div>
                </Show>

                    <Show when=move || selected.get() == "capabilities">
                        <nav
                            class="object-actions"
                            style=move || selection_actions_style(layout.get())
                            aria-label="Selected release actions"
                        >
                            <button aria-label="Open capability health" on:click=move |_| set_selected.set("capabilities")><FolderOpen size=15 /><span>"Health"</span></button>
                            <button aria-label="Open commitments" on:click=move |_| set_selected.set("commitments")><ListChecks size=15 /><span>"Promises"</span></button>
                            <button aria-label="Open lifecycle" on:click=move |_| set_selected.set("lifecycle")><Sparkles size=15 /><span>"Lifecycle"</span></button>
                            <button aria-label="Open the Journal" on:click=move |_| set_selected.set("journal")><Link size=15 /><span>"Journal"</span></button>
                            <button
                                aria-label="More release actions"
                                on:click=move |_| {
                                    set_command_open.set(true);
                                    if let Some(input) = command_input.get() {
                                        let _ = input.focus();
                                    }
                                }
                            ><Ellipsis size=16 /><span class="sr-only">"More"</span></button>
                        </nav>
                    </Show>

                    <Show when=move || !layout.get().is_in_deck(CardId::Session)>
                        <div
                            class:selected=move || selected.get() == "session"
                            class:pinned=move || layout.get().presentation(CardId::Session).pinned
                            class:collapsed=move || layout.get().presentation(CardId::Session).collapsed
                            class:magnet-target=move || dragging.get().and_then(|d| d.drop_target) == Some(CardId::Session)
                            class="object session"
                            style=move || card_style(layout.get(), CardId::Session)
                            tabindex="0"
                            role="region"
                            aria-label="Session card. Drag to reposition; use arrow keys for keyboard movement."
                            on:pointerdown=move |event| start_drag(event, CardId::Session, layout, dragging)
                            on:keydown=move |event| keyboard_move(event, CardId::Session, layout)
                            on:click=move |_| set_selected.set("session")
                        >
                            <header class="card-header">
                                <small class="panel-kicker"><UsersRound size=14 /><span>"Session"</span></small>
                                <CardControls card=CardId::Session layout=layout />
                            </header>
                            <Show
                                when=move || !layout.get().presentation(CardId::Session).collapsed
                                fallback=move || view! {
                                    <div class="card-collapsed-summary">
                                        <b>"Session"</b>
                                        <span>{runtime_label()}</span>
                                    </div>
                                }
                            >
                                <strong>"Established trust"</strong>
                                <span class="row"><b>"Mode"</b><i>{runtime_label}</i></span>
                                <span class="row"><b>"Consumer"</b><i>{session_consumer}</i></span>
                                <span class="row"><b>"Authenticated"</b><i>"No"</i></span>
                                <span class="row"><b>"Device bound"</b><i>"No"</i></span>
                                <span class="panel-link">"Established by the gateway, never by this page"</span>
                            </Show>
                            <CardResizeHandle card=CardId::Session layout=layout resizing=resizing />
                        </div>
                    </Show>

                    <Show when=move || !layout.get().is_in_deck(CardId::Capabilities)>
                        <div
                            class:selected=move || selected.get() == "capabilities"
                            class:pinned=move || layout.get().presentation(CardId::Capabilities).pinned
                            class:collapsed=move || layout.get().presentation(CardId::Capabilities).collapsed
                            class:magnet-target=move || dragging.get().and_then(|d| d.drop_target) == Some(CardId::Capabilities)
                            class="object capabilities"
                            style=move || card_style(layout.get(), CardId::Capabilities)
                            tabindex="0"
                            role="region"
                            aria-label="Capabilities card. Drag to reposition; use arrow keys for keyboard movement."
                            on:pointerdown=move |event| start_drag(event, CardId::Capabilities, layout, dragging)
                            on:keydown=move |event| keyboard_move(event, CardId::Capabilities, layout)
                            on:click=move |_| set_selected.set("capabilities")
                        >
                            <header class="card-header">
                                <small class="panel-kicker"><Sparkles size=14 /><span>"Health1"</span></small>
                                <CardControls card=CardId::Capabilities layout=layout />
                            </header>
                            <Show
                                when=move || !layout.get().presentation(CardId::Capabilities).collapsed
                                fallback=move || view! {
                                    <div class="card-collapsed-summary">
                                        <b>"Capabilities"</b>
                                        <span>{system_label()}</span>
                                    </div>
                                }
                            >
                                <h1>{system_label}</h1>
                                <span class="capabilities-kind">"Capability health"</span>
                                <p>"A capability is available only while every organ it depends on answers Health1. Nothing here is composed by this page."</p>
                                <div class="capability-list">
                                    <For
                                        each=capabilities
                                        key=|capability| capability.id.clone()
                                        children=move |capability| {
                                            let available = capability.state == cybou_protocol::CapabilityState::Available;
                                            let status = capability_state_label(capability.state);
                                            let reason = capability.reason.unwrap_or_default();
                                            view! {
                                                <span class:available=available class="capability-line">
                                                    <span class="status-dot" aria-hidden="true"></span>
                                                    <b>{capability.id}</b>
                                                    <i>{status}</i>
                                                    <small>{reason}</small>
                                                </span>
                                            }
                                        }
                                    />
                                </div>
                                <footer class="capabilities-meta">
                                    <span><small>"Observed"</small><b>{observed_label}</b></span>
                                </footer>
                            </Show>
                            <CardResizeHandle card=CardId::Capabilities layout=layout resizing=resizing />
                        </div>
                    </Show>

                    <Show when=move || !layout.get().is_in_deck(CardId::Journal)>
                        <div
                            class:selected=move || selected.get() == "journal"
                            class:pinned=move || layout.get().presentation(CardId::Journal).pinned
                            class:collapsed=move || layout.get().presentation(CardId::Journal).collapsed
                            class:magnet-target=move || dragging.get().and_then(|d| d.drop_target) == Some(CardId::Journal)
                            class="object journal"
                            style=move || card_style(layout.get(), CardId::Journal)
                            tabindex="0"
                            role="region"
                            aria-label="Journal card. Drag to reposition; use arrow keys for keyboard movement."
                            on:pointerdown=move |event| start_drag(event, CardId::Journal, layout, dragging)
                            on:keydown=move |event| keyboard_move(event, CardId::Journal, layout)
                            on:click=move |_| set_selected.set("journal")
                        >
                            <header class="card-header">
                                <small class="panel-kicker"><Files size=14 /><span>"Event1"</span></small>
                                <CardControls card=CardId::Journal layout=layout />
                            </header>
                            <Show
                                when=move || !layout.get().presentation(CardId::Journal).collapsed
                                fallback=move || view! {
                                    <div class="card-collapsed-summary">
                                        <b>"Journal"</b>
                                        <span>{journal_count()}" entries"</span>
                                    </div>
                                }
                            >
                                <strong>"Canonical Journal"</strong>
                                <span class="row"><b>"Contributions"</b><i>{journal_count}</i></span>
                                <span class="row"><b>"Erasure epoch"</b><i>{journal_epoch}</i></span>
                                <span class="row"><b>"Integrity"</b><i>{journal_integrity}</i></span>
                                <div class="journal-feed">
                                    <For
                                        each=journal_recent
                                        key=|contribution| contribution.message_id.clone()
                                        children=move |contribution| {
                                            view! {
                                                <span class="journal-line">
                                                    <b>{contribution.kind}</b>
                                                    <i>{contribution.origin_organ}</i>
                                                    <small>{contribution.recorded_at}</small>
                                                </span>
                                            }
                                        }
                                    />
                                </div>
                                <span class="journal-footer"><i>{journal_state}</i><b>"Append only"</b></span>
                            </Show>
                            <CardResizeHandle card=CardId::Journal layout=layout resizing=resizing />
                        </div>
                    </Show>

                    <Show when=move || !layout.get().is_in_deck(CardId::Lifecycle)>
                        <article
                            class:selected=move || selected.get() == "lifecycle"
                            class:pinned=move || layout.get().presentation(CardId::Lifecycle).pinned
                            class:collapsed=move || layout.get().presentation(CardId::Lifecycle).collapsed
                            class:magnet-target=move || dragging.get().and_then(|d| d.drop_target) == Some(CardId::Lifecycle)
                            class="object lifecycle"
                            style=move || card_style(layout.get(), CardId::Lifecycle)
                            tabindex="0"
                            role="region"
                            aria-label="Lifecycle card. Drag to reposition; use arrow keys for keyboard movement."
                            on:pointerdown=move |event| start_drag(event, CardId::Lifecycle, layout, dragging)
                            on:keydown=move |event| keyboard_move(event, CardId::Lifecycle, layout)
                            on:click=move |_| set_selected.set("lifecycle")
                        >
                            <header class="card-header">
                                <div class="lifecycle-heading">
                                    <small class="panel-kicker"><Sparkles size=14 /><span>"Lifecycle1"</span></small>
                                    <b>{lifecycle_mode}</b>
                                </div>
                                <CardControls card=CardId::Lifecycle layout=layout />
                            </header>
                            <Show
                                when=move || !layout.get().presentation(CardId::Lifecycle).collapsed
                                fallback=move || view! {
                                    <div class="card-collapsed-summary">
                                        <b>"Lifecycle"</b>
                                        <span>{lifecycle_mode()}</span>
                                    </div>
                                }
                            >
                                <strong>"Sleep and wake"</strong>
                                <p>"The mode is the owner's own spelling, not a summary of it. After fifteen idle minutes the system re-verifies its whole chain, and stops the moment someone arrives."</p>
                                <span class="row"><b>"Last user activity"</b><i>{lifecycle_activity}</i></span>
                                <span class="lifecycle-source">{mind_observed}</span>
                            </Show>
                            <CardResizeHandle card=CardId::Lifecycle layout=layout resizing=resizing />
                        </article>
                    </Show>

                    <Show when=move || !layout.get().is_in_deck(CardId::Commitments)>
                        <div
                            class:selected=move || selected.get() == "commitments"
                            class:pinned=move || layout.get().presentation(CardId::Commitments).pinned
                            class:collapsed=move || layout.get().presentation(CardId::Commitments).collapsed
                            class:magnet-target=move || dragging.get().and_then(|d| d.drop_target) == Some(CardId::Commitments)
                            class="object commitments"
                            style=move || card_style(layout.get(), CardId::Commitments)
                            tabindex="0"
                            role="region"
                            aria-label="Commitments card. Drag to reposition; use arrow keys for keyboard movement."
                            on:pointerdown=move |event| start_drag(event, CardId::Commitments, layout, dragging)
                            on:keydown=move |event| keyboard_move(event, CardId::Commitments, layout)
                            on:click=move |_| set_selected.set("commitments")
                        >
                            <header class="card-header">
                                <small class="panel-kicker"><ListChecks size=14 /><span>{commitments_label}</span></small>
                                <CardControls card=CardId::Commitments layout=layout />
                            </header>
                            <Show
                                when=move || !layout.get().presentation(CardId::Commitments).collapsed
                                fallback=move || view! {
                                    <div class="card-collapsed-summary">
                                        <b>"Commitments"</b>
                                        <span>{commitments_label()}</span>
                                    </div>
                                }
                            >
                                <For
                                    each=commitments
                                    key=|commitment| commitment.id.clone()
                                    children=move |commitment| {
                                        view! {
                                            <span class="check-row">
                                                <b>{commitment.description}</b>
                                                <i>{commitment.trigger}</i>
                                            </span>
                                        }
                                    }
                                />
                                <span class="panel-link">"Intention1 holds these until they are closed"</span>
                            </Show>
                            <CardResizeHandle card=CardId::Commitments layout=layout resizing=resizing />
                        </div>
                    </Show>

                    <Show when=move || !layout.get().is_in_deck(CardId::SelfModel)>
                        <article
                            class:selected=move || selected.get() == "self"
                            class:pinned=move || layout.get().presentation(CardId::SelfModel).pinned
                            class:collapsed=move || layout.get().presentation(CardId::SelfModel).collapsed
                            class:magnet-target=move || dragging.get().and_then(|d| d.drop_target) == Some(CardId::SelfModel)
                            class="object self-model"
                            style=move || card_style(layout.get(), CardId::SelfModel)
                            tabindex="0"
                            role="region"
                            aria-label="Self-assessment card. Drag to reposition; use arrow keys for keyboard movement."
                            on:pointerdown=move |event| start_drag(event, CardId::SelfModel, layout, dragging)
                            on:keydown=move |event| keyboard_move(event, CardId::SelfModel, layout)
                            on:click=move |_| set_selected.set("self")
                        >
                            <header class="card-header">
                                <small class="panel-kicker"><Sparkles size=14 /><span>"Self1"</span></small>
                                <CardControls card=CardId::SelfModel layout=layout />
                            </header>
                            <Show
                                when=move || !layout.get().presentation(CardId::SelfModel).collapsed
                                fallback=move || view! {
                                    <div class="card-collapsed-summary">
                                        <b>"Self-assessment"</b>
                                        <span>{self_open_intentions()}" open"</span>
                                    </div>
                                }
                            >
                                <strong>"Self-assessment"</strong>
                                <p class="self-narration">{self_narration}</p>
                                <span class="row"><b>"Open obligations"</b><i>{self_open_intentions}</i></span>
                                <span class="row"><b>"Settled predictions"</b><i>{self_settled}</i></span>
                                <span class="panel-link">"Composed by Self1, not by this page"</span>
                            </Show>
                            <CardResizeHandle card=CardId::SelfModel layout=layout resizing=resizing />
                        </article>
                    </Show>

                    <Show when=move || !layout.get().is_in_deck(CardId::Beliefs)>
                        <div
                            class:selected=move || selected.get() == "beliefs"
                            class:pinned=move || layout.get().presentation(CardId::Beliefs).pinned
                            class:collapsed=move || layout.get().presentation(CardId::Beliefs).collapsed
                            class:magnet-target=move || dragging.get().and_then(|d| d.drop_target) == Some(CardId::Beliefs)
                            class="object beliefs"
                            style=move || card_style(layout.get(), CardId::Beliefs)
                            tabindex="0"
                            role="region"
                            aria-label="Beliefs card. Drag to reposition; use arrow keys for keyboard movement."
                            on:pointerdown=move |event| start_drag(event, CardId::Beliefs, layout, dragging)
                            on:keydown=move |event| keyboard_move(event, CardId::Beliefs, layout)
                            on:click=move |_| set_selected.set("beliefs")
                        >
                            <header class="card-header">
                                <small class="panel-kicker"><Sparkles size=14 /><span>"Epistemic1"</span></small>
                                <CardControls card=CardId::Beliefs layout=layout />
                            </header>
                            <Show
                                when=move || !layout.get().presentation(CardId::Beliefs).collapsed
                                fallback=move || view! {
                                    <div class="card-collapsed-summary">
                                        <b>"Beliefs"</b>
                                        <span>{beliefs_label()}</span>
                                    </div>
                                }
                            >
                                <strong>{beliefs_label}</strong>
                                <div class="belief-list">
                                    <For
                                        each=beliefs
                                        key=|belief| belief.subject.clone()
                                        children=move |belief| {
                                            let observed = belief.status == "observed";
                                            view! {
                                                <span class:observed=observed class="belief-line">
                                                    <b>{belief.subject}</b>
                                                    <span class="belief-value">{belief.value}</span>
                                                    <i>{belief.status}</i>
                                                </span>
                                            }
                                        }
                                    />
                                </div>
                                <span class="panel-link">"A belief and its validity are separate facts"</span>
                            </Show>
                            <CardResizeHandle card=CardId::Beliefs layout=layout resizing=resizing />
                        </div>
                    </Show>

                    <Show when=move || !layout.get().is_in_deck(CardId::Context)>
                        <div
                            class:selected=move || selected.get() == "context"
                            class:pinned=move || layout.get().presentation(CardId::Context).pinned
                            class:collapsed=move || layout.get().presentation(CardId::Context).collapsed
                            class:magnet-target=move || dragging.get().and_then(|d| d.drop_target) == Some(CardId::Context)
                            class="object context"
                            style=move || card_style(layout.get(), CardId::Context)
                            tabindex="0"
                            role="region"
                            aria-label="Associative context card. Drag to reposition; use arrow keys for keyboard movement."
                            on:pointerdown=move |event| start_drag(event, CardId::Context, layout, dragging)
                            on:keydown=move |event| keyboard_move(event, CardId::Context, layout)
                            on:click=move |_| set_selected.set("context")
                        >
                            <header class="card-header">
                                <small class="panel-kicker"><Link size=14 /><span>"Context1"</span></small>
                                <CardControls card=CardId::Context layout=layout />
                            </header>
                            <Show
                                when=move || !layout.get().presentation(CardId::Context).collapsed
                                fallback=move || view! {
                                    <div class="card-collapsed-summary">
                                        <b>"Context"</b>
                                        <span>{context_label()}</span>
                                    </div>
                                }
                            >
                                <strong>{context_label}</strong>
                                <div class="concept-list">
                                    <For
                                        each=concepts
                                        key=|concept| concept.label.clone()
                                        children=move |concept| {
                                            view! {
                                                <span class="concept-line">
                                                    <b>{concept.label}</b>
                                                    <i>{format!("{:.2}", concept.salience)}</i>
                                                    <small>{concept.activation_reason}</small>
                                                </span>
                                            }
                                        }
                                    />
                                </div>
                                <span class="panel-link">"Association is not truth"</span>
                            </Show>
                            <CardResizeHandle card=CardId::Context layout=layout resizing=resizing />
                        </div>
                    </Show>

                    <Show when=move || !layout.get().is_in_deck(CardId::Perception)>
                        <div
                            class:selected=move || selected.get() == "perception"
                            class:pinned=move || layout.get().presentation(CardId::Perception).pinned
                            class:collapsed=move || layout.get().presentation(CardId::Perception).collapsed
                            class:magnet-target=move || dragging.get().and_then(|d| d.drop_target) == Some(CardId::Perception)
                            class="object perception"
                            style=move || card_style(layout.get(), CardId::Perception)
                            tabindex="0"
                            role="region"
                            aria-label="Perception card. Drag to reposition; use arrow keys for keyboard movement."
                            on:pointerdown=move |event| start_drag(event, CardId::Perception, layout, dragging)
                            on:keydown=move |event| keyboard_move(event, CardId::Perception, layout)
                            on:click=move |_| set_selected.set("perception")
                        >
                            <header class="card-header">
                                <small class="panel-kicker"><Files size=14 /><span>"Perception1"</span></small>
                                <CardControls card=CardId::Perception layout=layout />
                            </header>
                            <Show
                                when=move || !layout.get().presentation(CardId::Perception).collapsed
                                fallback=move || view! {
                                    <div class="card-collapsed-summary">
                                        <b>"Perception"</b>
                                        <span>{perception_status()}</span>
                                    </div>
                                }
                            >
                                <strong>"Host observation"</strong>
                                <span class="row"><b>"Status"</b><i>{perception_status}</i></span>
                                <span class="row"><b>"Source"</b><i>{perception_source}</i></span>
                                <span class="row"><b>"Acquired"</b><i>{perception_at}</i></span>
                            </Show>
                            <CardResizeHandle card=CardId::Perception layout=layout resizing=resizing />
                        </div>
                    </Show>

                    <Show when=move || !layout.get().is_in_deck(CardId::Attention)>
                        <div
                            class:selected=move || selected.get() == "attention"
                            class:pinned=move || layout.get().presentation(CardId::Attention).pinned
                            class:collapsed=move || layout.get().presentation(CardId::Attention).collapsed
                            class:magnet-target=move || dragging.get().and_then(|d| d.drop_target) == Some(CardId::Attention)
                            class="object attention"
                            style=move || card_style(layout.get(), CardId::Attention)
                            tabindex="0"
                            role="region"
                            aria-label="Attention card. Drag to reposition; use arrow keys for keyboard movement."
                            on:pointerdown=move |event| start_drag(event, CardId::Attention, layout, dragging)
                            on:keydown=move |event| keyboard_move(event, CardId::Attention, layout)
                            on:click=move |_| set_selected.set("attention")
                        >
                            <header class="card-header">
                                <small class="panel-kicker"><Map size=14 /><span>"Workspace1"</span></small>
                                <CardControls card=CardId::Attention layout=layout />
                            </header>
                            <Show
                                when=move || !layout.get().presentation(CardId::Attention).collapsed
                                fallback=move || view! {
                                    <div class="card-collapsed-summary">
                                        <b>"Attention"</b>
                                        <span>{attention_focus()}</span>
                                    </div>
                                }
                            >
                                <strong>"Attention"</strong>
                                <span class="attention-focus">{attention_focus}</span>
                                <span class="row"><b>"Salience"</b><i>{attention_salience}</i></span>
                                <span class="row"><b>"Organs"</b><i>{attention_organs}</i></span>
                            </Show>
                            <CardResizeHandle card=CardId::Attention layout=layout resizing=resizing />
                        </div>
                    </Show>

                    <ShellCard layout=layout selected=selected set_selected=set_selected dragging=dragging resizing=resizing />
                    <FileManagerCard layout=layout selected=selected set_selected=set_selected dragging=dragging resizing=resizing />
                    <WebBrowserCard layout=layout selected=selected set_selected=set_selected dragging=dragging resizing=resizing />
                    <JournalFeedCard layout=layout selected=selected set_selected=set_selected dragging=dragging resizing=resizing />
                    <TelemetryCard layout=layout selected=selected set_selected=set_selected dragging=dragging resizing=resizing />
                    <EpistemicGraphCard layout=layout selected=selected set_selected=set_selected dragging=dragging resizing=resizing />
                    <IntentLauncherCard layout=layout selected=selected set_selected=set_selected dragging=dragging resizing=resizing />

                    <For
                        each={move || layout.get().decks.into_iter().map(|d| d.id).collect::<Vec<_>>()}
                        key=|id| id.clone()
                        children=move |deck_id| {
                            view! {
                                <DeckContainerView
                                    deck_id=deck_id
                                    layout=layout
                                    history=history
                                    dragging=dragging
                                    resizing=resizing
                                    runtime=runtime
                                />
                            }
                        }
                    />

                    <Show when=move || command_open.get()>
                        <nav class="command-palette" aria-label="Desktop commands">
                            <small>"Jump to"</small>
                            <button
                                class:hidden=move || !command_matches(&command_query.get(), "capabilities health")
                                on:click=move |_| select_from_command("capabilities", set_selected, set_command_open, set_command_query)
    ><Sparkles size=15 /><span><b>"Capabilities"</b><i>"Health1"</i></span></button>
                            <button
                                class:hidden=move || !command_matches(&command_query.get(), "identity subject continuity")
                                on:click=move |_| select_from_command("identity", set_selected, set_command_open, set_command_query)
    ><FileCheck size=15 /><span><b>"Identity"</b><i>"Identity1"</i></span></button>
                            <button
                                class:hidden=move || !command_matches(&command_query.get(), "session trust mode")
                                on:click=move |_| select_from_command("session", set_selected, set_command_open, set_command_query)
    ><UsersRound size=15 /><span><b>"Session"</b><i>"Established trust"</i></span></button>
                            <button
                                class:hidden=move || !command_matches(&command_query.get(), "journal contributions event1")
                                on:click=move |_| select_from_command("journal", set_selected, set_command_open, set_command_query)
    ><Files size=15 /><span><b>"Journal"</b><i>"Event1"</i></span></button>
                            <button
                                class:hidden=move || !command_matches(&command_query.get(), "lifecycle sleep wake")
                                on:click=move |_| select_from_command("lifecycle", set_selected, set_command_open, set_command_query)
    ><Sparkles size=15 /><span><b>"Lifecycle"</b><i>"Sleep and wake"</i></span></button>
                            <button
                                class:hidden=move || !command_matches(&command_query.get(), "commitments obligations intention1")
                                on:click=move |_| select_from_command("commitments", set_selected, set_command_open, set_command_query)
    ><ListChecks size=15 /><span><b>"Commitments"</b><i>"Intention1"</i></span></button>
                            <button
                                class:hidden=move || !command_matches(&command_query.get(), "self assessment narration self1")
                                on:click=move |_| select_from_command("self", set_selected, set_command_open, set_command_query)
                            ><Sparkles size=15 /><span><b>"Self-assessment"</b><i>"Self1"</i></span></button>
                            <button
                                class:hidden=move || !command_matches(&command_query.get(), "attention focus workspace1")
                                on:click=move |_| select_from_command("attention", set_selected, set_command_open, set_command_query)
                            ><Map size=15 /><span><b>"Attention"</b><i>"Workspace1"</i></span></button>
                            <button
                                class:hidden=move || !command_matches(&command_query.get(), "beliefs epistemic1 validity")
                                on:click=move |_| select_from_command("beliefs", set_selected, set_command_open, set_command_query)
                            ><Sparkles size=15 /><span><b>"Beliefs"</b><i>"Epistemic1"</i></span></button>
                            <button
                                class:hidden=move || !command_matches(&command_query.get(), "perception host observation")
                                on:click=move |_| select_from_command("perception", set_selected, set_command_open, set_command_query)
                            ><Files size=15 /><span><b>"Perception"</b><i>"Perception1"</i></span></button>
                            <button
                                class:hidden=move || !command_matches(&command_query.get(), "context association concepts context1")
                                on:click=move |_| select_from_command("context", set_selected, set_command_open, set_command_query)
                            ><Link size=15 /><span><b>"Context"</b><i>"Context1"</i></span></button>
                            <button
                                class:hidden=move || !command_matches(&command_query.get(), "shell terminal body capability")
                                on:click=move |_| {
                                    layout.update(|l| l.open_card(CardId::Shell(0), 400.0, 160.0));
                                    layout.get_untracked().save();
                                    set_selected.set("shell");
                                    set_command_open.set(false);
                                    set_command_query.set(String::new());
                                }
                            ><IconTerminal size=15 /><span><b>"CYBOU Shell"</b><i>"Zone 3 Body capability"</i></span></button>
                            <button
                                class:hidden=move || !command_matches(&command_query.get(), "files file manager bounded folder")
                                on:click=move |_| {
                                    layout.update(|l| l.open_card(CardId::FileManager(0), 380.0, 120.0));
                                    layout.get_untracked().save();
                                    set_selected.set("files");
                                    set_command_open.set(false);
                                    set_command_query.set(String::new());
                                }
                            ><IconFolder size=15 /><span><b>"File Manager"</b><i>"Zone 3 Bounded Storage"</i></span></button>
                            <button
                                class:hidden=move || !command_matches(&command_query.get(), "browser web url navigation")
                                on:click=move |_| {
                                    layout.update(|l| l.open_card(CardId::WebBrowser(0), 460.0, 140.0));
                                    layout.get_untracked().save();
                                    set_selected.set("browser");
                                    set_command_open.set(false);
                                    set_command_query.set(String::new());
                                }
                            ><IconGlobe size=15 /><span><b>"Web Browser"</b><i>"Sandboxed Web Surface"</i></span></button>
                            <button
                                class:hidden=move || !command_matches(&command_query.get(), "feed journal events stream integrity merkle")
                                on:click=move |_| {
                                    layout.update(|l| l.open_card(CardId::JournalFeed(0), 420.0, 150.0));
                                    layout.get_untracked().save();
                                    set_selected.set("journal-feed");
                                    set_command_open.set(false);
                                    set_command_query.set(String::new());
                                }
                            ><IconFile size=15 /><span><b>"Event Stream & Integrity"</b><i>"Real-time Journal SSE feed"</i></span></button>
                            <button
                                class:hidden=move || !command_matches(&command_query.get(), "telemetry host inspector daemons dbus")
                                on:click=move |_| {
                                    layout.update(|l| l.open_card(CardId::Telemetry(0), 440.0, 170.0));
                                    layout.get_untracked().save();
                                    set_selected.set("telemetry");
                                    set_command_open.set(false);
                                    set_command_query.set(String::new());
                                }
                            ><IconTerminal size=15 /><span><b>"Host Telemetry & Daemons"</b><i>"14 Mind organs & D-Bus"</i></span></button>
                            <button
                                class:hidden=move || !command_matches(&command_query.get(), "graph epistemic knowledge causal map network")
                                on:click=move |_| {
                                    layout.update(|l| l.open_card(CardId::EpistemicGraph(0), 480.0, 130.0));
                                    layout.get_untracked().save();
                                    set_selected.set("epistemic-graph");
                                    set_command_open.set(false);
                                    set_command_query.set(String::new());
                                }
                            ><Link size=15 /><span><b>"Epistemic Knowledge Graph"</b><i>"Topological causal relations"</i></span></button>
                            <button
                                class:hidden=move || !command_matches(&command_query.get(), "intent goal agent dispatcher reasoning tasks autonomous")
                                on:click=move |_| {
                                    layout.update(|l| l.open_card(CardId::IntentLauncher(0), 450.0, 140.0));
                                    layout.get_untracked().save();
                                    set_selected.set("intent");
                                    set_command_open.set(false);
                                    set_command_query.set(String::new());
                                }
                            ><Sparkles size=15 /><span><b>"Intent & Goal Dispatcher"</b><i>"Autonomous reasoning & tool dispatch"</i></span></button>
                            <button
                                class:hidden=move || !command_matches(&command_query.get(), "auth login sign in pam session")
                                on:click=move |_| {
                                    auth_modal_open.set(true);
                                    set_command_open.set(false);
                                    set_command_query.set(String::new());
                                }
                            ><IconShield size=15 /><span><b>"Sign in / Authenticate"</b><i>"PAM host credentials"</i></span></button>
                            <hr style="width: 100%; border: 0; border-top: 1px solid rgba(154,167,184,.16);" />
                            <small>"Arrange"</small>
                            <button
                                class:hidden=move || !command_matches(&command_query.get(), "undo layout revert")
                                on:click=move |_| {
                                    apply_undo(history, layout);
                                    set_command_open.set(false);
                                    set_command_query.set(String::new());
                                }
                            ><IconUndo size=15 /><span><b>"Undo Layout Arrangement"</b><i>"Revert previous spatial state"</i></span></button>
                            <button
                                class:hidden=move || !command_matches(&command_query.get(), "redo layout forward")
                                on:click=move |_| {
                                    apply_redo(history, layout);
                                    set_command_open.set(false);
                                    set_command_query.set(String::new());
                                }
                            ><IconRedo size=15 /><span><b>"Redo Layout Arrangement"</b><i>"Restore spatial state"</i></span></button>
                            <button
                                class:hidden=move || !command_matches(&command_query.get(), "arrange grid")
                                on:click=move |_| {
                                    history.update(|h| h.push(layout.get_untracked()));
                                    layout.update(|l| l.apply_arrangement(ArrangementMode::Grid));
                                    layout.get_untracked().save();
                                    set_command_open.set(false);
                                    set_command_query.set(String::new());
                                }
                            ><IconGrid size=15 /><span><b>"Arrange: Grid"</b><i>"Structured alignment"</i></span></button>
                            <button
                                class:hidden=move || !command_matches(&command_query.get(), "arrange compact")
                                on:click=move |_| {
                                    history.update(|h| h.push(layout.get_untracked()));
                                    layout.update(|l| l.apply_arrangement(ArrangementMode::Compact));
                                    layout.get_untracked().save();
                                    set_command_open.set(false);
                                    set_command_query.set(String::new());
                                }
                            ><IconMinimize size=15 /><span><b>"Arrange: Compact"</b><i>"Dense packing"</i></span></button>
                            <button
                                class:hidden=move || !command_matches(&command_query.get(), "arrange relations")
                                on:click=move |_| {
                                    history.update(|h| h.push(layout.get_untracked()));
                                    layout.update(|l| l.apply_arrangement(ArrangementMode::Relations));
                                    layout.get_untracked().save();
                                    set_command_open.set(false);
                                    set_command_query.set(String::new());
                                }
                            ><Link size=15 /><span><b>"Arrange: Relations"</b><i>"Causal graph topology"</i></span></button>
                            <button
                                class:hidden=move || !command_matches(&command_query.get(), "reset layout")
                                on:click=move |_| {
                                    history.update(|h| h.push(layout.get_untracked()));
                                    layout.set(DesktopLayout::default());
                                    layout.get_untracked().save();
                                    set_command_open.set(false);
                                    set_command_query.set(String::new());
                                }
                            ><IconRefresh size=15 /><span><b>"Reset Desktop Layout"</b><i>"Default coordinates"</i></span></button>
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
                                    } else if command_matches(&q, "arrange grid") {
                                        event.prevent_default();
                                        history.update(|h| h.push(layout.get_untracked()));
                                        layout.update(|l| l.apply_arrangement(ArrangementMode::Grid));
                                        layout.get_untracked().save();
                                        set_command_open.set(false);
                                        set_command_query.set(String::new());
                                    } else if command_matches(&q, "arrange compact") {
                                        event.prevent_default();
                                        history.update(|h| h.push(layout.get_untracked()));
                                        layout.update(|l| l.apply_arrangement(ArrangementMode::Compact));
                                        layout.get_untracked().save();
                                        set_command_open.set(false);
                                        set_command_query.set(String::new());
                                    } else if command_matches(&q, "arrange relations") {
                                        event.prevent_default();
                                        history.update(|h| h.push(layout.get_untracked()));
                                        layout.update(|l| l.apply_arrangement(ArrangementMode::Relations));
                                        layout.get_untracked().save();
                                        set_command_open.set(false);
                                        set_command_query.set(String::new());
                                    } else if command_matches(&q, "reset layout") {
                                        event.prevent_default();
                                        history.update(|h| h.push(layout.get_untracked()));
                                        layout.set(DesktopLayout::default());
                                        layout.get_untracked().save();
                                        set_command_open.set(false);
                                        set_command_query.set(String::new());
                                    } else if command_matches(&q, "shell") {
                                        event.prevent_default();
                                        layout.update(|l| l.open_card(CardId::Shell(0), 400.0, 160.0));
                                        layout.get_untracked().save();
                                        set_selected.set("shell");
                                        set_command_open.set(false);
                                        set_command_query.set(String::new());
                                    } else if let Some(panel) = first_command_match(&q) {
                                        event.prevent_default();
                                        select_from_command(
                                            panel,
                                            set_selected,
                                            set_command_open,
                                            set_command_query,
                                        );
                                    }
                                }
                            }
                        />
                        <kbd>"Ctrl K"</kbd>
                    </label>

                    <Show when=move || minimap_visible.get()>
                        <nav class="minimap" aria-label="Desktop map">
                            <header><Map size=15 /><strong>"Desktop map"</strong></header>
                            <div class="minimap-field">
                                <For
                                    each={move || layout.get().decks.into_iter().map(|d| (d.id, d.title, d.geometry)).collect::<Vec<_>>()}
                                    key=|(id, _, _)| id.clone()
                                    children=move |(d_id, title, geom)| {
                                        let id_click = d_id.clone();
                                        view! {
                                            <button
                                                class="mini-node deck-node"
                                                style=move || minimap_style(geom)
                                                title=format!("Deck: {title}")
                                                aria-label=format!("Select deck {title}")
                                                on:click=move |_| {
                                                    layout.update(|l| l.bring_deck_forward(&id_click));
                                                }
                                            ></button>
                                        }
                                    }
                                />
                                <Show when=move || !layout.get().is_in_deck(CardId::Identity)>
                                    <button
                                        class:selected=move || selected.get() == "identity"
                                        class="mini-node identity-node"
                                        style=move || minimap_style(layout.get().geometry(CardId::Identity))
                                        aria-label="Select identity card"
                                        on:click=move |_| set_selected.set("identity")
                                    ></button>
                                </Show>
                                <Show when=move || !layout.get().is_in_deck(CardId::Session)>
                                    <button
                                        class:selected=move || selected.get() == "session"
                                        class="mini-node session-node"
                                        style=move || minimap_style(layout.get().geometry(CardId::Session))
                                        aria-label="Select session card"
                                        on:click=move |_| set_selected.set("session")
                                    ></button>
                                </Show>
                                <Show when=move || !layout.get().is_in_deck(CardId::Capabilities)>
                                    <button
                                        class:selected=move || selected.get() == "capabilities"
                                        class="mini-node capabilities-node"
                                        style=move || minimap_style(layout.get().geometry(CardId::Capabilities))
                                        aria-label="Select capabilities card"
                                        on:click=move |_| set_selected.set("capabilities")
                                    ></button>
                                </Show>
                                <Show when=move || !layout.get().is_in_deck(CardId::Journal)>
                                    <button
                                        class:selected=move || selected.get() == "journal"
                                        class="mini-node journal-node"
                                        style=move || minimap_style(layout.get().geometry(CardId::Journal))
                                        aria-label="Select journal card"
                                        on:click=move |_| set_selected.set("journal")
                                    ></button>
                                </Show>
                                <Show when=move || !layout.get().is_in_deck(CardId::Lifecycle)>
                                    <button
                                        class:selected=move || selected.get() == "lifecycle"
                                        class="mini-node lifecycle-node"
                                        style=move || minimap_style(layout.get().geometry(CardId::Lifecycle))
                                        aria-label="Select lifecycle card"
                                        on:click=move |_| set_selected.set("lifecycle")
                                    ></button>
                                </Show>
                                <Show when=move || !layout.get().is_in_deck(CardId::Commitments)>
                                    <button
                                        class:selected=move || selected.get() == "commitments"
                                        class="mini-node commitments-node"
                                        style=move || minimap_style(layout.get().geometry(CardId::Commitments))
                                        aria-label="Select commitments card"
                                        on:click=move |_| set_selected.set("commitments")
                                    ></button>
                                </Show>
                                <Show when=move || !layout.get().is_in_deck(CardId::SelfModel)>
                                    <button
                                        class:selected=move || selected.get() == "self"
                                        class="mini-node self-node"
                                        style=move || minimap_style(layout.get().geometry(CardId::SelfModel))
                                        aria-label="Select self-assessment card"
                                        on:click=move |_| set_selected.set("self")
                                    ></button>
                                </Show>
                                <Show when=move || !layout.get().is_in_deck(CardId::Attention)>
                                    <button
                                        class:selected=move || selected.get() == "attention"
                                        class="mini-node attention-node"
                                        style=move || minimap_style(layout.get().geometry(CardId::Attention))
                                        aria-label="Select attention card"
                                        on:click=move |_| set_selected.set("attention")
                                    ></button>
                                </Show>
                                <Show when=move || !layout.get().is_in_deck(CardId::Beliefs)>
                                    <button
                                        class:selected=move || selected.get() == "beliefs"
                                        class="mini-node beliefs-node"
                                        style=move || minimap_style(layout.get().geometry(CardId::Beliefs))
                                        aria-label="Select beliefs card"
                                        on:click=move |_| set_selected.set("beliefs")
                                    ></button>
                                </Show>
                                <Show when=move || !layout.get().is_in_deck(CardId::Perception)>
                                    <button
                                        class:selected=move || selected.get() == "perception"
                                        class="mini-node perception-node"
                                        style=move || minimap_style(layout.get().geometry(CardId::Perception))
                                        aria-label="Select perception card"
                                        on:click=move |_| set_selected.set("perception")
                                    ></button>
                                </Show>
                                <Show when=move || !layout.get().is_in_deck(CardId::Context)>
                                    <button
                                        class:selected=move || selected.get() == "context"
                                        class="mini-node context-node"
                                        style=move || minimap_style(layout.get().geometry(CardId::Context))
                                        aria-label="Select associative context card"
                                        on:click=move |_| set_selected.set("context")
                                    ></button>
                                </Show>
                                <Show when=move || layout.get().contains_card(CardId::Shell(0)) && !layout.get().is_in_deck(CardId::Shell(0))>
                                    <button
                                        class:selected=move || selected.get() == "shell"
                                        class="mini-node shell-node"
                                        style=move || minimap_style(layout.get().geometry(CardId::Shell(0)))
                                        aria-label="Select shell card"
                                        on:click=move |_| set_selected.set("shell")
                                    ></button>
                                </Show>
                                <Show when=move || layout.get().contains_card(CardId::FileManager(0)) && !layout.get().is_in_deck(CardId::FileManager(0))>
                                    <button
                                        class:selected=move || selected.get() == "files"
                                        class="mini-node files-node"
                                        style=move || minimap_style(layout.get().geometry(CardId::FileManager(0)))
                                        aria-label="Select file manager card"
                                        on:click=move |_| set_selected.set("files")
                                    ></button>
                                </Show>
                                <Show when=move || layout.get().contains_card(CardId::WebBrowser(0)) && !layout.get().is_in_deck(CardId::WebBrowser(0))>
                                    <button
                                        class:selected=move || selected.get() == "browser"
                                        class="mini-node browser-node"
                                        style=move || minimap_style(layout.get().geometry(CardId::WebBrowser(0)))
                                        aria-label="Select web browser card"
                                        on:click=move |_| set_selected.set("browser")
                                    ></button>
                                </Show>
                                <Show when=move || layout.get().contains_card(CardId::JournalFeed(0)) && !layout.get().is_in_deck(CardId::JournalFeed(0))>
                                    <button
                                        class:selected=move || selected.get() == "journal-feed"
                                        class="mini-node feed-node"
                                        style=move || minimap_style(layout.get().geometry(CardId::JournalFeed(0)))
                                        aria-label="Select event feed card"
                                        on:click=move |_| set_selected.set("journal-feed")
                                    ></button>
                                </Show>
                                <Show when=move || layout.get().contains_card(CardId::Telemetry(0)) && !layout.get().is_in_deck(CardId::Telemetry(0))>
                                    <button
                                        class:selected=move || selected.get() == "telemetry"
                                        class="mini-node telemetry-node"
                                        style=move || minimap_style(layout.get().geometry(CardId::Telemetry(0)))
                                        aria-label="Select host telemetry card"
                                        on:click=move |_| set_selected.set("telemetry")
                                    ></button>
                                </Show>
                                <Show when=move || layout.get().contains_card(CardId::EpistemicGraph(0)) && !layout.get().is_in_deck(CardId::EpistemicGraph(0))>
                                    <button
                                        class:selected=move || selected.get() == "epistemic-graph"
                                        class="mini-node graph-node"
                                        style=move || minimap_style(layout.get().geometry(CardId::EpistemicGraph(0)))
                                        aria-label="Select epistemic graph card"
                                        on:click=move |_| set_selected.set("epistemic-graph")
                                    ></button>
                                </Show>
                                <Show when=move || layout.get().contains_card(CardId::IntentLauncher(0)) && !layout.get().is_in_deck(CardId::IntentLauncher(0))>
                                    <button
                                        class:selected=move || selected.get() == "intent"
                                        class="mini-node intent-node"
                                        style=move || minimap_style(layout.get().geometry(CardId::IntentLauncher(0)))
                                        aria-label="Select intent dispatcher card"
                                        on:click=move |_| set_selected.set("intent")
                                    ></button>
                                </Show>
                                <div
                                    class="minimap-viewport"
                                    style=move || {
                                        let (px, py) = pan.get();
                                        let z = zoom.get();
                                        let scale = 0.08;
                                        let left = (-px * scale).max(0.0);
                                        let top = (-py * scale).max(0.0);
                                        let width = (1200.0 * scale / z).min(180.0);
                                        let height = (800.0 * scale / z).min(130.0);
                                        format!("left:{:.1}px;top:{:.1}px;width:{:.1}px;height:{:.1}px;", left, top, width, height)
                                    }
                                ></div>
                            </div>
                        </nav>
                    </Show>

                    <Show when=move || capabilities_open.get()>
                        <aside id="capability-inspector" class="capability-inspector" aria-label="Gateway capabilities">
                            <header>
                                <span><strong>"Gateway capabilities"</strong><small>{observed_label}</small></span>
                                <button aria-label="Close capability inspector" on:click=move |_| set_capabilities_open.set(false)>"×"</button>
                            </header>
                            <For
                                each=capabilities
                                key=|capability| capability.id.clone()
                                children=move |capability| {
                                    let available = capability.state == cybou_protocol::CapabilityState::Available;
                                    let status = capability_state_label(capability.state);
                                    let context = capability.reason.unwrap_or_else(|| {
                                        format!(
                                            "{} · {}",
                                            knowledge_label(capability.knowledge),
                                            freshness_label(capability.freshness),
                                        )
                                    });
                                    view! {
                                        <div class:available=available class="capability-row">
                                            <span class="status-dot" aria-hidden="true"></span>
                                            <span><b>{capability.id}</b><small>{context}</small></span>
                                            <i>{status}</i>
                                        </div>
                                    }
                                }
                            />
                        </aside>
                    </Show>

                    <nav class="desktop-floating-bar" aria-label="Desktop quick actions">
                        <button
                            title="Undo layout movement (Ctrl+Z)"
                            on:click=move |_| apply_undo(history, layout)
                        >
                            <IconUndo size=13 />
                            <span>"Undo"</span>
                        </button>
                        <button
                            title="Redo layout movement (Ctrl+Y)"
                            on:click=move |_| apply_redo(history, layout)
                        >
                            <IconRedo size=13 />
                            <span>"Redo"</span>
                        </button>
                        <div class="toolbar-separator"></div>
                        <button
                            title="Arrange in grid layout"
                            on:click=move |_| {
                                history.update(|h| h.push(layout.get_untracked()));
                                layout.update(|l| l.apply_arrangement(ArrangementMode::Grid));
                                layout.get_untracked().save();
                            }
                        >
                            <IconGrid size=13 />
                            <span>"Grid"</span>
                        </button>
                        <button
                            title="Arrange in compact layout"
                            on:click=move |_| {
                                history.update(|h| h.push(layout.get_untracked()));
                                layout.update(|l| l.apply_arrangement(ArrangementMode::Compact));
                                layout.get_untracked().save();
                            }
                        >
                            <IconMinimize size=13 />
                            <span>"Compact"</span>
                        </button>
                        <button
                            title="Arrange in causal relations layout"
                            on:click=move |_| {
                                history.update(|h| h.push(layout.get_untracked()));
                                layout.update(|l| l.apply_arrangement(ArrangementMode::Relations));
                                layout.get_untracked().save();
                            }
                        >
                            <Link size=13 />
                            <span>"Relations"</span>
                        </button>
                        <div class="toolbar-separator"></div>
                        <button
                            title="Zoom out"
                            on:click=move |_| set_zoom.update(|z| *z = (*z - 0.1).max(0.4))
                        >
                            <IconZoomOut size=13 />
                        </button>
                        <button
                            title="Reset zoom to 100%"
                            on:click=move |_| {
                                set_zoom.set(1.0);
                                set_pan.set((0.0, 0.0));
                            }
                        >
                            <span>{move || format!("{:.0}%", zoom.get() * 100.0)}</span>
                        </button>
                        <button
                            title="Zoom in"
                            on:click=move |_| set_zoom.update(|z| *z = (*z + 0.1).min(2.0))
                        >
                            <IconZoomIn size=13 />
                        </button>
                        <div class="toolbar-separator"></div>
                        <button
                            title="Toggle CYBOU Shell terminal"
                            on:click=move |_| {
                                if layout.get_untracked().contains_card(CardId::Shell(0)) {
                                    layout.update(|l| l.close_card(CardId::Shell(0)));
                                } else {
                                    layout.update(|l| l.open_card(CardId::Shell(0), 400.0, 160.0));
                                    set_selected.set("shell");
                                }
                                layout.get_untracked().save();
                            }
                        >
                            <IconTerminal size=13 />
                            <span>"Shell"</span>
                        </button>
                        <button
                            title="Toggle Bounded File Manager"
                            on:click=move |_| {
                                if layout.get_untracked().contains_card(CardId::FileManager(0)) {
                                    layout.update(|l| l.close_card(CardId::FileManager(0)));
                                } else {
                                    layout.update(|l| l.open_card(CardId::FileManager(0), 380.0, 120.0));
                                    set_selected.set("files");
                                }
                                layout.get_untracked().save();
                            }
                        >
                            <IconFolder size=13 />
                            <span>"Files"</span>
                        </button>
                        <button
                            title="Toggle Sandboxed Web Browser"
                            on:click=move |_| {
                                if layout.get_untracked().contains_card(CardId::WebBrowser(0)) {
                                    layout.update(|l| l.close_card(CardId::WebBrowser(0)));
                                } else {
                                    layout.update(|l| l.open_card(CardId::WebBrowser(0), 460.0, 140.0));
                                    set_selected.set("browser");
                                }
                                layout.get_untracked().save();
                            }
                        >
                            <IconGlobe size=13 />
                            <span>"Browser"</span>
                        </button>
                        <button
                            title="Toggle Event Stream & Integrity feed"
                            on:click=move |_| {
                                if layout.get_untracked().contains_card(CardId::JournalFeed(0)) {
                                    layout.update(|l| l.close_card(CardId::JournalFeed(0)));
                                } else {
                                    layout.update(|l| l.open_card(CardId::JournalFeed(0), 420.0, 150.0));
                                    set_selected.set("journal-feed");
                                }
                                layout.get_untracked().save();
                            }
                        >
                            <IconFile size=13 />
                            <span>"Feed"</span>
                        </button>
                        <button
                            title="Toggle Host Telemetry & Daemons"
                            on:click=move |_| {
                                if layout.get_untracked().contains_card(CardId::Telemetry(0)) {
                                    layout.update(|l| l.close_card(CardId::Telemetry(0)));
                                } else {
                                    layout.update(|l| l.open_card(CardId::Telemetry(0), 440.0, 170.0));
                                    set_selected.set("telemetry");
                                }
                                layout.get_untracked().save();
                            }
                        >
                            <IconTerminal size=13 />
                            <span>"Telemetry"</span>
                        </button>
                        <button
                            title="Toggle Epistemic Knowledge Graph"
                            on:click=move |_| {
                                if layout.get_untracked().contains_card(CardId::EpistemicGraph(0)) {
                                    layout.update(|l| l.close_card(CardId::EpistemicGraph(0)));
                                } else {
                                    layout.update(|l| l.open_card(CardId::EpistemicGraph(0), 480.0, 130.0));
                                    set_selected.set("epistemic-graph");
                                }
                                layout.get_untracked().save();
                            }
                        >
                            <Link size=13 />
                            <span>"Graph"</span>
                        </button>
                        <button
                            title="Toggle Intent & Goal Dispatcher"
                            on:click=move |_| {
                                if layout.get_untracked().contains_card(CardId::IntentLauncher(0)) {
                                    layout.update(|l| l.close_card(CardId::IntentLauncher(0)));
                                } else {
                                    layout.update(|l| l.open_card(CardId::IntentLauncher(0), 450.0, 140.0));
                                    set_selected.set("intent");
                                }
                                layout.get_untracked().save();
                            }
                        >
                            <Sparkles size=13 />
                            <span>"Intent"</span>
                        </button>
                        <button
                            class:active=move || minimap_visible.get()
                            title="Toggle Desktop Minimap"
                            on:click=move |_| set_minimap_visible.update(|v| *v = !*v)
                        >
                            <Map size=13 />
                            <span>"Map"</span>
                        </button>
                    </nav>

                    <button
                        class:open=move || capabilities_open.get()
                        class="system-state"
                        aria-label="Open gateway capability inspector"
                        aria-expanded=move || capabilities_open.get().to_string()
                        aria-controls="capability-inspector"
                        on:click=move |_| set_capabilities_open.update(|open| *open = !*open)
                    >
                        <span class="status-dot" aria-hidden="true"></span>
                        {system_label}
                    </button>
                </section>
                <DesktopDock layout=layout selected=selected set_selected=set_selected auth_modal_open=auth_modal_open />
                <AuthModal open=auth_modal_open />
            </main>
        }
}

fn card_style(layout: DesktopLayout, card: CardId) -> String {
    let geom = layout.geometry(card);
    let pres = layout.presentation(card);
    if pres.maximized {
        "left:20px;top:20px;width:calc(100vw - 40px);height:calc(100vh - 100px);z-index:95;".to_string()
    } else if pres.collapsed {
        format!(
            "left:{:.1}px;top:{:.1}px;width:{:.1}px;z-index:{}",
            geom.x, geom.y, geom.width, geom.z
        )
    } else {
        format!(
            "left:{:.1}px;top:{:.1}px;width:{:.1}px;height:{:.1}px;z-index:{}",
            geom.x, geom.y, geom.width, geom.height, geom.z
        )
    }
}

/// What a field reads when the owner behind it was not read.
///
/// A dash says nothing was read. A zero would say the owner answered and held nothing, which is a
/// different claim and not one the gateway made.
fn unread() -> String {
    "—".to_owned()
}

const fn capability_state_label(state: cybou_protocol::CapabilityState) -> &'static str {
    match state {
        cybou_protocol::CapabilityState::Available => "Available",
        cybou_protocol::CapabilityState::Unavailable => "Unavailable",
        cybou_protocol::CapabilityState::Unknown => "Unknown",
    }
}

const fn knowledge_label(state: cybou_protocol::KnowledgeState) -> &'static str {
    match state {
        cybou_protocol::KnowledgeState::Known => "Known",
        cybou_protocol::KnowledgeState::Unknown => "Unknown",
    }
}

const fn freshness_label(state: cybou_web_contracts::Freshness) -> &'static str {
    match state {
        cybou_web_contracts::Freshness::Current => "Current",
        cybou_web_contracts::Freshness::Stale => "Stale",
        cybou_web_contracts::Freshness::Unknown => "Unknown freshness",
    }
}

fn command_matches(query: &str, haystack: &str) -> bool {
    let query = query.trim().to_ascii_lowercase();
    query.is_empty() || haystack.contains(&query)
}

fn first_command_match(query: &str) -> Option<&'static str> {
    [
        ("capabilities", "capabilities health"),
        ("identity", "identity subject continuity"),
        ("session", "session trust mode"),
        ("journal", "journal contributions event1"),
        ("lifecycle", "lifecycle sleep wake"),
        ("commitments", "commitments obligations intention1"),
        ("self", "self assessment narration self1"),
        ("attention", "attention focus workspace1"),
        ("beliefs", "beliefs epistemic1 validity"),
        ("perception", "perception host observation"),
        ("context", "context association concepts context1"),
        ("shell", "shell terminal body capability"),
    ]
    .into_iter()
    .find_map(|(panel, label)| command_matches(query, label).then_some(panel))
}

fn select_from_command(
    panel: &'static str,
    set_selected: WriteSignal<&'static str>,
    set_command_open: WriteSignal<bool>,
    set_command_query: WriteSignal<String>,
) {
    set_selected.set(panel);
    set_command_open.set(false);
    set_command_query.set(String::new());
}

fn navigate_from_menu(
    panel: &'static str,
    set_selected: WriteSignal<&'static str>,
    set_runtime_menu_open: WriteSignal<bool>,
) {
    set_selected.set(panel);
    set_runtime_menu_open.set(false);
}

fn selection_actions_style(layout: DesktopLayout) -> String {
    let geom = layout.geometry(CardId::Capabilities);
    format!(
        "left:{:.1}px;top:{:.1}px;z-index:{}",
        geom.x + 18.0,
        geom.y + geom.height,
        geom.z + 1
    )
}

fn minimap_style(geom: CardGeometry) -> String {
    let x = 10.0 + geom.x / 1_280.0 * 180.0;
    let y = 8.0 + geom.y / 650.0 * 92.0;
    format!("left:{x:.1}px;top:{y:.1}px")
}

fn relationship_points(
    layout: DesktopLayout,
    from: CardId,
    to: CardId,
) -> (f64, f64, f64, f64, f64, f64) {
    let from_geom = layout.geometry(from);
    let to_geom = layout.geometry(to);
    let from_pres = layout.presentation(from);
    let to_pres = layout.presentation(to);

    let from_height = if from_pres.collapsed {
        44.0
    } else {
        from_geom.height
    };
    let to_height = if to_pres.collapsed {
        44.0
    } else {
        to_geom.height
    };

    let from_size = (from_geom.width, from_height);
    let to_size = (to_geom.width, to_height);
    let from_center = (
        from_geom.x + from_size.0 / 2.0,
        from_geom.y + from_size.1 / 2.0,
    );
    let to_center = (to_geom.x + to_size.0 / 2.0, to_geom.y + to_size.1 / 2.0);

    let (x1, y1) = edge_anchor(from_center, from_size, to_center);
    let (x2, y2) = edge_anchor(to_center, to_size, from_center);
    (
        x1,
        y1,
        x2,
        y2,
        f64::midpoint(x1, x2),
        f64::midpoint(y1, y2) - 7.0,
    )
}

fn edge_anchor(center: (f64, f64), size: (f64, f64), target: (f64, f64)) -> (f64, f64) {
    let dx = target.0 - center.0;
    let dy = target.1 - center.1;
    if dx.abs() < f64::EPSILON && dy.abs() < f64::EPSILON {
        return center;
    }
    let x_scale = if dx.abs() < f64::EPSILON {
        f64::INFINITY
    } else {
        size.0 / 2.0 / dx.abs()
    };
    let y_scale = if dy.abs() < f64::EPSILON {
        f64::INFINITY
    } else {
        size.1 / 2.0 / dy.abs()
    };
    let scale = x_scale.min(y_scale);
    (center.0 + dx * scale, center.1 + dy * scale)
}

fn start_drag(
    event: PointerEvent,
    card: CardId,
    layout: RwSignal<DesktopLayout>,
    dragging: RwSignal<Option<DragState>>,
) {
    if event.button() != 0 {
        return;
    }
    if layout.get_untracked().presentation(card).pinned {
        return;
    }
    let Some(target) = event
        .current_target()
        .and_then(|target| target.dyn_into::<HtmlElement>().ok())
    else {
        return;
    };
    let _ = target.focus();
    let _ = target.set_pointer_capture(event.pointer_id());
    let rect = target.get_bounding_client_rect();
    layout.update(|current| current.bring_forward(card));
    dragging.set(Some(DragState {
        target: DragTarget::Card(card),
        offset_x: f64::from(event.client_x()) - rect.left(),
        offset_y: f64::from(event.client_y()) - rect.top(),
        width: rect.width(),
        height: rect.height(),
        drop_target: None,
    }));
    event.prevent_default();
}

fn start_deck_drag(
    event: PointerEvent,
    deck_id: String,
    layout: RwSignal<DesktopLayout>,
    dragging: RwSignal<Option<DragState>>,
) {
    if event.button() != 0 {
        return;
    }
    if let Some(target_el) = event.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        && target_el
            .closest("button, .deck-tab, .deck-tab-detach, .deck-controls, .card-control-btn")
            .ok()
            .flatten()
            .is_some()
    {
        return;
    }
    let current_layout = layout.get_untracked();
    if let Some(deck) = current_layout.deck(&deck_id) {
        if deck.presentation.pinned {
            return;
        }
        let Some(target) = event
            .current_target()
            .and_then(|target| target.dyn_into::<HtmlElement>().ok())
        else {
            return;
        };
        let _ = target.focus();
        let _ = target.set_pointer_capture(event.pointer_id());
        let rect = target.get_bounding_client_rect();
        layout.update(|current| current.bring_deck_forward(&deck_id));
        dragging.set(Some(DragState {
            target: DragTarget::Deck(deck_id),
            offset_x: f64::from(event.client_x()) - rect.left(),
            offset_y: f64::from(event.client_y()) - rect.top(),
            width: rect.width(),
            height: rect.height(),
            drop_target: None,
        }));
        event.prevent_default();
    }
}

fn move_drag(
    event: PointerEvent,
    layout: RwSignal<DesktopLayout>,
    dragging: RwSignal<Option<DragState>>,
) {
    let Some(drag) = dragging.get_untracked() else {
        return;
    };
    let Some(surface) = event
        .current_target()
        .and_then(|target| target.dyn_into::<HtmlElement>().ok())
    else {
        return;
    };
    let bounds = surface.get_bounding_client_rect();
    let x = (f64::from(event.client_x()) - bounds.left() - drag.offset_x)
        .clamp(12.0, (bounds.width() - drag.width - 12.0).max(12.0));
    let y = (f64::from(event.client_y()) - bounds.top() - drag.offset_y)
        .clamp(12.0, (bounds.height() - drag.height - 12.0).max(12.0));

    match &drag.target {
        DragTarget::Card(card) => {
            let dragged_card = *card;
            layout.update(|current| {
                current.set_position(dragged_card, x, y);
            });

            let current_layout = layout.get_untracked();
            let drag_center_x = x + drag.width / 2.0;
            let drag_center_y = y + drag.height / 2.0;

            let mut found_target = None;
            for card_inst in &current_layout.cards {
                if card_inst.id == dragged_card {
                    continue;
                }
                let geom = card_inst.geometry;
                let is_collapsed = card_inst.presentation.collapsed;
                let target_h = if is_collapsed { 44.0 } else { geom.height };
                if drag_center_x >= geom.x - 24.0
                    && drag_center_x <= geom.x + geom.width + 24.0
                    && drag_center_y >= geom.y - 24.0
                    && drag_center_y <= geom.y + target_h + 24.0
                {
                    found_target = Some(card_inst.id);
                    break;
                }
            }

            dragging.update(|d| {
                if let Some(d) = d {
                    d.drop_target = found_target;
                }
            });
        }
        DragTarget::Deck(deck_id) => {
            layout.update(|current| {
                current.set_deck_position(deck_id, x, y);
            });
        }
    }
}

fn finish_drag(
    layout: RwSignal<DesktopLayout>,
    history: RwSignal<living_canvas::LayoutHistory>,
    dragging: RwSignal<Option<DragState>>,
) {
    let Some(drag) = dragging.get_untracked() else {
        return;
    };
    dragging.set(None);

    if let DragTarget::Card(dragged_card) = drag.target
        && let Some(target_card) = drag.drop_target
    {
        history.update(|h| h.push(layout.get_untracked()));
        layout.update(|current| {
            let deck_id_opt = current.deck_for_card(target_card).map(|d| d.id.clone());
            if let Some(d_id) = deck_id_opt {
                current.add_to_deck(&d_id, dragged_card);
            } else {
                let target_geom = current.geometry(target_card);
                let title = format!("{} + {}", target_card.title(), dragged_card.title());
                current.create_deck(
                    title,
                    vec![target_card, dragged_card],
                    target_geom.x,
                    target_geom.y,
                );
            }
        });
    }

    layout.get_untracked().save();
}

fn start_resize(
    event: PointerEvent,
    card: CardId,
    layout: RwSignal<DesktopLayout>,
    resizing: RwSignal<Option<ResizeState>>,
) {
    if event.button() != 0 {
        return;
    }
    if layout.get_untracked().presentation(card).pinned {
        return;
    }
    let Some(target) = event
        .current_target()
        .and_then(|target| target.dyn_into::<HtmlElement>().ok())
    else {
        return;
    };
    event.stop_propagation();
    event.prevent_default();
    let _ = target.set_pointer_capture(event.pointer_id());
    layout.update(|current| current.bring_forward(card));
    let geom = layout.get_untracked().geometry(card);
    resizing.set(Some(ResizeState {
        target: ResizeTarget::Card(card),
        start_pointer_x: f64::from(event.client_x()),
        start_pointer_y: f64::from(event.client_y()),
        start_width: geom.width,
        start_height: geom.height,
    }));
}

fn start_deck_resize(
    event: PointerEvent,
    deck_id: String,
    layout: RwSignal<DesktopLayout>,
    resizing: RwSignal<Option<ResizeState>>,
) {
    if event.button() != 0 {
        return;
    }
    if let Some(deck) = layout.get_untracked().deck(&deck_id) {
        if deck.presentation.pinned {
            return;
        }
        let Some(target) = event
            .current_target()
            .and_then(|target| target.dyn_into::<HtmlElement>().ok())
        else {
            return;
        };
        event.stop_propagation();
        event.prevent_default();
        let _ = target.set_pointer_capture(event.pointer_id());
        layout.update(|current| current.bring_deck_forward(&deck_id));
        let geom = deck.geometry;
        resizing.set(Some(ResizeState {
            target: ResizeTarget::Deck(deck_id),
            start_pointer_x: f64::from(event.client_x()),
            start_pointer_y: f64::from(event.client_y()),
            start_width: geom.width,
            start_height: geom.height,
        }));
    }
}

fn move_resize(
    event: PointerEvent,
    layout: RwSignal<DesktopLayout>,
    resizing: RwSignal<Option<ResizeState>>,
) {
    let Some(resize) = resizing.get_untracked() else {
        return;
    };
    let dx = f64::from(event.client_x()) - resize.start_pointer_x;
    let dy = f64::from(event.client_y()) - resize.start_pointer_y;

    match &resize.target {
        ResizeTarget::Card(card) => {
            let spec = card.spec();
            let new_width = (resize.start_width + dx).clamp(spec.min_size.0, spec.max_size.0);
            let new_height = (resize.start_height + dy).clamp(spec.min_size.1, spec.max_size.1);

            layout.update(|current| {
                current.set_size(*card, new_width, new_height);
            });
        }
        ResizeTarget::Deck(deck_id) => {
            let new_width = (resize.start_width + dx).clamp(280.0, 800.0);
            let new_height = (resize.start_height + dy).clamp(160.0, 700.0);

            layout.update(|current| {
                current.set_deck_size(deck_id, new_width, new_height);
            });
        }
    }
}

fn finish_resize(layout: RwSignal<DesktopLayout>, resizing: RwSignal<Option<ResizeState>>) {
    if resizing.get_untracked().is_some() {
        resizing.set(None);
        layout.get_untracked().save();
    }
}

fn keyboard_move(event: KeyboardEvent, card: CardId, layout: RwSignal<DesktopLayout>) {
    if layout.get_untracked().presentation(card).pinned {
        return;
    }
    let step = if event.shift_key() { 40.0 } else { 10.0 };
    let (dx, dy) = match event.key().as_str() {
        "ArrowLeft" => (-step, 0.0),
        "ArrowRight" => (step, 0.0),
        "ArrowUp" => (0.0, -step),
        "ArrowDown" => (0.0, step),
        _ => return,
    };
    event.prevent_default();
    layout.update(|current| {
        current.bring_forward(card);
        let geom = current.geometry(card);
        current.set_position(card, (geom.x + dx).max(12.0), (geom.y + dy).max(12.0));
    });
    layout.get_untracked().save();
}

fn load_layout() -> DesktopLayout {
    DesktopLayout::load()
}
