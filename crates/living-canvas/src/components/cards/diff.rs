// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Universal Diff Viewer and commit review tool card component (ADR-0045).

use cybou_web_contracts::SessionMode;
use leptos::prelude::*;
use std::sync::Arc;
use web_sys::PointerEvent;

use crate::{
    CardId, DesktopItemId, DesktopLayout,
    components::{
        card_frame::CardFrame,
        icons::{IconFile, IconShield},
    },
    interaction::{DragState, ResizeState},
    state::RuntimeState,
    tool_state::ToolCardStates,
};

/// Diff Viewer content component with side-by-side and unified diff visualization.
#[component]
pub fn DiffContent(
    runtime: RwSignal<RuntimeState>,
    auth_modal_open: RwSignal<bool>,
    /// Instance identifier.
    #[prop(optional)]
    instance: u32,
) -> impl IntoView {
    let is_public_preview = move || match runtime.get() {
        RuntimeState::Ready { mode, .. } => mode == SessionMode::PublicPreview,
        _ => false,
    };

    let state = expect_context::<ToolCardStates>().diff(CardId::Diff(instance));
    let title = state.title;
    let original_label = state.original_label;
    let proposed_label = state.proposed_label;
    let original_content = state.original_content;
    let proposed_content = state.proposed_content;
    let status_msg = state.status_msg;

    let compute_diff_lines = move || {
        let orig = original_content.get();
        let prop = proposed_content.get();
        let orig_lines: Vec<&str> = orig.lines().collect();
        let prop_lines: Vec<&str> = prop.lines().collect();

        let mut lines = Vec::new();
        let max_len = orig_lines.len().max(prop_lines.len());
        for i in 0..max_len {
            let o = orig_lines.get(i).copied().unwrap_or("");
            let p = prop_lines.get(i).copied().unwrap_or("");
            if o == p {
                lines.push(("ctx", o.to_string(), i + 1));
            } else {
                if !o.is_empty() {
                    lines.push(("del", o.to_string(), i + 1));
                }
                if !p.is_empty() {
                    lines.push(("add", p.to_string(), i + 1));
                }
            }
        }
        lines
    };

    view! {
        <Show
            when=move || !is_public_preview()
            fallback=move || view! {
                <div class="card-auth-gate">
                    <IconShield size=26 />
                    <strong>"Diff Viewer Locked"</strong>
                    <p>"Public preview does not permit inspection of pending changes. Sign in to unlock."</p>
                    <button class="primary-btn" on:click=move |_| auth_modal_open.set(true)>"Sign in"</button>
                </div>
            }
        >
            <div class="diff-body" on:pointerdown=move |e: PointerEvent| e.stop_propagation()>
                // Diff Header Bar
                <div class="diff-toolbar">
                    <div class="diff-title-box">
                        <IconFile size=14 />
                        <span class="diff-title">{move || title.get()}</span>
                    </div>

                    <div class="diff-actions" aria-label="Read-only comparison">
                        <span class="diff-btn secondary">"Review only · no write action"</span>
                    </div>
                </div>

                // Legend / Source Indicator
                <div class="diff-legend">
                    <span class="legend-chip orig">"[-] " {move || original_label.get()}</span>
                    <span class="legend-chip prop">"[+] " {move || proposed_label.get()}</span>
                </div>

                // Diff Lines Viewer
                <div class="diff-viewer-content">
                    <div class="diff-table">
                        <For
                            each=compute_diff_lines
                            key=|(kind, content, idx)| format!("{}-{}-{}", kind, idx, content)
                            children=move |(kind, content, idx)| {
                                let line_class = match kind {
                                    "del" => "diff-line del",
                                    "add" => "diff-line add",
                                    _ => "diff-line ctx",
                                };
                                let prefix = match kind {
                                    "del" => "-",
                                    "add" => "+",
                                    _ => " ",
                                };
                                view! {
                                    <div class=line_class>
                                        <span class="diff-num">{idx}</span>
                                        <span class="diff-prefix">{prefix}</span>
                                        <span class="diff-code">{content}</span>
                                    </div>
                                }
                            }
                        />
                    </div>
                </div>

                // Status Message
                <Show when=move || status_msg.get().is_some()>
                    <div class="diff-status-bar">
                        {move || status_msg.get().unwrap_or_default()}
                    </div>
                </Show>
            </div>
        </Show>
    }
}

/// Diff Viewer standalone tool card component.
#[component]
pub fn DiffCard(
    layout: RwSignal<DesktopLayout>,
    selected: ReadSignal<Option<DesktopItemId>>,
    set_selected: WriteSignal<Option<DesktopItemId>>,
    dragging: RwSignal<Option<DragState>>,
    resizing: RwSignal<Option<ResizeState>>,
    auth_modal_open: RwSignal<bool>,
    runtime: RwSignal<RuntimeState>,
    /// Which instance of this tool card this is.
    #[prop(optional)]
    instance: u32,
) -> impl IntoView {
    let card_id = CardId::Diff(instance);

    let collapsed = move || {
        view! {
            <div class="card-collapsed-summary">
                <b>"Diff Viewer"</b>
                <span>"Inspection"</span>
            </div>
        }
        .into_any()
    };

    view! {
        <CardFrame
            card=card_id
            layout=layout
            selected=selected
            set_selected=set_selected
            dragging=dragging
            resizing=resizing
            kicker_title="Diff Viewer"
            kicker_icon=Arc::new(|| view! { <IconFile size=14 /> }.into_any())
            collapsed_summary=Arc::new(collapsed)
        >
            <DiffContent runtime=runtime auth_modal_open=auth_modal_open instance=instance />
        </CardFrame>
    }
}
