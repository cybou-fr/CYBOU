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
    text_diff::{DiffLineKind, build_text_diff},
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

    let computed_diff =
        Memo::new(move |_| build_text_diff(&original_content.get(), &proposed_content.get()));

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
                        <Show when=move || computed_diff.get().hunks.is_empty()>
                            <div class="diff-empty">"No line changes"</div>
                        </Show>
                        <For
                            each=move || computed_diff.get().hunks
                            key=|hunk| (hunk.old_start, hunk.new_start)
                            children=move |hunk| {
                                let header = format!(
                                    "@@ -{},{} +{},{} @@",
                                    hunk.old_start, hunk.old_len, hunk.new_start, hunk.new_len
                                );
                                view! {
                                    <div class="diff-hunk">
                                        <div class="diff-hunk-header">{header}</div>
                                        <For
                                            each={move || hunk.lines.clone().into_iter().enumerate().collect::<Vec<_>>()}
                                            key=|(index, line)| (*index, line.old_line, line.new_line)
                                            children=move |(_, line)| {
                                                let (line_class, prefix) = match line.kind {
                                                    DiffLineKind::Delete => ("diff-line del", "-"),
                                                    DiffLineKind::Add => ("diff-line add", "+"),
                                                    DiffLineKind::Context => ("diff-line ctx", " "),
                                                };
                                                view! {
                                                    <div class=line_class>
                                                        <span class="diff-num old">{line.old_line.map(|n| n.to_string()).unwrap_or_default()}</span>
                                                        <span class="diff-num new">{line.new_line.map(|n| n.to_string()).unwrap_or_default()}</span>
                                                        <span class="diff-prefix">{prefix}</span>
                                                        <span class="diff-code">{line.content}</span>
                                                    </div>
                                                }
                                            }
                                        />
                                    </div>
                                }
                            }
                        />
                        <Show when=move || computed_diff.get().used_fallback>
                            <div class="diff-limit-note">
                                "Comparison exceeded the bounded edit budget; showing a coarse replacement."
                            </div>
                        </Show>
                    </div>
                </div>

                // Status Message
                <Show when=move || status_msg.get().is_some()>
                    <div class="diff-status-bar" role="status" aria-live="polite">
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
