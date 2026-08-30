// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Network Connections, Wi-Fi, and VPN tunnels card component.

use cybou_protocol::system::NetworkConnectionKind;
use leptos::prelude::*;

use crate::{
    CardId, MindClient,
    components::icons::{IconActivity, IconRefresh},
    tool_state::ToolCardStates,
};

#[component]
pub fn NetworkContent(card: CardId) -> impl IntoView {
    let client = crate::GatewayMindClient;
    let tool_states = expect_context::<ToolCardStates>();
    let signals = tool_states.network(card);

    let load_network = move || {
        signals.loading.set(true);
        leptos::task::spawn_local(async move {
            match client.get_network().await {
                Ok(proj) => {
                    signals.connections.set(proj.connections);
                    signals.status_msg.set(None);
                }
                Err(err) => {
                    signals
                        .status_msg
                        .set(Some(format!("Failed to load network: {err}")));
                }
            }
            signals.loading.set(false);
        });
    };

    let toggle_connect = move |conn_id: String, activate: bool| {
        leptos::task::spawn_local(async move {
            match client.connect_network(&conn_id, activate).await {
                Ok(outcome) => {
                    signals.status_msg.set(Some(outcome));
                    load_network();
                }
                Err(err) => {
                    signals
                        .status_msg
                        .set(Some(format!("Network update failed: {err}")));
                }
            }
        });
    };

    // Trigger initial load
    Effect::new(move |_| {
        load_network();
    });

    view! {
        <div class="network-panel" style="display: flex; flex-direction: column; height: 100%; width: 100%; background: var(--bg-card); color: var(--text-main); font-family: system-ui, -apple-system, sans-serif; overflow-y: auto;">
            // Header
            <div style="display: flex; align-items: center; justify-content: space-between; padding: 10px 12px; background: var(--bg-sunken); border-bottom: 1px solid var(--line);">
                <div style="display: flex; align-items: center; gap: 8px;">
                    <IconActivity size=14 />
                    <span style="font-weight: 600; font-size: 13px;">"Network & VPN Connections"</span>
                </div>
                <button
                    style="background: var(--fill-subtle); border: none; border-radius: 4px; padding: 4px 6px; color: inherit; cursor: pointer;"
                    title="Refresh network"
                    on:click=move |_| load_network()
                >
                    <IconRefresh size=13 />
                </button>
            </div>

            // Status message toast
            {move || signals.status_msg.get().map(|msg| {
                view! {
                    <div class="card-status-line" role="status" aria-live="polite">
                        <span>{msg}</span>
                        <button class="card-status-dismiss" title="Dismiss" on:click=move |_| signals.status_msg.set(None)>"×"</button>
                    </div>
                }
            })}

            // Connections List
            <div style="padding: 10px 12px; display: flex; flex-direction: column; gap: 8px;">
                <For
                    each=move || signals.connections.get()
                    key=|c| c.id.clone()
                    children=move |conn| {
                        let _conn_id = conn.id.clone();
                        let conn_id_btn = conn.id.clone();
                        let is_active = conn.is_active;
                        let rx_mb = conn.rx_bytes / (1024 * 1024);
                        let tx_mb = conn.tx_bytes / (1024 * 1024);

                        let kind_label = match conn.kind {
                            NetworkConnectionKind::Ethernet => "Ethernet",
                            NetworkConnectionKind::Wifi => "Wi-Fi",
                            NetworkConnectionKind::Tailscale => "Tailscale Mesh",
                            NetworkConnectionKind::Wireguard => "WireGuard VPN",
                            NetworkConnectionKind::Loopback => "Loopback",
                        };

                        view! {
                            <div style="background: var(--fill-faint); border: 1px solid var(--fill-subtle); border-radius: 6px; padding: 10px 12px; display: flex; flex-direction: column; gap: 8px;">
                                <div style="display: flex; align-items: center; justify-content: space-between;">
                                    <div style="display: flex; align-items: center; gap: 8px;">
                                        <span style="font-weight: 600; font-size: 12px; color: var(--text-bright); font-family: monospace;">{conn.name}</span>
                                        <span style="background: var(--fill-subtle); font-size: 9px; padding: 1px 5px; border-radius: 3px; color: var(--text-second);">
                                            {kind_label}
                                        </span>
                                        <span style=format!("font-size: 9px; font-weight: 700; padding: 1px 5px; border-radius: 3px; background: {}; color: {};", if is_active { "var(--ok-fill-strong)" } else { "rgba(156,163,175,0.2)" }, if is_active { "var(--ok)" } else { "#9ca3af" })>
                                            {if is_active { "CONNECTED" } else { "DISCONNECTED" }}
                                        </span>
                                    </div>

                                    <button
                                        style=format!(
                                            "border: none; border-radius: 4px; padding: 3px 8px; font-size: 10px; font-weight: 600; cursor: pointer; background: {}; color: {};",
                                            if is_active { "var(--danger-fill-strong)" } else { "var(--ok-fill-strong)" },
                                            if is_active { "var(--danger)" } else { "var(--ok)" }
                                        )
                                        on:click=move |_| toggle_connect(conn_id_btn.clone(), !is_active)
                                    >
                                        {if is_active { "Disconnect" } else { "Connect" }}
                                    </button>
                                </div>

                                // Connection IP Details
                                <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 6px; font-size: 10px; font-family: monospace; color: var(--text-strong); background: var(--bg-sunken); padding: 6px 8px; border-radius: 4px;">
                                    <div>
                                        <span style="color: var(--text-faint);">"IP: "</span>
                                        <span>{conn.ip_address.unwrap_or_else(|| "—".to_owned())}</span>
                                    </div>
                                    <div>
                                        <span style="color: var(--text-faint);">"Gateway: "</span>
                                        <span>{conn.gateway.unwrap_or_else(|| "—".to_owned())}</span>
                                    </div>
                                    <div>
                                        <span style="color: var(--text-faint);">"DNS: "</span>
                                        <span>{if conn.dns.is_empty() { "—".to_owned() } else { conn.dns.join(", ") }}</span>
                                    </div>
                                    <div>
                                        <span style="color: var(--text-faint);">"Traffic: "</span>
                                        <span>{format!("↓ {} MB  ↑ {} MB", rx_mb, tx_mb)}</span>
                                    </div>
                                </div>
                            </div>
                        }
                    }
                />
            </div>
        </div>
    }
}
