// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Panels that keep looking, and panels that say when they last looked.
//!
//! The System cards live off one snapshot the gateway pushes over an `EventSource`, so they are
//! current by construction. The tool panels — Monitor, Services, Processes, the log viewer — each
//! fetched once when they were opened and then showed that reading forever. A `MonitorSignals`
//! carried an `auto_refresh` flag, set to `true`, that nothing in the crate read.
//!
//! A frozen number is worse than a missing one: a CPU graph from eleven minutes ago looks exactly
//! like a CPU graph. So this module does two things, and the second matters as much as the first.
//! [`keep_reading`] asks again on a timer, and `Freshness` lets the panel say out loud how old what
//! it is showing is — because the timer can fail. When the gateway goes away, the panel that only
//! refreshes silently is once again showing a frozen number, and the age is the only thing that
//! still tells the truth.
//!
//! Everything reactive here is browser-only, because `leptos` is. [`age_in_words`] is not: it is
//! arithmetic and wording, it is where this module can get the answer wrong in a way a person would
//! notice, and it is tested where tests are cheap to run.

#[cfg(target_arch = "wasm32")]
pub use browser::{DesktopClock, Freshness, keep_reading, provide_desktop_clock};

/// How old a reading is, in words, or `None` when nothing has been read yet.
///
/// Deliberately coarse. This is a claim about staleness, not a stopwatch, and a label that resolved
/// to the second would invite reading it as one.
#[must_use]
pub fn age_in_words(read_at: Option<f64>, now: Option<f64>) -> Option<String> {
    let (read_at, now) = (read_at?, now?);
    // Clamped, because the wall clock underneath this can be moved by the person using it, and a
    // negative age would render as a panel claiming to know something before it asked.
    let seconds = ((now - read_at) / 1000.0).max(0.0);
    Some(if seconds < 10.0 {
        "just now".to_owned()
    } else if seconds < 90.0 {
        format!("{seconds:.0}s ago")
    } else if seconds < 5400.0 {
        format!("{:.0}m ago", seconds / 60.0)
    } else {
        format!("{:.0}h ago", seconds / 3600.0)
    })
}

#[cfg(target_arch = "wasm32")]
mod browser {
    use leptos::prelude::*;

    /// How often the shared clock advances, in milliseconds.
    ///
    /// Five seconds rather than one. Every age label in the desktop redraws on this tick, and the
    /// difference between "12 seconds ago" and "17 seconds ago" is not worth waking the whole
    /// viewport five times to say. A relative label may be one tick behind and still be honest.
    const CLOCK_TICK_MS: u32 = 5_000;

    /// The browser's own wall clock, in milliseconds.
    fn now_ms() -> f64 {
        js_sys::Date::now()
    }

    /// When a panel's last successful reading came back.
    ///
    /// The instant is a wall clock. It is used for a difference and never for ordering against
    /// anything the host said, which is why a clock the user could have set wrongly is good enough
    /// here and would not be anywhere else.
    #[derive(Clone, Copy)]
    pub struct Freshness {
        read_at: RwSignal<Option<f64>>,
    }

    impl Default for Freshness {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Freshness {
        /// A panel that has not yet read anything.
        #[must_use]
        pub fn new() -> Self {
            Self {
                read_at: RwSignal::new(None),
            }
        }

        /// Record that a reading arrived. Called on success only: a failed fetch leaves the age
        /// climbing, which is the whole point of showing it.
        pub fn arrived(self) {
            self.read_at.set(Some(now_ms()));
        }

        /// The instant of the last reading, or `None` if nothing has arrived yet.
        #[must_use]
        pub fn read_at(self) -> Option<f64> {
            self.read_at.get()
        }
    }

    /// A clock every age label reads, so a panel nobody is refreshing still visibly ages.
    ///
    /// Without this the label would only redraw when a reading arrived, so a gateway that had
    /// stopped answering would leave "3s ago" on screen indefinitely — the exact lie the label
    /// exists to stop.
    #[derive(Clone, Copy)]
    pub struct DesktopClock(RwSignal<f64>);

    impl DesktopClock {
        /// The current instant, as a reactive read.
        #[must_use]
        pub fn now(self) -> f64 {
            self.0.get()
        }
    }

    /// Start the shared clock and put it in context. Called once, by the app root.
    pub fn provide_desktop_clock() {
        let clock = DesktopClock(RwSignal::new(now_ms()));
        let interval = gloo_timers::callback::Interval::new(CLOCK_TICK_MS, move || {
            clock.0.set(now_ms());
        });
        // Held for the life of the app. Dropping an `Interval` cancels it, so it has to be kept
        // somewhere rather than let go at the end of this function.
        let held = StoredValue::new_local(Some(interval));
        on_cleanup(move || held.update_value(|slot| drop(slot.take())));
        provide_context(clock);
    }

    /// Whether the page is currently hidden — another tab, or a minimized window.
    ///
    /// A hidden desktop is not being read, and a timer that kept fetching for it would spend the
    /// host's work and the user's battery producing readings nobody sees. The panel does not become
    /// wrong while this is true: it becomes visibly old, which is what the age label is for.
    fn page_is_hidden() -> bool {
        web_sys::window()
            .and_then(|window| window.document())
            .is_some_and(|document| document.hidden())
    }

    /// Ask again, on a timer, for as long as the card is open.
    ///
    /// `enabled` is the panel's own toggle, so a person watching one number can stop the desktop
    /// talking to the host behind their back. `busy` is the panel's in-flight flag: a tick that
    /// fired while the last answer had not arrived is dropped rather than queued, because a slow
    /// gateway would otherwise accumulate one outstanding request per tick and never catch up.
    ///
    /// The interval is cancelled when the card closes. Leptos drops the reactive owner then, and an
    /// `Interval` that is dropped stops — so the timer cannot outlive the panel it read for.
    pub fn keep_reading(
        interval_ms: u32,
        enabled: RwSignal<bool>,
        busy: RwSignal<bool>,
        read: impl Fn() + 'static,
    ) {
        let interval = gloo_timers::callback::Interval::new(interval_ms, move || {
            if !enabled.get_untracked() || busy.get_untracked() || page_is_hidden() {
                return;
            }
            read();
        });
        let held = StoredValue::new_local(Some(interval));
        on_cleanup(move || held.update_value(|slot| drop(slot.take())));
    }
}

#[cfg(test)]
mod tests {
    use super::age_in_words;

    #[test]
    fn an_age_is_words_and_not_a_stopwatch() {
        let read = 1_000_000.0;
        assert_eq!(
            age_in_words(Some(read), Some(read + 3_000.0)).as_deref(),
            Some("just now")
        );
        assert_eq!(
            age_in_words(Some(read), Some(read + 42_000.0)).as_deref(),
            Some("42s ago")
        );
        assert_eq!(
            age_in_words(Some(read), Some(read + 600_000.0)).as_deref(),
            Some("10m ago")
        );
        assert_eq!(
            age_in_words(Some(read), Some(read + 7_200_000.0)).as_deref(),
            Some("2h ago")
        );
    }

    #[test]
    fn nothing_read_yet_is_not_an_age_of_zero() {
        // A panel that has never answered must not be able to render as fresh, which is what a
        // default of `0` would have made it.
        assert_eq!(age_in_words(None, Some(1_000_000.0)), None);
        assert_eq!(age_in_words(Some(1_000_000.0), None), None);
    }

    #[test]
    fn a_clock_that_ran_backwards_does_not_produce_a_reading_from_the_future() {
        let read = 1_000_000.0;
        assert_eq!(
            age_in_words(Some(read), Some(read - 60_000.0)).as_deref(),
            Some("just now")
        );
    }
}
