// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Conditional synchronization: late reads and other tabs never discard local work.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Loading,
    Ready,
    Conflict,
    LocalOnly,
    Invalid,
}

#[derive(Clone, Debug)]
pub struct WorkspaceSync {
    pub phase: Phase,
    pub revision: Option<String>,
    pub remote: Option<String>,
    baseline: String,
    pub busy: bool,
    pub offline: bool,
    conflict_read: bool,
}
impl WorkspaceSync {
    pub fn new(local: String) -> Self {
        Self {
            phase: Phase::Loading,
            revision: None,
            remote: None,
            baseline: local,
            busy: false,
            offline: false,
            conflict_read: false,
        }
    }
    pub fn loaded(
        &mut self,
        local: &str,
        remote: Option<String>,
        revision: Option<String>,
    ) -> Option<String> {
        self.busy = false;
        self.offline = false;
        self.revision = revision;
        self.remote.clone_from(&remote);
        if self.conflict_read
            || (local != self.baseline && remote.as_deref().is_some_and(|r| r != local))
        {
            self.phase = Phase::Conflict;
            return None;
        }
        self.phase = Phase::Ready;
        self.baseline = remote.clone().unwrap_or_default();
        remote
    }
    pub fn needs_save(&self, local: &str) -> bool {
        self.phase == Phase::Ready && !self.busy && local != self.baseline
    }
    pub fn has_pending_changes(&self, local: &str) -> bool {
        self.phase != Phase::LocalOnly && (self.phase == Phase::Conflict || local != self.baseline)
    }
    pub fn saved(&mut self, sent: String, revision: Option<String>) {
        self.baseline = sent;
        self.revision = revision;
        self.busy = false;
        self.offline = false;
    }
    pub fn rejected(&mut self) {
        self.phase = Phase::Loading;
        self.conflict_read = true;
        self.busy = false;
    }
    pub fn keep_local(&mut self) {
        self.baseline = self.remote.clone().unwrap_or_default();
        self.phase = Phase::Ready;
        self.conflict_read = false;
    }
}

#[cfg(target_arch = "wasm32")]
pub use browser::{WorkspaceSyncStatus, provide_workspace_sync};

#[cfg(target_arch = "wasm32")]
mod browser {
    use super::{Phase, WorkspaceSync};
    use crate::DesktopLayout;
    use gloo_net::http::Request;
    use leptos::prelude::*;

    fn serialize(layout: RwSignal<DesktopLayout>) -> String {
        serde_json::to_string(&layout.get_untracked()).expect("normalized layout")
    }
    fn sync(layout: RwSignal<DesktopLayout>, state: RwSignal<WorkspaceSync>) {
        let current = serialize(layout);
        let snapshot = state.get_untracked();
        if snapshot.busy {
            return;
        }
        let loading = snapshot.phase == Phase::Loading;
        if !loading && !snapshot.needs_save(&current) {
            return;
        }
        state.update(|s| s.busy = true);
        leptos::task::spawn_local(async move {
            let response = if loading {
                Request::get("/api/v1/desktop/layout").send().await
            } else {
                match Request::put("/api/v1/desktop/layout").json(
                    &cybou_web_contracts::DesktopLayoutSaveRequest {
                        layout: current.clone(),
                        expected_revision: snapshot.revision,
                    },
                ) {
                    Ok(request) => request.send().await,
                    Err(error) => Err(error),
                }
            };
            if state.is_disposed() {
                return;
            }
            match response {
                Ok(response) if response.ok() => {
                    let projection = response
                        .json::<cybou_web_contracts::DesktopLayoutProjection>()
                        .await;
                    if state.is_disposed() {
                        return;
                    }
                    let Ok(projection) = projection else {
                        state.update(|s| {
                            s.busy = false;
                            s.phase = Phase::Invalid;
                        });
                        return;
                    };
                    if loading {
                        let restored = projection
                            .layout
                            .as_deref()
                            .map(serde_json::from_str::<DesktopLayout>)
                            .transpose();
                        let Ok(restored) = restored else {
                            state.update(|s| {
                                s.busy = false;
                                s.phase = Phase::Invalid;
                            });
                            return;
                        };
                        let remote = restored.map(|mut value| {
                            value.validate_and_normalize();
                            serde_json::to_string(&value).expect("normalized layout")
                        });
                        let local = serialize(layout);
                        let mut adopt = None;
                        state.update(|s| adopt = s.loaded(&local, remote, projection.revision));
                        if let Some(saved) = adopt {
                            layout.set(serde_json::from_str(&saved).expect("validated layout"));
                            layout.get_untracked().save();
                        }
                    } else {
                        state.update(|s| s.saved(current, projection.revision));
                    }
                }
                Ok(response) if response.status() == 409 => state.update(WorkspaceSync::rejected),
                Ok(response) if matches!(response.status(), 401 | 403) => state.update(|s| {
                    s.busy = false;
                    s.phase = Phase::LocalOnly;
                }),
                Ok(response) if response.status() < 500 => state.update(|s| {
                    s.busy = false;
                    s.phase = Phase::Invalid;
                }),
                _ => state.update(|s| {
                    s.busy = false;
                    s.offline = true;
                }),
            }
        });
    }

    pub fn provide_workspace_sync(layout: RwSignal<DesktopLayout>) {
        let state = RwSignal::new(WorkspaceSync::new(serialize(layout)));
        provide_context(state);
        sync(layout, state);
        let interval = gloo_timers::callback::Interval::new(2_000, move || sync(layout, state));
        let held = StoredValue::new_local(Some(interval));
        on_cleanup(move || held.update_value(|slot| drop(slot.take())));
    }

    #[component]
    pub fn WorkspaceSyncStatus() -> impl IntoView {
        let state = use_context::<RwSignal<WorkspaceSync>>();
        let layout = use_context::<RwSignal<DesktopLayout>>();
        match (state, layout) {
            (Some(state), Some(layout)) => view! {
                <div class="workspace-sync" class:conflict=move || state.get().phase == Phase::Conflict>
                <span role="status" aria-live="polite">{move || {
                    let s = state.get();
                    if s.offline { return "Layout offline — retrying"; }
                    match s.phase {
                        Phase::Loading => "Loading layout…",
                        Phase::Conflict => "Layout changed elsewhere. Choose which to keep.",
                        Phase::LocalOnly => "Layout on this browser only",
                        Phase::Invalid => "Layout sync unavailable — local layout kept",
                        Phase::Ready => if s.busy { "Saving layout…" }
                            else if s.needs_save(&serde_json::to_string(&layout.get()).unwrap_or_default()) {
                                "Layout not saved yet"
                            } else { "Layout saved" },
                    }
                }}</span>
                <Show when=move || state.get().phase == Phase::Conflict>
                    <button class="history-btn" on:click=move |_| state.update(WorkspaceSync::keep_local)>
                        "Keep this layout"
                    </button>
                    <button class="history-btn" disabled=move || state.get().remote.is_none() on:click=move |_| {
                        if let Some(saved) = state.get_untracked().remote
                            && let Ok(restored) = serde_json::from_str::<DesktopLayout>(&saved) {
                                layout.set(restored);
                                layout.get_untracked().save();
                                state.update(|s| { s.keep_local(); s.baseline = saved; });
                        }
                    }>"Use saved layout"</button>
                </Show>
                </div>
            }.into_any(),
            _ => ().into_any(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_late_read_preserves_local_edits() {
        let mut s = WorkspaceSync::new("initial".into());
        assert!(!s.needs_save("edited"));
        assert_eq!(
            s.loaded("edited", Some("remote".into()), Some("r1".into())),
            None
        );
        assert_eq!(s.phase, Phase::Conflict);
        assert!(!s.needs_save("edited"));
        s.keep_local();
        assert!(s.needs_save("edited"));
        assert_eq!(s.revision.as_deref(), Some("r1"));
    }
    #[test]
    fn edits_during_a_save_remain_pending() {
        let mut s = WorkspaceSync::new("initial".into());
        s.loaded("initial", None, None);
        s.busy = true;
        assert!(!s.needs_save("second"));
        s.saved("first".into(), Some("r1".into()));
        assert!(s.needs_save("second"));
        assert!(!s.needs_save("first"));
    }
    #[test]
    fn a_conflict_refresh_never_silently_adopts_the_other_tab() {
        let mut s = WorkspaceSync::new("initial".into());
        s.rejected();
        assert_eq!(
            s.loaded("initial", Some("other".into()), Some("r2".into())),
            None
        );
        assert_eq!(s.phase, Phase::Conflict);
    }
    #[test]
    fn initial_load_adopts_saved_and_new_accounts_save_local() {
        let mut s = WorkspaceSync::new("initial".into());
        assert_eq!(
            s.loaded("initial", Some("saved".into()), Some("r1".into())),
            Some("saved".into())
        );
        assert!(!s.needs_save("saved"));
        let mut s = WorkspaceSync::new("initial".into());
        assert_eq!(s.loaded("initial", None, None), None);
        assert!(s.needs_save("initial"));
    }
}
