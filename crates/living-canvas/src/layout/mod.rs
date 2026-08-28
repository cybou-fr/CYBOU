// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Desktop layout persistence, migration, and spatial arrangement engine.

pub mod camera;
pub mod engine;
pub mod history;
pub mod migration;
pub mod minimap;
pub mod model;
pub mod placement;
pub mod relations;
pub mod selection;
pub mod snap;

pub use camera::{CameraHistory, CameraState};
#[cfg(target_arch = "wasm32")]
pub use camera::{apply_camera_back, apply_camera_fly_to, apply_camera_forward, camera_center};
pub use engine::DesktopLayout;
pub use history::LayoutHistory;
pub use migration::{CanvasLayoutV8, LAYOUT_KEY_V8, LAYOUT_KEY_V9, PointV8, from_v8};
pub use minimap::{
    MINIMAP_HEIGHT, MINIMAP_PADDING, MINIMAP_WIDTH, MinimapProjection, pan_centring,
    visible_desktop_rect,
};
pub use model::{
    ArrangementMode, CanvasAnchor, DesktopCluster, DesktopItem, DesktopItemId, DesktopViewMode,
    Rect, UsableViewport,
};
pub use placement::PlacementResolver;
pub use relations::{DesktopRelationshipGraph, RelationVisibility, Relationship, RelationshipKind};
pub use selection::{selected_rect, selected_z};
pub use snap::{SnapGuide, SnapResult, compute_snap};

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::card::{CardGeometry, CardId, CardInstance, CardPresentation};
    use crate::deck::{DeckError, DeckInstance};

    #[test]
    fn v8_to_v9_migration_preserves_coordinates() {
        let v8 = CanvasLayoutV8::default();
        let v9 = DesktopLayout::from_v8(&v8);

        assert_eq!(v9.schema_version, 9);
        // Eleven, because that is how many panels v8 had. Migration carries what the old layout
        // held and invents nothing; a card added after v8 is the business of normalization, which
        // this test deliberately does not run.
        assert_eq!(v9.cards.len(), 11);

        let id_geom = v9.geometry(CardId::Identity);
        assert!((id_geom.x - 70.0).abs() < 1e-6);
        assert!((id_geom.y - 50.0).abs() < 1e-6);

        let cap_geom = v9.geometry(CardId::Capabilities);
        assert!((cap_geom.x - 445.0).abs() < 1e-6);
        assert!((cap_geom.y - 70.0).abs() < 1e-6);
    }

    #[test]
    fn default_layout_has_all_system_cards() {
        let layout = DesktopLayout::default();
        assert_eq!(layout.schema_version, 9);
        assert_eq!(layout.cards.len(), CardId::ALL_SYSTEM_CARDS.len());

        for sys_id in CardId::ALL_SYSTEM_CARDS {
            assert!(layout.contains_card(sys_id));
        }
    }

    #[test]
    fn layout_invariants_l1_to_l4_deck_management() {
        let mut layout = DesktopLayout::default();

        // L4: Non-deckable card cannot enter deck
        let non_deckable = CardId::JournalFeed(0);
        let err = layout.create_deck(
            "Bad Deck",
            vec![CardId::Identity, non_deckable],
            100.0,
            100.0,
        );
        assert!(matches!(err, Err(DeckError::NonDeckableCard(_))));

        // L2: Deck requires >= 2 cards
        let err2 = layout.create_deck("Single", vec![CardId::Identity], 100.0, 100.0);
        assert!(matches!(err2, Err(DeckError::InsufficientCards)));

        // Create valid deck
        let d_id = layout
            .create_deck(
                "Core",
                vec![CardId::Identity, CardId::Session],
                100.0,
                100.0,
            )
            .expect("valid deck creation");

        // L1: Card belongs to at most one deck
        let err3 = layout.add_to_deck(&d_id, CardId::Identity);
        assert!(matches!(err3, Err(DeckError::CardAlreadyInDeck(_))));

        // L8: Desktop items exclude cards docked in decks
        let items = layout.desktop_items();
        // Two cards went into the deck, so they are one item between them.
        assert_eq!(items.len(), CardId::ALL_SYSTEM_CARDS.len() - 1);
        assert!(
            !items
                .iter()
                .any(|it| it.id == DesktopItemId::Card(CardId::Identity))
        );
        assert!(
            items
                .iter()
                .any(|it| it.id == DesktopItemId::Deck(d_id.clone()))
        );

        // Detach card and dissolve
        layout.detach_from_deck(&d_id, CardId::Identity, None);
        assert_eq!(layout.decks.len(), 0); // dissolved
        assert_eq!(layout.desktop_items().len(), CardId::ALL_SYSTEM_CARDS.len());
    }

    #[test]
    fn layout_invariants_l5_l6_l7_grid_and_compact_obstacle_avoidance_no_overlap() {
        let mut layout = DesktopLayout::default();

        // Pin Identity as an obstacle at (40, 40)
        layout.set_position(CardId::Identity, 40.0, 40.0);
        layout.set_pinned(CardId::Identity, true);

        // Add a wide Tool card (Shell)
        layout.open_card(CardId::Shell(1), 500.0, 500.0);

        let vp = UsableViewport {
            width: 1200.0,
            height: 800.0,
        };

        // Test Grid
        layout.apply_arrangement(ArrangementMode::Grid, Some(vp));

        // L5: Pinned item did not move
        let id_geom = layout.geometry(CardId::Identity);
        assert_eq!(id_geom.x, 40.0);
        assert_eq!(id_geom.y, 40.0);

        // L6 & L7: No overlapping items
        let items = layout.desktop_items();
        for i in 0..items.len() {
            for j in (i + 1)..items.len() {
                let r1 = items[i].effective_rect();
                let r2 = items[j].effective_rect();
                assert!(
                    !r1.intersects(&r2),
                    "Items {:?} and {:?} overlapped! r1={:?}, r2={:?}",
                    items[i].id,
                    items[j].id,
                    r1,
                    r2
                );
            }
        }

        // Test Compact
        layout.apply_arrangement(ArrangementMode::Compact, Some(vp));
        assert_eq!(layout.geometry(CardId::Identity).x, 40.0);
        let items_compact = layout.desktop_items();
        for i in 0..items_compact.len() {
            for j in (i + 1)..items_compact.len() {
                let r1 = items_compact[i].effective_rect();
                let r2 = items_compact[j].effective_rect();
                assert!(
                    !r1.intersects(&r2),
                    "Compact items {:?} and {:?} overlapped!",
                    items_compact[i].id,
                    items_compact[j].id
                );
            }
        }
    }

    #[test]
    fn layout_invariant_l11_arrangement_determinism() {
        let mut layout1 = DesktopLayout::default();
        let mut layout2 = DesktopLayout::default();
        let vp = UsableViewport::default();

        layout1.apply_arrangement(ArrangementMode::Relations, Some(vp));
        layout2.apply_arrangement(ArrangementMode::Relations, Some(vp));

        assert_eq!(layout1.cards, layout2.cards);
    }

    #[test]
    fn layout_invariant_l14_unified_monotonic_z_index() {
        let mut layout = DesktopLayout::default();
        let d_id = layout
            .create_deck(
                "Deck1",
                vec![CardId::Identity, CardId::Session],
                100.0,
                100.0,
            )
            .unwrap();

        // Bring deck forward
        layout.bring_item_forward(&DesktopItemId::Deck(d_id.clone()));
        let deck_z = layout.deck(&d_id).unwrap().geometry.z;

        // Bring Journal forward
        layout.bring_item_forward(&DesktopItemId::Card(CardId::Journal));
        let journal_z = layout.geometry(CardId::Journal).z;

        assert!(journal_z > deck_z);

        // Normalize
        layout.normalize_z_indices();
        let max_z = layout
            .cards
            .iter()
            .map(|c| c.geometry.z)
            .chain(layout.decks.iter().map(|d| d.geometry.z))
            .max()
            .unwrap();
        assert_eq!(max_z as usize, layout.cards.len() + layout.decks.len());
    }

    #[test]
    fn validate_and_normalize_recovers_missing_cards_and_corrupt_decks() {
        let mut corrupt = DesktopLayout::new();
        // Missing all cards, only has one corrupt deck
        corrupt.decks.push(DeckInstance {
            id: "bad".into(),
            title: "Bad".into(),
            card_ids: vec![CardId::Identity], // < 2 cards -> should dissolve
            active_card: CardId::Identity,
            geometry: CardGeometry::new(-500.0, -200.0, (100.0, 50.0), 9999),
            presentation: CardPresentation::default(),
        });

        corrupt.validate_and_normalize();

        assert_eq!(corrupt.cards.len(), CardId::ALL_SYSTEM_CARDS.len());
        assert_eq!(corrupt.decks.len(), 0); // dissolved
        for c in corrupt.cards {
            assert!(c.geometry.x >= 0.0);
            assert!(c.geometry.y >= 0.0);
            assert!(c.geometry.width >= c.id.spec().min_size.0);
        }
    }

    #[test]
    fn validate_and_normalize_migrates_legacy_cluster_keys_to_instances() {
        let mut layout = DesktopLayout::default();
        // Add dynamic cards
        layout.cards.push(CardInstance {
            id: CardId::Editor(0),
            geometry: CardGeometry::new(100.0, 100.0, (400.0, 300.0), 1),
            presentation: CardPresentation::default(),
        });
        layout.cards.push(CardInstance {
            id: CardId::Editor(1),
            geometry: CardGeometry::new(550.0, 100.0, (400.0, 300.0), 2),
            presentation: CardPresentation::default(),
        });

        // Add a cluster using a legacy type key "editor"
        layout.clusters.push(DesktopCluster {
            id: "work".into(),
            label: "Workspaces".into(),
            color: "cyan".into(),
            card_keys: vec!["editor".into(), "identity".into()],
        });

        layout.validate_and_normalize();

        let cluster = &layout.clusters[0];
        // "editor" should expand/migrate to ["editor:0", "editor:1"] and "identity" stays "identity"
        assert!(cluster.card_keys.contains(&"editor:0".to_string()));
        assert!(cluster.card_keys.contains(&"editor:1".to_string()));
        assert!(!cluster.card_keys.contains(&"editor".to_string()));
        assert!(cluster.card_keys.contains(&"identity".to_string()));
    }

    #[test]
    fn layout_history_undo_redo() {
        let mut history = LayoutHistory::new();
        let initial = DesktopLayout::default();
        history.push(initial.clone());

        let mut modified = initial.clone();
        modified.set_position(CardId::Identity, 999.0, 999.0);

        assert!(history.can_undo());
        assert!(!history.can_redo());

        let undone = history.undo(modified.clone()).expect("undo available");
        assert_eq!(
            undone.geometry(CardId::Identity).x,
            initial.geometry(CardId::Identity).x
        );
        assert!(history.can_redo());

        let redone = history.redo(undone).expect("redo available");
        assert_eq!(redone.geometry(CardId::Identity).x, 999.0);
    }

    #[test]
    fn placement_resolver_finds_safe_candidate() {
        let layout = DesktopLayout::default();
        let items = layout.desktop_items();
        let vp = UsableViewport {
            width: 1440.0,
            height: 900.0,
        };

        let pref = Rect::new(100.0, 100.0, 300.0, 200.0);
        let (x, y) = PlacementResolver::find_placement(&items, 400.0, 300.0, Some(pref), vp);

        assert!(x >= 20.0);
        assert!(y >= 20.0);

        let candidate = Rect::new(x, y, 400.0, 300.0);
        for item in &items {
            assert!(!item.effective_rect().intersects(&candidate));
        }
    }

    #[test]
    fn a_merged_deck_stands_where_the_target_stood_and_is_its_size() {
        // Dropping one card onto another used to replace both with a deck starting at a constant
        // 420x480 that only ever grew, so a merge could double the footprint of what a person had
        // just arranged.
        let mut layout = DesktopLayout::canonical(None);
        layout.set_size(CardId::Context, 330.0, 200.0);
        let target = layout.geometry(CardId::Context);

        let deck_id = layout
            .create_deck_over(
                "Context + Beliefs",
                vec![CardId::Context, CardId::Beliefs],
                target.x,
                target.y,
                Some((target.width, target.height)),
            )
            .expect("a deck");
        let deck = layout.deck(&deck_id).expect("the deck");

        assert_eq!(deck.geometry.x, target.x);
        assert_eq!(deck.geometry.y, target.y);
        // No larger than the place it took, except where a member's own minimum requires it.
        let floor_w = CardId::Context
            .spec()
            .min_size
            .0
            .max(CardId::Beliefs.spec().min_size.0)
            .max(340.0);
        assert_eq!(deck.geometry.width, target.width.max(floor_w));
        assert!(
            deck.geometry.width < 420.0 || floor_w >= 420.0,
            "the deck grew to the old constant"
        );
    }

    #[test]
    fn a_merged_deck_still_grows_to_fit_what_it_holds() {
        // The footprint is a wish, not an override: a deck smaller than one of its own cards would
        // be a container that cannot show its contents.
        let mut layout = DesktopLayout::canonical(None);
        let deck_id = layout
            .create_deck_over(
                "Tiny",
                vec![CardId::Capabilities, CardId::Journal],
                10.0,
                10.0,
                Some((100.0, 80.0)),
            )
            .expect("a deck");
        let deck = layout.deck(&deck_id).expect("the deck");
        assert!(deck.geometry.width >= CardId::Capabilities.spec().min_size.0);
        assert!(deck.geometry.height >= CardId::Capabilities.spec().min_size.1);
    }

    #[test]
    fn a_layout_saved_with_the_old_maximized_flag_still_loads() {
        // The flag was removed on 2026-08-22 because nothing set it and nothing read it. A desktop
        // saved while it existed must still open: dropping a field is only safe if the field's
        // absence and its presence both parse, and a person's saved layout is not something to
        // discard over a value that never meant anything.
        let saved = r#"{
            "schema_version": 9,
            "cards": [{
                "id": "identity",
                "geometry": {"x": 70.0, "y": 70.0, "width": 220.0, "height": 188.0, "z": 1},
                "presentation": {"collapsed": true, "pinned": false, "maximized": true}
            }],
            "decks": []
        }"#;

        let layout = DesktopLayout::parse_json(saved).expect("an old v9 layout still parses");
        assert!(layout.contains_card(CardId::Identity));
        assert!(layout.presentation(CardId::Identity).collapsed);
        assert!(!layout.presentation(CardId::Identity).pinned);
    }

    #[test]
    fn a_presentation_written_now_does_not_mention_focus() {
        // Focus lives in DesktopViewMode and nowhere else. A persisted flag claiming to answer the
        // same question would be a second truth about it, and the persisted one was the one that
        // never knew.
        let written =
            serde_json::to_string(&CardPresentation::default()).expect("serialize presentation");
        assert!(!written.contains("maximized"), "{written}");
        assert!(written.contains("collapsed"));
        assert!(written.contains("pinned"));
    }

    #[test]
    fn parse_json_supports_both_schemas() {
        let v8_json = serde_json::to_string(&CanvasLayoutV8::default()).unwrap();
        let layout_v8 = DesktopLayout::parse_json(&v8_json).expect("parses v8");
        assert_eq!(layout_v8.schema_version, 9);
        // Eleven for the same reason as above: this is what v8 carried, not what v9 requires.
        assert_eq!(layout_v8.cards.len(), 11);

        let v9_json = serde_json::to_string(&DesktopLayout::default()).unwrap();
        let layout_v9 = DesktopLayout::parse_json(&v9_json).expect("parses v9");
        assert_eq!(layout_v9.schema_version, 9);
    }

    #[test]
    fn snap_calculation_aligns_edges_and_generates_guides() {
        let layout = DesktopLayout::default();
        let id_geom = layout.geometry(CardId::Identity);
        let id_right = id_geom.x + id_geom.width;
        // Place candidate very close to Identity's right edge and top edge
        let candidate_x = id_right + 3.0;
        let candidate_y = id_geom.y + 0.5;

        let snap = layout.compute_snap(
            &DesktopItemId::Card(CardId::Session),
            candidate_x,
            candidate_y,
            300.0,
            200.0,
            8.0,
        );

        // Snapped X should exactly align with Identity's right edge
        assert_eq!(snap.snapped_x, id_right);
        // Snapped Y should exactly align with Identity's top edge
        assert_eq!(snap.snapped_y, id_geom.y);
        assert_eq!(snap.guides.len(), 2);
    }

    #[test]
    fn bounding_rect_encloses_all_items() {
        let layout = DesktopLayout::default();
        let bbox = layout.bounding_rect().expect("bounding rect exists");

        for item in layout.desktop_items() {
            let r = item.effective_rect();
            assert!(r.x >= bbox.x);
            assert!(r.y >= bbox.y);
            assert!(r.right() <= bbox.right());
            assert!(r.bottom() <= bbox.bottom());
        }
    }

    #[test]
    fn fit_to_viewport_centers_bounding_box() {
        let bbox = Rect::new(100.0, 100.0, 800.0, 600.0);
        let (zoom, (pan_x, pan_y)) = DesktopLayout::fit_to_viewport(bbox, 1920.0, 1080.0, 60.0);

        assert!((0.4..=1.2).contains(&zoom));
        // The center of the zoomed bounding box plus pan should be close to viewport center (960, 540)
        let center_x = pan_x + (bbox.x + bbox.width / 2.0) * zoom;
        let center_y = pan_y + (bbox.y + bbox.height / 2.0) * zoom;

        assert!((center_x - 960.0).abs() < 1e-4);
        assert!((center_y - 540.0).abs() < 1e-4);
    }

    #[test]
    fn anchor_mutations_enforce_unique_non_empty_names() {
        let mut layout = DesktopLayout::new();
        assert!(layout.add_anchor("  Home  ", 120.0, 240.0, 1.25));
        assert_eq!(layout.anchors[0].name, "Home");
        assert!(!layout.add_anchor("home", 0.0, 0.0, 1.0));
        assert!(!layout.add_anchor("  ", 0.0, 0.0, 1.0));

        let id = layout.anchors[0].id.clone();
        assert!(layout.rename_anchor(&id, " Focus "));
        assert_eq!(layout.anchors[0].name, "Focus");
        assert!(layout.remove_anchor(&id));
        assert!(layout.anchors.is_empty());
        assert!(!layout.remove_anchor(&id));
    }

    #[test]
    fn anchor_collection_is_bounded() {
        let mut layout = DesktopLayout::new();
        for index in 0..32 {
            assert!(layout.add_anchor(&format!("Anchor {index}"), 0.0, 0.0, 1.0));
        }
        assert!(!layout.add_anchor("One too many", 0.0, 0.0, 1.0));
        assert_eq!(layout.anchors.len(), 32);
    }
}
