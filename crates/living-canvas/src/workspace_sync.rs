// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Following one person's desktop from whatever screen they are sitting at.
//!
//! The arrangement was kept in `localStorage` and nowhere else, which is per browser, per profile
//! and per machine. Signing in from a second computer produced a stranger's desktop; clearing site
//! data threw the arrangement away along with the cookies. For a desktop whose argument is that it
//! is the same desktop wherever you reach it, that was the wrong place to keep it.
//!
//! `localStorage` stays, because it is the only copy that is there before the first request comes
//! back and the only one left when the gateway is unreachable. What changes is that it is no longer
//! the only copy: what a seat saves is sent to the gateway, and what the gateway has is adopted at
//! startup.
//!
//! The gateway wins at startup, deliberately. Two copies eventually disagree, and the one the
//! account carries is the one a person means when they open the desktop somewhere new. The local
//! copy is a cache in front of it, not a peer.

#[cfg(target_arch = "wasm32")]
pub use browser::provide_workspace_sync;

#[cfg(target_arch = "wasm32")]
mod browser {
    use gloo_net::http::Request;
    use leptos::prelude::*;

    use crate::DesktopLayout;

    /// How often a changed desktop is sent, in milliseconds.
    ///
    /// Two seconds. A drag emits a position on every frame and each of them is a change; sending
    /// them would be a request per frame for something nobody reads until the next sign-in. What
    /// matters is that the arrangement is durable shortly after a person stops moving things, not
    /// that every intermediate position was recorded.
    const SYNC_INTERVAL_MS: u32 = 2_000;

    /// Adopt the account's saved desktop, then keep sending this one back to it.
    ///
    /// Called once, by the app root, with the layout signal the whole desktop reads and writes.
    pub fn provide_workspace_sync(layout: RwSignal<DesktopLayout>) {
        // Set while this module is the one writing to `layout`, so adopting the server's copy is
        // not mistaken for a person rearranging their desktop and sent straight back.
        let adopting = StoredValue::new_local(false);
        let dirty = RwSignal::new(false);
        // Turned off for good on the first refusal. A desktop with no seat — a public preview —
        // has nowhere to save an arrangement to, and retrying every two seconds forever would be
        // this page asking a question it has already been answered.
        let syncing = RwSignal::new(true);

        leptos::task::spawn_local(async move {
            let Ok(response) = Request::get("/api/v1/desktop/layout").send().await else {
                return;
            };
            if !response.ok() {
                // 403 is the ordinary answer for a reader with no seat, and it is not a failure of
                // anything: it means this desktop is local to this browser and always was.
                syncing.set(false);
                return;
            }
            let Ok(projection) = response
                .json::<cybou_web_contracts::DesktopLayoutProjection>()
                .await
            else {
                return;
            };
            // No saved arrangement is not an empty desktop. A seat that has never saved keeps
            // whatever this browser already had, which is how a first sign-in on a machine somebody
            // has been using anonymously does not wipe their desktop.
            let Some(saved) = projection.layout else {
                return;
            };
            let Ok(mut restored) = serde_json::from_str::<DesktopLayout>(&saved) else {
                return;
            };
            restored.validate_and_normalize();
            adopting.set_value(true);
            layout.set(restored);
            // Written through to the local copy as well, so a reload with the gateway down opens
            // what the account last had rather than what this browser last had.
            layout.get_untracked().save();
            adopting.set_value(false);
        });

        // Anything that changes the desktop changes this signal, so this is the one place that has
        // to know a change happened — rather than every call site that makes one remembering to say
        // so, which is the arrangement that let the local copy be the only copy for so long.
        Effect::new(move |_| {
            layout.track();
            if !adopting.get_value() {
                dirty.set(true);
            }
        });

        let interval = gloo_timers::callback::Interval::new(SYNC_INTERVAL_MS, move || {
            if !syncing.get_untracked() || !dirty.get_untracked() {
                return;
            }
            let Ok(body) = serde_json::to_string(&layout.get_untracked()) else {
                return;
            };
            // Cleared before the request rather than after it. A change made while this one is in
            // flight must survive: clearing on success would drop it, and the next tick would find
            // nothing to send.
            dirty.set(false);
            leptos::task::spawn_local(async move {
                let sent = Request::put("/api/v1/desktop/layout")
                    .json(&cybou_web_contracts::DesktopLayoutSaveRequest { layout: body });
                let Ok(request) = sent else {
                    return;
                };
                match request.send().await {
                    Ok(response) if response.ok() => {}
                    // A refusal means there is nowhere to save to; anything else is this gateway
                    // being unreachable for a moment, and the next change will try again.
                    Ok(response) if response.status() == 403 => syncing.set(false),
                    _ => dirty.set(true),
                }
            });
        });
        let held = StoredValue::new_local(Some(interval));
        on_cleanup(move || held.update_value(|slot| drop(slot.take())));
    }
}
