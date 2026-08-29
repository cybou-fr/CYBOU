// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Live Presence1 snapshot SSE stream tool card and content component.

use leptos::prelude::*;
use leptos::task::spawn_local;
use lucide_leptos::{Activity, Check, Copy, FileText, Pause, Play, Search, Trash2, X};
use std::sync::Arc;
use wasm_bindgen::{JsCast, closure::Closure};
use web_sys::{Event, EventSource, MessageEvent, PointerEvent};

use crate::{
    MindClient,
    CardId, DesktopItemId, DesktopLayout,
    components::card_frame::CardFrame,
    interaction::{DragState, ResizeState},
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

/// Connection state established from EventSource callbacks, never inferred from pause state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StreamState {
    Connecting,
    Live,
    Stale,
    Unavailable,
}

/// Live Presence projection stream domain content presentation.
#[component]
pub fn JournalFeedContent() -> impl IntoView {
    let (events, set_events) = signal(Vec::<(String, String, String, String)>::new());
    let (filter, set_filter) = signal("all".to_string());
    let (search_query, set_search_query) = signal(String::new());
    let (is_paused, set_is_paused) = signal(false);
    let (selected_event, set_selected_event) =
        signal(Option::<(String, String, String, String)>::None);
    let (copied, set_copied) = signal(false);
    let (stream_state, set_stream_state) = signal(StreamState::Connecting);

    let es_handle: StoredValue<Option<EventSource>> = StoredValue::new(None);

    Effect::new(move |_| {
        if es_handle.get_value().is_none() {
            if let Ok(es) = EventSource::new("/api/v1/events") {
                let on_open = Closure::<dyn FnMut(Event)>::new(move |_| {
                    set_stream_state.set(StreamState::Live);
                });
                es.set_onopen(Some(on_open.as_ref().unchecked_ref()));
                on_open.forget();

                let on_error = Closure::<dyn FnMut(Event)>::new(move |_| {
                    set_stream_state.set(StreamState::Unavailable);
                });
                es.set_onerror(Some(on_error.as_ref().unchecked_ref()));
                on_error.forget();

                let on_snap =
                    Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
                        set_stream_state.set(StreamState::Live);
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
                let _ = es
                    .add_event_listener_with_callback("snapshot", on_snap.as_ref().unchecked_ref());
                on_snap.forget();

                let on_projection_error =
                    Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
                        let Some(data) = event.data().as_string() else {
                            return;
                        };
                        let retryable = serde_json::from_str::<serde_json::Value>(&data)
                            .ok()
                            .and_then(|value| {
                                value.get("retryable").and_then(|value| value.as_bool())
                            })
                            .unwrap_or(false);
                        set_stream_state.set(if retryable {
                            StreamState::Stale
                        } else {
                            StreamState::Unavailable
                        });
                        if is_paused.get_untracked() {
                            return;
                        }
                        #[cfg(target_arch = "wasm32")]
                        let now = js_sys::Date::new_0().to_locale_time_string("en-US");
                        #[cfg(not(target_arch = "wasm32"))]
                        let now = "12:00:00".to_string();
                        set_events.update(|list| {
                            list.push((
                                now.into(),
                                "projection.error".into(),
                                "Presence projection unavailable".into(),
                                data,
                            ));
                            if list.len() > 100 {
                                list.remove(0);
                            }
                        });
                    });
                let _ = es.add_event_listener_with_callback(
                    "projection-error",
                    on_projection_error.as_ref().unchecked_ref(),
                );
                on_projection_error.forget();
                es_handle.set_value(Some(es));
            } else {
                set_stream_state.set(StreamState::Unavailable);
            }
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
                let matches_filter = if f == "all" {
                    true
                } else {
                    topic.to_lowercase().contains(&f.to_lowercase())
                };
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

    let total_events = move || events.get().len();

    view! {
        <div class="jf-body" on:pointerdown=move |e: PointerEvent| e.stop_propagation()>
            // Top Toolbar
            <div class="jf-toolbar">
                <div class="jf-filter-group">
                    <button
                        type="button"
                        class="jf-filter-btn"
                        class:active=move || filter.get() == "all"
                        on:click=move |_| set_filter.set("all".into())
                    >
                        "All"
                    </button>
                    <button
                        type="button"
                        class="jf-filter-btn"
                        class:active=move || filter.get() == "snapshot"
                        on:click=move |_| set_filter.set("snapshot".into())
                    >
                        "Snapshot"
                    </button>
                    <button
                        type="button"
                        class="jf-filter-btn"
                        class:active=move || filter.get() == "projection.error"
                        on:click=move |_| set_filter.set("projection.error".into())
                    >
                        "Errors"
                    </button>
                </div>

                <div class="jf-actions">
                    <button
                        type="button"
                        class="jf-action-btn"
                        class:paused=move || is_paused.get()
                        on:click=move |_| set_is_paused.update(|p| *p = !*p)
                    >
                        {move || if is_paused.get() {
                            view! { <Play size=12 /> <span>"Resume"</span> }.into_any()
                        } else {
                            view! { <Pause size=12 /> <span>"Pause"</span> }.into_any()
                        }}
                    </button>
                    <button
                        type="button"
                        class="jf-action-btn"
                        title="Clear event buffer"
                        on:click=move |_| set_events.set(Vec::new())
                    >
                        <Trash2 size=12 />
                        <span>"Clear"</span>
                    </button>
                </div>
            </div>

            // Search Bar & Stream Status
            <div class="jf-search-row">
                <div class="jf-search-input-wrap">
                    <Search size=12 />
                    <input
                        type="text"
                        class="jf-search-input"
                        placeholder="Search topics, timestamps, payloads…"
                        prop:value=search_query
                        on:input=move |e| set_search_query.set(event_target_value(&e))
                    />
                    <Show when=move || !search_query.get().is_empty()>
                        <button
                            type="button"
                            class="jf-clear-search-btn"
                            title="Clear search"
                            on:click=move |_| set_search_query.set(String::new())
                        >
                            <X size=11 />
                        </button>
                    </Show>
                </div>
                <div class="jf-stream-stats">
                    <span
                        class="jf-status-pill"
                        class:streaming=move || !is_paused.get() && stream_state.get() == StreamState::Live
                        class:paused=move || is_paused.get()
                    >
                        {move || if is_paused.get() {
                            "⏸ Paused"
                        } else {
                            match stream_state.get() {
                                StreamState::Connecting => "◌ Connecting",
                                StreamState::Live => "● Live",
                                StreamState::Stale => "◐ Stale",
                                StreamState::Unavailable => "○ Unavailable",
                            }
                        }}
                    </span>
                    <span class="jf-count-pill">{move || format!("{} events", total_events())}</span>
                </div>
            </div>

            // Integrity Banner
            <div class="jf-hash-banner">
                <span><b>"Projection:"</b> <code>"Presence1 snapshots (not Event1 Journal)"</code></span>
                <span class="jf-integrity-pill unverified">"Integrity details unavailable"</span>
            </div>

            // Event Stream List
            <div class="jf-stream-list">
                <Show when=move || events.get().is_empty()>
                    <div class="jf-empty">
                        <Activity size=20 />
                        <span>"Waiting for Presence snapshots from gateway…"</span>
                    </div>
                </Show>
                <For
                    each=filtered_events
                    key=|(time, topic, _desc, payload)| format!("{time}-{topic}-{payload}")
                    children=move |(time, topic, desc, payload)| {
                        let t = time.clone();
                        let top = topic.clone();
                        let d = desc.clone();
                        let pl = payload.clone();
                        let selected_time = t.clone();
                        let selected_topic = top.clone();
                        let is_selected = move || {
                            selected_event.get().as_ref().map_or(false, |ev| {
                                ev.0 == selected_time && ev.1 == selected_topic
                            })
                        };
                        view! {
                            <div
                                class="jf-event-row"
                                class:selected=is_selected
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

            // Event Inspector Drawer
            <Show when=move || selected_event.get().is_some()>
                <aside class="jf-inspector">
                    <header class="jf-insp-header">
                        <div class="jf-insp-title">
                            <FileText size=12 />
                            <span><b>{move || selected_event.get().unwrap().1}</b> " · " {move || selected_event.get().unwrap().0}</span>
                        </div>
                        <div class="jf-insp-actions">
                            <button
                                type="button"
                                class="fm-btn"
                                on:click=move |_| {
                                    if let Some(ev) = selected_event.get() {
                                        copy_json(ev.3);
                                    }
                                }
                            >
                                {move || if copied.get() {
                                    view! { <Check size=11 /> <span>"Copied!"</span> }.into_any()
                                } else {
                                    view! { <Copy size=11 /> <span>"Copy JSON"</span> }.into_any()
                                }}
                            </button>
                            <button
                                type="button"
                                class="fm-btn"
                                title="Close inspector"
                                on:click=move |_| set_selected_event.set(None)
                            >
                                <X size=12 />
                            </button>
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
    }
}

/// Live Presence projection stream card component.
#[component]
pub fn JournalFeedCard(
    layout: RwSignal<DesktopLayout>,
    selected: ReadSignal<Option<DesktopItemId>>,
    set_selected: WriteSignal<Option<DesktopItemId>>,
    dragging: RwSignal<Option<DragState>>,
    resizing: RwSignal<Option<ResizeState>>,
    /// Which instance of this tool card this is.
    #[prop(optional)]
    instance: u32,
) -> impl IntoView {
    let card_id = CardId::JournalFeed(instance);

    let collapsed = move || {
        view! {
            <div class="card-collapsed-summary">
                <b>"Presence Stream"</b>
                <span>"Presence1 SSE"</span>
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
            kicker_title="Presence Stream"
            kicker_icon=Arc::new(|| view! { <Activity size=14 /> }.into_any())
            collapsed_summary=Arc::new(collapsed)
        >
            <JournalFeedContent />
        </CardFrame>
    }
}
