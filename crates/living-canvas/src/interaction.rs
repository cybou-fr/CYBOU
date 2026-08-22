// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Interactive pointer and keyboard event handling, dragging, resizing, and snapping.

use leptos::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlElement, KeyboardEvent, PointerEvent};

use crate::{
    CardGeometry, CardId, DesktopItemId, DesktopLayout, DesktopViewMode, LayoutHistory, SnapGuide,
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
    } else {
        format!(
            "left:{:.1}px;top:{:.1}px;width:{:.1}px;height:{:.1}px;z-index:{}",
            geom.x, geom.y, geom.width, geom.height, geom.z
        )
    }
}

/// Inline style for the release quick-action bar attached to Capabilities card.
#[must_use]
pub fn selection_actions_style(layout: DesktopLayout) -> String {
    let geom = layout.geometry(CardId::Capabilities);
    format!(
        "left:{:.1}px;top:{:.1}px;z-index:{}",
        geom.x + 18.0,
        geom.y + geom.height,
        geom.z + 1
    )
}

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
    if let Some(target_el) = event
        .target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
        && target_el
            .closest("button, .deck-tab, .deck-tab-detach, .deck-controls, .card-control-btn")
            .ok()
            .flatten()
            .is_some()
    {
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
    let raw_x = (f64::from(event.client_x()) - bounds.left() - drag.offset_x).max(12.0);
    let raw_y = (f64::from(event.client_y()) - bounds.top() - drag.offset_y).max(12.0);

    let target_id = match &drag.target {
        DragTarget::Card(card) => DesktopItemId::Card(*card),
        DragTarget::Deck(deck_id) => DesktopItemId::Deck(deck_id.clone()),
    };

    let snap =
        layout
            .get_untracked()
            .compute_snap(&target_id, raw_x, raw_y, drag.width, drag.height, 8.0);

    let x = snap.snapped_x.max(12.0);
    let y = snap.snapped_y.max(12.0);
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

            let mut found_target = None;
            for card_inst in &current_layout.cards {
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
                let _ = current.create_deck(
                    title,
                    vec![target_card, dragged_card],
                    target_geom.x,
                    target_geom.y,
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
