// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Security Sandboxing Policy & Audit Log card component.

use crate::{CardId, MindClient, components::icons::IconRefresh, tool_state::ToolCardStates};
use leptos::prelude::*;

#[component]
pub fn SecurityContent(card: CardId) -> impl IntoView {
    let client = crate::GatewayMindClient;
    let tool_states = expect_context::<ToolCardStates>();
    let signals = tool_states.security(card);

    let load_security = move || {
        signals.loading.set(true);
        leptos::task::spawn_local(async move {
            match client.get_security_settings().await {
                Ok(proj) => {
                    signals.policy.set(Some(proj.policy));
                    signals.audit_log.set(proj.audit_log);
                    signals.status_msg.set(None);
                }
                Err(err) => {
                    signals
                        .status_msg
                        .set(Some(format!("Failed to load security: {err}")));
                }
            }
            signals.loading.set(false);
        });
    };

    let toggle_policy =
        move |update_fn: Box<dyn FnOnce(&mut cybou_protocol::system::SecurityPolicyRecord)>| {
            let mut cur =
                signals
                    .policy
                    .get()
                    .unwrap_or(cybou_protocol::system::SecurityPolicyRecord {
                        landlock_enabled: true,
                        bubblewrap_enabled: true,
                        apparmor_enforcing: true,
                        seccomp_strict: true,
                        egress_firewall_strict: true,
                    });
            update_fn(&mut cur);
            let req = cybou_web_contracts::UpdateSecurityPolicyRequest {
                landlock_enabled: cur.landlock_enabled,
                bubblewrap_enabled: cur.bubblewrap_enabled,
                apparmor_enforcing: cur.apparmor_enforcing,
                seccomp_strict: cur.seccomp_strict,
                egress_firewall_strict: cur.egress_firewall_strict,
            };
            leptos::task::spawn_local(async move {
                match client.update_security_policy(req).await {
                    Ok(pol) => {
                        signals.policy.set(Some(pol));
                        signals
                            .status_msg
                            .set(Some("Security policy successfully updated".to_owned()));
                        load_security();
                    }
                    Err(err) => {
                        signals
                            .status_msg
                            .set(Some(format!("Policy update failed: {err}")));
                    }
                }
            });
        };

    // Trigger initial load
    Effect::new(move |_| {
        load_security();
    });

    view! {
        <div class="security-panel" style="display: flex; flex-direction: column; height: 100%; width: 100%; overflow-y: auto;">
            // Header
            <div style="display: flex; align-items: center; justify-content: space-between; padding: 10px 12px; background: var(--bg-sunken); border-bottom: 1px solid var(--line);">
                <div style="display: flex; align-items: center; gap: 8px;">
                    <span style="font-weight: 600; font-size: 13px;">"Security & Sandboxing Policy"</span>
                </div>
                <button
                    style="background: var(--fill-subtle); border: none; border-radius: 4px; padding: 4px 6px; color: inherit; cursor: pointer;"
                    title="Refresh security status"
                    on:click=move |_| load_security()
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

            <div style="padding: 12px; display: flex; flex-direction: column; gap: 14px;">
                // Policy Rules
                {move || signals.policy.get().map(|pol| {
                    let pol_l = pol.clone();
                    let pol_b = pol.clone();
                    let pol_a = pol.clone();
                    let pol_s = pol.clone();
                    let pol_e = pol.clone();

                    view! {
                        <div style="background: var(--fill-faint); border: 1px solid var(--fill-subtle); border-radius: 6px; padding: 10px 12px; display: flex; flex-direction: column; gap: 8px;">
                            <div style="font-weight: 600; font-size: 11px; margin-bottom: 2px;">"Kernel Confinement Subsystems"</div>

                            // Landlock
                            <div style="display: flex; align-items: center; justify-content: space-between; background: var(--bg-sunken); padding: 8px 10px; border-radius: 4px; font-size: 11px;">
                                <div>
                                    <div style="font-weight: 600; color: var(--text-bright);">"Landlock LSM"</div>
                                    <div style="font-size: 10px; color: var(--text-dim);">"Unprivileged filesystem access restriction"</div>
                                </div>
                                <button
                                    style=format!("border: none; border-radius: 4px; padding: 3px 8px; font-size: 10px; font-weight: 700; cursor: pointer; background: {}; color: {};", if pol_l.landlock_enabled { "var(--ok-fill-strong)" } else { "var(--danger-fill-strong)" }, if pol_l.landlock_enabled { "var(--ok)" } else { "var(--danger)" })
                                    on:click=move |_| toggle_policy(Box::new(|p| p.landlock_enabled = !p.landlock_enabled))
                                >
                                    {if pol_l.landlock_enabled { "ENFORCED" } else { "DISABLED" }}
                                </button>
                            </div>

                            // Bubblewrap
                            <div style="display: flex; align-items: center; justify-content: space-between; background: var(--bg-sunken); padding: 8px 10px; border-radius: 4px; font-size: 11px;">
                                <div>
                                    <div style="font-weight: 600; color: var(--text-bright);">"Bubblewrap Namespaces"</div>
                                    <div style="font-size: 10px; color: var(--text-dim);">"Isolated user/mount/network namespace sandboxes"</div>
                                </div>
                                <button
                                    style=format!("border: none; border-radius: 4px; padding: 3px 8px; font-size: 10px; font-weight: 700; cursor: pointer; background: {}; color: {};", if pol_b.bubblewrap_enabled { "var(--ok-fill-strong)" } else { "var(--danger-fill-strong)" }, if pol_b.bubblewrap_enabled { "var(--ok)" } else { "var(--danger)" })
                                    on:click=move |_| toggle_policy(Box::new(|p| p.bubblewrap_enabled = !p.bubblewrap_enabled))
                                >
                                    {if pol_b.bubblewrap_enabled { "ENFORCED" } else { "DISABLED" }}
                                </button>
                            </div>

                            // AppArmor
                            <div style="display: flex; align-items: center; justify-content: space-between; background: var(--bg-sunken); padding: 8px 10px; border-radius: 4px; font-size: 11px;">
                                <div>
                                    <div style="font-weight: 600; color: var(--text-bright);">"AppArmor LSM"</div>
                                    <div style="font-size: 10px; color: var(--text-dim);">"Mandatory Access Control profile enforcement"</div>
                                </div>
                                <button
                                    style=format!("border: none; border-radius: 4px; padding: 3px 8px; font-size: 10px; font-weight: 700; cursor: pointer; background: {}; color: {};", if pol_a.apparmor_enforcing { "var(--ok-fill-strong)" } else { "var(--danger-fill-strong)" }, if pol_a.apparmor_enforcing { "var(--ok)" } else { "var(--danger)" })
                                    on:click=move |_| toggle_policy(Box::new(|p| p.apparmor_enforcing = !p.apparmor_enforcing))
                                >
                                    {if pol_a.apparmor_enforcing { "ENFORCED" } else { "DISABLED" }}
                                </button>
                            </div>

                            // Seccomp
                            <div style="display: flex; align-items: center; justify-content: space-between; background: var(--bg-sunken); padding: 8px 10px; border-radius: 4px; font-size: 11px;">
                                <div>
                                    <div style="font-weight: 600; color: var(--text-bright);">"Strict Seccomp-BPF"</div>
                                    <div style="font-size: 10px; color: var(--text-dim);">"Kernel syscall attack surface minimization"</div>
                                </div>
                                <button
                                    style=format!("border: none; border-radius: 4px; padding: 3px 8px; font-size: 10px; font-weight: 700; cursor: pointer; background: {}; color: {};", if pol_s.seccomp_strict { "var(--ok-fill-strong)" } else { "var(--danger-fill-strong)" }, if pol_s.seccomp_strict { "var(--ok)" } else { "var(--danger)" })
                                    on:click=move |_| toggle_policy(Box::new(|p| p.seccomp_strict = !p.seccomp_strict))
                                >
                                    {if pol_s.seccomp_strict { "STRICT" } else { "PERMISSIVE" }}
                                </button>
                            </div>

                            // Egress Firewall
                            <div style="display: flex; align-items: center; justify-content: space-between; background: var(--bg-sunken); padding: 8px 10px; border-radius: 4px; font-size: 11px;">
                                <div>
                                    <div style="font-weight: 600; color: var(--text-bright);">"Governed Egress Firewall"</div>
                                    <div style="font-size: 10px; color: var(--text-dim);">"Block all unauthorized outbound network requests"</div>
                                </div>
                                <button
                                    style=format!("border: none; border-radius: 4px; padding: 3px 8px; font-size: 10px; font-weight: 700; cursor: pointer; background: {}; color: {};", if pol_e.egress_firewall_strict { "var(--ok-fill-strong)" } else { "var(--danger-fill-strong)" }, if pol_e.egress_firewall_strict { "var(--ok)" } else { "var(--danger)" })
                                    on:click=move |_| toggle_policy(Box::new(|p| p.egress_firewall_strict = !p.egress_firewall_strict))
                                >
                                    {if pol_e.egress_firewall_strict { "STRICT" } else { "PERMISSIVE" }}
                                </button>
                            </div>
                        </div>
                    }
                })}

                // Security Audit Log Events
                <div style="background: var(--fill-faint); border: 1px solid var(--fill-subtle); border-radius: 6px; padding: 10px 12px;">
                    <div style="font-weight: 600; font-size: 11px; margin-bottom: 8px;">
                        {move || format!("Security Audit Feed ({})", signals.audit_log.get().len())}
                    </div>
                    <div style="display: flex; flex-direction: column; gap: 6px;">
                        {move || signals.audit_log.get().into_iter().map(|entry| {
                            let (bg_badge, fg_badge) = match entry.severity.as_str() {
                                "warning" => ("var(--caution-fill-strong)", "var(--caution)"),
                                "critical" => ("var(--danger-fill-strong)", "var(--danger)"),
                                _ => ("var(--accent-fill-strong)", "var(--accent-light)"),
                            };

                            view! {
                                <div style="background: var(--bg-sunken); border-radius: 4px; padding: 8px 10px; font-size: 11px; display: flex; flex-direction: column; gap: 2px;">
                                    <div style="display: flex; align-items: center; justify-content: space-between;">
                                        <div style="display: flex; align-items: center; gap: 6px;">
                                            <span style=format!("background: {}; color: {}; font-size: 9px; font-weight: 700; padding: 1px 5px; border-radius: 3px;", bg_badge, fg_badge)>
                                                {entry.category.to_uppercase()}
                                            </span>
                                            <span style="font-weight: 600; color: var(--text-bright);">{entry.message}</span>
                                        </div>
                                        <span style="font-size: 10px; color: var(--text-faint); font-family: monospace;">{entry.timestamp}</span>
                                    </div>
                                </div>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                </div>
            </div>
        </div>
    }
}
