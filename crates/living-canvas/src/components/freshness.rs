// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! The line in a panel's header that says how old what you are looking at is.

use leptos::prelude::*;

use crate::{
    components::icons::IconRefresh,
    refresh::{DesktopClock, Freshness, age_in_words},
};

/// The age of a panel's reading, its auto-refresh toggle, and the button to ask now.
///
/// One component rather than four copies, because these three controls only make sense together: a
/// person who turns the timer off needs the age to know what they are now looking at, and a person
/// who sees an age that has stopped moving needs somewhere to press.
#[component]
pub fn FreshnessControls(
    /// When the last reading arrived.
    freshness: Freshness,
    /// The panel's own auto-refresh toggle.
    auto_refresh: RwSignal<bool>,
    /// Whether a fetch is in flight, so the button can say so.
    loading: RwSignal<bool>,
    /// Ask now.
    #[prop(into)]
    refresh_now: Callback<()>,
) -> impl IntoView {
    let clock = use_context::<DesktopClock>();
    let age = move || age_in_words(freshness.read_at(), clock.map(DesktopClock::now));

    view! {
        <div class="panel-freshness">
            // Aria-live, because this text changes on its own and a person who cannot see it change
            // would otherwise have no way to learn that the panel had gone stale.
            <span class="panel-freshness-age" aria-live="polite">
                {move || age().unwrap_or_else(||
                    // Never "0s ago": a panel that has not answered yet has no age, and saying it
                    // did would be the panel's first lie rather than its last.
                    "not read yet".to_owned()
                )}
            </span>
            <button
                class="panel-freshness-toggle"
                class:active=move || auto_refresh.get()
                role="switch"
                aria-checked=move || if auto_refresh.get() { "true" } else { "false" }
                title=move || if auto_refresh.get() {
                    "Refreshing on its own — click to stop asking"
                } else {
                    "Not refreshing — click to keep it current"
                }
                aria-label="Refresh this panel automatically"
                on:click=move |_| auto_refresh.update(|on| *on = !*on)
            >
                {move || if auto_refresh.get() { "live" } else { "held" }}
            </button>
            <button
                class="panel-freshness-now"
                title="Read again now"
                aria-label="Read again now"
                disabled=move || loading.get()
                on:click=move |_| refresh_now.run(())
            >
                <IconRefresh size=13 />
            </button>
        </div>
    }
}
