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
