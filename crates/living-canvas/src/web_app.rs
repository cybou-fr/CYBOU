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
fn CardControls(card: CardId, layout: RwSignal<DesktopLayout>) -> impl IntoView {
    let is_pinned = move || layout.get().presentation(card).pinned;
    let is_collapsed = move || layout.get().presentation(card).collapsed;

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
                {move || if is_collapsed() {
                    view! { <IconMaximize size=12 /> }.into_any()
                } else {
                    view! { <IconMinimize size=12 /> }.into_any()
                }}
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
    let card_open = move || layout.get().contains_card(card_id);
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
                }
                on:pointerdown=move |event: PointerEvent| start_drag(event, card_id, layout, dragging)
                on:keydown=move |event: KeyboardEvent| keyboard_move(event, card_id, layout)
            >
                <header class="card-header">
                    <small class="panel-kicker"><IconTerminal size=14 /><span>"Shell · Zone 3 Body"</span></small>
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
                    <div class="shell-body">
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
                        <div class="shell-input-line">
                            <span class="shell-prompt">{move || format!("cybou:{} $", cwd.get())}</span>
                            <input
                                type="text"
                                class="shell-input"
                                placeholder=move || if running.get() { "running…" } else { "type a command ('help')…" }
                                disabled=move || running.get()
                                prop:value=move || input_val.get()
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
                    <div class="deck-tabs">
                        <For
                            each=move || cards.get()
                            key=|card| *card
                            children=move |card| {
                                let is_active = move || active_card.get() == card;
                                view! {
                                    <div
                                        class="deck-tab"
                                        class:active=is_active
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
                    <div class="deck-controls">
                        <button
                            class="card-control-btn"
                            title="Ungroup deck into separate cards"
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
                    aria-label="CYBOU Desktop"
                    on:pointermove=move |event: PointerEvent| {
                        // One pointer move can be both: a card being dragged and another being
                        // resized are separate gestures, and each helper reads the same event.
                        move_drag(event.clone(), layout, dragging);
                        move_resize(event, layout, resizing);
                    }
                    on:pointerup=move |_| {
                        finish_drag(layout, history, dragging);
                        finish_resize(layout, resizing);
                    }
                    on:pointercancel=move |_| {
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
            </main>
        }
}

fn card_style(layout: DesktopLayout, card: CardId) -> String {
    let geom = layout.geometry(card);
    let pres = layout.presentation(card);
    if pres.collapsed {
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
