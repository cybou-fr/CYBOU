// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Desktop bottom taskbar and shelf dock component.

use cybou_web_contracts::SessionMode;
use leptos::prelude::*;
use wasm_bindgen::{JsCast, closure::Closure};

use crate::instant_label;
use crate::{
    ArrangementMode, CardId, DesktopItemId, DesktopLayout,
    components::icons::{
        IconActivity, IconBell, IconBot, IconFile, IconFolder, IconGrid, IconHome, IconLayers,
        IconRefresh, IconShield, IconTerminal,
    },
    state::RuntimeState,
};

/// Bottom desktop taskbar dock component.
#[component]
pub fn DesktopDock(
    layout: RwSignal<DesktopLayout>,
    selected: ReadSignal<Option<DesktopItemId>>,
    set_selected: WriteSignal<Option<DesktopItemId>>,
    auth_modal_open: RwSignal<bool>,
    runtime: RwSignal<RuntimeState>,
) -> impl IntoView {
    let (time_str, set_time_str) = signal(String::new());
    let pan = use_context::<ReadSignal<(f64, f64)>>();
    let set_pan = use_context::<WriteSignal<(f64, f64)>>();
    let zoom = use_context::<ReadSignal<f64>>();
    let set_zoom = use_context::<WriteSignal<f64>>();
    let camera_history = use_context::<RwSignal<crate::CameraHistory>>();

    #[cfg(target_arch = "wasm32")]
    {
        let update_clock = move || {
            let d = js_sys::Date::new_0();
            set_time_str.set(format!(
                "{:02}:{:02}:{:02} UTC",
                d.get_utc_hours(),
                d.get_utc_minutes(),
                d.get_utc_seconds()
            ));
        };
        update_clock();
        if let Some(w) = web_sys::window() {
            let cb = Closure::<dyn FnMut()>::new(update_clock);
            let _ = w.set_interval_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref(),
                1000,
            );
            cb.forget();
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        set_time_str.set("12:00:00 UTC".to_string());
    }

    let is_public_preview = move || match runtime.get() {
        RuntimeState::Ready { mode, .. } => mode == SessionMode::PublicPreview,
        _ => false,
    };

    let user_label = move || match runtime.get() {
        RuntimeState::SignInRequired => "Not signed in".to_string(),
        RuntimeState::Ready { mode, mind, .. } => match mode {
            SessionMode::PublicPreview => "Open to anyone".to_string(),
            SessionMode::SignInRequired => "Not signed in".to_string(),
            SessionMode::LocalDesktop => "This machine".to_string(),
            SessionMode::RemoteBrowser => {
                if let Some(m) = mind {
                    if let Some(origin) = m.identity.origin {
                        // The subject's origin instant, which is what Identity1 holds. Shown as a
                        // date rather than a nanosecond string; the exact value is on the card.
                        format!("Since {} ●", instant_label(&origin))
                    } else {
                        "authenticated ●".to_string()
                    }
                } else {
                    "authenticated ●".to_string()
                }
            }
        },
        RuntimeState::Loading => "connecting…".to_string(),
        RuntimeState::Error(_) => "offline ✕".to_string(),
    };

    let open_or_focus = move |card_id: CardId, def_w: f64, def_h: f64| {
        if !layout.get().contains_card(card_id) {
            layout.update(|l| l.open_card(card_id, def_w, def_h));
        } else if layout.get().presentation(card_id).collapsed {
            layout.update(|l| l.set_collapsed(card_id, false));
        }
        layout.update(|l| l.bring_forward(card_id));
        set_selected.set(Some(DesktopItemId::Card(card_id)));
        layout.get_untracked().save();
    };

    let focus_anchor = move |anchor_id: String| {
        let current_layout = layout.get_untracked();
        let Some(anchor) = current_layout
            .anchors
            .iter()
            .find(|anchor| anchor.id == anchor_id)
        else {
            return;
        };
        if let (Some(pan), Some(set_pan), Some(zoom), Some(set_zoom)) =
            (pan, set_pan, zoom, set_zoom)
        {
            crate::apply_camera_fly_to(
                camera_history,
                pan,
                set_pan,
                zoom,
                set_zoom,
                anchor.center_x,
                anchor.center_y,
                anchor.preferred_zoom,
            );
        }
    };

    view! {
        <footer class="desktop-dock" aria-label="Desktop Card Shelf and Taskbar">
            <div class="dock-apps">
                <button
                    class="dock-item"
                    class:active=move || selected.get().as_ref().is_some_and(|item| matches!(item, DesktopItemId::Card(card) if card.key() == "insight"))
                    title="Home / System Overview"
                    on:click=move |_| {
                        layout.update(|l| l.apply_arrangement(ArrangementMode::Home, None));
                        open_or_focus(CardId::Insight, 380.0, 480.0);
                    }
                >
                    <IconHome size=18 />
                    <span class="dock-tooltip">"Home"</span>
                </button>
                <button
                    class="dock-item"
                    class:active=move || selected.get().as_ref().is_some_and(|item| matches!(item, DesktopItemId::Card(card) if card.key() == "agents"))
                    title="Agents"
                    on:click=move |_| open_or_focus(CardId::Agents, 460.0, 480.0)
                >
                    <lucide_leptos::UsersRound size=18 />
                    <span class="dock-tooltip">"Agents"</span>
                </button>
                <button
                    class="dock-item"
                    class:active=move || selected.get().as_ref().is_some_and(|item| matches!(item, DesktopItemId::Card(card) if card.key() == "files"))
                    title="File Manager"
                    on:click=move |_| open_or_focus(CardId::FileManager(0), 380.0, 320.0)
                >
                    <IconFolder size=18 />
                    <span class="dock-tooltip">"Files"</span>
                </button>
                <button
                    class="dock-item"
                    class:active=move || selected.get().as_ref().is_some_and(|item| matches!(item, DesktopItemId::Card(card) if card.key() == "editor"))
                    title="Text Editor"
                    on:click=move |_| open_or_focus(CardId::Editor(0), 400.0, 200.0)
                >
                    <IconFile size=18 />
                    <span class="dock-tooltip">"Editor"</span>
                </button>
                <button
                    class="dock-item"
                    class:active=move || selected.get().as_ref().is_some_and(|item| matches!(item, DesktopItemId::Card(card) if card.key() == "diff"))
                    title="Diff Viewer"
                    on:click=move |_| open_or_focus(CardId::Diff(0), 420.0, 220.0)
                >
                    <IconFile size=18 />
                    <span class="dock-tooltip">"Diff"</span>
                </button>
                <button
                    class="dock-item"
                    class:active=move || selected.get().as_ref().is_some_and(|item| matches!(item, DesktopItemId::Card(card) if card.key() == "shell"))
                    title="Shell"
                    on:click=move |_| open_or_focus(CardId::Shell(0), 400.0, 240.0)
                >
                    <IconTerminal size=18 />
                    <span class="dock-tooltip">"Shell"</span>
                </button>
                <button
                    class="dock-item"
                    class:active=move || selected.get().as_ref().is_some_and(|item| matches!(item, DesktopItemId::Card(card) if card.key() == "inspector"))
                    title="Universal Inspector"
                    on:click=move |_| open_or_focus(CardId::Inspector(0), 380.0, 480.0)
                >
                    <IconLayers size=18 />
                    <span class="dock-tooltip">"Inspector"</span>
                </button>
                <button
                    class="dock-item"
                    class:active=move || selected.get().as_ref().is_some_and(|item| matches!(item, DesktopItemId::Card(card) if card.key() == "operations"))
                    title="Operations Monitor"
                    on:click=move |_| open_or_focus(CardId::Operations(0), 560.0, 420.0)
                >
                    <IconActivity size=18 />
                    <span class="dock-tooltip">"Operations"</span>
                </button>
                <button
                    class="dock-item"
                    class:active=move || selected.get().as_ref().is_some_and(|item| matches!(item, DesktopItemId::Card(card) if card.key() == "notifications"))
                    title="Notifications Center"
                    on:click=move |_| open_or_focus(CardId::Notifications(0), 520.0, 440.0)
                >
                    <IconBell size=18 />
                    <span class="dock-tooltip">"Notifications"</span>
                </button>
                <button
                    class="dock-item"
                    class:active=move || selected.get().as_ref().is_some_and(|item| matches!(item, DesktopItemId::Card(card) if card.key() == "services"))
                    title="Services Manager"
                    on:click=move |_| open_or_focus(CardId::Services(0), 580.0, 420.0)
                >
                    <IconRefresh size=18 />
                    <span class="dock-tooltip">"Services"</span>
                </button>
                <button
                    class="dock-item"
                    class:active=move || selected.get().as_ref().is_some_and(|item| matches!(item, DesktopItemId::Card(card) if card.key() == "processes"))
                    title="Process Manager"
                    on:click=move |_| open_or_focus(CardId::Processes(0), 600.0, 440.0)
                >
                    <IconLayers size=18 />
                    <span class="dock-tooltip">"Processes"</span>
                </button>
                <button
                    class="dock-item"
                    class:active=move || selected.get().as_ref().is_some_and(|item| matches!(item, DesktopItemId::Card(card) if card.key() == "monitor"))
                    title="System Monitor"
                    on:click=move |_| open_or_focus(CardId::Monitor(0), 560.0, 460.0)
                >
                    <IconActivity size=18 />
                    <span class="dock-tooltip">"Monitor"</span>
                </button>
                <button
                    class="dock-item"
                    class:active=move || selected.get().as_ref().is_some_and(|item| matches!(item, DesktopItemId::Card(card) if card.key() == "system-logs"))
                    title="System Logs"
                    on:click=move |_| open_or_focus(CardId::SystemLogs(0), 600.0, 420.0)
                >
                    <IconFile size=18 />
                    <span class="dock-tooltip">"Logs"</span>
                </button>
                <button
                    class="dock-item"
                    class:active=move || selected.get().as_ref().is_some_and(|item| matches!(item, DesktopItemId::Card(card) if card.key() == "storage"))
                    title="Storage & Snapshots"
                    on:click=move |_| open_or_focus(CardId::Storage(0), 580.0, 440.0)
                >
                    <IconFile size=18 />
                    <span class="dock-tooltip">"Storage"</span>
                </button>
                <button
                    class="dock-item"
                    class:active=move || selected.get().as_ref().is_some_and(|item| matches!(item, DesktopItemId::Card(card) if card.key() == "network"))
                    title="Network Connections"
                    on:click=move |_| open_or_focus(CardId::Network(0), 560.0, 420.0)
                >
                    <IconActivity size=18 />
                    <span class="dock-tooltip">"Network"</span>
                </button>
                <button
                    class="dock-item"
                    class:active=move || selected.get().as_ref().is_some_and(|item| matches!(item, DesktopItemId::Card(card) if card.key() == "packages"))
                    title="Package Manager"
                    on:click=move |_| open_or_focus(CardId::Packages(0), 600.0, 460.0)
                >
                    <IconLayers size=18 />
                    <span class="dock-tooltip">"Packages"</span>
                </button>
                <button
                    class="dock-item"
                    class:active=move || selected.get().as_ref().is_some_and(|item| matches!(item, DesktopItemId::Card(card) if card.key() == "updates"))
                    title="System Updates"
                    on:click=move |_| open_or_focus(CardId::Updates(0), 560.0, 420.0)
                >
                    <IconRefresh size=18 />
                    <span class="dock-tooltip">"Updates"</span>
                </button>
                <button
                    class="dock-item"
                    class:active=move || selected.get().as_ref().is_some_and(|item| matches!(item, DesktopItemId::Card(card) if card.key() == "user-settings"))
                    title="Users & SSH Keys"
                    on:click=move |_| open_or_focus(CardId::UserSettings(0), 580.0, 440.0)
                >
                    <IconGrid size=18 />
                    <span class="dock-tooltip">"Users"</span>
                </button>
                <button
                    class="dock-item"
                    class:active=move || selected.get().as_ref().is_some_and(|item| matches!(item, DesktopItemId::Card(card) if card.key() == "security"))
                    title="Security & Sandboxing"
                    on:click=move |_| open_or_focus(CardId::Security(0), 560.0, 440.0)
                >
                    <IconShield size=18 />
                    <span class="dock-tooltip">"Security"</span>
                </button>
                <button
                    class="dock-item"
                    class:active=move || selected.get().as_ref().is_some_and(|item| matches!(item, DesktopItemId::Card(card) if card.key() == "backup"))
                    title="Backup Vault"
                    on:click=move |_| open_or_focus(CardId::Backup(0), 580.0, 460.0)
                >
                    <IconFile size=18 />
                    <span class="dock-tooltip">"Backup"</span>
                </button>
                <button
                    class="dock-item"
                    class:active=move || selected.get().as_ref().is_some_and(|item| matches!(item, DesktopItemId::Card(card) if card.key() == "mail"))
                    title="Mail & Messages"
                    on:click=move |_| open_or_focus(CardId::Mail(0), 640.0, 480.0)
                >
                    <IconBell size=18 />
                    <span class="dock-tooltip">"Mail"</span>
                </button>
                <button
                    class="dock-item"
                    class:active=move || selected.get().as_ref().is_some_and(|item| matches!(item, DesktopItemId::Card(card) if card.key() == "calendar"))
                    title="Calendar & Schedule"
                    on:click=move |_| open_or_focus(CardId::Calendar(0), 600.0, 460.0)
                >
                    <IconActivity size=18 />
                    <span class="dock-tooltip">"Calendar"</span>
                </button>
                <button
                    class="dock-item"
                    class:active=move || selected.get().as_ref().is_some_and(|item| matches!(item, DesktopItemId::Card(card) if card.key() == "notes"))
                    title="Notes & Ideas"
                    on:click=move |_| open_or_focus(CardId::Notes(0), 580.0, 460.0)
                >
                    <IconFile size=18 />
                    <span class="dock-tooltip">"Notes"</span>
                </button>
                <button
                    class="dock-item"
                    class:active=move || selected.get().as_ref().is_some_and(|item| matches!(item, DesktopItemId::Card(card) if card.key() == "contacts"))
                    title="Contacts Directory"
                    on:click=move |_| open_or_focus(CardId::Contacts(0), 560.0, 440.0)
                >
                    <IconGrid size=18 />
                    <span class="dock-tooltip">"Contacts"</span>
                </button>
                <button
                    class="dock-item"
                    class:active=move || selected.get().as_ref().is_some_and(|item| matches!(item, DesktopItemId::Card(card) if card.key() == "cognitive-graph"))
                    title="Cognitive Graph & Causal DAG"
                    on:click=move |_| open_or_focus(CardId::CognitiveGraph(0), 680.0, 500.0)
                >
                    <IconLayers size=18 />
                    <span class="dock-tooltip">"Cognitive Graph"</span>
                </button>
                <button
                    class="dock-item"
                    class:active=move || selected.get().as_ref().is_some_and(|item| matches!(item, DesktopItemId::Card(card) if card.key() == "event-journal"))
                    title="Canonical Event1 Journal"
                    on:click=move |_| open_or_focus(CardId::EventJournal(0), 640.0, 460.0)
                >
                    <IconActivity size=18 />
                    <span class="dock-tooltip">"Event Journal"</span>
                </button>
                <button
                    class="dock-item"
                    class:active=move || selected.get().as_ref().is_some_and(|item| matches!(item, DesktopItemId::Card(card) if card.key() == "meaning"))
                    title="Meaning & Dialogue Assistant"
                    on:click=move |_| open_or_focus(CardId::Meaning(0), 600.0, 480.0)
                >
                    <IconBot size=18 />
                    <span class="dock-tooltip">"Meaning"</span>
                </button>
                <button
                    class="dock-item"
                    class:active=move || selected.get().as_ref().is_some_and(|item| matches!(item, DesktopItemId::Card(card) if card.key() == "learning"))
                    title="Lifelong Learning & Governance"
                    on:click=move |_| open_or_focus(CardId::Learning(0), 620.0, 480.0)
                >
                    <IconLayers size=18 />
                    <span class="dock-tooltip">"Learning"</span>
                </button>
                <button
                    class="dock-item"
                    class:active=move || selected.get().as_ref().is_some_and(|item| matches!(item, DesktopItemId::Card(card) if card.key() == "outline"))
                    title="Canvas Outline"
                    on:click=move |_| open_or_focus(CardId::Outline, 300.0, 460.0)
                >
                    <IconLayers size=18 />
                    <span class="dock-tooltip">"Outline"</span>
                </button>
                <button
                    class="dock-item"
                    title="Mind Explorer"
                    on:click=move |_| {
                        layout.update(|l| l.apply_arrangement(ArrangementMode::Relations, None));
                        open_or_focus(CardId::Context, 380.0, 360.0);
                    }
                >
                    <IconLayers size=18 />
                    <span class="dock-tooltip">"Mind"</span>
                </button>
            </div>

            <Show when=move || !layout.get().anchors.is_empty()>
                <div class="dock-separator"></div>
                <nav class="dock-anchors" aria-label="Spatial anchors">
                    <For
                        each={move || layout.get().anchors.into_iter().take(6).collect::<Vec<_>>()}
                        key=|anchor| anchor.id.clone()
                        children=move |anchor| {
                            let anchor_id = anchor.id.clone();
                            let name = anchor.name.clone();
                            let title = anchor.name.clone();
                            let aria_name = format!("Go to anchor {}", anchor.name);
                            view! {
                                <button
                                    class="dock-anchor-pill"
                                    type="button"
                                    title=title
                                    aria-label=aria_name
                                    on:click=move |_| focus_anchor(anchor_id.clone())
                                >
                                    <span aria-hidden="true">"⚓"</span>
                                    <span class="dock-anchor-name">{name}</span>
                                </button>
                            }
                        }
                    />
                </nav>
            </Show>

            <div class="dock-separator"></div>

            <div class="dock-windows">
                <For
                    each={move || layout.get().cards.into_iter().filter(|c| !c.id.is_system()).collect::<Vec<_>>()}
                    key=|c| format!("{:?}", c.id)
                    children=move |c| {
                        let id_click = c.id;
                        let title = c.id.title();
                        let is_active = move || selected.get() == Some(DesktopItemId::Card(id_click));
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
                                    set_selected.set(Some(DesktopItemId::Card(id_click)));
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
                <button
                    class="dock-tray-user"
                    class:public-preview=is_public_preview
                    title="Session State / Sign In"
                    on:click=move |_| auth_modal_open.set(true)
                >
                    <IconShield size=13 />
                    <span>{user_label}</span>
                </button>
                <div class="dock-tray-clock">
                    {move || time_str.get()}
                </div>
            </div>
        </footer>
    }
}
