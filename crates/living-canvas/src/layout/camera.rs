// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Bounded spatial camera history for infinite canvas navigation (ADR-0044).

use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use std::{cell::RefCell, rc::Rc};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, closure::Closure};

#[cfg(target_arch = "wasm32")]
thread_local! {
    static CAMERA_ANIMATION_GENERATION: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Immutable snapshot of a camera viewport state.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct CameraState {
    /// Pan offset in X (pixels).
    pub pan_x: f64,
    /// Pan offset in Y (pixels).
    pub pan_y: f64,
    /// Viewport zoom level.
    pub zoom: f64,
}

impl CameraState {
    /// Create a new camera state.
    #[must_use]
    pub const fn new(pan_x: f64, pan_y: f64, zoom: f64) -> Self {
        Self { pan_x, pan_y, zoom }
    }

    /// Check if this state is approximately identical to another.
    #[must_use]
    pub fn is_close_to(&self, other: &Self) -> bool {
        (self.pan_x - other.pan_x).abs() < 5.0
            && (self.pan_y - other.pan_y).abs() < 5.0
            && (self.zoom - other.zoom).abs() < 0.05
    }
}

/// Return the canvas point currently displayed at the viewport center.
#[must_use]
pub fn camera_center(pan: (f64, f64), zoom: f64, viewport: (f64, f64)) -> (f64, f64) {
    let safe_zoom = if zoom.is_finite() && zoom > 0.0 {
        zoom
    } else {
        1.0
    };
    (
        (viewport.0 / 2.0 - pan.0) / safe_zoom,
        (viewport.1 / 2.0 - pan.1) / safe_zoom,
    )
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn camera_ease(progress: f64) -> f64 {
    let remaining = 1.0 - progress.clamp(0.0, 1.0);
    1.0 - remaining * remaining * remaining
}

/// Bounded undo/redo stack for spatial camera positions.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CameraHistory {
    past: Vec<CameraState>,
    future: Vec<CameraState>,
    max_entries: usize,
}

impl Default for CameraHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl CameraHistory {
    /// Create a new camera history with default capacity (32 snapshots).
    #[must_use]
    pub fn new() -> Self {
        Self {
            past: Vec::with_capacity(32),
            future: Vec::with_capacity(32),
            max_entries: 32,
        }
    }

    /// Record a significant camera transition.
    pub fn record(&mut self, state: CameraState) {
        if self
            .past
            .last()
            .is_some_and(|last| last.is_close_to(&state))
        {
            return;
        }
        if self.past.len() >= self.max_entries {
            self.past.remove(0);
        }
        self.past.push(state);
        self.future.clear();
    }

    /// Navigate back in camera history.
    pub fn back(&mut self, current: CameraState) -> Option<CameraState> {
        let prev = self.past.pop()?;
        self.future.push(current);
        Some(prev)
    }

    /// Navigate forward in camera history.
    pub fn forward(&mut self, current: CameraState) -> Option<CameraState> {
        let next = self.future.pop()?;
        self.past.push(current);
        Some(next)
    }

    /// Can navigate back.
    #[must_use]
    pub fn can_back(&self) -> bool {
        !self.past.is_empty()
    }

    /// Can navigate forward.
    #[must_use]
    pub fn can_forward(&self) -> bool {
        !self.future.is_empty()
    }
}

/// Apply a camera history back operation to signals.
#[cfg(target_arch = "wasm32")]
pub fn apply_camera_back(
    camera_history: RwSignal<CameraHistory>,
    pan: ReadSignal<(f64, f64)>,
    set_pan: WriteSignal<(f64, f64)>,
    zoom: ReadSignal<f64>,
    set_zoom: WriteSignal<f64>,
) -> bool {
    let current = CameraState::new(
        pan.get_untracked().0,
        pan.get_untracked().1,
        zoom.get_untracked(),
    );
    let mut restored = None;
    camera_history.update(|h| {
        restored = h.back(current);
    });
    if let Some(target) = restored {
        set_pan.set((target.pan_x, target.pan_y));
        set_zoom.set(target.zoom);
        true
    } else {
        false
    }
}

/// Apply a camera history forward operation to signals.
#[cfg(target_arch = "wasm32")]
pub fn apply_camera_forward(
    camera_history: RwSignal<CameraHistory>,
    pan: ReadSignal<(f64, f64)>,
    set_pan: WriteSignal<(f64, f64)>,
    zoom: ReadSignal<f64>,
    set_zoom: WriteSignal<f64>,
) -> bool {
    let current = CameraState::new(
        pan.get_untracked().0,
        pan.get_untracked().1,
        zoom.get_untracked(),
    );
    let mut restored = None;
    camera_history.update(|h| {
        restored = h.forward(current);
    });
    if let Some(target) = restored {
        set_pan.set((target.pan_x, target.pan_y));
        set_zoom.set(target.zoom);
        true
    } else {
        false
    }
}

/// Fly camera to target center coordinates and zoom level, recording history.
#[cfg(target_arch = "wasm32")]
pub fn apply_camera_fly_to(
    camera_history: Option<RwSignal<CameraHistory>>,
    pan: ReadSignal<(f64, f64)>,
    set_pan: WriteSignal<(f64, f64)>,
    zoom: ReadSignal<f64>,
    set_zoom: WriteSignal<f64>,
    target_center_x: f64,
    target_center_y: f64,
    target_zoom: f64,
) {
    let current = CameraState::new(
        pan.get_untracked().0,
        pan.get_untracked().1,
        zoom.get_untracked(),
    );
    if let Some(ch) = camera_history {
        ch.update(|h| h.record(current));
    }

    let (w, h) = (
        web_sys::window()
            .and_then(|w| w.inner_width().ok())
            .and_then(|v| v.as_f64())
            .unwrap_or(1440.0),
        web_sys::window()
            .and_then(|w| w.inner_height().ok())
            .and_then(|v| v.as_f64())
            .unwrap_or(900.0),
    );

    let target_zoom = target_zoom.clamp(0.4, 2.0);
    let new_pan_x = (w / 2.0) - (target_center_x * target_zoom);
    let new_pan_y = (h / 2.0) - (target_center_y * target_zoom);
    let generation = CAMERA_ANIMATION_GENERATION.with(|current| {
        let next = current.get().wrapping_add(1);
        current.set(next);
        next
    });
    let reduced_motion = web_sys::window()
        .and_then(|window| {
            window
                .match_media("(prefers-reduced-motion: reduce)")
                .ok()
                .flatten()
        })
        .is_some_and(|query| query.matches());
    if reduced_motion {
        set_zoom.set(target_zoom);
        set_pan.set((new_pan_x, new_pan_y));
        return;
    }

    let start_pan = pan.get_untracked();
    let start_zoom = zoom.get_untracked();
    let started_at = Rc::new(RefCell::new(None::<f64>));
    let animation = Rc::new(RefCell::new(None::<Closure<dyn FnMut(f64)>>));
    let animation_callback = Rc::clone(&animation);
    let started_at_callback = Rc::clone(&started_at);

    *animation_callback.borrow_mut() = Some(Closure::wrap(Box::new(move |timestamp: f64| {
        let still_current = CAMERA_ANIMATION_GENERATION.with(|current| current.get() == generation);
        if !still_current {
            animation.borrow_mut().take();
            return;
        }

        let start = *started_at_callback.borrow_mut().get_or_insert(timestamp);
        let progress = ((timestamp - start) / 200.0).clamp(0.0, 1.0);
        let eased = camera_ease(progress);
        set_pan.set((
            start_pan.0 + (new_pan_x - start_pan.0) * eased,
            start_pan.1 + (new_pan_y - start_pan.1) * eased,
        ));
        set_zoom.set(start_zoom + (target_zoom - start_zoom) * eased);

        if progress < 1.0 {
            if let (Some(window), Some(callback)) = (web_sys::window(), animation.borrow().as_ref())
            {
                let _ = window.request_animation_frame(callback.as_ref().unchecked_ref());
            }
        } else {
            animation.borrow_mut().take();
        }
    }) as Box<dyn FnMut(f64)>));

    if let (Some(window), Some(callback)) =
        (web_sys::window(), animation_callback.borrow().as_ref())
    {
        let _ = window.request_animation_frame(callback.as_ref().unchecked_ref());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_history_records_and_navigates_back_forward() {
        let mut history = CameraHistory::new();
        let state1 = CameraState::new(0.0, 0.0, 1.0);
        let state2 = CameraState::new(200.0, 300.0, 1.2);
        let state3 = CameraState::new(500.0, 800.0, 0.8);

        history.record(state1);
        history.record(state2);

        assert!(history.can_back());
        assert!(!history.can_forward());

        // Back from state3
        let back1 = history.back(state3).unwrap();
        assert_eq!(back1, state2);
        assert!(history.can_forward());

        // Back again
        let back2 = history.back(back1).unwrap();
        assert_eq!(back2, state1);

        // Forward
        let fwd1 = history.forward(back2).unwrap();
        assert_eq!(fwd1, state2);

        let fwd2 = history.forward(fwd1).unwrap();
        assert_eq!(fwd2, state3);
    }

    #[test]
    fn camera_center_inverts_fly_to_pan() {
        let viewport = (1440.0, 900.0);
        let target = (620.0, 410.0);
        let zoom = 1.25;
        let pan = (
            viewport.0 / 2.0 - target.0 * zoom,
            viewport.1 / 2.0 - target.1 * zoom,
        );

        assert_eq!(camera_center(pan, zoom, viewport), target);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn camera_easing_is_bounded_and_reaches_both_endpoints() {
        assert_eq!(camera_ease(-1.0), 0.0);
        assert_eq!(camera_ease(0.0), 0.0);
        assert!(camera_ease(0.5) > 0.5);
        assert_eq!(camera_ease(1.0), 1.0);
        assert_eq!(camera_ease(2.0), 1.0);
    }
}

/// How far outside the window a card is still drawn.
///
/// Wide on purpose. The cost this saves is drawing a panel nobody can see; the cost it risks is a
/// panel arriving late as somebody pans towards it, and the second is the one a person notices. A
/// margin of this size means a card is ready well before its edge reaches the window.
pub const OFFSCREEN_MARGIN: f64 = 600.0;

/// Whether a card at this geometry is near enough to the window to be worth drawing.
///
/// ADR-0044 names this as the cost of an infinite canvas: dozens of live reactive panels, all of
/// them in the DOM whether or not anybody can see them. Every card here holds signals that update
/// on a timer, so an off-screen panel is not merely idle markup — it is work.
///
/// The stage is drawn as `translate3d(pan) scale(zoom)` from an origin at its top left, so a card
/// at canvas `(x, y)` reaches the window at `pan + (x, y) * zoom`. That is the whole of the
/// arithmetic, and it is here rather than in the component so it can be checked without one.
///
/// Answers `true` whenever the camera cannot be believed — a zoom of zero, a viewport nothing has
/// measured yet, an infinity from a division somewhere upstream. A card must never be hidden
/// because a number arrived wrong: the failure of this function has to be drawing too much.
#[must_use]
pub fn is_within_view(
    geometry: crate::CardGeometry,
    pan: (f64, f64),
    zoom: f64,
    viewport: (f64, f64),
) -> bool {
    if !(zoom.is_finite() && zoom > 0.0) {
        return true;
    }
    if !(pan.0.is_finite() && pan.1.is_finite()) {
        return true;
    }
    // A viewport of zero is a window nobody has measured, not a window nothing fits in.
    if !(viewport.0.is_finite() && viewport.1.is_finite()) || viewport.0 <= 0.0 || viewport.1 <= 0.0
    {
        return true;
    }

    let left = pan.0 + geometry.x * zoom;
    let top = pan.1 + geometry.y * zoom;
    let right = left + geometry.width * zoom;
    let bottom = top + geometry.height * zoom;

    right >= -OFFSCREEN_MARGIN
        && bottom >= -OFFSCREEN_MARGIN
        && left <= viewport.0 + OFFSCREEN_MARGIN
        && top <= viewport.1 + OFFSCREEN_MARGIN
}

#[cfg(test)]
mod culling_tests {
    use super::{OFFSCREEN_MARGIN, is_within_view};
    use crate::CardGeometry;

    const WINDOW: (f64, f64) = (1280.0, 800.0);

    fn card(x: f64, y: f64) -> CardGeometry {
        CardGeometry::new(x, y, (360.0, 260.0), 1)
    }

    #[test]
    fn a_card_in_the_window_is_drawn() {
        assert!(is_within_view(card(100.0, 100.0), (0.0, 0.0), 1.0, WINDOW));
    }

    #[test]
    fn a_card_far_away_is_not() {
        // The case this exists for: an infinite canvas somebody has panned across, with the panels
        // they left behind still updating on a timer.
        assert!(!is_within_view(card(9000.0, 0.0), (0.0, 0.0), 1.0, WINDOW));
        assert!(!is_within_view(card(0.0, 9000.0), (0.0, 0.0), 1.0, WINDOW));
        assert!(!is_within_view(card(-9000.0, 0.0), (0.0, 0.0), 1.0, WINDOW));
    }

    #[test]
    fn a_card_just_outside_the_window_is_still_drawn() {
        // Inside the margin, so panning towards it finds it already there rather than watching it
        // arrive. The margin is the difference between culling nobody notices and a desktop that
        // flickers at its edges.
        let just_past = WINDOW.0 + OFFSCREEN_MARGIN / 2.0;
        assert!(is_within_view(
            card(just_past, 0.0),
            (0.0, 0.0),
            1.0,
            WINDOW
        ));
    }

    #[test]
    fn the_pan_moves_what_is_visible() {
        let far = card(4000.0, 0.0);
        assert!(!is_within_view(far, (0.0, 0.0), 1.0, WINDOW));
        // Panned so that card is now in front of the window.
        assert!(is_within_view(far, (-3900.0, 0.0), 1.0, WINDOW));
    }

    #[test]
    fn zooming_out_brings_distant_cards_back() {
        // At a tenth scale the canvas that did not fit now does, and every card on it has to be
        // drawn again — the overview is exactly when a person is looking at all of them.
        let far = card(4000.0, 2000.0);
        assert!(!is_within_view(far, (0.0, 0.0), 1.0, WINDOW));
        assert!(is_within_view(far, (0.0, 0.0), 0.1, WINDOW));
    }

    #[test]
    fn a_camera_that_cannot_be_believed_draws_everything() {
        // The failure of this function has to be drawing too much. A card hidden because a number
        // arrived wrong is a panel a person cannot find and cannot explain.
        let anywhere = card(9000.0, 9000.0);
        assert!(is_within_view(anywhere, (0.0, 0.0), 0.0, WINDOW));
        assert!(is_within_view(anywhere, (0.0, 0.0), -1.0, WINDOW));
        assert!(is_within_view(anywhere, (0.0, 0.0), f64::NAN, WINDOW));
        assert!(is_within_view(anywhere, (f64::NAN, 0.0), 1.0, WINDOW));
        assert!(is_within_view(anywhere, (f64::INFINITY, 0.0), 1.0, WINDOW));
    }

    #[test]
    fn a_window_nobody_has_measured_draws_everything() {
        // Zero is a viewport that has not been laid out, not a viewport nothing fits in. Reading it
        // as the second would blank the desktop on the first frame.
        let anywhere = card(9000.0, 9000.0);
        assert!(is_within_view(anywhere, (0.0, 0.0), 1.0, (0.0, 0.0)));
        assert!(is_within_view(anywhere, (0.0, 0.0), 1.0, (1280.0, 0.0)));
        assert!(is_within_view(anywhere, (0.0, 0.0), 1.0, (f64::NAN, 800.0)));
    }

    #[test]
    fn a_card_larger_than_the_window_is_drawn_from_either_edge() {
        // A panel dragged wider than the screen is visible while neither of its corners is.
        let huge = CardGeometry::new(-500.0, -500.0, (4000.0, 3000.0), 1);
        assert!(is_within_view(huge, (0.0, 0.0), 1.0, WINDOW));
    }
}

/// The closest and furthest the canvas may be taken.
///
/// The same pair the wheel and the camera flights already use, written once here so a gesture and a
/// keystroke cannot disagree about how far out the desktop goes.
pub const MIN_ZOOM: f64 = 0.4;
/// See [`MIN_ZOOM`].
pub const MAX_ZOOM: f64 = 2.0;

/// Where the canvas is after one step of a two-finger gesture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pinch {
    /// The new scale.
    pub zoom: f64,
    /// The new translation.
    pub pan: (f64, f64),
}

/// Advance a pinch by one move.
///
/// Touch had no way to zoom at all: the wheel gesture wants a modifier key and the buttons are a
/// control rather than a gesture, so a person on a tablet could pan an infinite canvas and never
/// change how much of it they could see.
///
/// Two things happen at once and both belong here. The distance between the fingers scales the
/// canvas about the point between them, which is what makes a pinch feel like it is holding the
/// thing under the fingers rather than the corner of the screen. And the point between them moves,
/// which pans. Doing only the first makes the canvas slide out from under a two-finger drag.
///
/// The scale is applied after clamping rather than before, or a pinch that ran past the limit would
/// keep translating as though it had not: the fingers would still spread and the canvas would still
/// drift while its size stayed put.
#[must_use]
pub fn pinch_step(
    zoom: f64,
    pan: (f64, f64),
    previous: ((f64, f64), (f64, f64)),
    current: ((f64, f64), (f64, f64)),
) -> Pinch {
    let unchanged = Pinch { zoom, pan };

    if !(zoom.is_finite() && zoom > 0.0) {
        return unchanged;
    }
    for point in [previous.0, previous.1, current.0, current.1] {
        if !(point.0.is_finite() && point.1.is_finite()) {
            return unchanged;
        }
    }

    let distance = |a: (f64, f64), b: (f64, f64)| (b.0 - a.0).hypot(b.1 - a.1);
    let midpoint =
        |a: (f64, f64), b: (f64, f64)| (f64::midpoint(a.0, b.0), f64::midpoint(a.1, b.1));

    let was = distance(previous.0, previous.1);
    let now = distance(current.0, current.1);
    // Two fingers in the same place have no distance to compare, and dividing by it would send the
    // canvas to infinity on the frame somebody's fingers happened to touch.
    if was < 1.0 || now < 1.0 {
        return unchanged;
    }

    let new_zoom = (zoom * (now / was)).clamp(MIN_ZOOM, MAX_ZOOM);
    let applied = new_zoom / zoom;

    let from = midpoint(previous.0, previous.1);
    let to = midpoint(current.0, current.1);

    Pinch {
        zoom: new_zoom,
        pan: (
            to.0 - (from.0 - pan.0) * applied,
            to.1 - (from.1 - pan.1) * applied,
        ),
    }
}

#[cfg(test)]
mod pinch_tests {
    use super::{MAX_ZOOM, MIN_ZOOM, Pinch, pinch_step};

    /// Two fingers 100 apart, centred on the same point, spread to 200.
    #[test]
    fn spreading_the_fingers_zooms_in() {
        let step = pinch_step(
            1.0,
            (0.0, 0.0),
            ((450.0, 400.0), (550.0, 400.0)),
            ((400.0, 400.0), (600.0, 400.0)),
        );
        assert!((step.zoom - 2.0).abs() < 1e-9, "{step:?}");
    }

    #[test]
    fn pinching_them_together_zooms_out() {
        let step = pinch_step(
            1.0,
            (0.0, 0.0),
            ((400.0, 400.0), (600.0, 400.0)),
            ((450.0, 400.0), (550.0, 400.0)),
        );
        assert!((step.zoom - 0.5).abs() < 1e-9, "{step:?}");
    }

    #[test]
    fn the_point_between_the_fingers_stays_where_it_was() {
        // What makes a pinch feel like it is holding the thing under the fingers rather than the
        // corner of the screen. The canvas point under the midpoint before must be under it after.
        let pan = (120.0, -60.0);
        let zoom = 1.0;
        let previous = ((450.0, 380.0), (550.0, 420.0));
        let current = ((400.0, 360.0), (600.0, 440.0));
        let midpoint = (500.0, 400.0);

        let before = ((midpoint.0 - pan.0) / zoom, (midpoint.1 - pan.1) / zoom);
        let step = pinch_step(zoom, pan, previous, current);
        let after = (
            (midpoint.0 - step.pan.0) / step.zoom,
            (midpoint.1 - step.pan.1) / step.zoom,
        );

        assert!((before.0 - after.0).abs() < 1e-9, "{before:?} {after:?}");
        assert!((before.1 - after.1).abs() < 1e-9, "{before:?} {after:?}");
    }

    #[test]
    fn two_fingers_moving_together_pan_without_zooming() {
        // A two-finger drag at a constant separation. Zooming about the midpoint without also
        // following it is what makes the canvas slide out from under the gesture.
        let step = pinch_step(
            1.0,
            (0.0, 0.0),
            ((400.0, 400.0), (500.0, 400.0)),
            ((430.0, 380.0), (530.0, 380.0)),
        );
        assert!((step.zoom - 1.0).abs() < 1e-9);
        assert!((step.pan.0 - 30.0).abs() < 1e-9, "{step:?}");
        assert!((step.pan.1 + 20.0).abs() < 1e-9, "{step:?}");
    }

    #[test]
    fn the_canvas_stops_at_its_limits() {
        let wide_apart = ((0.0, 400.0), (1200.0, 400.0));
        let close = ((595.0, 400.0), (605.0, 400.0));

        let zoomed_in = pinch_step(1.9, (0.0, 0.0), close, wide_apart);
        assert!((zoomed_in.zoom - MAX_ZOOM).abs() < 1e-9, "{zoomed_in:?}");

        let zoomed_out = pinch_step(0.5, (0.0, 0.0), wide_apart, close);
        assert!((zoomed_out.zoom - MIN_ZOOM).abs() < 1e-9, "{zoomed_out:?}");
    }

    #[test]
    fn a_gesture_past_the_limit_stops_translating_too() {
        // The reason the scale is applied after clamping. If it were not, the fingers would keep
        // spreading, the size would stay put, and the canvas would drift away underneath them.
        let close = ((595.0, 400.0), (605.0, 400.0));
        let wider = ((300.0, 400.0), (900.0, 400.0));
        let widest = ((0.0, 400.0), (1200.0, 400.0));

        let first = pinch_step(MAX_ZOOM, (0.0, 0.0), close, wider);
        let second = pinch_step(first.zoom, first.pan, wider, widest);

        // Already at the limit, so the only movement left is the midpoint, which has not moved.
        // Compared exactly on purpose: `clamp` returns the bound itself, so anything but the bound
        // means the clamp did not happen — which is the thing being asserted.
        #[allow(
            clippy::float_cmp,
            reason = "clamp returns the bound, so equality is the claim"
        )]
        {
            assert_eq!(first.zoom, MAX_ZOOM);
            assert_eq!(second.zoom, MAX_ZOOM);
        }
        assert!(
            (second.pan.0 - first.pan.0).abs() < 1e-9,
            "{first:?} {second:?}"
        );
    }

    #[test]
    fn fingers_in_the_same_place_change_nothing() {
        // Dividing by that distance would send the canvas to infinity on the frame two fingers
        // happened to touch.
        let same = ((500.0, 400.0), (500.0, 400.0));
        let apart = ((400.0, 400.0), (600.0, 400.0));

        assert_eq!(
            pinch_step(1.0, (7.0, 9.0), same, apart),
            Pinch {
                zoom: 1.0,
                pan: (7.0, 9.0)
            }
        );
        assert_eq!(
            pinch_step(1.0, (7.0, 9.0), apart, same),
            Pinch {
                zoom: 1.0,
                pan: (7.0, 9.0)
            }
        );
    }

    #[test]
    fn a_gesture_that_cannot_be_believed_changes_nothing() {
        let apart = ((400.0, 400.0), (600.0, 400.0));
        let broken = ((f64::NAN, 400.0), (600.0, 400.0));

        assert_eq!(
            pinch_step(1.0, (0.0, 0.0), apart, broken),
            Pinch {
                zoom: 1.0,
                pan: (0.0, 0.0)
            }
        );
        assert_eq!(
            pinch_step(f64::NAN, (0.0, 0.0), apart, apart).pan,
            (0.0, 0.0)
        );
    }
}

/// The narrowest a window can be and still be a canvas.
///
/// Below this a panel is wider than the screen it is on, so a spatial desktop asks somebody to pan
/// sideways to read a sentence. The number is where the smallest card this build opens — 380 pixels
/// at its minimum width — stops fitting beside anything at all with room to hold it.
pub const NARROWEST_CANVAS: f64 = 760.0;

/// How the desktop is laid out, which is a fact about the window rather than a preference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Presentation {
    /// Panels at the coordinates the layout gives them, on a plane that pans and zooms.
    Spatial,
    /// Panels in one column, full width, in the order the layout holds them.
    ///
    /// ADR-0044 calls this a cluster stack view. It is not a smaller canvas: pan and zoom are
    /// meaningless when everything is already as wide as the screen, and a person on a phone
    /// scrolls rather than flies.
    Stacked,
}

/// Which of the two a window of this width gets.
///
/// A window nobody has measured is `Spatial`, because zero is a viewport that has not been laid out
/// and stacking the desktop on the strength of it would rearrange every panel on the first frame
/// and rearrange them back on the second.
#[must_use]
pub fn presentation_for(viewport_width: f64) -> Presentation {
    if !viewport_width.is_finite() || viewport_width <= 0.0 {
        return Presentation::Spatial;
    }
    if viewport_width < NARROWEST_CANVAS {
        Presentation::Stacked
    } else {
        Presentation::Spatial
    }
}

#[cfg(test)]
mod presentation_tests {
    use super::{NARROWEST_CANVAS, Presentation, presentation_for};

    #[test]
    fn a_phone_gets_a_stack_and_a_desk_gets_a_canvas() {
        assert_eq!(presentation_for(390.0), Presentation::Stacked);
        assert_eq!(presentation_for(1440.0), Presentation::Spatial);
    }

    #[test]
    fn the_boundary_belongs_to_the_canvas() {
        // Exactly at the threshold there is room, so the canvas keeps it. Stacking a window that
        // fits would be taking the desktop away from somebody who could use it.
        assert_eq!(presentation_for(NARROWEST_CANVAS), Presentation::Spatial);
        assert_eq!(
            presentation_for(NARROWEST_CANVAS - 1.0),
            Presentation::Stacked
        );
    }

    #[test]
    fn a_window_nobody_has_measured_is_not_a_narrow_one() {
        // Zero is a viewport that has not been laid out. Reading it as narrow would stack every
        // panel on the first frame and unstack them on the second, in front of the person.
        assert_eq!(presentation_for(0.0), Presentation::Spatial);
        assert_eq!(presentation_for(f64::NAN), Presentation::Spatial);
        assert_eq!(presentation_for(-1.0), Presentation::Spatial);
    }
}
