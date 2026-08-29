// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Deep Cognitive Graph & Causal DAG card component.

use leptos::prelude::*;
use crate::{
    MindClient,
    CardId,
    components::icons::{IconLayers, IconRefresh},
    tool_state::ToolCardStates,
};

#[component]
pub fn CognitiveGraphContent(card: CardId) -> impl IntoView {
    let client = crate::GatewayMindClient;
    let tool_states = expect_context::<ToolCardStates>();
    let signals = tool_states.cognitive_graph(card);

    let load_graph = move || {
        signals.loading.set(true);
        leptos::task::spawn_local(async move {
            let query_text = signals.search_query.get();
            let res = if query_text.trim().is_empty() {
                client.get_cognitive_graph(None).await
            } else {
                client.query_cognitive_graph(cybou_web_contracts::CognitiveQueryRequest {
                    query: query_text,
                    node_types: None,
                    focus_id: signals.selected_node_id.get(),
                    max_depth: Some(2),
                }).await
            };

            match res {
                Ok(proj) => {
                    signals.graph.set(Some(proj));
                    signals.status_msg.set(None);
                }
                Err(err) => {
                    signals.status_msg.set(Some(format!("Failed to load cognitive graph: {err}")));
                }
            }
            signals.loading.set(false);
        });
    };

    // Trigger initial load
    Effect::new(move |_| {
        load_graph();
    });

    view! {
        <div class="cognitive-graph-panel" style="display: flex; flex-direction: column; height: 100%; width: 100%; background: var(--bg-card, #1e1e24); color: var(--text-main, #e0e0e0); font-family: system-ui, -apple-system, sans-serif; overflow: hidden;">
            // Header
            <div style="display: flex; align-items: center; justify-content: space-between; padding: 10px 12px; background: rgba(0,0,0,0.2); border-bottom: 1px solid rgba(255,255,255,0.08);">
                <div style="display: flex; align-items: center; gap: 8px;">
                    <IconLayers size=16 />
                    <span style="font-weight: 600; font-size: 13px;">"Cognitive Graph & Causal DAG"</span>
                </div>
                <div style="display: flex; align-items: center; gap: 6px;">
                    <input
                        type="text"
                        placeholder="Search entities & relations..."
                        prop:value=move || signals.search_query.get()
                        on:input=move |e| {
                            signals.search_query.set(event_target_value(&e));
                            load_graph();
                        }
                        style="background: rgba(0,0,0,0.3); border: 1px solid rgba(255,255,255,0.1); border-radius: 4px; padding: 4px 8px; font-size: 11px; color: inherit; width: 160px;"
                    />
                    <button
                        style="background: rgba(255,255,255,0.06); border: none; border-radius: 4px; padding: 4px 6px; color: inherit; cursor: pointer;"
                        title="Refresh graph"
                        on:click=move |_| load_graph()
                    >
                        <IconRefresh size=13 />
                    </button>
                </div>
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

            // Main 2-column view: Nodes & Causal DAG list + Inspector
            <div style="display: flex; flex: 1; overflow: hidden;">
                // Graph Nodes & Edges Grid
                <div style="flex: 1; overflow-y: auto; padding: 12px; display: flex; flex-direction: column; gap: 12px;">
                    <div style="font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.5px; color: rgba(255,255,255,0.5);">
                        "Entities & Subsystems (" {move || signals.graph.get().map(|g| g.graph.nodes.len()).unwrap_or(0)} ")"
                    </div>
                    <div style="display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 8px;">
                        {move || signals.graph.get().map(|proj| {
                            proj.graph.nodes.into_iter().map(|node| {
                                let node_id = node.id.clone();
                                let node_clone = node.clone();
                                let is_selected = move || signals.selected_node_id.get().as_ref() == Some(&node_id);
                                let (badge_bg, badge_fg) = match node.node_type.category_name() {
                                    "Agent" => ("rgba(99, 102, 241, 0.2)", "#818cf8"),
                                    "Service" => ("rgba(16, 185, 129, 0.2)", "#34d399"),
                                    "Security" => ("rgba(239, 68, 68, 0.2)", "#f87171"),
                                    "Finding" => ("rgba(245, 158, 11, 0.2)", "#fbbf24"),
                                    "Mail" | "Note" | "Contact" => ("rgba(236, 72, 153, 0.2)", "#f472b6"),
                                    _ => ("rgba(255, 255, 255, 0.1)", "#e5e7eb"),
                                };

                                view! {
                                    <div
                                        style=move || format!("background: rgba(0,0,0,0.2); border: 1px solid {}; border-radius: 6px; padding: 10px; cursor: pointer; display: flex; flex-direction: column; gap: 6px; transition: border-color 0.15s ease;", if is_selected() { "var(--accent, #6366f1)" } else { "rgba(255,255,255,0.08)" })
                                        on:click=move |_| signals.selected_node_id.set(Some(node_clone.id.clone()))
                                    >
                                        <div style="display: flex; align-items: center; justify-content: space-between;">
                                            <span style=format!("font-size: 9px; font-weight: 700; padding: 2px 6px; border-radius: 3px; background: {}; color: {}; text-transform: uppercase;", badge_bg, badge_fg)>
                                                {node.node_type.category_name()}
                                            </span>
                                            <span style="font-size: 9px; color: rgba(255,255,255,0.4); font-family: monospace;">
                                                {format!("{:.0}%", node.confidence * 100.0)}
                                            </span>
                                        </div>
                                        <div style="font-weight: 600; font-size: 12px; color: #f3f4f6;">
                                            {node.label}
                                        </div>
                                        <div style="font-size: 9px; color: rgba(255,255,255,0.4); font-family: monospace; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                                            {node.id}
                                        </div>
                                    </div>
                                }
                            }).collect::<Vec<_>>()
                        })}
                    </div>

                    // Causal Relations / Edges Section
                    <div style="font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.5px; color: rgba(255,255,255,0.5); margin-top: 10px;">
                        "Causal Relations & Governance Edges (" {move || signals.graph.get().map(|g| g.graph.edges.len()).unwrap_or(0)} ")"
                    </div>
                    <div style="display: flex; flex-direction: column; gap: 6px;">
                        {move || signals.graph.get().map(|proj| {
                            proj.graph.edges.into_iter().map(|edge| {
                                view! {
                                    <div style="background: rgba(0,0,0,0.15); border: 1px solid rgba(255,255,255,0.06); border-radius: 4px; padding: 8px 10px; display: flex; align-items: center; justify-content: space-between; font-size: 11px;">
                                        <div style="display: flex; align-items: center; gap: 6px;">
                                            <span style="font-family: monospace; color: #93c5fd;">{edge.source_id}</span>
                                            <span style="font-weight: 700; color: #c084fc; font-size: 10px; padding: 1px 5px; border-radius: 3px; background: rgba(192, 132, 252, 0.1);">
                                                {edge.edge_type.label()}
                                            </span>
                                            <span style="font-family: monospace; color: #a7f3d0;">{edge.target_id}</span>
                                        </div>
                                        <div style="font-size: 10px; color: rgba(255,255,255,0.5);">
                                            {edge.description}
                                        </div>
                                    </div>
                                }
                            }).collect::<Vec<_>>()
                        })}
                    </div>
                </div>

                // Selected Node Inspector Sidebar
                {move || signals.selected_node_id.get().and_then(|sel_id| {
                    signals.graph.get().and_then(|g| {
                        g.graph.nodes.into_iter().find(|n| n.id == sel_id).map(|node| {
                            view! {
                                <div style="width: 260px; border-left: 1px solid rgba(255,255,255,0.08); padding: 12px; background: rgba(0,0,0,0.15); overflow-y: auto; display: flex; flex-direction: column; gap: 10px;">
                                    <div style="display: flex; justify-content: space-between; align-items: flex-start;">
                                        <div>
                                            <div style="font-weight: 700; font-size: 13px; color: #f3f4f6;">{node.label}</div>
                                            <div style="font-size: 10px; color: #818cf8; font-family: monospace; margin-top: 2px;">{node.id}</div>
                                        </div>
                                        <button
                                            style="background: none; border: none; color: rgba(255,255,255,0.5); cursor: pointer;"
                                            on:click=move |_| signals.selected_node_id.set(None)
                                        >
                                            "✕"
                                        </button>
                                    </div>
                                    <div style="font-size: 11px; display: flex; flex-direction: column; gap: 4px;">
                                        <div style="color: rgba(255,255,255,0.6);">"Category: " <b style="color: #f3f4f6;">{node.node_type.category_name()}</b></div>
                                        <div style="color: rgba(255,255,255,0.6);">"Confidence: " <b style="color: #f3f4f6;">{format!("{:.1}%", node.confidence * 100.0)}</b></div>
                                        <div style="color: rgba(255,255,255,0.6);">"Standing: " <b style="color: #34d399;">"Observed"</b></div>
                                    </div>
                                    {if !node.metadata.is_empty() {
                                        view! {
                                            <div style="border-top: 1px solid rgba(255,255,255,0.06); padding-top: 8px;">
                                                <div style="font-size: 10px; font-weight: 700; color: rgba(255,255,255,0.5); text-transform: uppercase; margin-bottom: 4px;">"Attributes"</div>
                                                {node.metadata.into_iter().map(|(k, v)| view! {
                                                    <div style="font-size: 10px; font-family: monospace; display: flex; justify-content: space-between; margin-bottom: 2px;">
                                                        <span style="color: rgba(255,255,255,0.5);">{k}:</span>
                                                        <span style="color: #e5e7eb;">{v}</span>
                                                    </div>
                                                }).collect::<Vec<_>>()}
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! { <div></div> }.into_any()
                                    }}
                                </div>
                            }
                        })
                    })
                })}
            </div>
        </div>
    }
}
