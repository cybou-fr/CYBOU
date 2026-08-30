// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Where the camera is, for the parts of the desktop that need to know without being the viewport.
//!
//! Only one thing needs it so far — a card deciding whether it is worth drawing — and that is
//! reason enough for it to travel as context rather than as four more props threaded through every
//! card component.

use leptos::prelude::*;
use wasm_bindgen::{JsCast as _, prelude::Closure};

/// Pan, zoom, and the size of the window they are drawn into.
#[derive(Clone, Copy)]
pub struct CanvasCamera {
    /// Stage translation, in screen pixels.
    pub pan: ReadSignal<(f64, f64)>,
    /// Stage scale.
    pub zoom: ReadSignal<f64>,
    /// The window, as last measured.
    ///
    /// A signal rather than a read at the moment of use, because a window resized while nothing
    /// else changes would otherwise leave every card deciding from a size that is no longer true —
    /// and the cards that decided they were off-screen would stay gone.
    pub viewport: ReadSignal<(f64, f64)>,
}

impl CanvasCamera {
    /// Whether this card is near enough to the window to be worth drawing.
    #[must_use]
    pub fn shows(&self, geometry: crate::CardGeometry) -> bool {
        crate::layout::camera::is_within_view(
            geometry,
            self.pan.get(),
            self.zoom.get(),
            self.viewport.get(),
        )
    }
}

/// Measure the window, and keep measuring it.
///
/// Returns the signal to put in [`CanvasCamera`]. The listener is leaked deliberately: it lives as
/// long as the document does, and the desktop is the document.
#[must_use]
pub fn window_size() -> ReadSignal<(f64, f64)> {
    let (viewport, set_viewport) = signal(measure());
    set_viewport.set(measure());

    if let Some(window) = web_sys::window() {
        let on_resize = Closure::<dyn FnMut()>::new(move || set_viewport.set(measure()));
        let _ =
            window.add_event_listener_with_callback("resize", on_resize.as_ref().unchecked_ref());
        on_resize.forget();
    }

    viewport
}

/// The window as two numbers, or zeroes if it cannot be asked.
///
/// Zero is read downstream as *nobody has measured this*, which draws everything — the honest
/// answer for a window this function could not see.
fn measure() -> (f64, f64) {
    let Some(window) = web_sys::window() else {
        return (0.0, 0.0);
    };
    let width = window.inner_width().ok().and_then(|value| value.as_f64());
    let height = window.inner_height().ok().and_then(|value| value.as_f64());
    (width.unwrap_or(0.0), height.unwrap_or(0.0))
}
