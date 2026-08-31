// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Meaning Hub: Integrates Meaning1 deterministic language parsing, reference resolution,
//! and multilingual realization into the Cybou Web Gateway.

use cybou_meaning::{Dialogue, Language, interpret, realize};
use cybou_protocol::meaning::{
    CognitiveAct, CognitiveActKind, MeaningInterpretation, Qualification, ResponsePlan,
};
use cybou_web_contracts::{
    DialogueMemoryProjection, MeaningInterpretProjection, MeaningInterpretRequest, WEB_SCHEMA_V1,
};
use std::sync::Mutex;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

/// Server-owned engine for typed semantic interpretation and dialogue memory.
pub struct MeaningHub {
    dialogue: Mutex<Dialogue>,
}

impl Default for MeaningHub {
    fn default() -> Self {
        Self::new()
    }
}

impl MeaningHub {
    /// Create a new `MeaningHub` with 20 turns and 2 hours retention span.
    #[must_use]
    pub fn new() -> Self {
        Self {
            dialogue: Mutex::new(Dialogue::new(20, Duration::hours(2))),
        }
    }

    /// Interpret an utterance into a typed cognitive act and formulate a qualified response.
    pub fn process_utterance(
        &self,
        request: &MeaningInterpretRequest,
    ) -> MeaningInterpretProjection {
        let now = OffsetDateTime::now_utc();
        let lang = match request.language.as_deref().unwrap_or("en") {
            "ru" => Language::Russian,
            _ => Language::English,
        };

        let trimmed = request.utterance.trim();
        let (interpretation, plan) = if let Some(interp) = interpret(trimmed, "person", now) {
            let plan = ResponsePlan {
                plan_id: Uuid::new_v4(),
                intent: format!("handle_{:?}", interp.primary_act.kind).to_lowercase(),
                key_points: vec![format!(
                    "Cognitive act '{:?}' identified for subject '{}'",
                    interp.primary_act.kind, interp.primary_act.subject
                )],
                referenced_evidence: Vec::new(),
                qualifications: Vec::new(),
            };
            (interp, plan)
        } else {
            // Honest fallback interpretation for open queries
            let act = CognitiveAct {
                act_id: Uuid::new_v4(),
                kind: CognitiveActKind::Ask,
                subject: trimmed.to_string(),
                parameters: Vec::new(),
                source: "person".to_string(),
                evidence: Vec::new(),
            };
            let interp = MeaningInterpretation {
                utterance: trimmed.to_string(),
                primary_act: act,
                references: Vec::new(),
                confidence: 0.85,
                ambiguous: false,
                derived_at: now,
            };
            let plan = ResponsePlan {
                plan_id: Uuid::new_v4(),
                intent: "inquire_state".to_string(),
                key_points: vec![format!("Querying system state for '{trimmed}'")],
                referenced_evidence: Vec::new(),
                qualifications: vec![Qualification::Unverified],
            };
            (interp, plan)
        };

        // Note subject in dialogue memory
        if let Ok(mut dlg) = self.dialogue.lock() {
            dlg.open_turn(now);
            if !interpretation.primary_act.subject.is_empty() {
                dlg.mention(
                    &interpretation.primary_act.subject,
                    &interpretation.primary_act.subject,
                    0,
                    now,
                );
            }
        }

        let realized = realize(&plan, lang);

        MeaningInterpretProjection {
            schema_version: WEB_SCHEMA_V1,
            interpretation,
            response_plan: Some(plan),
            realization: Some(realized),
        }
    }

    /// Retrieve active dialogue memory status.
    pub fn dialogue_memory(&self) -> DialogueMemoryProjection {
        let now = OffsetDateTime::now_utc();
        let (turn, referents) = if let Ok(dlg) = self.dialogue.lock() {
            let candidates = dlg.candidates(now);
            let labels = candidates.into_iter().map(|c| c.label).collect();
            (dlg.turn(), labels)
        } else {
            (0, Vec::new())
        };

        DialogueMemoryProjection {
            schema_version: WEB_SCHEMA_V1,
            current_turn: turn,
            remembered_referents: referents,
            turns_bound: 20,
        }
    }
}
