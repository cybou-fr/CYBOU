// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Where the layout is kept between visits, and what happens when there is nowhere to keep it.
//!
//! Only the browser build has somewhere to persist to. The native build is not a degraded version
//! of that: it has no `localStorage`, so it says so with a loader that returns the validated
//! default and a save that does nothing, rather than pretending at a store it does not have.

use super::DesktopLayout;

#[cfg(target_arch = "wasm32")]
use crate::layout::migration::{CanvasLayoutV8, LAYOUT_KEY_V8, LAYOUT_KEY_V9};

#[cfg(target_arch = "wasm32")]
impl DesktopLayout {
    /// Load layout from browser `localStorage`, seamlessly migrating from v8 if necessary.
    #[must_use]
    pub fn load() -> Self {
        let storage = web_sys::window().and_then(|w| w.local_storage().ok().flatten());
        let Some(storage) = storage else {
            let mut def = Self::default();
            def.validate_and_normalize();
            return def;
        };

        // 1. Try v9 key first
        if let Ok(Some(v9_str)) = storage.get_item(LAYOUT_KEY_V9)
            && let Ok(mut v9) = serde_json::from_str::<Self>(&v9_str)
            && v9.schema_version == 9
        {
            v9.validate_and_normalize();
            return v9;
        }

        // 2. Try legacy v8 key
        if let Ok(Some(v8_str)) = storage.get_item(LAYOUT_KEY_V8)
            && let Ok(v8) = serde_json::from_str::<CanvasLayoutV8>(&v8_str)
        {
            let mut migrated = Self::from_v8(&v8);
            migrated.validate_and_normalize();
            migrated.save();
            return migrated;
        }

        let mut default_layout = Self::default();
        default_layout.validate_and_normalize();
        default_layout.save();
        default_layout
    }

    /// Save current layout to browser `localStorage` under `cybou.desktop.layout.v9`.
    pub fn save(&self) {
        let storage = web_sys::window().and_then(|w| w.local_storage().ok().flatten());
        if let Some(storage) = storage
            && let Ok(serialized) = serde_json::to_string(self)
        {
            let _ = storage.set_item(LAYOUT_KEY_V9, &serialized);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl DesktopLayout {
    /// Non-WASM loader returning validated default layout.
    #[must_use]
    pub fn load() -> Self {
        let mut def = Self::default();
        def.validate_and_normalize();
        def
    }

    /// Non-WASM save no-op.
    pub fn save(&self) {}
}
