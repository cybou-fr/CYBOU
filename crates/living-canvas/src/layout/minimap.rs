// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Fitting the whole desktop into a small rectangle, and saying where you are in it.
//!
//! The minimap used to divide every coordinate by a constant `1280 x 650`. That is not a projection
//! of the desktop, it is a projection of one particular desktop: a card dragged past those numbers
//! left the map entirely, and nothing on the map ever moved when the canvas was zoomed or panned,
//! because the map had no idea either had happened.
//!
//! The transform here is derived from the desktop's own bounding rectangle, so whatever the layout
//! is, it fits. Scale is uniform on both axes — a map that stretched one of them would draw a wide
//! desktop as a square one, and the whole point of an overview is that shape survives it.

use crate::layout::model::Rect;

/// Width of the minimap surface, in CSS pixels.
///
/// Declared here rather than only in the stylesheet because the projection has to agree with the
/// element it draws into. Two numbers that must match are one number.
pub const MINIMAP_WIDTH: f64 = 196.0;

/// Height of the minimap surface, in CSS pixels.
pub const MINIMAP_HEIGHT: f64 = 112.0;

/// Breathing room kept between the desktop's edge and the surface's.
pub const MINIMAP_PADDING: f64 = 6.0;

/// How desktop coordinates become minimap coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MinimapProjection {
    /// Uniform scale from desktop pixels to minimap pixels.
    scale: f64,
    /// Where the desktop's origin lands on the surface.
    offset_x: f64,
    /// Where the desktop's origin lands on the surface.
    offset_y: f64,
}

impl MinimapProjection {
    /// Fit a desktop of these bounds into a surface of this size.
    ///
    /// The scale is the smaller of the two axis ratios, so the whole desktop is inside the surface
    /// rather than merely most of it, and the result is centred in whichever axis has room left.
    #[must_use]
    pub fn fit(bounds: Rect, width: f64, height: f64, padding: f64) -> Self {
        let usable_width = (width - padding * 2.0).max(1.0);
        let usable_height = (height - padding * 2.0).max(1.0);
        // A desktop cannot have zero extent — `bounding_rect` floors it at ten — but this function
        // is total whatever it is handed, because a scale of infinity would put every card in the
        // same place and give no sign that anything was wrong.
        let scale = if bounds.width > 0.0 && bounds.height > 0.0 {
            (usable_width / bounds.width).min(usable_height / bounds.height)
        } else {
            1.0
        };
        let scale = if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        };

        let drawn_width = bounds.width * scale;
        let drawn_height = bounds.height * scale;
        Self {
            scale,
            offset_x: padding + (usable_width - drawn_width) / 2.0 - bounds.x * scale,
            offset_y: padding + (usable_height - drawn_height) / 2.0 - bounds.y * scale,
        }
    }

    /// Where a desktop rectangle lands on the surface.
    #[must_use]
    pub fn project(&self, rect: Rect) -> Rect {
        Rect::new(
            self.offset_x + rect.x * self.scale,
            self.offset_y + rect.y * self.scale,
            // Floored so a small card stays visible rather than becoming nothing. A dot a person
            // cannot see is the same as a card the map did not draw.
            (rect.width * self.scale).max(2.0),
            (rect.height * self.scale).max(2.0),
        )
    }

    /// The scale this projection settled on.
    #[must_use]
    pub const fn scale(&self) -> f64 {
        self.scale
    }
}

/// Which part of the desktop is on screen right now.
///
/// The canvas is drawn as `translate(pan) scale(zoom)`, so a screen point maps back to
/// `(screen - pan) / zoom`. Without this the minimap showed where the cards were and never where
/// the person was, which is the one thing an overview is for.
#[must_use]
pub fn visible_desktop_rect(pan: (f64, f64), zoom: f64, viewport: (f64, f64)) -> Rect {
    let zoom = if zoom.is_finite() && zoom > 0.0 {
        zoom
    } else {
        1.0
    };
    Rect::new(
        -pan.0 / zoom,
        -pan.1 / zoom,
        viewport.0 / zoom,
        viewport.1 / zoom,
    )
}

/// The pan that would put this desktop rectangle in the middle of the screen.
#[must_use]
pub fn pan_centring(rect: Rect, zoom: f64, viewport: (f64, f64)) -> (f64, f64) {
    let zoom = if zoom.is_finite() && zoom > 0.0 {
        zoom
    } else {
        1.0
    };
    (
        viewport.0 / 2.0 - (rect.x + rect.width / 2.0) * zoom,
        viewport.1 / 2.0 - (rect.y + rect.height / 2.0) * zoom,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const SURFACE: (f64, f64, f64) = (MINIMAP_WIDTH, MINIMAP_HEIGHT, MINIMAP_PADDING);

    fn fit(bounds: Rect) -> MinimapProjection {
        MinimapProjection::fit(bounds, SURFACE.0, SURFACE.1, SURFACE.2)
    }

    /// Whether two lengths are the same, allowing for floating point.
    fn close(left: f64, right: f64) -> bool {
        (left - right).abs() < 0.001
    }

    /// Whether a projected rectangle is inside the surface, allowing for floating point.
    fn inside(rect: Rect) -> bool {
        rect.x >= -0.001
            && rect.y >= -0.001
            && rect.x + rect.width <= MINIMAP_WIDTH + 0.001
            && rect.y + rect.height <= MINIMAP_HEIGHT + 0.001
    }

    #[test]
    fn a_desktop_of_any_size_lands_inside_the_surface() {
        // The old transform divided by a constant 1280x650, so a card dragged past those numbers
        // was drawn outside the map and simply disappeared.
        for bounds in [
            Rect::new(0.0, 0.0, 1280.0, 650.0),
            Rect::new(0.0, 0.0, 12800.0, 6500.0),
            Rect::new(0.0, 0.0, 40.0, 40.0),
            Rect::new(-900.0, -400.0, 3000.0, 300.0),
        ] {
            let projection = fit(bounds);
            assert!(
                inside(projection.project(bounds)),
                "bounds {bounds:?} were drawn outside the surface"
            );
        }
    }

    #[test]
    fn shape_survives_the_projection() {
        // One scale for both axes. A map that stretched an axis would draw a wide desktop as a
        // square one, and an overview that changes the shape of what it shows is not one.
        let bounds = Rect::new(0.0, 0.0, 2000.0, 400.0);
        let projection = fit(bounds);
        let drawn = projection.project(bounds);
        let source_ratio = bounds.width / bounds.height;
        let drawn_ratio = drawn.width / drawn.height;
        assert!((source_ratio - drawn_ratio).abs() < 0.001);
    }

    #[test]
    fn a_card_at_the_origin_is_not_drawn_at_the_origin_of_the_surface() {
        // The desktop is centred in whichever axis has room left over, so a wide layout is not
        // pinned to the top edge with empty space beneath it.
        let bounds = Rect::new(0.0, 0.0, 2000.0, 200.0);
        let projection = fit(bounds);
        let drawn = projection.project(Rect::new(0.0, 0.0, 10.0, 10.0));
        assert!(drawn.y > MINIMAP_PADDING, "the layout was not centred");
    }

    #[test]
    fn a_desktop_that_starts_away_from_zero_still_fills_the_surface() {
        // Cards can sit at negative coordinates. Projecting from zero rather than from the bounds
        // would leave the map mostly empty and push the desktop off one edge.
        let bounds = Rect::new(-1500.0, -900.0, 800.0, 400.0);
        let projection = fit(bounds);
        let drawn = projection.project(bounds);
        assert!(inside(drawn));
        assert!(
            drawn.width > MINIMAP_WIDTH / 2.0,
            "the map was mostly empty"
        );
    }

    #[test]
    fn nothing_is_drawn_too_small_to_see() {
        let bounds = Rect::new(0.0, 0.0, 20000.0, 20000.0);
        let projection = fit(bounds);
        let speck = projection.project(Rect::new(0.0, 0.0, 220.0, 188.0));
        assert!(speck.width >= 2.0 && speck.height >= 2.0);
    }

    #[test]
    fn a_degenerate_desktop_does_not_produce_a_degenerate_scale() {
        let projection = fit(Rect::new(0.0, 0.0, 0.0, 0.0));
        assert!(projection.scale().is_finite() && projection.scale() > 0.0);
    }

    #[test]
    fn the_visible_rectangle_follows_pan_and_zoom() {
        // At zoom 1 with no pan, the screen shows the desktop starting at the origin.
        let at_rest = visible_desktop_rect((0.0, 0.0), 1.0, (1440.0, 900.0));
        assert!(close(at_rest.x, 0.0) && close(at_rest.y, 0.0));
        assert!(close(at_rest.width, 1440.0) && close(at_rest.height, 900.0));

        // Panning the canvas right shows the desktop further left.
        let panned = visible_desktop_rect((-200.0, -100.0), 1.0, (1440.0, 900.0));
        assert!(close(panned.x, 200.0));
        assert!(close(panned.y, 100.0));

        // Zooming in shows less of it.
        let zoomed = visible_desktop_rect((0.0, 0.0), 2.0, (1440.0, 900.0));
        assert!(close(zoomed.width, 720.0));
        assert!(close(zoomed.height, 450.0));
    }

    #[test]
    fn centring_puts_the_middle_of_a_card_in_the_middle_of_the_screen() {
        let card = Rect::new(400.0, 300.0, 200.0, 100.0);
        let viewport = (1440.0, 900.0);
        let pan = pan_centring(card, 1.5, viewport);
        let visible = visible_desktop_rect(pan, 1.5, viewport);
        let card_centre = (card.x + card.width / 2.0, card.y + card.height / 2.0);
        let visible_centre = (
            visible.x + visible.width / 2.0,
            visible.y + visible.height / 2.0,
        );
        assert!((card_centre.0 - visible_centre.0).abs() < 0.001);
        assert!((card_centre.1 - visible_centre.1).abs() < 0.001);
    }

    #[test]
    fn a_zoom_that_makes_no_sense_does_not_produce_coordinates_that_make_none() {
        for zoom in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            let rect = visible_desktop_rect((10.0, 10.0), zoom, (100.0, 100.0));
            assert!(rect.width.is_finite() && rect.height.is_finite());
        }
    }
}
