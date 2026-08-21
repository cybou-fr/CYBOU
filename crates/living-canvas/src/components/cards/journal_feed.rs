// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Real-time Journal SSE stream tool card component.

use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{EventSource, MessageEvent, PointerEvent};

use crate::{
    CardId, DesktopLayout,
    components::{
        card_controls::{CardControls, CardResizeHandle},
        icons::IconFile,
    },
    interaction::{DragState, ResizeState, card_style, start_drag},
};

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

/// Real-time Journal event stream card component.
#[component]
pub fn JournalFeedCard(
    layout: RwSignal<DesktopLayout>,
    selected: ReadSignal<&'static str>,
    set_selected: WriteSignal<&'static str>,
    dragging: RwSignal<Option<DragState>>,
    resizing: RwSignal<Option<ResizeState>>,
) -> impl IntoView {
    let card_id = CardId::JournalFeed(0);
    let card_open =
        move || layout.get().contains_card(card_id) && !layout.get().is_in_deck(card_id);
    let is_collapsed = move || layout.get().presentation(card_id).collapsed;

    let (events, set_events) = signal(Vec::<(String, String, String, String)>::new());
    let (filter, set_filter) = signal("all".to_string());
    let (search_query, set_search_query) = signal(String::new());
    let (is_paused, set_is_paused) = signal(false);
    let (selected_event, set_selected_event) =
        signal(Option::<(String, String, String, String)>::None);
    let (copied, set_copied) = signal(false);

    let es_handle: StoredValue<Option<EventSource>> = StoredValue::new(None);

    Effect::new(move |_| {
        let is_open = card_open();
        if is_open {
            if es_handle.get_value().is_none() {
                if let Ok(es) = EventSource::new("/api/v1/events") {
                    let on_snap =
                        Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
                            if is_paused.get_untracked() {
                                return;
                            }
                            if let Some(data) = event.data().as_string() {
                                #[cfg(target_arch = "wasm32")]
                                let now = js_sys::Date::new_0().to_locale_time_string("en-US");
                                #[cfg(not(target_arch = "wasm32"))]
                                let now = "12:00:00".to_string();

                                let payload = data.clone();
                                set_events.update(|list| {
                                    list.push((
                                        now.into(),
                                        "snapshot.update".into(),
                                        "Mind state projection update".into(),
                                        payload,
                                    ));
                                    if list.len() > 100 {
                                        list.remove(0);
                                    }
                                });
                            }
                        });
                    let _ = es.add_event_listener_with_callback(
                        "snapshot",
                        on_snap.as_ref().unchecked_ref(),
                    );
                    on_snap.forget();
                    es_handle.set_value(Some(es));
                }
            }
        } else if let Some(es) = es_handle.get_value() {
            es.close();
            es_handle.set_value(None);
        }
    });

    on_cleanup(move || {
        if let Some(es) = es_handle.get_value() {
            es.close();
            es_handle.set_value(None);
        }
    });

    let filtered_events = move || {
        let f = filter.get();
        let q = search_query.get().to_lowercase();
        let list = events.get();
        list.into_iter()
            .filter(|(time, topic, desc, payload)| {
                let matches_filter = if f == "all" { true } else { topic.contains(&f) };
                let matches_search = if q.is_empty() {
                    true
                } else {
                    time.to_lowercase().contains(&q)
                        || topic.to_lowercase().contains(&q)
                        || desc.to_lowercase().contains(&q)
                        || payload.to_lowercase().contains(&q)
                };
                matches_filter && matches_search
            })
            .collect::<Vec<_>>()
    };

    let copy_json = move |payload: String| {
        if let Some(window) = web_sys::window() {
            let clipboard = window.navigator().clipboard();
            let _ = clipboard.write_text(&payload);
        }
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
                aria-label="Journal Event Stream"
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
                        <strong class="card-title">"Event Stream"</strong>
                        <small class="card-badge">"Live SSE"</small>
                    </span>
                    <CardControls card=card_id layout=layout />
                </header>

                <Show when=move || !is_collapsed()>
                    <div class="jf-body" on:pointerdown=move |e: PointerEvent| e.stop_propagation()>
                        <div class="jf-toolbar">
                            <div class="jf-filter-group">
                                <button class="jf-filter-btn" class:active=move || filter.get() == "all" on:click=move |_| set_filter.set("all".into())>"All"</button>
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
                            <span><b>"Integrity State:"</b> <code>"Live Event1 Projection"</code></span>
                            <span class="jf-integrity-pill unverified">"Integrity details unavailable"</span>
                        </div>

                        <div class="jf-stream-list">
                            <Show when=move || events.get().is_empty()>
                                <div class="jf-empty">"Listening for live events from gateway…"</div>
                            </Show>
                            <For
                                each=filtered_events
                                key=|(time, topic, _desc, payload)| format!("{time}-{topic}-{payload}")
                                children=move |(time, topic, desc, payload)| {
                                    let t = time.clone();
                                    let top = topic.clone();
                                    let d = desc.clone();
                                    let pl = payload.clone();
                                    view! {
                                        <div
                                            class="jf-event-row"
                                            title="Click to inspect event payload"
                                            on:click=move |_| {
                                                set_selected_event.set(Some((t.clone(), top.clone(), d.clone(), pl.clone())));
                                            }
                                        >
                                            <span class="jf-event-time">{time}</span>
                                            <span class="jf-event-topic">{topic}</span>
                                            <span class="jf-event-desc">{desc}</span>
                                        </div>
                                    }
                                }
                            />
                        </div>

                        <Show when=move || selected_event.get().is_some()>
                            <aside class="jf-inspector">
                                <header class="jf-insp-header">
                                    <div class="jf-insp-title">
                                        <IconFile size=12 />
                                        <span><b>{move || selected_event.get().unwrap().1}</b> " · " {move || selected_event.get().unwrap().0}</span>
                                    </div>
                                    <div class="jf-insp-actions">
                                        <button
                                            class="fm-btn"
                                            on:click=move |_| {
                                                if let Some(ev) = selected_event.get() {
                                                    copy_json(ev.3);
                                                }
                                            }
                                        >
                                            {move || if copied.get() { "Copied!" } else { "Copy JSON" }}
                                        </button>
                                        <button class="fm-btn" on:click=move |_| set_selected_event.set(None)>"×"</button>
                                    </div>
                                </header>
                                <pre class="jf-json-view">
                                    {move || {
                                        if let Some(ev) = selected_event.get() {
                                            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&ev.3) {
                                                serde_json::to_string_pretty(&parsed).unwrap_or(ev.3)
                                            } else {
                                                ev.3
                                            }
                                        } else {
                                            String::new()
                                        }
                                    }}
                                </pre>
                            </aside>
                        </Show>
                    </div>
                </Show>
                <CardResizeHandle card=card_id layout=layout resizing=resizing />
            </div>
        </Show>
    }
}
