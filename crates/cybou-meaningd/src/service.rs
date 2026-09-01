// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! D-Bus `org.cybou.Mind.Meaning1` service implementation on zbus.

// The `#[interface]` expansion emits part of its dispatch surface with the attribute's own span,
// which an `allow` on the impl block cannot reach. Every handler written here is documented.
#![allow(missing_docs)]

use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use cybou_fabric::{CONTEXT, EVENT, event_client::EventClient};
use cybou_meaning::{Candidate, Dialogue, Language, interpret, realize, resolve_reference};
use cybou_protocol::meaning::{DialogueMemory, MeaningResponse, ResponsePlan};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;
use zbus::interface;

use crate::contributions_for;

/// D-Bus service exporting `org.cybou.Mind.Meaning1`.
pub struct Meaning1Service {
    /// Whether this organ has proven it can put an interpretation into the Journal.
    ///
    /// A meaning boundary that cannot record what it understood is not ready: the acts it produces
    /// would exist only in the reply to whoever asked, which is precisely the hidden state ADR-0031
    /// exists to prevent. It is watched rather than only set on use, so the answer is about now
    /// and not about the last time somebody happened to say something.
    journal_reachable: Arc<AtomicBool>,
    dialogues: Mutex<HashMap<String, Dialogue>>,
}

impl Meaning1Service {
    /// Create a new Meaning1 D-Bus service handler around the flag its watcher keeps current.
    #[must_use]
    pub fn new(journal_reachable: Arc<AtomicBool>) -> Self {
        Self {
            journal_reachable,
            dialogues: Mutex::new(HashMap::new()),
        }
    }

    /// The concepts Context1 currently holds, as candidates a reference can be resolved against.
    ///
    /// Empty when Context1 cannot be reached — which resolves nothing rather than resolving
    /// wrongly, because a reference with no candidates is unresolved by construction.
    async fn candidates(conn: &zbus::Connection) -> Vec<Candidate> {
        let Ok(reply) = conn
            .call_method(
                Some(CONTEXT.service),
                CONTEXT.object_path,
                Some(CONTEXT.interface),
                "ActiveContext",
                &(),
            )
            .await
        else {
            return Vec::new();
        };
        let Ok(encoded) = reply.body().deserialize::<Vec<u8>>() else {
            return Vec::new();
        };
        let Ok(labels) = ciborium::from_reader::<Vec<String>, _>(encoded.as_slice()) else {
            return Vec::new();
        };
        labels
            .into_iter()
            .map(|label| Candidate {
                target_id: label.clone(),
                label,
            })
            .collect()
    }

    /// Whether the Journal holds the contribution a caller named.
    ///
    /// An Event1 that cannot be reached answers nothing, and nothing is not confirmation.
    async fn journal_holds(conn: &zbus::Connection, message_id: &str) -> bool {
        conn.call_method(
            Some(EVENT.service),
            EVENT.object_path,
            Some(EVENT.interface),
            "Contains",
            &(message_id,),
        )
        .await
        .ok()
        .and_then(|reply| reply.body().deserialize::<bool>().ok())
        .unwrap_or(false)
    }

    /// Submit an utterance and its interpretation, and answer with the act identity.
    ///
    /// An empty answer means nothing was recorded. There is no third outcome: an act identity for
    /// an interpretation the Journal refused would name a reading no biography contains.
    async fn record(
        &self,
        utterance: &str,
        source: &str,
        supersedes: Option<Uuid>,
        conn: &zbus::Connection,
    ) -> Vec<u8> {
        let now = OffsetDateTime::now_utc();
        let Some(mut interpreted) = interpret(utterance, source, now) else {
            return Vec::new();
        };

        // The subject of the act is also the reference to settle: "cancel the debian server" is
        // about something, and which something is a question the person may still have to answer.
        let candidates = Self::candidates(conn).await;
        let resolution = resolve_reference(
            &interpreted.primary_act.subject,
            &candidates,
            interpreted.primary_act.kind,
        );
        // An unresolved reference makes the interpretation ambiguous, which is what an interface
        // reads to know it has to ask rather than act.
        interpreted.ambiguous = interpreted.ambiguous || !resolution.resolved;
        interpreted.references = vec![resolution];

        let Some(pair) = contributions_for(&interpreted, now, supersedes) else {
            return Vec::new();
        };
        let Ok(client) = EventClient::session().await else {
            self.journal_reachable.store(false, Ordering::Release);
            return Vec::new();
        };
        if client.submit(&pair.utterance).await.is_err() {
            self.journal_reachable.store(false, Ordering::Release);
            return Vec::new();
        }
        // The interpretation cites the utterance, so it is submitted second and only if the first
        // was accepted. The other order would leave a reading of a sentence nobody said.
        if client.submit(&pair.interpretation).await.is_err() {
            self.journal_reachable.store(false, Ordering::Release);
            return Vec::new();
        }
        self.journal_reachable.store(true, Ordering::Release);

        let mut encoded = Vec::new();
        if ciborium::into_writer(&interpreted, &mut encoded).is_err() {
            return Vec::new();
        }
        encoded
    }
}

#[allow(
    clippy::unused_async,
    reason = "zbus dispatches every exported handler as a future"
)]
#[interface(name = "org.cybou.Mind.Meaning1")]
impl Meaning1Service {
    /// Whether this organ has put an interpretation into the Journal.
    ///
    /// It answers false until it has actually done so once. Readiness that meant "the process
    /// started" would be true of an organ whose every act goes nowhere.
    async fn ready(&self) -> bool {
        self.journal_reachable.load(Ordering::Acquire)
    }

    /// Overall health summary.
    async fn health(&self) -> String {
        if self.journal_reachable.load(Ordering::Acquire) {
            "healthy".to_owned()
        } else {
            "degraded".to_owned()
        }
    }

    /// Last error diagnostic.
    async fn last_error(&self) -> String {
        String::new()
    }

    /// Interpret an utterance and return the `MeaningInterpretation` as CBOR.
    ///
    /// Empty when the utterance is not in this build's vocabulary, or when the Journal did not
    /// accept it. Both are refusals rather than failures, and both are better than an act.
    async fn interpret(
        &self,
        #[zbus(connection)] conn: &zbus::Connection,
        utterance: String,
        source: String,
    ) -> Vec<u8> {
        self.record(&utterance, &source, None, conn).await
    }

    /// Interpret, record, plan and realize one utterance entirely inside Meaning1.
    async fn process(
        &self,
        #[zbus(connection)] conn: &zbus::Connection,
        utterance: String,
        source: String,
        language: String,
    ) -> Vec<u8> {
        let encoded = self.record(&utterance, &source, None, conn).await;
        let Ok(interpretation) = ciborium::from_reader::<
            cybou_protocol::meaning::MeaningInterpretation,
            _,
        >(encoded.as_slice()) else {
            return Vec::new();
        };
        let plan = ResponsePlan {
            plan_id: Uuid::new_v4(),
            intent: format!("handle_{:?}", interpretation.primary_act.kind).to_lowercase(),
            key_points: vec![format!(
                "Cognitive act '{:?}' identified for subject '{}'",
                interpretation.primary_act.kind, interpretation.primary_act.subject
            )],
            referenced_evidence: interpretation.primary_act.evidence.clone(),
            qualifications: Vec::new(),
        };
        let language = match language.as_str() {
            "ru" => Language::Russian,
            "en" => Language::English,
            _ => return Vec::new(),
        };
        let realization = realize(&plan, language);
        let now = OffsetDateTime::now_utc();
        if let Ok(mut dialogues) = self.dialogues.lock() {
            let dialogue = dialogues
                .entry(source.clone())
                .or_insert_with(|| Dialogue::new(20, Duration::hours(2)));
            dialogue.open_turn(now);
            if !interpretation.primary_act.subject.is_empty() {
                dialogue.mention(
                    &interpretation.primary_act.subject,
                    &interpretation.primary_act.subject,
                    0,
                    now,
                );
            }
        }
        let response = MeaningResponse {
            interpretation,
            response_plan: plan,
            realization,
        };
        let mut encoded = Vec::new();
        ciborium::into_writer(&response, &mut encoded).map_or_else(|_| Vec::new(), |()| encoded)
    }

    /// Return bounded dialogue state held by Meaning1.
    async fn dialogue(&self, source: String) -> Vec<u8> {
        let now = OffsetDateTime::now_utc();
        let Ok(dialogues) = self.dialogues.lock() else {
            return Vec::new();
        };
        let Some(dialogue) = dialogues.get(&source) else {
            let memory = DialogueMemory {
                current_turn: 0,
                remembered_referents: Vec::new(),
                turns_bound: 20,
            };
            let mut encoded = Vec::new();
            return ciborium::into_writer(&memory, &mut encoded)
                .map_or_else(|_| Vec::new(), |()| encoded);
        };
        let memory = DialogueMemory {
            current_turn: dialogue.turn(),
            remembered_referents: dialogue
                .candidates(now)
                .into_iter()
                .map(|candidate| candidate.label)
                .collect(),
            turns_bound: 20,
        };
        let mut encoded = Vec::new();
        ciborium::into_writer(&memory, &mut encoded).map_or_else(|_| Vec::new(), |()| encoded)
    }

    /// Interpret an utterance that corrects an earlier interpretation.
    ///
    /// The earlier act is named as evidence and left exactly as it was recorded. What was
    /// previously understood remains auditable; a correction is new evidence, not a rewrite.
    async fn correct(
        &self,
        #[zbus(connection)] conn: &zbus::Connection,
        prior_act_id: String,
        utterance: String,
        source: String,
    ) -> Vec<u8> {
        let Ok(prior) = Uuid::parse_str(&prior_act_id) else {
            return Vec::new();
        };
        // A correction of an interpretation the Journal never held corrects nothing. Event1 would
        // refuse it anyway, because evidence has to exist — but arriving there by accident would
        // make an ordinary rejection indistinguishable from the Journal being unreachable, and
        // leave the caller without the one thing that tells them apart.
        if !Self::journal_holds(conn, &prior_act_id).await {
            println!("[cybou-meaningd] Correction refused: the Journal does not hold act {prior}");
            return Vec::new();
        }
        self.record(&utterance, &source, Some(prior), conn).await
    }

    /// Render a `ResponsePlan` given as CBOR into prose in the named language.
    ///
    /// The plan is the only input. A renderer able to reach anything else could add a claim Mind
    /// never made, and no care in its wording would make that safe.
    async fn realize(&self, plan: Vec<u8>, language: String) -> String {
        let Ok(plan) = ciborium::from_reader::<ResponsePlan, _>(plan.as_slice()) else {
            return String::new();
        };
        let language = match language.as_str() {
            "ru" => Language::Russian,
            "en" => Language::English,
            _ => return String::new(),
        };
        realize(&plan, language)
    }
}
