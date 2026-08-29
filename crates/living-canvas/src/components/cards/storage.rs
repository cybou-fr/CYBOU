// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Storage and Btrfs Snapshots card component.

use leptos::prelude::*;
use crate::{
    MindClient,
    CardId,
    components::icons::{IconCheckCircle, IconFile, IconRefresh},
    state::RuntimeState,
    tool_state::ToolCardStates,
};

#[component]
pub fn StorageContent(card: CardId) -> impl IntoView {
    let client = crate::GatewayMindClient;
    let tool_states = expect_context::<ToolCardStates>();
    let signals = tool_states.storage(card);

    let load_storage = move || {
        signals.loading.set(true);
        leptos::task::spawn_local(async move {
            match client.get_storage().await {
                Ok(proj) => {
                    signals.storage.set(Some(proj));
                    signals.status_msg.set(None);
                }
                Err(err) => {
                    signals.status_msg.set(Some(format!("Failed to load storage: {err}")));
                }
            }
            signals.loading.set(false);
        });
    };

    let trigger_create_snapshot = move || {
        let subvol = signals.selected_subvolume.get().unwrap_or_else(|| "@home".to_owned());
        let name = signals.new_snap_name.get();
        if name.trim().is_empty() {
            signals.status_msg.set(Some("Please enter a snapshot name".to_owned()));
            return;
        }
        signals.loading.set(true);
        leptos::task::spawn_local(async move {
            match client.create_snapshot(&subvol, &name, true).await {
                Ok(_) => {
                    signals.status_msg.set(Some(format!("Created snapshot '{name}' on {subvol}")));
                    signals.new_snap_name.set(String::new());
                    load_storage();
                }
                Err(err) => {
                    signals.status_msg.set(Some(format!("Snapshot failed: {err}")));
                }
            }
            signals.loading.set(false);
        });
    };

    let trigger_restore_snapshot = move |snap_id: String| {
        leptos::task::spawn_local(async move {
            match client.restore_snapshot(&snap_id).await {
                Ok(outcome) => {
                    signals.status_msg.set(Some(outcome));
                    load_storage();
                }
                Err(err) => {
                    signals.status_msg.set(Some(format!("Restore failed: {err}")));
                }
            }
        });
    };

    // Trigger initial load
    Effect::new(move |_| {
        load_storage();
    });

    view! {
        <div class="storage-panel" style="display: flex; flex-direction: column; height: 100%; width: 100%; background: var(--bg-card, #1e1e24); color: var(--text-main, #e0e0e0); font-family: system-ui, -apple-system, sans-serif; overflow-y: auto;">
            // Header
            <div style="display: flex; align-items: center; justify-content: space-between; padding: 10px 12px; background: rgba(0,0,0,0.2); border-bottom: 1px solid rgba(255,255,255,0.08);">
                <div style="display: flex; align-items: center; gap: 8px;">
                    <IconFile size=14 />
                    <span style="font-weight: 600; font-size: 13px;">"Storage & Btrfs Snapshots"</span>
                </div>
                <button
                    style="background: rgba(255,255,255,0.06); border: none; border-radius: 4px; padding: 4px 6px; color: inherit; cursor: pointer;"
                    title="Refresh storage"
                    on:click=move |_| load_storage()
                >
                    <IconRefresh size=13 />
                </button>
            </div>

            // Status message toast
            {move || signals.status_msg.get().map(|msg| {
                view! {
                    <div style="background: rgba(99, 102, 241, 0.15); color: #c7d2fe; font-size: 11px; padding: 6px 12px; border-bottom: 1px solid rgba(99, 102, 241, 0.3); display: flex; justify-content: space-between;">
                        <span>{msg}</span>
                        <button style="background: none; border: none; color: inherit; cursor: pointer;" on:click=move |_| signals.status_msg.set(None)>"×"</button>
                    </div>
                }
            })}

            {move || signals.storage.get().map(|st| {
                let used_bytes = st.total_space_bytes.saturating_sub(st.free_space_bytes);
                let used_gb = used_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                let total_gb = st.total_space_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                let pct = (used_bytes as f64 / st.total_space_bytes as f64) * 100.0;

                view! {
                    <div style="padding: 12px; display: flex; flex-direction: column; gap: 14px;">
                        // Storage Pool Capacity
                        <div style="background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.06); border-radius: 6px; padding: 10px 12px;">
                            <div style="display: flex; justify-content: space-between; margin-bottom: 4px; font-size: 11px;">
                                <span style="font-weight: 600;">"Btrfs Pool Capacity"</span>
                                <span style="font-family: monospace; color: #38bdf8;">{format!("{:.0} / {:.0} GB ({:.1}%)", used_gb, total_gb, pct)}</span>
                            </div>
                            <div style="width: 100%; height: 6px; background: rgba(255,255,255,0.08); border-radius: 3px; overflow: hidden;">
                                <div style=format!("width: {}%; height: 100%; background: #38bdf8; border-radius: 3px;", pct.min(100.0)) />
                            </div>
                        </div>

                        // Subvolumes
                        <div style="background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.06); border-radius: 6px; padding: 10px 12px;">
                            <div style="font-weight: 600; font-size: 11px; margin-bottom: 8px;">"Subvolumes"</div>
                            <div style="display: flex; flex-direction: column; gap: 6px;">
                                {st.subvolumes.into_iter().map(|sub| {
                                    let path = sub.path.clone();
                                    let path_select = sub.path.clone();
                                    let is_sel = move || signals.selected_subvolume.get().as_ref() == Some(&path_select);
                                    view! {
                                        <div
                                            style=move || format!(
                                                "background: {}; border: 1px solid rgba(255,255,255,0.05); border-radius: 4px; padding: 6px 10px; display: flex; align-items: center; justify-content: space-between; font-size: 11px; cursor: pointer;",
                                                if is_sel() { "rgba(99, 102, 241, 0.2)" } else { "rgba(0,0,0,0.2)" }
                                            )
                                            on:click=move |_| signals.selected_subvolume.set(Some(path.clone()))
                                        >
                                            <span style="font-family: monospace; font-weight: 600; color: #f3f4f6;">{sub.path}</span>
                                            <span style="font-size: 10px; color: rgba(255,255,255,0.5);">{format!("ID {}", sub.id)}</span>
                                        </div>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        </div>

                        // Create Snapshot Section
                        <div style="background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.06); border-radius: 6px; padding: 10px 12px;">
                            <div style="font-weight: 600; font-size: 11px; margin-bottom: 8px;">"Create Point-in-Time Snapshot"</div>
                            <div style="display: flex; gap: 8px; align-items: center;">
                                <input
                                    type="text"
                                    placeholder="Snapshot label..."
                                    prop:value=move || signals.new_snap_name.get()
                                    on:input=move |e| signals.new_snap_name.set(event_target_value(&e))
                                    style="background: rgba(0,0,0,0.3); border: 1px solid rgba(255,255,255,0.1); border-radius: 4px; padding: 4px 8px; font-size: 11px; color: inherit; flex: 1;"
                                />
                                <button
                                    style="background: rgba(99, 102, 241, 0.2); border: 1px solid rgba(99, 102, 241, 0.4); border-radius: 4px; padding: 4px 10px; font-size: 11px; color: #c7d2fe; font-weight: 600; cursor: pointer;"
                                    on:click=move |_| trigger_create_snapshot()
                                >
                                    "Snapshot"
                                </button>
                            </div>
                        </div>

                        // Snapshots List
                        <div style="background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.06); border-radius: 6px; padding: 10px 12px;">
                            <div style="font-weight: 600; font-size: 11px; margin-bottom: 8px;">
                                {format!("Snapshots ({})", st.snapshots.len())}
                            </div>
                            <div style="display: flex; flex-direction: column; gap: 6px;">
                                {st.snapshots.into_iter().map(|snap| {
                                    let snap_id = snap.id.clone();
                                    let size_mb = snap.size_bytes / (1024 * 1024);
                                    view! {
                                        <div style="background: rgba(0,0,0,0.2); border-radius: 4px; padding: 8px 10px; display: flex; align-items: center; justify-content: space-between; font-size: 11px;">
                                            <div>
                                                <div style="font-weight: 600; color: #f3f4f6; margin-bottom: 2px;">{snap.name}</div>
                                                <div style="font-size: 10px; color: rgba(255,255,255,0.4); font-family: monospace;">
                                                    {format!("{} • {} MB • {}", snap.subvolume_path, size_mb, snap.timestamp)}
                                                </div>
                                            </div>
                                            <button
                                                style="background: rgba(245, 158, 11, 0.15); border: 1px solid rgba(245, 158, 11, 0.3); border-radius: 4px; padding: 3px 8px; font-size: 10px; color: #fbbf24; cursor: pointer;"
                                                title="Restore snapshot"
                                                on:click=move |_| trigger_restore_snapshot(snap_id.clone())
                                            >
                                                "Restore"
                                            </button>
                                        </div>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        </div>
                    </div>
                }
            })}
        </div>
    }
}
