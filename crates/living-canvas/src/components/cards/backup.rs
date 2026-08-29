// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Borg & Btrfs Automated Backup Vault card component.

use leptos::prelude::*;
use crate::{
    MindClient,
    CardId,
    components::icons::{IconCheckCircle, IconFile, IconRefresh},
    state::RuntimeState,
    tool_state::ToolCardStates,
};

#[component]
pub fn BackupContent(card: CardId) -> impl IntoView {
    let client = crate::GatewayMindClient;
    let tool_states = expect_context::<ToolCardStates>();
    let signals = tool_states.backup(card);

    let load_backup = move || {
        signals.loading.set(true);
        leptos::task::spawn_local(async move {
            match client.get_backup_settings().await {
                Ok(proj) => {
                    signals.backup_settings.set(Some(proj));
                    signals.status_msg.set(None);
                }
                Err(err) => {
                    signals.status_msg.set(Some(format!("Failed to load backup vault: {err}")));
                }
            }
            signals.loading.set(false);
        });
    };

    let trigger_backup = move || {
        let name_str = signals.new_backup_name.get();
        let name_opt = if name_str.trim().is_empty() { None } else { Some(name_str.trim().to_owned()) };
        signals.loading.set(true);
        leptos::task::spawn_local(async move {
            match client.trigger_backup(name_opt).await {
                Ok(arch) => {
                    signals.status_msg.set(Some(format!("Backup archive '{}' completed in {}s", arch.name, arch.duration_seconds)));
                    signals.new_backup_name.set(String::new());
                    load_backup();
                }
                Err(err) => {
                    signals.status_msg.set(Some(format!("Backup failed: {err}")));
                }
            }
            signals.loading.set(false);
        });
    };

    let trigger_restore = move |arch_id: String| {
        leptos::task::spawn_local(async move {
            match client.restore_archive(&arch_id, None).await {
                Ok(outcome) => {
                    signals.status_msg.set(Some(outcome));
                    load_backup();
                }
                Err(err) => {
                    signals.status_msg.set(Some(format!("Restore failed: {err}")));
                }
            }
        });
    };

    let toggle_schedule = move |enabled: bool| {
        let cur_sched = signals
            .backup_settings
            .get()
            .map(|s| s.schedule)
            .unwrap_or(cybou_protocol::system::BackupScheduleRecord {
                enabled: true,
                frequency: "daily".to_owned(),
                retention_daily: 7,
                retention_weekly: 4,
                retention_monthly: 12,
            });
        let req = cybou_web_contracts::UpdateBackupScheduleRequest {
            enabled,
            frequency: cur_sched.frequency,
            retention_daily: cur_sched.retention_daily,
            retention_weekly: cur_sched.retention_weekly,
            retention_monthly: cur_sched.retention_monthly,
        };
        leptos::task::spawn_local(async move {
            match client.update_backup_schedule(req).await {
                Ok(_) => {
                    signals.status_msg.set(Some(format!("Backup schedule {}", if enabled { "enabled" } else { "disabled" })));
                    load_backup();
                }
                Err(err) => {
                    signals.status_msg.set(Some(format!("Schedule update failed: {err}")));
                }
            }
        });
    };

    // Trigger initial load
    Effect::new(move |_| {
        load_backup();
    });

    view! {
        <div class="backup-panel" style="display: flex; flex-direction: column; height: 100%; width: 100%; background: var(--bg-card, #1e1e24); color: var(--text-main, #e0e0e0); font-family: system-ui, -apple-system, sans-serif; overflow-y: auto;">
            // Header
            <div style="display: flex; align-items: center; justify-content: space-between; padding: 10px 12px; background: rgba(0,0,0,0.2); border-bottom: 1px solid rgba(255,255,255,0.08);">
                <div style="display: flex; align-items: center; gap: 8px;">
                    <span style="font-weight: 600; font-size: 13px;">"Deduplicating Backup Vault"</span>
                </div>
                <button
                    style="background: rgba(255,255,255,0.06); border: none; border-radius: 4px; padding: 4px 6px; color: inherit; cursor: pointer;"
                    title="Refresh backup vault"
                    on:click=move |_| load_backup()
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

            {move || signals.backup_settings.get().map(|bs| {
                let repo = bs.repository;
                let archives = bs.archives;
                let schedule = bs.schedule;
                let size_gb = repo.total_size_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                let is_sched_enabled = schedule.enabled;

                view! {
                    <div style="padding: 12px; display: flex; flex-direction: column; gap: 14px;">
                        // Vault Repository Overview
                        <div style="background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.06); border-radius: 6px; padding: 12px; display: flex; align-items: center; justify-content: space-between;">
                            <div>
                                <div style="font-weight: 700; font-size: 13px; color: #f3f4f6; margin-bottom: 2px;">{repo.name}</div>
                                <div style="font-size: 10px; color: rgba(255,255,255,0.5); font-family: monospace;">
                                    {format!("{} • {:.2} GB deduplicated • {} archives", repo.destination, size_gb, repo.total_archives)}
                                </div>
                                <div style="font-size: 10px; color: #4ade80; font-family: monospace; margin-top: 2px;">
                                    {format!("Encryption: {} • Last: {}", repo.encryption, repo.last_backup_time.unwrap_or_else(|| "Never".to_owned()))}
                                </div>
                            </div>

                            <button
                                style="background: linear-gradient(135deg, #6366f1, #8b5cf6); border: none; border-radius: 6px; padding: 6px 14px; font-size: 11px; font-weight: 700; color: #fff; cursor: pointer; box-shadow: 0 2px 8px rgba(99,102,241,0.3);"
                                on:click=move |_| trigger_backup()
                            >
                                "Backup Now"
                            </button>
                        </div>

                        // Automation & Retention Schedule
                        <div style="background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.06); border-radius: 6px; padding: 10px 12px; display: flex; align-items: center; justify-content: space-between; font-size: 11px;">
                            <div>
                                <div style="font-weight: 600; color: #f3f4f6;">"Automated Snapshot Schedule"</div>
                                <div style="font-size: 10px; color: rgba(255,255,255,0.5);">
                                    {format!("Frequency: {} • Retain: {} daily, {} weekly, {} monthly", schedule.frequency, schedule.retention_daily, schedule.retention_weekly, schedule.retention_monthly)}
                                </div>
                            </div>
                            <button
                                style=format!("border: none; border-radius: 4px; padding: 3px 8px; font-size: 10px; font-weight: 700; cursor: pointer; background: {}; color: {};", if is_sched_enabled { "rgba(34,197,94,0.2)" } else { "rgba(156,163,175,0.2)" }, if is_sched_enabled { "#4ade80" } else { "#9ca3af" })
                                on:click=move |_| toggle_schedule(!is_sched_enabled)
                            >
                                {if is_sched_enabled { "ENABLED" } else { "DISABLED" }}
                            </button>
                        </div>

                        // Historical Archives Timeline
                        <div style="background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.06); border-radius: 6px; padding: 10px 12px;">
                            <div style="font-weight: 600; font-size: 11px; margin-bottom: 8px;">
                                {format!("Snapshot Archives ({})", archives.len())}
                            </div>
                            <div style="display: flex; flex-direction: column; gap: 6px;">
                                {archives.into_iter().map(|arch| {
                                    let arch_id = arch.id.clone();
                                    let size_mb = arch.size_bytes / (1024 * 1024);
                                    view! {
                                        <div style="background: rgba(0,0,0,0.2); border-radius: 4px; padding: 8px 10px; display: flex; align-items: center; justify-content: space-between; font-size: 11px;">
                                            <div>
                                                <div style="font-weight: 600; color: #f3f4f6; margin-bottom: 2px;">{arch.name}</div>
                                                <div style="font-size: 10px; color: rgba(255,255,255,0.4); font-family: monospace;">
                                                    {format!("{} MB • {}s • {}", size_mb, arch.duration_seconds, arch.timestamp)}
                                                </div>
                                            </div>
                                            <button
                                                style="background: rgba(245, 158, 11, 0.15); border: 1px solid rgba(245, 158, 11, 0.3); border-radius: 4px; padding: 3px 8px; font-size: 10px; color: #fbbf24; cursor: pointer;"
                                                title="Restore archive files"
                                                on:click=move |_| trigger_restore(arch_id.clone())
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
