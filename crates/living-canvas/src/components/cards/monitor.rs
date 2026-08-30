// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! System Resource Monitor and Hardware Telemetry card component.

use crate::{
    CardId, MindClient,
    components::icons::{IconActivity, IconRefresh},
    tool_state::ToolCardStates,
};
use leptos::prelude::*;

#[component]
pub fn MonitorContent(card: CardId) -> impl IntoView {
    let client = crate::GatewayMindClient;
    let tool_states = expect_context::<ToolCardStates>();
    let signals = tool_states.monitor(card);

    let load_monitor = move || {
        signals.loading.set(true);
        leptos::task::spawn_local(async move {
            match client.get_system_monitor().await {
                Ok(proj) => {
                    signals.monitor.set(Some(proj));
                    signals.status_msg.set(None);
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

    view! {
        <div class="monitor-panel" style="display: flex; flex-direction: column; height: 100%; width: 100%; background: var(--bg-card, #1e1e24); color: var(--text-main, #e0e0e0); font-family: system-ui, -apple-system, sans-serif; overflow-y: auto;">
            // Header
            <div style="display: flex; align-items: center; justify-content: space-between; padding: 10px 12px; background: rgba(0,0,0,0.2); border-bottom: 1px solid rgba(255,255,255,0.08);">
                <div style="display: flex; align-items: center; gap: 8px;">
                    <IconActivity size=14 />
                    <span style="font-weight: 600; font-size: 13px;">"Hardware Telemetry & Monitor"</span>
                </div>
                <button
                    style="background: rgba(255,255,255,0.06); border: none; border-radius: 4px; padding: 4px 6px; color: inherit; cursor: pointer;"
                    title="Refresh metrics"
                    on:click=move |_| load_monitor()
                >
                    <IconRefresh size=13 />
                </button>
            </div>

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
                        <div style="background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.06); border-radius: 6px; padding: 10px 12px; display: grid; grid-template-columns: 1fr 1fr; gap: 8px; font-size: 11px;">
                            <div>
                                <div style="color: rgba(255,255,255,0.4); font-size: 10px;">"HOSTNAME"</div>
                                <div style="font-weight: 600; color: #f3f4f6;">{mon.hostname}</div>
                            </div>
                            <div>
                                <div style="color: rgba(255,255,255,0.4); font-size: 10px;">"OS RELEASE"</div>
                                <div style="color: rgba(255,255,255,0.8);">{mon.os_release}</div>
                            </div>
                            <div>
                                <div style="color: rgba(255,255,255,0.4); font-size: 10px;">"UPTIME"</div>
                                <div style="color: #38bdf8; font-weight: 600;">{format!("{uptime_hours} hours")}</div>
                            </div>
                            <div>
                                <div style="color: rgba(255,255,255,0.4); font-size: 10px;">"LOAD AVERAGE"</div>
                                <div style="font-family: monospace;">{format!("{:.2}, {:.2}, {:.2}", mon.load_avg[0], mon.load_avg[1], mon.load_avg[2])}</div>
                            </div>
                        </div>

                        // CPU Utilization
                        <div style="background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.06); border-radius: 6px; padding: 10px 12px;">
                            <div style="display: flex; justify-content: space-between; margin-bottom: 6px; font-size: 11px;">
                                <span style="font-weight: 600;">"CPU Utilization"</span>
                                <span style="font-weight: 700; color: #818cf8; font-family: monospace;">{format!("{:.1}%", mon.total_cpu_percent)}</span>
                            </div>
                            <div style="width: 100%; height: 6px; background: rgba(255,255,255,0.08); border-radius: 3px; overflow: hidden; margin-bottom: 10px;">
                                <div style=format!("width: {}%; height: 100%; background: linear-gradient(90deg, #6366f1, #a855f7); border-radius: 3px;", mon.total_cpu_percent.min(100.0)) />
                            </div>

                            // Cores grid
                            <div style="display: grid; grid-template-columns: repeat(auto-fill, minmax(110px, 1fr)); gap: 6px;">
                                {mon.cores.into_iter().map(|core| {
                                    view! {
                                        <div style="background: rgba(0,0,0,0.2); padding: 4px 6px; border-radius: 4px; font-size: 10px;">
                                            <div style="display: flex; justify-content: space-between; margin-bottom: 2px;">
                                                <span style="color: rgba(255,255,255,0.5);">{format!("Core {}", core.core_id)}</span>
                                                <span style="font-family: monospace;">{format!("{:.0}%", core.usage_percent)}</span>
                                            </div>
                                            <div style="width: 100%; height: 3px; background: rgba(255,255,255,0.08); border-radius: 2px; overflow: hidden;">
                                                <div style=format!("width: {}%; height: 100%; background: #818cf8;", core.usage_percent.min(100.0)) />
                                            </div>
                                        </div>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        </div>

                        // Memory & Swap
                        <div style="background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.06); border-radius: 6px; padding: 10px 12px; display: flex; flex-direction: column; gap: 10px;">
                            <div>
                                <div style="display: flex; justify-content: space-between; margin-bottom: 4px; font-size: 11px;">
                                    <span style="font-weight: 600;">"Physical RAM"</span>
                                    <span style="font-family: monospace; color: #38bdf8;">{format!("{:.1} / {:.1} GB ({:.0}%)", ram_used_gb, ram_total_gb, ram_pct)}</span>
                                </div>
                                <div style="width: 100%; height: 6px; background: rgba(255,255,255,0.08); border-radius: 3px; overflow: hidden;">
                                    <div style=format!("width: {}%; height: 100%; background: #38bdf8; border-radius: 3px;", ram_pct.min(100.0)) />
                                </div>
                            </div>

                            {if swap_total_gb > 0.0 {
                                Some(view! {
                                    <div>
                                        <div style="display: flex; justify-content: space-between; margin-bottom: 4px; font-size: 11px;">
                                            <span style="font-weight: 600;">"Swap Space"</span>
                                            <span style="font-family: monospace; color: rgba(255,255,255,0.6);">{format!("{:.1} / {:.1} GB ({:.0}%)", swap_used_gb, swap_total_gb, swap_pct)}</span>
                                        </div>
                                        <div style="width: 100%; height: 6px; background: rgba(255,255,255,0.08); border-radius: 3px; overflow: hidden;">
                                            <div style=format!("width: {}%; height: 100%; background: #94a3b8; border-radius: 3px;", swap_pct.min(100.0)) />
                                        </div>
                                    </div>
                                })
                            } else {
                                None
                            }}
                        </div>

                        // Disk Storage
                        <div style="background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.06); border-radius: 6px; padding: 10px 12px;">
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
                                                    <span style="font-weight: 600; color: #f3f4f6;">{disk.mount_point}</span>
                                                    <span style="font-size: 10px; color: rgba(255,255,255,0.4); margin-left: 6px;">{format!("({} / {})", disk.device, disk.fs_type)}</span>
                                                </div>
                                                <span style="font-family: monospace; font-size: 10px; color: rgba(255,255,255,0.8);">{format!("{:.0} / {:.0} GB ({:.0}%)", used_gb, total_gb, pct)}</span>
                                            </div>
                                            <div style="width: 100%; height: 5px; background: rgba(255,255,255,0.08); border-radius: 2px; overflow: hidden;">
                                                <div style=format!("width: {}%; height: 100%; background: #10b981; border-radius: 2px;", pct.min(100.0)) />
                                            </div>
                                        </div>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        </div>

                        // Network Interfaces
                        <div style="background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.06); border-radius: 6px; padding: 10px 12px;">
                            <div style="font-weight: 600; font-size: 11px; margin-bottom: 8px;">"Network Interfaces"</div>
                            <div style="display: grid; grid-template-columns: repeat(auto-fill, minmax(160px, 1fr)); gap: 6px;">
                                {mon.network_interfaces.into_iter().map(|iface| {
                                    let rx_mb = iface.rx_bytes / (1024 * 1024);
                                    let tx_mb = iface.tx_bytes / (1024 * 1024);
                                    view! {
                                        <div style="background: rgba(0,0,0,0.2); padding: 6px 8px; border-radius: 4px; font-size: 10px;">
                                            <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 4px;">
                                                <span style="font-weight: 600; font-family: monospace; color: #f3f4f6;">{iface.name}</span>
                                                <span style=format!("font-size: 8px; padding: 1px 4px; border-radius: 3px; font-weight: 700; background: {}; color: {};", if iface.is_up { "rgba(34,197,94,0.2)" } else { "rgba(239,68,68,0.2)" }, if iface.is_up { "#4ade80" } else { "#f87171" })>
                                                    {if iface.is_up { "UP" } else { "DOWN" }}
                                                </span>
                                            </div>
                                            <div style="display: flex; justify-content: space-between; color: rgba(255,255,255,0.6); font-family: monospace;">
                                                <span>{format!("RX: {} MB", rx_mb)}</span>
                                                <span>{format!("TX: {} MB", tx_mb)}</span>
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
