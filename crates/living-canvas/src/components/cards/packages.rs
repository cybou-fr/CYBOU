// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Software Package Manager card component with governed Action1 operations.

use cybou_protocol::SubjectRef;
use cybou_protocol::system::{PackageActionKind, PackageStatus};
use leptos::prelude::*;

use crate::{
    CardId, MindClient,
    components::icons::{IconLayers, IconRefresh},
    tool_state::ToolCardStates,
};

#[component]
pub fn PackagesContent(card: CardId) -> impl IntoView {
    let client = crate::GatewayMindClient;
    let tool_states = expect_context::<ToolCardStates>();
    let signals = tool_states.packages(card);

    let load_packages = move || {
        signals.loading.set(true);
        leptos::task::spawn_local(async move {
            match client.get_packages().await {
                Ok(proj) => {
                    signals.packages.set(proj.packages);
                    signals.status_msg.set(None);
                }
                Err(err) => {
                    signals
                        .status_msg
                        .set(Some(format!("Failed to load packages: {err}")));
                }
            }
            signals.loading.set(false);
        });
    };

    let trigger_action = move |name: String, action: PackageActionKind| {
        leptos::task::spawn_local(async move {
            match client.execute_package_action(&name, action).await {
                Ok(outcome) => {
                    signals.status_msg.set(Some(outcome));
                    load_packages();
                }
                Err(err) => {
                    signals
                        .status_msg
                        .set(Some(format!("Package action failed: {err}")));
                }
            }
        });
    };

    let inspect_package = move |name: String, version: Option<String>| {
        let inspector_signals = tool_states.inspector(CardId::Inspector(0));
        inspector_signals
            .target_subject
            .set(Some(SubjectRef::Package {
                name,
                installed_version: version,
            }));
    };

    // Trigger initial load
    Effect::new(move |_| {
        load_packages();
    });

    let filtered_packages = move || {
        let all = signals.packages.get();
        let tab = signals.active_tab.get();
        let search = signals.search_query.get().to_lowercase();

        all.into_iter()
            .filter(|p| match tab.as_str() {
                "installed" => {
                    p.status == PackageStatus::Installed || p.status == PackageStatus::Upgradable
                }
                "upgradable" => p.status == PackageStatus::Upgradable,
                _ => true,
            })
            .filter(|p| {
                if search.is_empty() {
                    true
                } else {
                    p.name.to_lowercase().contains(&search)
                        || p.description.to_lowercase().contains(&search)
                }
            })
            .collect::<Vec<_>>()
    };

    view! {
        <div class="packages-panel" style="display: flex; flex-direction: column; height: 100%; width: 100%; background: var(--bg-card); color: var(--text-main); font-family: system-ui, -apple-system, sans-serif;">
            // Header & Tabs
            <div style="display: flex; flex-direction: column; gap: 8px; padding: 10px 12px; background: var(--bg-sunken); border-bottom: 1px solid var(--line);">
                <div style="display: flex; align-items: center; justify-content: space-between;">
                    <span style="font-weight: 600; font-size: 13px;">"Software Packages"</span>
                    <button
                        style="background: var(--fill-subtle); border: none; border-radius: 4px; padding: 4px 6px; color: inherit; cursor: pointer;"
                        title="Refresh repositories"
                        on:click=move |_| load_packages()
                    >
                        <IconRefresh size=13 />
                    </button>
                </div>

                // Tabs & Search
                <div style="display: flex; align-items: center; justify-content: space-between; gap: 8px;">
                    <div style="display: flex; gap: 4px;">
                        <button
                            style=move || format!(
                                "background: {}; border: none; font-size: 11px; padding: 3px 8px; border-radius: 12px; color: inherit; cursor: pointer;",
                                if signals.active_tab.get() == "installed" { "var(--accent-line)" } else { "var(--fill-faintest)" }
                            )
                            on:click=move |_| signals.active_tab.set("installed".to_owned())
                        >
                            "Installed"
                        </button>
                        <button
                            style=move || format!(
                                "background: {}; border: none; font-size: 11px; padding: 3px 8px; border-radius: 12px; color: var(--caution); cursor: pointer;",
                                if signals.active_tab.get() == "upgradable" { "var(--caution-line)" } else { "var(--fill-faintest)" }
                            )
                            on:click=move |_| signals.active_tab.set("upgradable".to_owned())
                        >
                            "Upgradable"
                        </button>
                        <button
                            style=move || format!(
                                "background: {}; border: none; font-size: 11px; padding: 3px 8px; border-radius: 12px; color: inherit; cursor: pointer;",
                                if signals.active_tab.get() == "all" { "var(--accent-line)" } else { "var(--fill-faintest)" }
                            )
                            on:click=move |_| signals.active_tab.set("all".to_owned())
                        >
                            "All Repos"
                        </button>
                    </div>

                    <input
                        type="text"
                        placeholder="Search packages..."
                        prop:value=move || signals.search_query.get()
                        on:input=move |e| signals.search_query.set(event_target_value(&e))
                        style="background: var(--bg-sunken-strong); border: 1px solid var(--fill-hover); border-radius: 4px; padding: 3px 8px; font-size: 11px; color: inherit; width: 140px;"
                    />
                </div>
            </div>

            // Status message toast
            {move || signals.status_msg.get().map(|msg| {
                view! {
                    <div style="background: var(--accent-fill); color: var(--accent-text); font-size: 11px; padding: 6px 12px; border-bottom: 1px solid var(--accent-line); display: flex; justify-content: space-between;">
                        <span>{msg}</span>
                        <button style="background: none; border: none; color: inherit; cursor: pointer;" on:click=move |_| signals.status_msg.set(None)>"×"</button>
                    </div>
                }
            })}

            // Packages List
            <div style="flex: 1; overflow-y: auto; padding: 8px 12px; display: flex; flex-direction: column; gap: 6px;">
                <For
                    each=filtered_packages
                    key=|p| p.name.clone()
                    children=move |pkg| {
                        let name = pkg.name.clone();
                        let name_action = pkg.name.clone();
                        let name_inspect = pkg.name.clone();
                        let inst_ver = pkg.installed_version.clone();
                        let is_installed = pkg.status == PackageStatus::Installed || pkg.status == PackageStatus::Upgradable;
                        let is_upgradable = pkg.status == PackageStatus::Upgradable;

                        view! {
                            <div style="background: var(--fill-faint); border: 1px solid var(--fill-subtle); border-radius: 6px; padding: 8px 12px; display: flex; align-items: center; justify-content: space-between; gap: 12px;">
                                <div style="flex: 1; min-width: 0;">
                                    <div style="display: flex; align-items: center; gap: 8px; margin-bottom: 2px;">
                                        <span style="font-weight: 600; font-size: 12px; color: var(--text-bright); font-family: monospace;">{name}</span>
                                        <span style="background: var(--fill-subtle); font-size: 9px; padding: 1px 5px; border-radius: 3px; color: var(--text-second);">
                                            {pkg.repository}
                                        </span>
                                        {if let Some(ref v) = pkg.installed_version {
                                            view! { <span style="font-size: 10px; color: var(--ok); font-family: monospace;">{format!("v{v}")}</span> }.into_any()
                                        } else {
                                            view! { <span style="font-size: 10px; color: var(--text-faint); font-family: monospace;">"not installed"</span> }.into_any()
                                        }}
                                        {if is_upgradable {
                                            Some(view! {
                                                <span style="background: var(--caution-fill-strong); color: var(--caution); font-size: 9px; padding: 1px 5px; border-radius: 3px; font-weight: 700;">
                                                    {format!("upgrade to v{}", pkg.candidate_version.as_deref().unwrap_or("latest"))}
                                                </span>
                                            })
                                        } else {
                                            None
                                        }}
                                    </div>
                                    <div style="font-size: 11px; color: var(--text-second); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                                        {pkg.description}
                                    </div>
                                </div>

                                // Action Buttons
                                <div style="display: flex; align-items: center; gap: 6px; flex-shrink: 0;">
                                    {if is_upgradable {
                                        view! {
                                            <button
                                                style="background: var(--caution-fill-strong); border: 1px solid rgba(245, 158, 11, 0.4); border-radius: 4px; padding: 3px 8px; font-size: 10px; color: var(--caution); font-weight: 600; cursor: pointer;"
                                                on:click=move |_| trigger_action(name_action.clone(), PackageActionKind::Upgrade)
                                            >
                                                "Upgrade"
                                            </button>
                                        }.into_any()
                                    } else if is_installed {
                                        view! {
                                            <button
                                                style="background: var(--danger-fill); border: 1px solid var(--danger-line); border-radius: 4px; padding: 3px 8px; font-size: 10px; color: var(--danger); cursor: pointer;"
                                                on:click=move |_| trigger_action(name_action.clone(), PackageActionKind::Remove)
                                            >
                                                "Remove"
                                            </button>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <button
                                                style="background: var(--ok-fill-strong); border: 1px solid rgba(34, 197, 94, 0.4); border-radius: 4px; padding: 3px 8px; font-size: 10px; color: var(--ok); font-weight: 600; cursor: pointer;"
                                                on:click=move |_| trigger_action(name_action.clone(), PackageActionKind::Install)
                                            >
                                                "Install"
                                            </button>
                                        }.into_any()
                                    }}

                                    <button
                                        style="background: var(--accent-fill); border: 1px solid var(--accent-line); border-radius: 4px; padding: 3px 6px; font-size: 10px; color: var(--accent-light); cursor: pointer;"
                                        title="Inspect in Universal Inspector"
                                        on:click=move |_| inspect_package(name_inspect.clone(), inst_ver.clone())
                                    >
                                        <IconLayers size=11 />
                                    </button>
                                </div>
                            </div>
                        }
                    }
                />
            </div>
        </div>
    }
}
