// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Meaning & Dialogue Assistant card component (ADR-0031 & Milestone 8).
//!
//! Provides deterministic semantic interpretation, qualified response planning,
//! reference resolution, and multilingual realization without LLM guessing.

use leptos::prelude::*;
use crate::{
    MindClient,
    CardId,
    components::icons::{IconCheckCircle, IconBot, IconRefresh, IconSearch},
    state::RuntimeState,
    tool_state::ToolCardStates,
};

#[component]
pub fn MeaningContent(card: CardId) -> impl IntoView {
    let client = crate::GatewayMindClient;
    let tool_states = expect_context::<ToolCardStates>();
    let signals = tool_states.meaning(card);

    let load_dialogue_memory = move || {
        leptos::task::spawn_local(async move {
            if let Ok(mem) = client.get_dialogue_memory().await {
                signals.memory.set(Some(mem));
            }
        });
    };

    let submit_query = move || {
        let text = signals.query.get();
        if text.trim().is_empty() {
            return;
        }
        signals.loading.set(true);
        signals.status_msg.set(None);
        let lang = signals.language.get();

        leptos::task::spawn_local(async move {
            let res = client.interpret_meaning(&cybou_web_contracts::MeaningInterpretRequest {
                utterance: text,
                language: Some(lang),
            }).await;

            match res {
                Ok(proj) => {
                    signals.projection.set(Some(proj));
                    signals.status_msg.set(None);
                    load_dialogue_memory();
                }
                Err(err) => {
                    signals.status_msg.set(Some(format!("Interpretation refused: {err}")));
                }
            }
            signals.loading.set(false);
        });
    };

    // Load dialogue memory on mount
    Effect::new(move |_| {
        load_dialogue_memory();
    });

    let quick_queries = [
        ("explain cybou-web-gateway", "en"),
        ("show me memory", "en"),
        ("inspect active agents", "en"),
        ("verify security policies", "en"),
        ("расскажи про систему", "ru"),
        ("проверь процессы", "ru"),
    ];

    view! {
        <div class="meaning-panel" style="display: flex; flex-direction: column; height: 100%; width: 100%; background: var(--bg-card, #1e1e24); color: var(--text-main, #e0e0e0); font-family: system-ui, -apple-system, sans-serif; overflow: hidden;">
            // Header
            <div style="display: flex; align-items: center; justify-content: space-between; padding: 10px 12px; background: rgba(0,0,0,0.2); border-bottom: 1px solid rgba(255,255,255,0.08);">
                <div style="display: flex; align-items: center; gap: 8px;">
                    <IconBot size=16 />
                    <span style="font-weight: 600; font-size: 13px;">"Meaning1 & Dialogue Assistant"</span>
                </div>
                <div style="display: flex; align-items: center; gap: 6px;">
                    <select
                        prop:value=move || signals.language.get()
                        on:change=move |e| signals.language.set(event_target_value(&e))
                        style="background: rgba(0,0,0,0.3); border: 1px solid rgba(255,255,255,0.1); border-radius: 4px; padding: 3px 6px; font-size: 11px; color: inherit;"
                    >
                        <option value="en">"English (EN)"</option>
                        <option value="ru">"Русский (RU)"</option>
                        <option value="de">"Deutsch (DE)"</option>
                        <option value="fr">"Français (FR)"</option>
                    </select>
                    <button
                        style="background: rgba(255,255,255,0.06); border: none; border-radius: 4px; padding: 4px 6px; color: inherit; cursor: pointer;"
                        title="Refresh memory"
                        on:click=move |_| load_dialogue_memory()
                    >
                        <IconRefresh size=13 />
                    </button>
                </div>
            </div>

            // Status message toast
            {move || signals.status_msg.get().map(|msg| {
                view! {
                    <div style="background: rgba(239, 68, 68, 0.15); color: #fca5a5; font-size: 11px; padding: 6px 12px; border-bottom: 1px solid rgba(239, 68, 68, 0.3); display: flex; justify-content: space-between;">
                        <span>{msg}</span>
                        <button style="background: none; border: none; color: inherit; cursor: pointer;" on:click=move |_| signals.status_msg.set(None)>"×"</button>
                    </div>
                }
            })}

            // Natural Language Query Input & Quick Chips
            <div style="padding: 10px 12px; background: rgba(0,0,0,0.15); border-bottom: 1px solid rgba(255,255,255,0.06); display: flex; flex-direction: column; gap: 8px;">
                <div style="display: flex; gap: 6px;">
                    <input
                        type="text"
                        placeholder="Enter natural language query or command (e.g. 'explain why cybou-web-gateway restarted')..."
                        prop:value=move || signals.query.get()
                        on:input=move |e| signals.query.set(event_target_value(&e))
                        on:keydown=move |e| {
                            if e.key() == "Enter" {
                                submit_query();
                            }
                        }
                        style="flex: 1; background: rgba(0,0,0,0.3); border: 1px solid rgba(255,255,255,0.15); border-radius: 6px; padding: 7px 10px; font-size: 12px; color: inherit;"
                    />
                    <button
                        style="background: #4f46e5; color: #ffffff; border: none; border-radius: 6px; padding: 0 14px; font-size: 12px; font-weight: 600; cursor: pointer; display: flex; align-items: center; gap: 4px;"
                        on:click=move |_| submit_query()
                    >
                        <IconSearch size=13 />
                        {move || if signals.loading.get() { "Parsing..." } else { "Interpret" }}
                    </button>
                </div>

                // Quick presets
                <div style="display: flex; gap: 6px; flex-wrap: wrap; align-items: center;">
                    <small style="font-size: 10px; color: rgba(255,255,255,0.4); text-transform: uppercase; font-weight: 700;">"Quick Queries:"</small>
                    {quick_queries.into_iter().map(|(q, lang)| {
                        view! {
                            <button
                                style="background: rgba(255,255,255,0.05); border: 1px solid rgba(255,255,255,0.1); border-radius: 12px; padding: 2px 8px; font-size: 10px; color: rgba(255,255,255,0.75); cursor: pointer;"
                                on:click=move |_| {
                                    signals.query.set(q.to_string());
                                    signals.language.set(lang.to_string());
                                    submit_query();
                                }
                            >
                                {q}
                            </button>
                        }
                    }).collect_view()}
                </div>
            </div>

            // Main Interpretation & Realization Display
            <div style="flex: 1; overflow-y: auto; padding: 12px; display: flex; flex-direction: column; gap: 12px;">
                {move || match signals.projection.get() {
                    None => view! {
                        <div style="padding: 30px; text-align: center; color: rgba(255,255,255,0.4); font-size: 12px;">
                            <IconBot size=28 />
                            <p style="margin-top: 8px;">"No active interpretation. Enter a natural language question or command above."</p>
                            <p style="font-size: 11px; color: rgba(255,255,255,0.3);">"Meaning1 deterministically maps language to typed Cognitive Acts without LLM guessing."</p>
                        </div>
                    }.into_any(),
                    Some(proj) => {
                        let act = proj.interpretation.primary_act;
                        let act_kind_str = format!("{:?}", act.kind);
                        let plan_view = proj.response_plan.map(|plan| {
                            view! {
                                <div style="background: rgba(0,0,0,0.25); border: 1px solid rgba(255,255,255,0.08); border-radius: 6px; padding: 10px; display: flex; flex-direction: column; gap: 6px;">
                                    <div style="display: flex; align-items: center; justify-content: space-between;">
                                        <span style="font-size: 11px; font-weight: 700; text-transform: uppercase; color: rgba(255,255,255,0.5);">"Formulated Response Plan"</span>
                                        <span style="font-size: 10px; font-family: monospace; color: #818cf8;">{plan.intent}</span>
                                    </div>
                                    <ul style="margin: 0; padding-left: 16px; font-size: 12px; color: rgba(255,255,255,0.85); display: flex; flex-direction: column; gap: 3px;">
                                        {plan.key_points.into_iter().map(|pt| view! { <li>{pt}</li> }).collect_view()}
                                    </ul>
                                    {(!plan.qualifications.is_empty()).then(|| {
                                        view! {
                                            <div style="display: flex; gap: 4px; align-items: center; margin-top: 4px;">
                                                <small style="font-size: 10px; color: rgba(255,255,255,0.4);">"Qualifications:"</small>
                                                {plan.qualifications.into_iter().map(|q| {
                                                    view! {
                                                        <span style="background: rgba(245, 158, 11, 0.15); color: #fbbf24; border: 1px solid rgba(245, 158, 11, 0.3); border-radius: 4px; padding: 1px 5px; font-size: 9px; font-weight: 600;">
                                                            {q.name()}
                                                        </span>
                                                    }
                                                }).collect_view()}
                                            </div>
                                        }
                                    })}
                                </div>
                            }
                        });

                        let realization_view = proj.realization.map(|text| {
                            view! {
                                <div style="background: rgba(79, 70, 229, 0.12); border: 1px solid rgba(79, 70, 229, 0.3); border-radius: 6px; padding: 12px;">
                                    <div style="font-size: 10px; font-weight: 700; text-transform: uppercase; color: #a5b4fc; margin-bottom: 4px;">
                                        "Realized Response (Deterministic)"
                                    </div>
                                    <div style="font-size: 13px; line-height: 1.5; color: #ffffff;">
                                        {text}
                                    </div>
                                </div>
                            }
                        });

                        view! {
                            <div style="display: flex; flex-direction: column; gap: 10px;">
                                // Cognitive Act Summary Card
                                <div style="background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.08); border-radius: 6px; padding: 10px;">
                                    <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 8px;">
                                        <div style="display: flex; align-items: center; gap: 6px;">
                                            <span style="background: rgba(99, 102, 241, 0.2); color: #818cf8; border: 1px solid rgba(99, 102, 241, 0.4); border-radius: 4px; padding: 2px 8px; font-size: 11px; font-weight: 700;">
                                                {act_kind_str}
                                            </span>
                                            <span style="font-size: 13px; font-weight: 600;">{act.subject.clone()}</span>
                                        </div>
                                        <div style="display: flex; align-items: center; gap: 6px;">
                                            <span style="font-size: 10px; color: rgba(255,255,255,0.5);">"Confidence:"</span>
                                            <span style="font-size: 11px; font-weight: 700; color: #34d399;">
                                                {format!("{:.0}%", proj.interpretation.confidence * 100.0)}
                                            </span>
                                        </div>
                                    </div>
                                    <div style="font-size: 11px; color: rgba(255,255,255,0.6);">
                                        "Utterance: \"" {proj.interpretation.utterance} "\""
                                    </div>
                                </div>

                                {plan_view}
                                {realization_view}
                            </div>
                        }.into_any()
                    }
                }}

                // Dialogue Memory Status
                {move || signals.memory.get().map(|mem| {
                    view! {
                        <div style="margin-top: auto; padding: 8px 10px; background: rgba(0,0,0,0.3); border: 1px solid rgba(255,255,255,0.06); border-radius: 6px; display: flex; align-items: center; justify-content: space-between;">
                            <div style="display: flex; align-items: center; gap: 6px; font-size: 11px; color: rgba(255,255,255,0.6);">
                                <span>"Dialogue Turn: " <b style="color: #ffffff;">{mem.current_turn}</b></span>
                                <span>"·"</span>
                                <span>"Referents Held: " <b style="color: #ffffff;">{mem.remembered_referents.len()}</b></span>
                            </div>
                            <div style="display: flex; gap: 4px;">
                                {mem.remembered_referents.into_iter().take(4).map(|ref_label| {
                                    view! {
                                        <span style="background: rgba(255,255,255,0.06); border-radius: 3px; padding: 1px 4px; font-size: 9px; font-family: monospace;">
                                            {ref_label}
                                        </span>
                                    }
                                }).collect_view()}
                            </div>
                        </div>
                    }
                })}
            </div>
        </div>
    }
}
