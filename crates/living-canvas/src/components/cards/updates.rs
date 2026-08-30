// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! System Software & Kernel Updates card component.

use crate::{CardId, MindClient, components::icons::IconRefresh, tool_state::ToolCardStates};
use leptos::prelude::*;

#[component]
pub fn UpdatesContent(card: CardId) -> impl IntoView {
    let client = crate::GatewayMindClient;
    let tool_states = expect_context::<ToolCardStates>();
    let signals = tool_states.updates(card);

    let load_updates = move || {
        signals.loading.set(true);
        leptos::task::spawn_local(async move {
            match client.get_system_updates().await {
                Ok(proj) => {
                    signals.updates.set(Some(proj));
                    signals.status_msg.set(None);
                }
                Err(err) => {
                    signals
                        .status_msg
                        .set(Some(format!("Failed to load updates: {err}")));
                }
            }
            signals.loading.set(false);
        });
    };

    let apply_all_updates = move || {
        signals.loading.set(true);
        leptos::task::spawn_local(async move {
            match client.apply_system_updates(None).await {
                Ok(outcome) => {
                    signals.status_msg.set(Some(outcome));
                    load_updates();
                }
                Err(err) => {
                    signals
                        .status_msg
                        .set(Some(format!("Update failed: {err}")));
                }
            }
            signals.loading.set(false);
        });
    };

    // Trigger initial load
    Effect::new(move |_| {
        load_updates();
    });

    view! {
        <div class="updates-panel" style="display: flex; flex-direction: column; height: 100%; width: 100%; background: var(--bg-card, #1e1e24); color: var(--text-main, #e0e0e0); font-family: system-ui, -apple-system, sans-serif; overflow-y: auto;">
            // Header
            <div style="display: flex; align-items: center; justify-content: space-between; padding: 10px 12px; background: rgba(0,0,0,0.2); border-bottom: 1px solid rgba(255,255,255,0.08);">
                <div style="display: flex; align-items: center; gap: 8px;">
                    <IconRefresh size=14 />
                    <span style="font-weight: 600; font-size: 13px;">"System & Kernel Updates"</span>
                </div>
                <button
                    style="background: rgba(255,255,255,0.06); border: none; border-radius: 4px; padding: 4px 6px; color: inherit; cursor: pointer;"
                    title="Check for updates"
                    on:click=move |_| load_updates()
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

            {move || signals.updates.get().map(|u| {
                let summary = u.summary;
                let has_updates = summary.pending_count > 0;
                let download_mb = summary.total_download_bytes / (1024 * 1024);

                view! {
                    <div style="padding: 12px; display: flex; flex-direction: column; gap: 14px;">
                        // Status Card
                        <div style="background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.06); border-radius: 6px; padding: 12px; display: flex; align-items: center; justify-content: space-between;">
                            <div>
                                <div style="display: flex; align-items: center; gap: 8px; margin-bottom: 4px;">
                                    <span style={if has_updates { "font-weight: 700; font-size: 14px; color: #fbbf24;" } else { "font-weight: 700; font-size: 14px; color: #4ade80;" }}>
                                        {if has_updates {
                                            format!("{} Updates Pending", summary.pending_count)
                                        } else {
                                            "System Up to Date".to_owned()
                                        }}
                                    </span>
                                    {if summary.security_updates_count > 0 {
                                        Some(view! {
                                            <span style="background: rgba(239,68,68,0.2); color: #f87171; font-size: 10px; font-weight: 700; padding: 1px 6px; border-radius: 10px;">
                                                {format!("{} Security", summary.security_updates_count)}
                                            </span>
                                        })
                                    } else {
                                        None
                                    }}
                                    {if summary.kernel_update {
                                        Some(view! {
                                            <span style="background: rgba(147, 51, 234, 0.2); color: #c084fc; font-size: 10px; font-weight: 700; padding: 1px 6px; border-radius: 10px;">
                                                "Linux Kernel"
                                            </span>
                                        })
                                    } else {
                                        None
                                    }}
                                </div>
                                <div style="font-size: 11px; color: rgba(255,255,255,0.6);">
                                    {if has_updates {
                                        format!("Download payload: ~{} MB • Reboot required: {}", download_mb, if summary.reboot_required { "Yes" } else { "No" })
                                    } else {
                                        "All packages and security components match current release baseline.".to_owned()
                                    }}
                                </div>
                            </div>

                            {if has_updates {
                                Some(view! {
                                    <button
                                        style="background: linear-gradient(135deg, #6366f1, #8b5cf6); border: none; border-radius: 6px; padding: 8px 16px; font-size: 12px; font-weight: 700; color: #fff; cursor: pointer; box-shadow: 0 2px 8px rgba(99,102,241,0.3);"
                                        on:click=move |_| apply_all_updates()
                                    >
                                        "Apply All Updates"
                                    </button>
                                })
                            } else {
                                None
                            }}
                        </div>

                        // Pending Packages List
                        {if has_updates {
                            Some(view! {
                                <div style="display: flex; flex-direction: column; gap: 6px;">
                                    <div style="font-weight: 600; font-size: 11px; color: rgba(255,255,255,0.5);">"Pending Updates List"</div>
                                    {summary.packages.into_iter().map(|pkg| {
                                        let size_mb = pkg.download_size_bytes.unwrap_or(0) / (1024 * 1024);
                                        view! {
                                            <div style="background: rgba(0,0,0,0.2); border-radius: 4px; padding: 8px 10px; display: flex; align-items: center; justify-content: space-between; font-size: 11px;">
                                                <div>
                                                    <div style="font-weight: 600; color: #f3f4f6; margin-bottom: 2px;">{pkg.name}</div>
                                                    <div style="font-size: 10px; color: rgba(255,255,255,0.4); font-family: monospace;">
                                                        {format!("v{} → v{} • {} MB • {}", pkg.installed_version.as_deref().unwrap_or("?"), pkg.candidate_version.as_deref().unwrap_or("latest"), size_mb, pkg.repository)}
                                                    </div>
                                                </div>
                                            </div>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                            })
                        } else {
                            None
                        }}
                    </div>
                }
            })}
        </div>
    }
}
