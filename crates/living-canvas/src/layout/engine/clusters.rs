// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Cluster computation and manipulation engine for Living Canvas.

use crate::{
    DesktopLayout,
    layout::model::{DesktopCluster, Rect},
};

impl DesktopLayout {
    /// List all semantic clusters.
    #[must_use]
    pub fn clusters(&self) -> &[DesktopCluster] {
        &self.clusters
    }

    /// Add or update a semantic cluster.
    pub fn add_cluster(&mut self, cluster: DesktopCluster) {
        if let Some(existing) = self.clusters.iter_mut().find(|c| c.id == cluster.id) {
            *existing = cluster;
        } else {
            self.clusters.push(cluster);
        }
    }

    /// Remove a cluster by ID.
    pub fn remove_cluster(&mut self, id: &str) {
        self.clusters.retain(|c| c.id != id);
    }

    /// Compute the 2D bounding hull rectangle of a cluster including padding.
    #[must_use]
    pub fn cluster_rect(&self, cluster: &DesktopCluster) -> Option<Rect> {
        let matching_cards: Vec<_> = self
            .cards
            .iter()
            .filter(|card| {
                cluster
                    .card_keys
                    .iter()
                    .any(|key| card.id.matches_persisted_key(key))
            })
            .collect();

        if matching_cards.is_empty() {
            return None;
        }

        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for card in matching_cards {
            let left = card.geometry.x;
            let top = card.geometry.y;
            let right = left + card.geometry.width;
            let h = if card.presentation.collapsed {
                44.0
            } else {
                card.geometry.height
            };
            let bottom = top + h;

            if left < min_x {
                min_x = left;
            }
            if top < min_y {
                min_y = top;
            }
            if right > max_x {
                max_x = right;
            }
            if bottom > max_y {
                max_y = bottom;
            }
        }

        // Bounding padding: 24px sides/bottom, 48px top for header bar
        let pad_x = 24.0;
        let pad_top = 48.0;
        let pad_bottom = 24.0;

        Some(Rect::new(
            min_x - pad_x,
            min_y - pad_top,
            (max_x - min_x) + (pad_x * 2.0),
            (max_y - min_y) + pad_top + pad_bottom,
        ))
    }
}
