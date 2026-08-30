// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! The terminal card: a socket, a screen, and a keyboard.
//!
//! Everything that decides anything is elsewhere. The screen is
//! [`crate::terminal::TerminalScreen`], which is why it can be tested without a browser; the
//! boundary is the account, held by the kernel on the far side of
//! [ADR-0047](../../../../docs/adr/ADR-0047-interactive-terminal-under-the-authenticated-account.md).
//! What is here is the part that only a browser can do: draw a grid, and turn keys into bytes.

use leptos::prelude::*;
use wasm_bindgen::{JsCast as _, prelude::Closure};
use web_sys::{BinaryType, CloseEvent, KeyboardEvent, MessageEvent, WebSocket};

use crate::{
    CardId,
    terminal::{Cell, TerminalScreen, cell_style, fitting_window, key_to_bytes},
    tool_state::ToolCardStates,
};

/// How often the panel is measured while somebody is dragging its edge.
///
/// A resize frame reaches `TIOCSWINSZ` and every program in the session gets `SIGWINCH`, so one per
/// animation frame would be a drag that asks a shell to re-lay-out a hundred times. Slow enough to
/// be cheap, fast enough that letting go feels immediate.
const RESIZE_INTERVAL_MS: u32 = 150;

/// One row, as a string, for a reader that only wants the text.
#[must_use]
fn row_text(row: &[Cell]) -> String {
    row.iter()
        .map(|cell| {
            if cell.text.is_empty() {
                " "
            } else {
                cell.text.as_str()
            }
        })
        .collect()
}

/// The address this page's terminal socket is at.
///
/// Derived from where the page was served rather than configured, so a deployment behind a
/// different name or port needs nothing said twice. `wss` follows `https` for the same reason a
/// cookie is `Secure`: the bytes are a person's keystrokes.
fn socket_url() -> Option<String> {
    let window = web_sys::window()?;
    let host = window.location().host().ok()?;
    let scheme = window.location().protocol().ok().map_or("ws", |protocol| {
        if protocol == "https:" { "wss" } else { "ws" }
    });
    Some(format!("{scheme}://{host}/api/v1/terminal"))
}

/// Encode one frame and put it on the socket.
fn send(socket: &WebSocket, frame: &cybou_web_contracts::TerminalFromGateway) {
    let mut body = Vec::new();
    if ciborium::into_writer(frame, &mut body).is_ok() {
        let _ = socket.send_with_u8_array(&body);
    }
}

/// What one frame from the owner does to this card.
fn receive(
    signals: crate::tool_state::TerminalSignals,
    frame: cybou_web_contracts::TerminalFromOwner,
) {
    match frame {
        cybou_web_contracts::TerminalFromOwner::Opened => {
            signals.status.set("Connected".to_owned());
        }
        cybou_web_contracts::TerminalFromOwner::Output(bytes) => {
            signals.screen.update(|screen| screen.feed(&bytes));
            signals.generation.update(|generation| *generation += 1);
        }
        cybou_web_contracts::TerminalFromOwner::Ended { code, signal } => {
            // Carried rather than flattened: "you typed exit" and "it was killed" are different
            // things to read on a screen that has stopped.
            let ending = match (code, signal) {
                (Some(code), _) => format!("The shell exited with status {code}."),
                (_, Some(signal)) => format!("The shell was ended by signal {signal}."),
                _ => "The shell ended.".to_owned(),
            };
            signals.status.set("Ended".to_owned());
            signals.refusal.set(Some(ending));
        }
        cybou_web_contracts::TerminalFromOwner::Refused(refusal) => {
            signals.status.set("Closed".to_owned());
            signals.refusal.set(Some(refusal.explain().to_owned()));
        }
    }
}

/// Open a socket for this card and say what each of its events means.
///
/// A function rather than a closure inside the component: it is one thing, it is the whole
/// reason the component was long, and it does not need re-creating on every render.
fn connect(signals: crate::tool_state::TerminalSignals) {
    signals.status.set("Connecting…".to_owned());
    signals.refusal.set(None);

    let Some(url) = socket_url() else {
        signals
            .refusal
            .set(Some("This page has no host to connect to.".to_owned()));
        return;
    };
    let Ok(socket) = WebSocket::new(&url) else {
        signals
            .refusal
            .set(Some("This host has no terminal for you.".to_owned()));
        return;
    };
    // Frames are CBOR. Without this the browser hands back a Blob and every read becomes a
    // second async hop for bytes that are already here.
    socket.set_binary_type(BinaryType::Arraybuffer);

    let opened = socket.clone();
    let on_open = Closure::<dyn FnMut()>::new(move || {
        // Whatever the panel already measured, rather than a constant this then corrects — a
        // shell that drew its first prompt at eighty columns inside a wider panel would repaint
        // once for no reason, in the one place a person is watching for the prompt.
        let (columns, rows) = signals.window.get_untracked();
        send(
            &opened,
            &cybou_web_contracts::TerminalFromGateway::Open { columns, rows },
        );
    });
    socket.set_onopen(Some(on_open.as_ref().unchecked_ref()));
    on_open.forget();

    let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
        let Ok(buffer) = event.data().dyn_into::<js_sys::ArrayBuffer>() else {
            return;
        };
        let bytes = js_sys::Uint8Array::new(&buffer).to_vec();
        let Ok(frame) =
            ciborium::from_reader::<cybou_web_contracts::TerminalFromOwner, _>(bytes.as_slice())
        else {
            return;
        };
        receive(signals, frame);
    });
    socket.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    on_message.forget();

    let on_close = Closure::<dyn FnMut(CloseEvent)>::new(move |_| {
        signals.status.set("Closed".to_owned());
        signals.socket.set(None);
    });
    socket.set_onclose(Some(on_close.as_ref().unchecked_ref()));
    on_close.forget();

    signals.socket.set(Some(socket));
}

/// Terminal card content.
#[component]
pub fn TerminalContent(
    /// Which Terminal card this is, taken from `CardId::Terminal(n)`.
    #[prop(optional)]
    instance: u32,
) -> impl IntoView {
    let states = expect_context::<ToolCardStates>();
    let signals = states.terminal(CardId::Terminal(instance));

    let screen_ref: NodeRef<leptos::html::Div> = NodeRef::new();
    let probe_ref: NodeRef<leptos::html::Span> = NodeRef::new();

    // Measure the panel and tell both ends, when and only when the answer changed.
    //
    // On a timer rather than a resize observer: a card is resized by this desktop's own
    // interaction code, which moves and sizes it without the element firing anything a browser
    // calls a resize.
    let measure = move || {
        let (Some(screen), Some(probe)) = (screen_ref.get_untracked(), probe_ref.get_untracked())
        else {
            return;
        };
        let cell = probe.get_bounding_client_rect();
        let window = fitting_window(
            f64::from(screen.client_width()),
            f64::from(screen.client_height()),
            cell.width(),
            cell.height(),
        );
        if window == signals.window.get_untracked() {
            return;
        }

        let (columns, rows) = window;
        signals.window.set(window);
        // Both ends hear it: the host so programs re-lay-out, and the local screen so the grid
        // drawn here is the grid they are drawing into.
        signals.screen.update(|screen| screen.resize(columns, rows));
        signals.generation.update(|generation| *generation += 1);
        if let Some(socket) = signals.socket.get_untracked() {
            send(
                &socket,
                &cybou_web_contracts::TerminalFromGateway::Resize { columns, rows },
            );
        }
    };

    Effect::new(move |_| {
        gloo_timers::callback::Interval::new(RESIZE_INTERVAL_MS, measure).forget();
    });

    let send_key = move |event: KeyboardEvent| {
        let Some(socket) = signals.socket.get_untracked() else {
            return;
        };
        // Taken before the early return, so a key this card handles never also reaches the
        // browser's own shortcuts — Ctrl-C would otherwise interrupt a program and copy a
        // selection, and Tab would leave the panel while completing a filename.
        let Some(bytes) = key_to_bytes(&event.key(), event.ctrl_key(), event.alt_key()) else {
            return;
        };
        event.prevent_default();

        send(
            &socket,
            &cybou_web_contracts::TerminalFromGateway::Input(bytes),
        );
    };

    let rows = move || {
        // Read so the view re-runs when bytes arrive: the screen itself is not a reactive value,
        // and a grid that only redrew when something else happened to change would be a terminal
        // that answers late.
        let _ = signals.generation.get();
        signals.screen.with(TerminalScreen::rows)
    };

    view! {
        <div class="terminal-panel">
            <div class="terminal-bar">
                <span class="terminal-status">{move || signals.status.get()}</span>
                <button
                    class="terminal-btn"
                    disabled=move || signals.socket.get().is_some()
                    on:click=move |_| connect(signals)
                >
                    {move || if signals.socket.get().is_some() { "Connected" } else { "Connect" }}
                </button>
            </div>

            // The refusal is what the person acts on, so it is prose and it is never a status
            // light. "This host has no terminal for you" sends somebody to an operator; "your
            // terminal died" sends them to reconnect, and a card that showed one for the other
            // would send them to the wrong place.
            {move || signals.refusal.get().map(|reason| view! {
                <div class="terminal-refusal">{reason}</div>
            })}

            <div
                class="terminal-screen"
                node_ref=screen_ref
                tabindex="0"
                role="application"
                aria-label="Interactive terminal. Keystrokes are sent to the host."
                on:keydown=send_key
            >
                // One cell, measured rather than assumed. A monospace cell's size comes from the
                // font the browser actually resolved, which no constant here can know: it changes
                // with the theme, the zoom level and whichever fallback the platform supplied.
                <span class="terminal-probe" node_ref=probe_ref aria-hidden="true">"M"</span>

                <For
                    each=rows
                    key=|row| row_text(row)
                    children=move |row| {
                        view! {
                            <div class="terminal-row">
                                {row.iter().map(|cell| {
                                    let style = cell_style(cell);
                                    let text = if cell.text.is_empty() {
                                        " ".to_owned()
                                    } else {
                                        cell.text.clone()
                                    };
                                    view! { <span style=style>{text}</span> }
                                }).collect_view()}
                            </div>
                        }
                    }
                />
            </div>
        </div>
    }
}
