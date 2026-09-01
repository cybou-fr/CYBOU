// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! System Resource Monitor and Hardware Telemetry card component.

use crate::{
    CardId, MindClient,
    components::{freshness::FreshnessControls, icons::IconActivity},
    refresh::Freshness,
    tool_state::ToolCardStates,
};
use leptos::prelude::*;

/// How often telemetry is re-read while the panel is open and visible.
///
/// Five seconds. Load, memory and temperature move on the scale of seconds, and a person watching
/// this panel is usually watching it because something is happening now.
const MONITOR_INTERVAL_MS: u32 = 5_000;

#[component]
pub fn MonitorContent(card: CardId) -> impl IntoView {
    let client = crate::GatewayMindClient;
    let tool_states = expect_context::<ToolCardStates>();
    let signals = tool_states.monitor(card);

    // Telemetry is the panel this matters most for: a load average is a claim about right now,
    // and one from eleven minutes ago is indistinguishable on screen from one from a second ago.
    let freshness = Freshness::new();

    let load_monitor = move || {
        signals.loading.set(true);
        leptos::task::spawn_local(async move {
            match client.get_system_monitor().await {
                Ok(proj) => {
                    signals.monitor.set(Some(proj));
                    signals.status_msg.set(None);
                    freshness.arrived();
                }
                Err(err) => {
                    signals
                        .status_msg
                        .set(Some(format!("Failed to load telemetry: {err}")));
                }
            }
            signals.loading.set(false);
        });
    };

    // Trigger initial load
    Effect::new(move |_| {
        load_monitor();
    });

    // And keep asking. `auto_refresh` has been on this struct since the panel was written and was
    // read by nothing, so the toggle existed and the timer did not.
    crate::refresh::keep_reading(
        MONITOR_INTERVAL_MS,
        signals.auto_refresh,
        signals.loading,
        load_monitor,
    );

    view! {
        <div class="monitor-panel" style="display: flex; flex-direction: column; height: 100%; width: 100%; overflow-y: auto;">
            // Header
            <div style="display: flex; align-items: center; justify-content: space-between; padding: 10px 12px; background: var(--bg-sunken); border-bottom: 1px solid var(--line);">
                <div style="display: flex; align-items: center; gap: 8px;">
                    <IconActivity size=14 />
                    <span style="font-weight: 600; font-size: 13px;">"Hardware Telemetry & Monitor"</span>
                </div>
                <FreshnessControls
                    freshness=freshness
                    auto_refresh=signals.auto_refresh
                    loading=signals.loading
                    refresh_now=move |()| load_monitor()
                />
            </div>

            // The panel wrote "Failed to load telemetry" into a signal nothing read, so a host
            // whose gateway could not be reached drew an empty Monitor: indistinguishable from a
            // machine doing nothing, which is the one reading this panel must never give.
            {move || signals.status_msg.get().map(|message| view! {
                <div class="card-status-line" role="status">
                    <span>{message}</span>
                    <button
                        class="card-status-dismiss"
                        title="Dismiss"
                        on:click=move |_| signals.status_msg.set(None)
                    >
                        "×"
                    </button>
                </div>
            })}

            // And an unread projection is not an empty one. Until telemetry arrives this says it
            // has not arrived, rather than drawing a host with no memory and no disks.
            {move || (signals.monitor.get().is_none() && signals.status_msg.get().is_none()).then(|| view! {
                <div class="card-unread">
                    {move || if signals.loading.get() {
                        "Reading this host…"
                    } else {
                        "Not read yet."
                    }}
                </div>
            })}

            {move || signals.monitor.get().map(|mon| {
                let uptime_hours = mon.uptime_seconds / 3600;
                let ram_used_gb = mon.memory_used_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                let ram_total_gb = mon.memory_total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                let ram_pct = if mon.memory_total_bytes > 0 {
                    (mon.memory_used_bytes as f64 / mon.memory_total_bytes as f64) * 100.0
                } else {
                    0.0
                };

                let swap_used_gb = mon.swap_used_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                let swap_total_gb = mon.swap_total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                let swap_pct = if mon.swap_total_bytes > 0 {
                    (mon.swap_used_bytes as f64 / mon.swap_total_bytes as f64) * 100.0
                } else {
                    0.0
                };

                view! {
                    <div style="padding: 12px; display: flex; flex-direction: column; gap: 14px;">
                        // Host identity summary card
                        <div style="background: var(--fill-faint); border: 1px solid var(--fill-subtle); border-radius: 6px; padding: 10px 12px; display: grid; grid-template-columns: 1fr 1fr; gap: 8px; font-size: 11px;">
                            <div>
                                <div style="color: var(--text-faint); font-size: 10px;">"HOSTNAME"</div>
                                <div style="font-weight: 600; color: var(--text-bright);">{mon.hostname}</div>
                            </div>
                            <div>
                                <div style="color: var(--text-faint); font-size: 10px;">"OS RELEASE"</div>
                                <div style="color: var(--text-second);">{mon.os_release}</div>
                            </div>
                            <div>
                                <div style="color: var(--text-faint); font-size: 10px;">"UPTIME"</div>
                                <div style="color: var(--info); font-weight: 600;">{format!("{uptime_hours} hours")}</div>
                            </div>
                            <div>
                                <div style="color: var(--text-faint); font-size: 10px;">"LOAD AVERAGE"</div>
                                <div style="font-family: monospace;">{format!("{:.2}, {:.2}, {:.2}", mon.load_avg[0], mon.load_avg[1], mon.load_avg[2])}</div>
                            </div>
                        </div>

                        // CPU Utilization
                        <div style="background: var(--fill-faint); border: 1px solid var(--fill-subtle); border-radius: 6px; padding: 10px 12px;">
                            <div style="display: flex; justify-content: space-between; margin-bottom: 6px; font-size: 11px;">
                                <span style="font-weight: 600;">"CPU Utilization"</span>
                                <span style="font-weight: 700; color: var(--accent-light); font-family: monospace;">{format!("{:.1}%", mon.total_cpu_percent)}</span>
                            </div>
                            <div style="width: 100%; height: 6px; background: var(--line); border-radius: 3px; overflow: hidden; margin-bottom: 10px;">
                                <div style=format!("width: {}%; height: 100%; background: linear-gradient(90deg, var(--accent-solid), var(--accent-solid)); border-radius: 3px;", mon.total_cpu_percent.min(100.0)) />
                            </div>

                            // Cores grid
                            <div style="display: grid; grid-template-columns: repeat(auto-fill, minmax(110px, 1fr)); gap: 6px;">
                                {mon.cores.into_iter().map(|core| {
                                    view! {
                                        <div style="background: var(--bg-sunken); padding: 4px 6px; border-radius: 4px; font-size: 10px;">
                                            <div style="display: flex; justify-content: space-between; margin-bottom: 2px;">
                                                <span style="color: var(--text-dim);">{format!("Core {}", core.core_id)}</span>
                                                <span style="font-family: monospace;">{format!("{:.0}%", core.usage_percent)}</span>
                                            </div>
                                            <div style="width: 100%; height: 3px; background: var(--line); border-radius: 2px; overflow: hidden;">
                                                <div style=format!("width: {}%; height: 100%; background: var(--accent-light);", core.usage_percent.min(100.0)) />
                                            </div>
                                        </div>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        </div>

                        // Memory & Swap
                        <div style="background: var(--fill-faint); border: 1px solid var(--fill-subtle); border-radius: 6px; padding: 10px 12px; display: flex; flex-direction: column; gap: 10px;">
                            <div>
                                <div style="display: flex; justify-content: space-between; margin-bottom: 4px; font-size: 11px;">
                                    <span style="font-weight: 600;">"Physical RAM"</span>
                                    <span style="font-family: monospace; color: var(--info);">{format!("{ram_used_gb:.1} / {ram_total_gb:.1} GB ({ram_pct:.0}%)")}</span>
                                </div>
                                <div style="width: 100%; height: 6px; background: var(--line); border-radius: 3px; overflow: hidden;">
                                    <div style=format!("width: {}%; height: 100%; background: var(--info); border-radius: 3px;", ram_pct.min(100.0)) />
                                </div>
                            </div>

                            {if swap_total_gb > 0.0 {
                                Some(view! {
                                    <div>
                                        <div style="display: flex; justify-content: space-between; margin-bottom: 4px; font-size: 11px;">
                                            <span style="font-weight: 600;">"Swap Space"</span>
                                            <span style="font-family: monospace; color: var(--text-second);">{format!("{swap_used_gb:.1} / {swap_total_gb:.1} GB ({swap_pct:.0}%)")}</span>
                                        </div>
                                        <div style="width: 100%; height: 6px; background: var(--line); border-radius: 3px; overflow: hidden;">
                                            <div style=format!("width: {}%; height: 100%; background: var(--text-muted); border-radius: 3px;", swap_pct.min(100.0)) />
                                        </div>
                                    </div>
                                })
                            } else {
                                None
                            }}
                        </div>

                        // Disk Storage
                        <div style="background: var(--fill-faint); border: 1px solid var(--fill-subtle); border-radius: 6px; padding: 10px 12px;">
                            <div style="font-weight: 600; font-size: 11px; margin-bottom: 8px;">"Storage Partitions"</div>
                            <div style="display: flex; flex-direction: column; gap: 8px;">
                                {mon.disk_partitions.into_iter().map(|disk| {
                                    let used_gb = disk.used_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                                    let total_gb = disk.total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                                    let pct = if disk.total_bytes > 0 {
                                        (disk.used_bytes as f64 / disk.total_bytes as f64) * 100.0
                                    } else {
                                        0.0
                                    };

                                    view! {
                                        <div>
                                            <div style="display: flex; justify-content: space-between; font-size: 11px; margin-bottom: 2px;">
                                                <div>
                                                    <span style="font-weight: 600; color: var(--text-bright);">{disk.mount_point}</span>
                                                    <span style="font-size: 10px; color: var(--text-faint); margin-left: 6px;">{format!("({} / {})", disk.device, disk.fs_type)}</span>
                                                </div>
                                                <span style="font-family: monospace; font-size: 10px; color: var(--text-second);">{format!("{used_gb:.0} / {total_gb:.0} GB ({pct:.0}%)")}</span>
                                            </div>
                                            <div style="width: 100%; height: 5px; background: var(--line); border-radius: 2px; overflow: hidden;">
                                                <div style=format!("width: {}%; height: 100%; background: var(--ok); border-radius: 2px;", pct.min(100.0)) />
                                            </div>
                                        </div>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        </div>

                        // Network Interfaces
                        <div style="background: var(--fill-faint); border: 1px solid var(--fill-subtle); border-radius: 6px; padding: 10px 12px;">
                            <div style="font-weight: 600; font-size: 11px; margin-bottom: 8px;">"Network Interfaces"</div>
                            <div style="display: grid; grid-template-columns: repeat(auto-fill, minmax(160px, 1fr)); gap: 6px;">
                                {mon.network_interfaces.into_iter().map(|iface| {
                                    let rx_mb = iface.rx_bytes / (1024 * 1024);
                                    let tx_mb = iface.tx_bytes / (1024 * 1024);
                                    view! {
                                        <div style="background: var(--bg-sunken); padding: 6px 8px; border-radius: 4px; font-size: 10px;">
                                            <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 4px;">
                                                <span style="font-weight: 600; font-family: monospace; color: var(--text-bright);">{iface.name}</span>
                                                <span style=format!("font-size: 8px; padding: 1px 4px; border-radius: 3px; font-weight: 700; background: {}; color: {};", if iface.is_up { "var(--ok-fill-strong)" } else { "var(--danger-fill-strong)" }, if iface.is_up { "var(--ok)" } else { "var(--danger)" })>
                                                    {if iface.is_up { "UP" } else { "DOWN" }}
                                                </span>
                                            </div>
                                            <div style="display: flex; justify-content: space-between; color: var(--text-second); font-family: monospace;">
                                                <span>{format!("RX: {rx_mb} MB")}</span>
                                                <span>{format!("TX: {tx_mb} MB")}</span>
                                            </div>
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
