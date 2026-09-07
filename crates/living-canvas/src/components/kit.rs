// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! The parts every tool panel is built from, written once.
//!
//! Twenty panels had written the same status toast by hand, and the four commonest layouts in the
//! desktop were carried as inline `style` attributes repeated between eleven and sixteen times
//! each. Nothing was broken by that — every copy said the same thing — and that is exactly the
//! problem: they said the same thing until one of them was edited, and then the desktop had two
//! opinions about what a tool panel looks like with nothing to say which was current.
//!
//! What is here is only what was already duplicated. A kit assembled from components nobody uses
//! yet is a second vocabulary to keep in step with the first, so each of these replaced something
//! that existed, at every site that had it.

use leptos::prelude::*;

/// What a panel has just done, and a way to stop being told.
///
/// Twenty panels rendered this by hand. They agreed, which is why replacing them changes nothing on
/// screen — and why it was worth doing before one of them stopped agreeing.
///
/// `role="status"` rather than an alert: this reports what happened after somebody asked for it,
/// and a screen reader interrupting them to say a service restarted is reading it back the news
/// they already had.
#[component]
pub fn StatusLine(
    /// The message to show, and the signal cleared when it is dismissed.
    message: RwSignal<Option<String>>,
) -> impl IntoView {
    view! {
        {move || {
            message.get().map(|text| {
                view! {
                    <div class="card-status-line" role="status" aria-live="polite">
                        <span>{text}</span>
                        <button
                            class="card-status-dismiss"
                            title="Dismiss"
                            aria-label="Dismiss this message"
                            on:click=move |_| message.set(None)
                        >
                            "×"
                        </button>
                    </div>
                }
            })
        }}
    }
}
