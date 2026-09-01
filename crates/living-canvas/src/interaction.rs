// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Interactive pointer and keyboard event handling, dragging, resizing, and snapping.

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlElement, KeyboardEvent, PointerEvent};

use crate::{
    CardId, DesktopItemId, DesktopLayout, DesktopViewMode, LayoutHistory, SnapGuide, UsableViewport,
};

/// Target item for a pointer drag operation.
#[derive(Clone, Debug, PartialEq)]
pub enum DragTarget {
    /// Standalone Card instance.
    Card(CardId),
    /// Deck grouping.
    Deck(String),
}

/// Active pointer drag operation state.
#[derive(Clone, Debug, PartialEq)]
pub struct DragState {
    /// Dragged target.
    pub target: DragTarget,
    /// Offset X within the dragged card element.
    pub offset_x: f64,
    /// Offset Y within the dragged card element.
    pub offset_y: f64,
    /// Width of the dragged card.
    pub width: f64,
    /// Height of the dragged card.
    pub height: f64,
    /// Hovered card candidate for magnetic deck grouping.
    pub drop_target: Option<CardId>,
}

/// Target item for a resize operation.
#[derive(Clone, Debug, PartialEq)]
pub enum ResizeTarget {
    /// Standalone Card instance.
    Card(CardId),
    /// Deck grouping.
    Deck(String),
}

/// Active resize operation state.
#[derive(Clone, Debug, PartialEq)]
pub struct ResizeState {
    /// Resized target.
    pub target: ResizeTarget,
    /// Initial pointer X position.
    pub start_pointer_x: f64,
    /// Initial pointer Y position.
    pub start_pointer_y: f64,
    /// Initial target width.
    pub start_width: f64,
    /// Initial target height.
    pub start_height: f64,
}

/// How much room the desktop actually has, in CSS pixels.
///
/// Every arrangement used to be called with `None`, which meant a hardcoded 1440x900 whatever the
/// window was. On a maximised screen the cards were laid out for a smaller desktop than the one
/// they were on, and "Fit All" then had to shrink the result to something like 61% — which is what
/// a person saw as the arrangement changing the zoom by itself.
///
/// The topbar and the dock are subtracted because a card placed under either is a card the person
/// cannot reach.
#[must_use]
pub fn usable_viewport() -> UsableViewport {
    const TOPBAR: f64 = 64.0;
    const DOCK: f64 = 72.0;
    let window = web_sys::window();
    let measure = |value: Option<f64>, fallback: f64| match value {
        Some(size) if size.is_finite() && size > 0.0 => size,
        _ => fallback,
    };
    let width = measure(
        window
            .as_ref()
            .and_then(|w| w.inner_width().ok())
            .and_then(|value| value.as_f64()),
        1440.0,
    );
    let height = measure(
        window
            .as_ref()
            .and_then(|w| w.inner_height().ok())
            .and_then(|value| value.as_f64()),
        900.0,
    );
    UsableViewport {
        width: (width - 48.0).max(640.0),
        height: (height - TOPBAR - DOCK).max(480.0),
    }
}

/// The part of the canvas the window is currently showing, in canvas coordinates.
///
/// The stage is translated by the pan and scaled about its own origin, so a window pixel `s` is the
/// canvas point `(s - pan) / zoom`. Everything that has to place something where a person is
/// looking needs this, and computing it twice in two files is how the two would come to disagree.
#[must_use]
pub fn visible_canvas_rect(pan: (f64, f64), zoom: f64) -> crate::layout::model::Rect {
    let viewport = usable_viewport();
    let zoom = if zoom.is_finite() && zoom > 0.0 {
        zoom
    } else {
        1.0
    };
    crate::layout::model::Rect::new(
        -pan.0 / zoom,
        -pan.1 / zoom,
        viewport.width / zoom,
        viewport.height / zoom,
    )
}

/// Generate CSS inline style for a card item given current layout and focus mode.
#[must_use]
pub fn card_style(layout: DesktopLayout, card: CardId) -> String {
    let geom = layout.geometry(card);
    let pres = layout.presentation(card);
    let view_mode =
        use_context::<RwSignal<DesktopViewMode>>().map_or(DesktopViewMode::Spatial, |vm| vm.get());

    if view_mode == DesktopViewMode::Focus(DesktopItemId::Card(card)) {
        "position:fixed;left:20px;top:20px;width:calc(100vw - 40px);height:calc(100vh - 100px);z-index:9999;box-shadow:0 0 0 9999px rgba(0,0,0,0.65);"
            .to_string()
    } else if pres.collapsed {
        format!(
            "left:{:.1}px;top:{:.1}px;width:{:.1}px;z-index:{}",
            geom.x, geom.y, geom.width, geom.z
        )
    } else if pres.representation == crate::PanelRepresentation::Glance {
        format!(
            "left:{:.1}px;top:{:.1}px;width:{:.1}px;height:74px;z-index:{}",
            geom.x,
            geom.y,
            geom.width.clamp(200.0, 280.0),
            geom.z
        )
    } else if pres.representation == crate::PanelRepresentation::Expanded {
        format!(
            "left:{:.1}px;top:{:.1}px;width:{:.1}px;height:{:.1}px;z-index:{}",
            geom.x,
            geom.y,
            geom.width.max(520.0),
            geom.height.max(400.0),
            geom.z
        )
    } else {
        format!(
            "left:{:.1}px;top:{:.1}px;width:{:.1}px;height:{:.1}px;z-index:{}",
            geom.x, geom.y, geom.width, geom.height, geom.z
        )
    }
}

/// Absolute placement for the action attached to the current selection.
///
/// The arithmetic is in [`crate::layout::selection`], where it can be tested without a browser —
/// this used to resolve the selection through a kind key, so clicking the third Shell card acted on

/// Compute start, end, and label center coordinates for a relationship edge between two cards.
#[must_use]
pub fn relationship_points(
    layout: DesktopLayout,
    from: CardId,
    to: CardId,
) -> (f64, f64, f64, f64, f64, f64) {
    let from_geom = layout.geometry(from);
    let to_geom = layout.geometry(to);
    let from_pres = layout.presentation(from);
    let to_pres = layout.presentation(to);

    let from_height = if from_pres.collapsed {
        44.0
    } else {
        from_geom.height
    };
    let to_height = if to_pres.collapsed {
        44.0
    } else {
        to_geom.height
    };

    let from_size = (from_geom.width, from_height);
    let to_size = (to_geom.width, to_height);
    let from_center = (
        from_geom.x + from_size.0 / 2.0,
        from_geom.y + from_size.1 / 2.0,
    );
    let to_center = (to_geom.x + to_size.0 / 2.0, to_geom.y + to_size.1 / 2.0);

    let (x1, y1) = edge_anchor(from_center, from_size, to_center);
    let (x2, y2) = edge_anchor(to_center, to_size, from_center);
    (
        x1,
        y1,
        x2,
        y2,
        f64::midpoint(x1, x2),
        f64::midpoint(y1, y2) - 7.0,
    )
}

/// Find boundary intersection point for an edge pointing towards target.
#[must_use]
pub fn edge_anchor(center: (f64, f64), size: (f64, f64), target: (f64, f64)) -> (f64, f64) {
    let dx = target.0 - center.0;
    let dy = target.1 - center.1;
    if dx.abs() < f64::EPSILON && dy.abs() < f64::EPSILON {
        return center;
    }
    let x_scale = if dx.abs() < f64::EPSILON {
        f64::INFINITY
    } else {
        size.0 / 2.0 / dx.abs()
    };
    let y_scale = if dy.abs() < f64::EPSILON {
        f64::INFINITY
    } else {
        size.1 / 2.0 / dy.abs()
    };
    let scale = x_scale.min(y_scale);
    (center.0 + dx * scale, center.1 + dy * scale)
}

/// Start dragging a card.
pub fn start_drag(
    event: PointerEvent,
    card: CardId,
    layout: RwSignal<DesktopLayout>,
    dragging: RwSignal<Option<DragState>>,
) {
    if event.button() != 0 {
        return;
    }
    if went_down_on_a_control(&event) {
        return;
    }
    if layout.get_untracked().presentation(card).pinned {
        return;
    }
    let Some(target) = event
        .current_target()
        .and_then(|target| target.dyn_into::<HtmlElement>().ok())
    else {
        return;
    };
    let _ = target.focus();
    let _ = target.set_pointer_capture(event.pointer_id());
    let rect = target.get_bounding_client_rect();
    layout.update(|current| current.bring_forward(card));
    dragging.set(Some(DragState {
        target: DragTarget::Card(card),
        offset_x: f64::from(event.client_x()) - rect.left(),
        offset_y: f64::from(event.client_y()) - rect.top(),
        width: rect.width(),
        height: rect.height(),
        drop_target: None,
    }));
    event.prevent_default();
}

/// Whether this press landed on something that is there to be pressed.
///
/// A drag captures the pointer, and a captured pointer delivers its `pointerup` and its `click` to
/// the capturing element. So a card that starts dragging on every press swallows the click of every
/// button inside it — which is what happened to Restart, to the process table's sort controls, and
/// to every filter chip in every panel. They were not broken; their clicks were being delivered
/// somewhere else.
///
/// Asked of the element the press landed on rather than of the card, and by `closest`, because a
/// press usually lands on an icon or a span inside the control rather than on the control itself.
///
/// A drag still starts anywhere else on the card: the header, the padding, the space between rows.
fn went_down_on_a_control(event: &PointerEvent) -> bool {
    // Deck tabs are not `<button>` and are pressed like one. The terminal screen is a `div` that
    // takes focus and reads keystrokes, which is a control by every meaning except its tag name —
    // and while it went unnamed here, pressing it dragged the card and captured the pointer, so the
    // screen never received the focus a keystroke needs and nothing could be typed into it.
    const CONTROLS: &str = "button, a, input, select, textarea, label, [role='button'], [contenteditable='true'], .deck-tab, .deck-controls, .terminal-screen";

    event
        .target()
        .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
        .and_then(|element| element.closest(CONTROLS).ok().flatten())
        .is_some()
}

/// Start dragging a deck grouping.
pub fn start_deck_drag(
    event: PointerEvent,
    deck_id: String,
    layout: RwSignal<DesktopLayout>,
    dragging: RwSignal<Option<DragState>>,
) {
    if event.button() != 0 {
        return;
    }
    // The same guard the card drag needs, which this one has had all along — and which the card
    // drag went without, so every button inside every panel had its click swallowed by a pointer
    // capture while every button inside a deck worked. One lesson, learned once.
    if went_down_on_a_control(&event) {
        return;
    }
    let current_layout = layout.get_untracked();
    if let Some(deck) = current_layout.deck(&deck_id) {
        if deck.presentation.pinned {
            return;
        }
        let Some(target) = event
            .current_target()
            .and_then(|target| target.dyn_into::<HtmlElement>().ok())
        else {
            return;
        };
        let _ = target.focus();
        let _ = target.set_pointer_capture(event.pointer_id());
        let rect = target.get_bounding_client_rect();
        layout.update(|current| current.bring_deck_forward(&deck_id));
        dragging.set(Some(DragState {
            target: DragTarget::Deck(deck_id),
            offset_x: f64::from(event.client_x()) - rect.left(),
            offset_y: f64::from(event.client_y()) - rect.top(),
            width: rect.width(),
            height: rect.height(),
            drop_target: None,
        }));
        event.prevent_default();
    }
}

/// Update position and magnet detection during a drag operation.
pub fn move_drag(
    event: PointerEvent,
    layout: RwSignal<DesktopLayout>,
    dragging: RwSignal<Option<DragState>>,
    snap_guides: RwSignal<Vec<SnapGuide>>,
    zoom: f64,
    magnet: bool,
) {
    let Some(drag) = dragging.get_untracked() else {
        return;
    };
    let Some(surface) = event
        .current_target()
        .and_then(|target| target.dyn_into::<HtmlElement>().ok())
    else {
        return;
    };
    let bounds = surface.get_bounding_client_rect();
    // The surface carries the canvas transform, so the distance from its edge is in screen pixels
    // and the card lives in canvas ones. Without dividing, a card dragged at 40% zoom moved two and
    // a half times as far as the pointer — and the offset the grab started with is in the same
    // screen pixels, so it is converted with it.
    let scale = if zoom.is_finite() && zoom > 0.0 {
        zoom
    } else {
        1.0
    };
    let raw_x = (f64::from(event.client_x()) - bounds.left() - drag.offset_x) / scale;
    let raw_y = (f64::from(event.client_y()) - bounds.top() - drag.offset_y) / scale;

    let target_id = match &drag.target {
        DragTarget::Card(card) => DesktopItemId::Card(*card),
        DragTarget::Deck(deck_id) => DesktopItemId::Deck(deck_id.clone()),
    };

    // With the magnet off there is nothing to compute: the card goes where the pointer is, and no
    // guides are drawn because nothing is aligning to anything.
    let snap = if magnet {
        layout
            .get_untracked()
            .compute_snap(&target_id, raw_x, raw_y, drag.width, drag.height, 8.0)
    } else {
        crate::layout::snap::SnapResult {
            snapped_x: raw_x,
            snapped_y: raw_y,
            guides: Vec::new(),
        }
    };

    // No floor. The canvas is unbounded in every direction and the origin is not a corner of
    // anything a person can see; clamping here put a wall twelve pixels from it that nothing on
    // screen explained.
    let x = snap.snapped_x;
    let y = snap.snapped_y;
    snap_guides.set(snap.guides);

    match &drag.target {
        DragTarget::Card(card) => {
            let dragged_card = *card;
            layout.update(|current| {
                current.set_position(dragged_card, x, y);
            });

            let current_layout = layout.get_untracked();
            let drag_center_x = x + drag.width / 2.0;
            let drag_center_y = y + drag.height / 2.0;

            // A drop target is what turns a drag into a deck merge. Off, a card dragged across a
            // small window is a card dragged across a small window.
            let mut found_target = None;
            for card_inst in current_layout.cards.iter().filter(|_| magnet) {
                if card_inst.id == dragged_card {
                    continue;
                }
                let geom = card_inst.geometry;
                let is_collapsed = card_inst.presentation.collapsed;
                let target_h = if is_collapsed { 44.0 } else { geom.height };
                if drag_center_x >= geom.x - 24.0
                    && drag_center_x <= geom.x + geom.width + 24.0
                    && drag_center_y >= geom.y - 24.0
                    && drag_center_y <= geom.y + target_h + 24.0
                {
                    found_target = Some(card_inst.id);
                    break;
                }
            }

            dragging.update(|d| {
                if let Some(d) = d {
                    d.drop_target = found_target;
                }
            });
        }
        DragTarget::Deck(deck_id) => {
            layout.update(|current| {
                current.set_deck_position(deck_id, x, y);
            });
        }
    }
}

/// Finalize drag operation, persisting layout and performing deck merges if magnetic target hovered.
pub fn finish_drag(
    layout: RwSignal<DesktopLayout>,
    history: RwSignal<LayoutHistory>,
    dragging: RwSignal<Option<DragState>>,
    snap_guides: RwSignal<Vec<SnapGuide>>,
) {
    snap_guides.set(Vec::new());
    let Some(drag) = dragging.get_untracked() else {
        return;
    };
    dragging.set(None);

    if let DragTarget::Card(dragged_card) = drag.target
        && let Some(target_card) = drag.drop_target
    {
        history.update(|h| h.push(layout.get_untracked()));
        layout.update(|current| {
            let deck_id_opt = current.deck_for_card(target_card).map(|d| d.id.clone());
            if let Some(d_id) = deck_id_opt {
                let _ = current.add_to_deck(&d_id, dragged_card);
            } else {
                let target_geom = current.geometry(target_card);
                let title = format!("{} + {}", target_card.title(), dragged_card.title());
                // The deck takes the place of the card it was dropped onto, size included. It used
                // to start from a constant and only grow, so a merge could double the footprint of
                // what a person had just arranged.
                let _ = current.create_deck_over(
                    title,
                    vec![target_card, dragged_card],
                    target_geom.x,
                    target_geom.y,
                    Some((target_geom.width, target_geom.height)),
                );
            }
        });
    }

    layout.get_untracked().save();
}

/// Start resizing a card.
pub fn start_resize(
    event: PointerEvent,
    card: CardId,
    layout: RwSignal<DesktopLayout>,
    resizing: RwSignal<Option<ResizeState>>,
) {
    if event.button() != 0 {
        return;
    }
    if layout.get_untracked().presentation(card).pinned {
        return;
    }
    let Some(target) = event
        .current_target()
        .and_then(|target| target.dyn_into::<HtmlElement>().ok())
    else {
        return;
    };
    event.stop_propagation();
    event.prevent_default();
    let _ = target.set_pointer_capture(event.pointer_id());
    layout.update(|current| current.bring_forward(card));
    let geom = layout.get_untracked().geometry(card);
    resizing.set(Some(ResizeState {
        target: ResizeTarget::Card(card),
        start_pointer_x: f64::from(event.client_x()),
        start_pointer_y: f64::from(event.client_y()),
        start_width: geom.width,
        start_height: geom.height,
    }));
}

/// Start resizing a deck.
pub fn start_deck_resize(
    event: PointerEvent,
    deck_id: String,
    layout: RwSignal<DesktopLayout>,
    resizing: RwSignal<Option<ResizeState>>,
) {
    if event.button() != 0 {
        return;
    }
    if let Some(deck) = layout.get_untracked().deck(&deck_id) {
        if deck.presentation.pinned {
            return;
        }
        let Some(target) = event
            .current_target()
            .and_then(|target| target.dyn_into::<HtmlElement>().ok())
        else {
            return;
        };
        event.stop_propagation();
        event.prevent_default();
        let _ = target.set_pointer_capture(event.pointer_id());
        layout.update(|current| current.bring_deck_forward(&deck_id));
        let geom = deck.geometry;
        resizing.set(Some(ResizeState {
            target: ResizeTarget::Deck(deck_id),
            start_pointer_x: f64::from(event.client_x()),
            start_pointer_y: f64::from(event.client_y()),
            start_width: geom.width,
            start_height: geom.height,
        }));
    }
}

/// Update dimensions during a resize operation.
pub fn move_resize(
    event: PointerEvent,
    layout: RwSignal<DesktopLayout>,
    resizing: RwSignal<Option<ResizeState>>,
) {
    let Some(resize) = resizing.get_untracked() else {
        return;
    };
    let dx = f64::from(event.client_x()) - resize.start_pointer_x;
    let dy = f64::from(event.client_y()) - resize.start_pointer_y;

    match &resize.target {
        ResizeTarget::Card(card) => {
            let spec = card.spec();
            let new_width = (resize.start_width + dx).clamp(spec.min_size.0, spec.max_size.0);
            let new_height = (resize.start_height + dy).clamp(spec.min_size.1, spec.max_size.1);

            layout.update(|current| {
                current.set_size(*card, new_width, new_height);
            });
        }
        ResizeTarget::Deck(deck_id) => {
            let new_width = (resize.start_width + dx).clamp(280.0, 800.0);
            let new_height = (resize.start_height + dy).clamp(160.0, 700.0);

            layout.update(|current| {
                current.set_deck_size(deck_id, new_width, new_height);
            });
        }
    }
}

/// Finalize resize operation and save layout.
pub fn finish_resize(layout: RwSignal<DesktopLayout>, resizing: RwSignal<Option<ResizeState>>) {
    if resizing.get_untracked().is_some() {
        resizing.set(None);
        layout.get_untracked().save();
    }
}

/// Keyboard movement and resizing for accessibility (Alt + Arrows, Alt + Shift + Arrows).
pub fn keyboard_move(event: KeyboardEvent, card: CardId, layout: RwSignal<DesktopLayout>) {
    if !event.alt_key() && !event.meta_key() {
        return;
    }
    if layout.get_untracked().presentation(card).pinned {
        return;
    }
    let key = event.key();
    let is_arrow = matches!(
        key.as_str(),
        "ArrowLeft" | "ArrowRight" | "ArrowUp" | "ArrowDown"
    );
    if !is_arrow {
        return;
    }
    event.prevent_default();

    if event.shift_key() {
        let delta = 20.0;
        let (dw, dh) = match key.as_str() {
            "ArrowLeft" => (-delta, 0.0),
            "ArrowRight" => (delta, 0.0),
            "ArrowUp" => (0.0, -delta),
            "ArrowDown" => (0.0, delta),
            _ => (0.0, 0.0),
        };
        layout.update(|current| {
            current.bring_forward(card);
            let geom = current.geometry(card);
            current.set_size(card, geom.width + dw, geom.height + dh);
        });
    } else {
        let step = 20.0;
        let (dx, dy) = match key.as_str() {
            "ArrowLeft" => (-step, 0.0),
            "ArrowRight" => (step, 0.0),
            "ArrowUp" => (0.0, -step),
            "ArrowDown" => (0.0, step),
            _ => (0.0, 0.0),
        };
        layout.update(|current| {
            current.bring_forward(card);
            let geom = current.geometry(card);
            current.set_position(card, (geom.x + dx).max(12.0), (geom.y + dy).max(12.0));
        });
    }
    layout.get_untracked().save();
}

/// Keyboard movement and resizing for a deck container.
pub fn keyboard_deck_move(event: KeyboardEvent, deck_id: &str, layout: RwSignal<DesktopLayout>) {
    if !event.alt_key() && !event.meta_key() {
        return;
    }
    let is_pinned = layout
        .get_untracked()
        .deck(deck_id)
        .is_some_and(|d| d.presentation.pinned);
    if is_pinned {
        return;
    }
    let key = event.key();
    let is_arrow = matches!(
        key.as_str(),
        "ArrowLeft" | "ArrowRight" | "ArrowUp" | "ArrowDown"
    );
    if !is_arrow {
        return;
    }
    event.prevent_default();

    if event.shift_key() {
        let delta = 20.0;
        let (dw, dh) = match key.as_str() {
            "ArrowLeft" => (-delta, 0.0),
            "ArrowRight" => (delta, 0.0),
            "ArrowUp" => (0.0, -delta),
            "ArrowDown" => (0.0, delta),
            _ => (0.0, 0.0),
        };
        layout.update(|current| {
            if let Some(deck) = current.deck_mut(deck_id) {
                let w = (deck.geometry.width + dw).max(280.0);
                let h = (deck.geometry.height + dh).max(180.0);
                deck.geometry.width = w;
                deck.geometry.height = h;
            }
            current.bring_deck_forward(deck_id);
        });
    } else {
        let step = 20.0;
        let (dx, dy) = match key.as_str() {
            "ArrowLeft" => (-step, 0.0),
            "ArrowRight" => (step, 0.0),
            "ArrowUp" => (0.0, -step),
            "ArrowDown" => (0.0, step),
            _ => (0.0, 0.0),
        };
        layout.update(|current| {
            if let Some(deck) = current.deck_mut(deck_id) {
                deck.geometry.x = (deck.geometry.x + dx).max(12.0);
                deck.geometry.y = (deck.geometry.y + dy).max(12.0);
            }
            current.bring_deck_forward(deck_id);
        });
    }
    layout.get_untracked().save();
}

/// Apply undo step on layout history.
pub fn apply_undo(history: RwSignal<LayoutHistory>, layout: RwSignal<DesktopLayout>) {
    let mut target = None;
    history.update(|h| {
        target = h.undo(layout.get_untracked());
    });
    if let Some(prev) = target {
        layout.set(prev);
        layout.get_untracked().save();
    }
}

/// Apply redo step on layout history.
pub fn apply_redo(history: RwSignal<LayoutHistory>, layout: RwSignal<DesktopLayout>) {
    let mut target = None;
    history.update(|h| {
        target = h.redo(layout.get_untracked());
    });
    if let Some(next) = target {
        layout.set(next);
        layout.get_untracked().save();
    }
}
