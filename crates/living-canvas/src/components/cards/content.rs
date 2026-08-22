// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Unified card content dispatcher rendering presentation surfaces for standalone cards and decks.

use leptos::prelude::*;

use crate::{
    CardId,
    components::cards::{
        attention::AttentionContent,
        beliefs::BeliefsContent,
        capabilities::CapabilitiesContent,
        commitments::CommitmentsContent,
        context::ContextContent,
        file_manager::FileManagerContent,
        identity::IdentityContent,
        journal::JournalContent,
        journal_feed::JournalFeedContent,
        lifecycle::LifecycleContent,
        perception::PerceptionContent,
        self_model::SelfModelContent,
        session::SessionContent,
        shell::ShellContent,
    },
    state::RuntimeState,
};

/// Universal dispatcher rendering the domain content of any `CardId`.
///
/// Invariant:
/// - "Card identity survives composition. Card capability survives composition."
/// - Renders the identical interactive surface whether hosted standalone in `CardFrame` or tabbed inside `DeckFrame`.
#[component]
pub fn CardContent(
    card: CardId,
    runtime: RwSignal<RuntimeState>,
    #[prop(optional)] auth_modal_open: Option<RwSignal<bool>>,
) -> impl IntoView {
    let auth = auth_modal_open.unwrap_or_else(|| RwSignal::new(false));

    match card {
        CardId::Identity => view! { <IdentityContent runtime=runtime /> }.into_any(),
        CardId::Session => view! { <SessionContent runtime=runtime /> }.into_any(),
        CardId::Capabilities => view! { <CapabilitiesContent runtime=runtime /> }.into_any(),
        CardId::Journal => view! { <JournalContent runtime=runtime /> }.into_any(),
        CardId::Lifecycle => view! { <LifecycleContent runtime=runtime /> }.into_any(),
        CardId::Commitments => view! { <CommitmentsContent runtime=runtime /> }.into_any(),
        CardId::SelfModel => view! { <SelfModelContent runtime=runtime /> }.into_any(),
        CardId::Attention => view! { <AttentionContent runtime=runtime /> }.into_any(),
        CardId::Beliefs => view! { <BeliefsContent runtime=runtime /> }.into_any(),
        CardId::Perception => view! { <PerceptionContent runtime=runtime /> }.into_any(),
        CardId::Context => view! { <ContextContent runtime=runtime /> }.into_any(),
        CardId::Shell(_) => {
            view! { <ShellContent runtime=runtime auth_modal_open=auth /> }.into_any()
        }
        CardId::FileManager(_) => {
            view! { <FileManagerContent runtime=runtime auth_modal_open=auth /> }.into_any()
        }
        CardId::JournalFeed(_) => view! { <JournalFeedContent /> }.into_any(),
    }
}
