// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Lifelong Learning, Skill Induction & Governance card component (ADR-0032, ADR-0033 & Milestone 10).
//!
//! Provides inspectable candidate extraction, multi-layered skill induction,
//! deterministic promotion evaluation, artifact lineage, and task-scoped capability governance.

use leptos::prelude::*;
use crate::{
    MindClient,
    CardId,
    components::icons::{IconCheckCircle, IconLayers, IconPlus, IconRefresh},
    tool_state::ToolCardStates,
};
use cybou_protocol::learning::LearningLayer;

#[component]
pub fn LearningContent(card: CardId) -> impl IntoView {
    let client = crate::GatewayMindClient;
    let tool_states = expect_context::<ToolCardStates>();
    let signals = tool_states.learning(card);

    let active_tab = RwSignal::new("candidates".to_string());

    let load_all = move || {
        signals.loading.set(true);
        leptos::task::spawn_local(async move {
            let filter = signals.layer_filter.get();
            if let Ok(cands) = client.get_learning_candidates(filter).await {
                signals.candidates.set(cands.candidates);
            }
            if let Ok(arts) = client.get_learned_artifacts().await {
                signals.artifacts.set(arts.artifacts);
            }
            if let Ok(sc) = client.get_governance_scopes().await {
                signals.scopes.set(sc.scopes);
            }
            signals.loading.set(false);
        });
    };

    let evaluate_candidate = move |candidate_id: uuid::Uuid| {
        signals.loading.set(true);
        leptos::task::spawn_local(async move {
            match client.evaluate_learning_candidate(candidate_id, None).await {
                Ok(eval_proj) => {
                    signals.evaluation.set(Some(eval_proj));
                    signals.status_msg.set(Some("Candidate evaluated against promotion gate.".to_string()));
                    load_all();
                }
                Err(err) => {
                    signals.status_msg.set(Some(format!("Evaluation failed: {err}")));
                }
            }
            signals.loading.set(false);
        });
    };

    let revoke_artifact = move |artifact_id: uuid::Uuid| {
        signals.loading.set(true);
        leptos::task::spawn_local(async move {
            match client.revoke_learned_artifact(artifact_id, "User governance veto").await {
                Ok(()) => {
                    signals.status_msg.set(Some("Learned artifact revoked.".to_string()));
                    load_all();
                }
                Err(err) => {
                    signals.status_msg.set(Some(format!("Revocation failed: {err}")));
                }
            }
            signals.loading.set(false);
        });
    };

    let submit_proposal = move || {
        let gen_text = signals.new_generalization.get();
        let scope = signals.new_scope.get();
        if gen_text.trim().is_empty() || scope.trim().is_empty() {
            return;
        }

        let layer = match signals.new_layer.get().as_str() {
            "behavioral" => cybou_protocol::learning::LearningLayer::Behavioral,
            "epistemic" => cybou_protocol::learning::LearningLayer::Epistemic,
            "neural" => cybou_protocol::learning::LearningLayer::Neural,
            _ => cybou_protocol::learning::LearningLayer::Procedural,
        };

        signals.loading.set(true);
        leptos::task::spawn_local(async move {
            let req = cybou_web_contracts::ProposeLearningCandidateRequest {
                layer,
                generalization: gen_text,
                scope,
                source_evidence: vec![uuid::Uuid::new_v4()],
                outcome_evidence: vec![uuid::Uuid::new_v4()],
            };
            match client.propose_learning_candidate(&req).await {
                Ok(_) => {
                    signals.new_generalization.set(String::new());
                    signals.new_scope.set(String::new());
                    signals.is_proposing.set(false);
                    signals.status_msg.set(Some("New learning candidate proposed.".to_string()));
                    load_all();
                }
                Err(err) => {
                    signals.status_msg.set(Some(format!("Proposal failed: {err}")));
                }
            }
            signals.loading.set(false);
        });
    };

    // Initial load
    Effect::new(move |_| {
        load_all();
    });

    view! {
        <div class="learning-panel" style="display: flex; flex-direction: column; height: 100%; width: 100%; background: var(--bg-card, #1e1e24); color: var(--text-main, #e0e0e0); font-family: system-ui, -apple-system, sans-serif; overflow: hidden;">
            // Header
            <div style="display: flex; align-items: center; justify-content: space-between; padding: 10px 12px; background: rgba(0,0,0,0.2); border-bottom: 1px solid rgba(255,255,255,0.08);">
                <div style="display: flex; align-items: center; gap: 8px;">
                    <IconLayers size=16 />
                    <span style="font-weight: 600; font-size: 13px;">"Lifelong Learning & Governance"</span>
                </div>
                <div style="display: flex; align-items: center; gap: 6px;">
                    <button
                        style="background: rgba(99, 102, 241, 0.2); color: #c7d2fe; border: 1px solid rgba(99, 102, 241, 0.4); border-radius: 4px; padding: 3px 8px; font-size: 11px; font-weight: 600; cursor: pointer; display: flex; align-items: center; gap: 4px;"
                        on:click=move |_| signals.is_proposing.update(|v| *v = !*v)
                    >
                        <IconPlus size=12 />
                        "Propose"
                    </button>
                    <button
                        style="background: rgba(255,255,255,0.06); border: none; border-radius: 4px; padding: 4px 6px; color: inherit; cursor: pointer;"
                        title="Refresh"
                        on:click=move |_| load_all()
                    >
                        <IconRefresh size=13 />
                    </button>
                </div>
            </div>

            // Navigation Tabs (Candidates / Promoted Artifacts / Governance Scopes)
            <div style="display: flex; background: rgba(0,0,0,0.15); border-bottom: 1px solid rgba(255,255,255,0.06); padding: 0 12px;">
                <button
                    style=move || if active_tab.get() == "candidates" {
                        "background: none; border: none; border-bottom: 2px solid #6366f1; color: #ffffff; padding: 8px 12px; font-size: 11px; font-weight: 600; cursor: pointer;"
                    } else {
                        "background: none; border: none; border-bottom: 2px solid transparent; color: rgba(255,255,255,0.6); padding: 8px 12px; font-size: 11px; cursor: pointer;"
                    }
                    on:click=move |_| active_tab.set("candidates".to_string())
                >
                    "Candidates (" {move || signals.candidates.get().len()} ")"
                </button>
                <button
                    style=move || if active_tab.get() == "artifacts" {
                        "background: none; border: none; border-bottom: 2px solid #6366f1; color: #ffffff; padding: 8px 12px; font-size: 11px; font-weight: 600; cursor: pointer;"
                    } else {
                        "background: none; border: none; border-bottom: 2px solid transparent; color: rgba(255,255,255,0.6); padding: 8px 12px; font-size: 11px; cursor: pointer;"
                    }
                    on:click=move |_| active_tab.set("artifacts".to_string())
                >
                    "Promoted Artifacts (" {move || signals.artifacts.get().len()} ")"
                </button>
                <button
                    style=move || if active_tab.get() == "scopes" {
                        "background: none; border: none; border-bottom: 2px solid #6366f1; color: #ffffff; padding: 8px 12px; font-size: 11px; font-weight: 600; cursor: pointer;"
                    } else {
                        "background: none; border: none; border-bottom: 2px solid transparent; color: rgba(255,255,255,0.6); padding: 8px 12px; font-size: 11px; cursor: pointer;"
                    }
                    on:click=move |_| active_tab.set("scopes".to_string())
                >
                    "Task Scopes (" {move || signals.scopes.get().len()} ")"
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

            // Proposal Drawer
            <Show when=move || signals.is_proposing.get()>
                <div style="padding: 10px 12px; background: rgba(0,0,0,0.25); border-bottom: 1px solid rgba(255,255,255,0.08); display: flex; flex-direction: column; gap: 8px;">
                    <div style="font-size: 11px; font-weight: 700; text-transform: uppercase; color: rgba(255,255,255,0.5);">"Propose Learning Candidate"</div>
                    <div style="display: flex; gap: 8px;">
                        <select
                            prop:value=move || signals.new_layer.get()
                            on:change=move |e| signals.new_layer.set(event_target_value(&e))
                            style="background: rgba(0,0,0,0.3); border: 1px solid rgba(255,255,255,0.15); border-radius: 4px; padding: 4px 6px; font-size: 11px; color: inherit;"
                        >
                            <option value="procedural">"Procedural (L4)"</option>
                            <option value="behavioral">"Behavioral (L3)"</option>
                            <option value="epistemic">"Epistemic (L1)"</option>
                            <option value="neural">"Neural (L5)"</option>
                        </select>
                        <input
                            type="text"
                            placeholder="Applicability Scope (e.g. 'service.nginx', 'dialogue.concise')..."
                            prop:value=move || signals.new_scope.get()
                            on:input=move |e| signals.new_scope.set(event_target_value(&e))
                            style="background: rgba(0,0,0,0.3); border: 1px solid rgba(255,255,255,0.15); border-radius: 4px; padding: 4px 8px; font-size: 11px; color: inherit; width: 220px;"
                        />
                    </div>
                    <input
                        type="text"
                        placeholder="Proposed Generalization or Behavior Rule..."
                        prop:value=move || signals.new_generalization.get()
                        on:input=move |e| signals.new_generalization.set(event_target_value(&e))
                        style="background: rgba(0,0,0,0.3); border: 1px solid rgba(255,255,255,0.15); border-radius: 4px; padding: 6px 8px; font-size: 11px; color: inherit;"
                    />
                    <div style="display: flex; justify-content: flex-end; gap: 6px;">
                        <button
                            style="background: rgba(255,255,255,0.06); border: none; border-radius: 4px; padding: 4px 10px; font-size: 11px; color: inherit; cursor: pointer;"
                            on:click=move |_| signals.is_proposing.set(false)
                        >
                            "Cancel"
                        </button>
                        <button
                            style="background: #4f46e5; color: #ffffff; border: none; border-radius: 4px; padding: 4px 12px; font-size: 11px; font-weight: 600; cursor: pointer;"
                            on:click=move |_| submit_proposal()
                        >
                            "Submit Proposal"
                        </button>
                    </div>
                </div>
            </Show>

            // Main Content Area
            <div style="flex: 1; overflow-y: auto; padding: 12px;">
                // TAB 1: Candidates
                <Show when=move || active_tab.get() == "candidates">
                    <div style="display: flex; flex-direction: column; gap: 8px;">
                        {move || signals.candidates.get().into_iter().map(|cand| {
                            let cand_id = cand.candidate_id;
                            let layer_str = format!("{:?}", cand.layer);
                            view! {
                                <div style="background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.08); border-radius: 6px; padding: 10px; display: flex; flex-direction: column; gap: 6px;">
                                    <div style="display: flex; align-items: center; justify-content: space-between;">
                                        <div style="display: flex; align-items: center; gap: 6px;">
                                            <span style="background: rgba(99, 102, 241, 0.2); color: #818cf8; border: 1px solid rgba(99, 102, 241, 0.4); border-radius: 4px; padding: 2px 6px; font-size: 10px; font-weight: 700;">
                                                {layer_str}
                                            </span>
                                            <span style="background: rgba(255,255,255,0.06); border-radius: 4px; padding: 2px 6px; font-size: 10px; font-family: monospace;">
                                                {cand.scope}
                                            </span>
                                        </div>
                                        <button
                                            style="background: #4f46e5; color: #ffffff; border: none; border-radius: 4px; padding: 3px 8px; font-size: 10px; font-weight: 600; cursor: pointer;"
                                            on:click=move |_| evaluate_candidate(cand_id)
                                        >
                                            "Evaluate Gate"
                                        </button>
                                    </div>
                                    <div style="font-size: 12px; font-weight: 500; color: #ffffff; line-height: 1.4;">
                                        {cand.generalization}
                                    </div>
                                    <div style="display: flex; gap: 12px; font-size: 10px; color: rgba(255,255,255,0.4);">
                                        <span>"Source Evidence: " <b>{cand.source_evidence.len()}</b></span>
                                        <span>"Outcome Evidence: " <b>{cand.outcome_evidence.len()}</b></span>
                                        <span>"Version: " <b>{cand.derivation_version}</b></span>
                                    </div>
                                </div>
                            }
                        }).collect_view()}
                    </div>
                </Show>

                // TAB 2: Promoted Artifacts
                <Show when=move || active_tab.get() == "artifacts">
                    <div style="display: flex; flex-direction: column; gap: 8px;">
                        {move || signals.artifacts.get().into_iter().map(|art| {
                            let art_id = art.artifact_id;
                            let layer_str = format!("{:?}", art.layer);
                            let status_str = format!("{:?}", art.status);
                            let is_revoked = art.status == cybou_protocol::learning::ArtifactStatus::Revoked;

                            view! {
                                <div style="background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.08); border-radius: 6px; padding: 10px; display: flex; flex-direction: column; gap: 6px;">
                                    <div style="display: flex; align-items: center; justify-content: space-between;">
                                        <div style="display: flex; align-items: center; gap: 6px;">
                                            <span style="background: rgba(16, 185, 129, 0.2); color: #34d399; border: 1px solid rgba(16, 185, 129, 0.4); border-radius: 4px; padding: 2px 6px; font-size: 10px; font-weight: 700;">
                                                {layer_str}
                                            </span>
                                            <span style=move || if is_revoked {
                                                "background: rgba(239, 68, 68, 0.2); color: #fca5a5; border: 1px solid rgba(239, 68, 68, 0.4); border-radius: 4px; padding: 2px 6px; font-size: 10px; font-weight: 700;"
                                            } else {
                                                "background: rgba(59, 130, 246, 0.2); color: #93c5fd; border: 1px solid rgba(59, 130, 246, 0.4); border-radius: 4px; padding: 2px 6px; font-size: 10px; font-weight: 700;"
                                            }>
                                                {status_str}
                                            </span>
                                        </div>
                                        {(!is_revoked).then(|| {
                                            view! {
                                                <button
                                                    style="background: rgba(239, 68, 68, 0.15); color: #fca5a5; border: 1px solid rgba(239, 68, 68, 0.3); border-radius: 4px; padding: 3px 8px; font-size: 10px; font-weight: 600; cursor: pointer;"
                                                    on:click=move |_| revoke_artifact(art_id)
                                                >
                                                    "Revoke"
                                                </button>
                                            }
                                        })}
                                    </div>
                                    <div style="font-size: 11px; font-family: monospace; color: rgba(255,255,255,0.7);">
                                        "Artifact ID: " {art.artifact_id.to_string()}
                                    </div>
                                    <div style="display: flex; gap: 12px; font-size: 10px; color: rgba(255,255,255,0.4);">
                                        <span>"Epoch: " <b>{art.erasure_epoch}</b></span>
                                        <span>"Contributing Candidates: " <b>{art.contributing_candidates.len()}</b></span>
                                        <span>"Evidence Count: " <b>{art.source_evidence.len()}</b></span>
                                    </div>
                                </div>
                            }
                        }).collect_view()}
                    </div>
                </Show>

                // TAB 3: Governance Scopes
                <Show when=move || active_tab.get() == "scopes">
                    <div style="display: flex; flex-direction: column; gap: 8px;">
                        {move || signals.scopes.get().into_iter().map(|scope| {
                            let kind_str = format!("{:?}", scope.kind);
                            view! {
                                <div style="background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.08); border-radius: 6px; padding: 10px; display: flex; flex-direction: column; gap: 6px;">
                                    <div style="display: flex; align-items: center; justify-content: space-between;">
                                        <div style="display: flex; align-items: center; gap: 6px;">
                                            <span style="background: rgba(245, 158, 11, 0.2); color: #fbbf24; border: 1px solid rgba(245, 158, 11, 0.4); border-radius: 4px; padding: 2px 6px; font-size: 10px; font-weight: 700;">
                                                {kind_str}
                                            </span>
                                            <span style="font-size: 11px; font-family: monospace; color: rgba(255,255,255,0.7);">
                                                {scope.actor_id.to_string()}
                                            </span>
                                        </div>
                                        <span style="font-size: 10px; color: rgba(255,255,255,0.4);">
                                            "TTL: " {scope.ttl_seconds} "s"
                                        </span>
                                    </div>
                                    <div style="display: flex; flex-direction: column; gap: 3px; font-size: 11px;">
                                        <div>
                                            <span style="color: rgba(255,255,255,0.4);">"Capabilities: "</span>
                                            {scope.capabilities.join(", ")}
                                        </div>
                                        <div>
                                            <span style="color: rgba(255,255,255,0.4);">"Tool Grants: "</span>
                                            {if scope.tool_grants.is_empty() { "none".to_string() } else { scope.tool_grants.join(", ") }}
                                        </div>
                                    </div>
                                </div>
                            }
                        }).collect_view()}
                    </div>
                </Show>
            </div>
        </div>
    }
}
