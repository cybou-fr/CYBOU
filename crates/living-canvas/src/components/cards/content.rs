// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Unified card content dispatcher rendering presentation surfaces for standalone cards and decks.

use leptos::prelude::*;

use crate::{
    CardId,
    components::cards::{
        agents::AgentsContent, attention::AttentionContent, beliefs::BeliefsContent,
        capabilities::CapabilitiesContent, commitments::CommitmentsContent,
        context::ContextContent, diff::DiffContent, disclosure::DisclosureContent,
        editor::EditorContent, file_manager::FileManagerContent, identity::IdentityContent,
        insight::InsightContent, inspector::InspectorContent, journal::JournalContent,
        journal_feed::JournalFeedContent, lifecycle::LifecycleContent, outline::OutlineContent,
        perception::PerceptionContent, self_model::SelfModelContent, session::SessionContent,
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
        CardId::Disclosure => view! { <DisclosureContent runtime=runtime /> }.into_any(),
        CardId::Insight => view! { <InsightContent runtime=runtime /> }.into_any(),
        CardId::Agents => view! { <AgentsContent runtime=runtime /> }.into_any(),
        CardId::Shell(instance) => {
            view! { <ShellContent runtime=runtime auth_modal_open=auth instance=instance /> }
                .into_any()
        }
        CardId::FileManager(instance) => {
            view! { <FileManagerContent runtime=runtime auth_modal_open=auth instance=instance /> }
                .into_any()
        }
        CardId::Editor(instance) => {
            view! { <EditorContent runtime=runtime auth_modal_open=auth instance=instance /> }
                .into_any()
        }
        CardId::Diff(instance) => {
            view! { <DiffContent runtime=runtime auth_modal_open=auth instance=instance /> }
                .into_any()
        }
        CardId::Inspector(instance) => {
            view! { <InspectorContent runtime=runtime auth_modal_open=auth instance=instance /> }
                .into_any()
        }
        CardId::Operations(_) => {
            view! { <crate::components::cards::operations::OperationsContent card=card /> }
                .into_any()
        }
        CardId::Notifications(_) => {
            view! { <crate::components::cards::notifications::NotificationsContent card=card /> }
                .into_any()
        }
        CardId::Services(_) => {
            view! { <crate::components::cards::services::ServicesContent card=card /> }
                .into_any()
        }
        CardId::Processes(_) => {
            view! { <crate::components::cards::processes::ProcessesContent card=card /> }
                .into_any()
        }
        CardId::Monitor(_) => {
            view! { <crate::components::cards::monitor::MonitorContent card=card /> }
                .into_any()
        }
        CardId::SystemLogs(_) => {
            view! { <crate::components::cards::system_logs::SystemLogsContent card=card /> }
                .into_any()
        }
        CardId::Storage(_) => {
            view! { <crate::components::cards::storage::StorageContent card=card /> }
                .into_any()
        }
        CardId::Network(_) => {
            view! { <crate::components::cards::network::NetworkContent card=card /> }
                .into_any()
        }
        CardId::Packages(_) => {
            view! { <crate::components::cards::packages::PackagesContent card=card /> }
                .into_any()
        }
        CardId::Updates(_) => {
            view! { <crate::components::cards::updates::UpdatesContent card=card /> }
                .into_any()
        }
        CardId::UserSettings(_) => {
            view! { <crate::components::cards::user_settings::UserSettingsContent card=card /> }
                .into_any()
        }
        CardId::Security(_) => {
            view! { <crate::components::cards::security::SecurityContent card=card /> }
                .into_any()
        }
        CardId::Backup(_) => {
            view! { <crate::components::cards::backup::BackupContent card=card /> }
                .into_any()
        }
        CardId::Mail(_) => {
            view! { <crate::components::cards::mail::MailContent card=card /> }
                .into_any()
        }
        CardId::Calendar(_) => {
            view! { <crate::components::cards::calendar::CalendarContent card=card /> }
                .into_any()
        }
        CardId::Notes(_) => {
            view! { <crate::components::cards::notes::NotesContent card=card /> }
                .into_any()
        }
        CardId::Contacts(_) => {
            view! { <crate::components::cards::contacts::ContactsContent card=card /> }
                .into_any()
        }
        CardId::CognitiveGraph(_) => {
            view! { <crate::components::cards::cognitive_graph::CognitiveGraphContent card=card /> }
                .into_any()
        }
        CardId::EventJournal(_) => {
            view! { <crate::components::cards::event_journal::EventJournalContent card=card /> }
                .into_any()
        }
        CardId::Meaning(_) => {
            view! { <crate::components::cards::meaning::MeaningContent card=card /> }
                .into_any()
        }
        CardId::Learning(_) => {
            view! { <crate::components::cards::learning::LearningContent card=card /> }
                .into_any()
        }
        CardId::Outline => view! { <OutlineContent /> }.into_any(),
        CardId::JournalFeed(_) => view! { <JournalFeedContent /> }.into_any(),
    }
}
