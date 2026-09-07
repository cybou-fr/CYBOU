// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Cluster computation and manipulation engine for Living Canvas.

use uuid::Uuid;

use crate::{
    CardId, DesktopLayout,
    layout::model::{ClusterOrigin, DesktopCluster, Rect},
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

    /// Gather cards the desktop offered into one cluster for the episode they belong to.
    ///
    /// Marked [`ClusterOrigin::Suggested`] so it can be taken away again when the episode is over.
    /// The identity is derived from the episode rather than generated, so a second offer about the
    /// same one replaces this cluster instead of stacking another beside it — and so that two runs
    /// of the same accept produce the same layout.
    pub fn gather_suggested(&mut self, correlation: Option<Uuid>, label: &str, cards: &[CardId]) {
        let card_keys: Vec<String> = cards.iter().map(|card| card.instance_key()).collect();
        if card_keys.is_empty() {
            return;
        }
        self.add_cluster(DesktopCluster {
            id: suggested_cluster_id(correlation, &card_keys),
            label: label.to_owned(),
            color: "amber".to_owned(),
            card_keys,
            origin: ClusterOrigin::Suggested { correlation },
        });
    }

    /// Take away a cluster the desktop offered, leaving its cards exactly where they are.
    ///
    /// Answers `false` for a cluster a person built, which is the point of the whole origin field:
    /// dismissing an offer must never be a way to delete somebody's own grouping, however alike the
    /// two look on screen.
    pub fn dismiss_suggested_cluster(&mut self, id: &str) -> bool {
        let removable = self
            .clusters
            .iter()
            .any(|cluster| cluster.id == id && cluster.origin.is_suggested());
        if removable {
            self.remove_cluster(id);
        }
        removable
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

/// The identity of the cluster offered for an episode.
///
/// An episode names it where there is one. Where there is not, the cards do: an offer with no
/// correlation still has to be the same cluster when it is made twice, and a generated identity
/// would leave a second copy of it behind every time.
#[must_use]
pub fn suggested_cluster_id(correlation: Option<Uuid>, card_keys: &[String]) -> String {
    correlation.map_or_else(
        || format!("suggested:cards:{}", card_keys.join("+")),
        |correlation| format!("suggested:{correlation}"),
    )
}
