// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Desktop bottom taskbar and shelf dock component.

use cybou_web_contracts::SessionMode;
use leptos::prelude::*;
use wasm_bindgen::{JsCast, closure::Closure};

use crate::{
    CardId, DesktopLayout,
    components::icons::{IconFile, IconFolder, IconShield, IconTerminal},
    state::RuntimeState,
};

/// Bottom desktop taskbar dock component.
#[component]
pub fn DesktopDock(
    layout: RwSignal<DesktopLayout>,
    selected: ReadSignal<&'static str>,
    set_selected: WriteSignal<&'static str>,
    auth_modal_open: RwSignal<bool>,
    runtime: RwSignal<RuntimeState>,
) -> impl IntoView {
    let (time_str, set_time_str) = signal(String::new());

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
        RuntimeState::Ready { mode, mind, .. } => match mode {
            SessionMode::PublicPreview => "Public Preview 🔒".to_string(),
            SessionMode::LocalDesktop => "local · authenticated ●".to_string(),
            SessionMode::RemoteBrowser => {
                if let Some(m) = mind {
                    if let Some(origin) = m.identity.origin {
                        format!("{origin} · authenticated ●")
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
        <footer class="desktop-dock" aria-label="Desktop Card Shelf and Taskbar">
            <div class="dock-apps">
                <button class="dock-item" class:active=move || selected.get() == "shell" title="CYBOU Shell" on:click=move |_| open_or_focus(CardId::Shell(0), "shell", 400.0, 160.0)>
                    <IconTerminal size=18 />
                    <span class="dock-tooltip">"Shell"</span>
                </button>
                <button class="dock-item" class:active=move || selected.get() == "files" title="File Manager" on:click=move |_| open_or_focus(CardId::FileManager(0), "files", 380.0, 120.0)>
                    <IconFolder size=18 />
                    <span class="dock-tooltip">"Files"</span>
                </button>
                <button class="dock-item" class:active=move || selected.get() == "journal-feed" title="Event Stream" on:click=move |_| open_or_focus(CardId::JournalFeed(0), "journal-feed", 420.0, 150.0)>
                    <IconFile size=18 />
                    <span class="dock-tooltip">"Events"</span>
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
