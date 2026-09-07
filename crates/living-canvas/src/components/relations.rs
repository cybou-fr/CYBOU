// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! SVG relationship connection line and graph layer between dependent cognitive cards.

use leptos::prelude::*;

use crate::{
    CardId, DesktopItemId, DesktopLayout,
    interaction::relationship_points,
    layout::relations::{DesktopRelationshipGraph, Relationship},
    tool_state::ToolCardStates,
};

/// A dynamic contextual dependency edge between active cards on the desktop.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DynamicRelation {
    pub from: CardId,
    pub to: CardId,
    pub label: &'static str,
    pub amber: bool,
}

/// Compute active dynamic relations between open cards on the Living Canvas.
#[must_use]
pub fn active_dynamic_relations(
    layout: &DesktopLayout,
    tool_states: &ToolCardStates,
) -> Vec<DynamicRelation> {
    let mut relations = Vec::new();

    // 1. Services -> Inspector (when Inspector inspects a service)
    if layout.contains_card(CardId::Services(0)) && layout.contains_card(CardId::Inspector(0)) {
        let inspector_signals = tool_states.inspector(CardId::Inspector(0));
        if let Some(cybou_protocol::SubjectRef::Service { .. }) =
            inspector_signals.target_subject.get()
        {
            relations.push(DynamicRelation {
                from: CardId::Services(0),
                to: CardId::Inspector(0),
                label: "Inspects Unit",
                amber: false,
            });
        }
    }

    // 2. Services -> SystemLogs (when logs query is filtered for a service)
    if layout.contains_card(CardId::Services(0)) && layout.contains_card(CardId::SystemLogs(0)) {
        let logs_signals = tool_states.system_logs(CardId::SystemLogs(0));
        if !logs_signals.search_query.get().is_empty() {
            relations.push(DynamicRelation {
                from: CardId::Services(0),
                to: CardId::SystemLogs(0),
                label: "Unit Journal",
                amber: false,
            });
        }
    }

    // 3. Processes -> Inspector (when Inspector inspects a process)
    if layout.contains_card(CardId::Processes(0)) && layout.contains_card(CardId::Inspector(0)) {
        let inspector_signals = tool_states.inspector(CardId::Inspector(0));
        if let Some(cybou_protocol::SubjectRef::Process { .. }) =
            inspector_signals.target_subject.get()
        {
            relations.push(DynamicRelation {
                from: CardId::Processes(0),
                to: CardId::Inspector(0),
                label: "Inspects Process",
                amber: false,
            });
        }
    }

    // 4. Insight -> Inspector (when investigating an anomaly)
    if layout.contains_card(CardId::Insight) && layout.contains_card(CardId::Inspector(0)) {
        relations.push(DynamicRelation {
            from: CardId::Insight,
            to: CardId::Inspector(0),
            label: "Investigates",
            amber: true,
        });
    }

    // 5. Insight -> Services (when remediation monitoring services)
    if layout.contains_card(CardId::Insight) && layout.contains_card(CardId::Services(0)) {
        relations.push(DynamicRelation {
            from: CardId::Insight,
            to: CardId::Services(0),
            label: "Remediates",
            amber: false,
        });
    }

    // 6. Editor -> Diff (when reviewing code edits)
    if layout.contains_card(CardId::Editor(0)) && layout.contains_card(CardId::Diff(0)) {
        relations.push(DynamicRelation {
            from: CardId::Editor(0),
            to: CardId::Diff(0),
            label: "Review Diff",
            amber: false,
        });
    }

    // 7. FileManager -> Editor (when editing files)
    if layout.contains_card(CardId::FileManager(0)) && layout.contains_card(CardId::Editor(0)) {
        relations.push(DynamicRelation {
            from: CardId::FileManager(0),
            to: CardId::Editor(0),
            label: "Edits File",
            amber: false,
        });
    }

    // 8. FileManager -> Inspector (when inspecting file metadata)
    if layout.contains_card(CardId::FileManager(0)) && layout.contains_card(CardId::Inspector(0)) {
        let inspector_signals = tool_states.inspector(CardId::Inspector(0));
        if let Some(cybou_protocol::SubjectRef::File { .. }) =
            inspector_signals.target_subject.get()
        {
            relations.push(DynamicRelation {
                from: CardId::FileManager(0),
                to: CardId::Inspector(0),
                label: "Inspects File",
                amber: false,
            });
        }
    }

    // 9. Notes -> Inspector (when annotating an inspected entity)
    if layout.contains_card(CardId::Notes(0)) && layout.contains_card(CardId::Inspector(0)) {
        let notes_signals = tool_states.notes(CardId::Notes(0));
        let inspector_signals = tool_states.inspector(CardId::Inspector(0));
        if notes_signals.edit_referenced_subject.get().is_some()
            && notes_signals.edit_referenced_subject.get() == inspector_signals.target_subject.get()
        {
            relations.push(DynamicRelation {
                from: CardId::Notes(0),
                to: CardId::Inspector(0),
                label: "Annotates",
                amber: false,
            });
        }
    }

    // 10. Notifications -> Inspector (when notifications reference inspected entity)
    if layout.contains_card(CardId::Notifications(0)) && layout.contains_card(CardId::Inspector(0))
    {
        let notif_signals = tool_states.notifications(CardId::Notifications(0));
        let inspector_signals = tool_states.inspector(CardId::Inspector(0));
        if let Some(target) = inspector_signals.target_subject.get() {
            if notif_signals
                .notifications
                .get()
                .iter()
                .any(|n| n.subject.as_ref() == Some(&target))
            {
                relations.push(DynamicRelation {
                    from: CardId::Notifications(0),
                    to: CardId::Inspector(0),
                    label: "Alerts on",
                    amber: true,
                });
            }
        }
    }

    // 11. Operations -> Inspector (when background operations target inspected entity)
    if layout.contains_card(CardId::Operations(0)) && layout.contains_card(CardId::Inspector(0)) {
        let op_signals = tool_states.operations(CardId::Operations(0));
        let inspector_signals = tool_states.inspector(CardId::Inspector(0));
        if let Some(target) = inspector_signals.target_subject.get() {
            if op_signals
                .operations
                .get()
                .iter()
                .any(|op| op.subject.as_ref() == Some(&target))
            {
                relations.push(DynamicRelation {
                    from: CardId::Operations(0),
                    to: CardId::Inspector(0),
                    label: "Operates on",
                    amber: false,
                });
            }
        }
    }

    // 12. Operations -> Services (when background operations mutate services)
    if layout.contains_card(CardId::Operations(0)) && layout.contains_card(CardId::Services(0)) {
        let op_signals = tool_states.operations(CardId::Operations(0));
        if op_signals.operations.get().iter().any(|op| {
            matches!(
                op.kind,
                cybou_protocol::operation::OperationKind::ServiceRestart
                    | cybou_protocol::operation::OperationKind::ServiceStop
            )
        }) {
            relations.push(DynamicRelation {
                from: CardId::Operations(0),
                to: CardId::Services(0),
                label: "Service Action",
                amber: false,
            });
        }
    }

    relations
}

/// SVG relationship edge component connecting two cards.
#[component]
pub fn RelationshipEdge(
    layout: RwSignal<DesktopLayout>,
    selected: ReadSignal<Option<DesktopItemId>>,
    from: CardId,
    to: CardId,
    label: &'static str,
    amber: bool,
) -> impl IntoView {
    let points = move || relationship_points(layout.get(), from, to);
    view! {
        <g
            class:amber=amber
            class:active=move || {
                let selected = selected.get();
                selected == Some(DesktopItemId::Card(from)) || selected == Some(DesktopItemId::Card(to))
            }
            class="relationship-edge"
        >
            <line
                x1=move || points().0.to_string()
                y1=move || points().1.to_string()
                x2=move || points().2.to_string()
                y2=move || points().3.to_string()
            />
            <text
                x=move || points().4.to_string()
                y=move || points().5.to_string()
                text-anchor="middle"
            >{label}</text>
        </g>
    }
}

/// Full SVG layer rendering all canonical semantic relationship connections and dynamic active card links.
#[component]
pub fn RelationshipsLayer(
    layout: RwSignal<DesktopLayout>,
    selected: ReadSignal<Option<DesktopItemId>>,
    #[prop(optional)] visibility: Option<ReadSignal<crate::layout::relations::RelationVisibility>>,
) -> impl IntoView {
    let tool_states = use_context::<ToolCardStates>();
    let canonical = DesktopRelationshipGraph::canonical();
    let vis = move || {
        visibility.map_or(
            crate::layout::relations::RelationVisibility::Selected,
            |v| v.get(),
        )
    };

    let should_show = move |from: CardId, to: CardId| {
        let lay = layout.get();
        if !lay.contains_card(from) || !lay.contains_card(to) {
            return false;
        }
        match vis() {
            crate::layout::relations::RelationVisibility::Off => false,
            crate::layout::relations::RelationVisibility::All => true,
            crate::layout::relations::RelationVisibility::Selected
            | crate::layout::relations::RelationVisibility::Neighborhood => {
                let sel = selected.get();
                sel == Some(DesktopItemId::Card(from)) || sel == Some(DesktopItemId::Card(to))
            }
        }
    };

    let dynamic_edges = move || {
        tool_states.map_or_else(Vec::new, |ts| active_dynamic_relations(&layout.get(), &ts))
    };

    view! {
        <Show when=move || vis() != crate::layout::relations::RelationVisibility::Off>
            <svg class="relationships" aria-hidden="true">
                // Canonical Mind Organ edges
                {canonical.iter().map(|rel: &Relationship| {
                    let from = rel.from;
                    let to = rel.to;
                    let label = rel.label;
                    let amber = rel.amber;
                    view! {
                        <Show when=move || should_show(from, to)>
                            <RelationshipEdge
                                layout=layout
                                selected=selected
                                from=from
                                to=to
                                label=label
                                amber=amber
                            />
                        </Show>
                    }
                }).collect_view()}

                // Dynamic Contextual Tool Edges
                {move || dynamic_edges().into_iter().map(|rel| {
                    let from = rel.from;
                    let to = rel.to;
                    let label = rel.label;
                    let amber = rel.amber;
                    view! {
                        <Show when=move || should_show(from, to)>
                            <RelationshipEdge
                                layout=layout
                                selected=selected
                                from=from
                                to=to
                                label=label
                                amber=amber
                            />
                        </Show>
                    }
                }).collect_view()}
            </svg>
        </Show>
    }
}
