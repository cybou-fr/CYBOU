// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Stateless HTTP-boundary client for the canonical Meaning1 owner.

use cybou_protocol::meaning::{DialogueMemory, MeaningResponse};
use cybou_web_contracts::{
    DialogueMemoryProjection, MeaningInterpretProjection, MeaningInterpretRequest, WEB_SCHEMA_V1,
};

use crate::state::GatewayError;

/// Transport to Meaning1. Holds no dialogue, referents, interpretations or response plans.
pub struct MeaningHub;

impl Default for MeaningHub {
    fn default() -> Self {
        Self::new()
    }
}

impl MeaningHub {
    /// Create a stateless Meaning1 client.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Ask Meaning1 to interpret, record, plan and realize an utterance.
    pub async fn process_utterance(
        &self,
        request: &MeaningInterpretRequest,
        source: &str,
    ) -> Result<MeaningInterpretProjection, GatewayError> {
        #[cfg(target_os = "linux")]
        {
            let language = request.language.as_deref().unwrap_or("en");
            let encoded: Vec<u8> = zbus::Connection::session()
                .await
                .map_err(|_| GatewayError::Unavailable)?
                .call_method(
                    Some(cybou_fabric::MEANING.service),
                    cybou_fabric::MEANING.object_path,
                    Some(cybou_fabric::MEANING.interface),
                    "Process",
                    &(request.utterance.trim(), source, language),
                )
                .await
                .map_err(|_| GatewayError::Unavailable)?
                .body()
                .deserialize()
                .map_err(|_| GatewayError::InvalidProjection)?;
            if encoded.is_empty() {
                return Err(GatewayError::Refused);
            }
            let response: MeaningResponse = ciborium::from_reader(encoded.as_slice())
                .map_err(|_| GatewayError::InvalidProjection)?;
            return Ok(MeaningInterpretProjection {
                schema_version: WEB_SCHEMA_V1,
                interpretation: response.interpretation,
                response_plan: Some(response.response_plan),
                realization: Some(response.realization),
            });
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (request, source);
            Err(GatewayError::Unavailable)
        }
    }

    /// Ask Meaning1 for the bounded dialogue state it owns.
    pub async fn dialogue_memory(
        &self,
        source: &str,
    ) -> Result<DialogueMemoryProjection, GatewayError> {
        #[cfg(target_os = "linux")]
        {
            let encoded: Vec<u8> = zbus::Connection::session()
                .await
                .map_err(|_| GatewayError::Unavailable)?
                .call_method(
                    Some(cybou_fabric::MEANING.service),
                    cybou_fabric::MEANING.object_path,
                    Some(cybou_fabric::MEANING.interface),
                    "Dialogue",
                    &(source,),
                )
                .await
                .map_err(|_| GatewayError::Unavailable)?
                .body()
                .deserialize()
                .map_err(|_| GatewayError::InvalidProjection)?;
            let memory: DialogueMemory = ciborium::from_reader(encoded.as_slice())
                .map_err(|_| GatewayError::InvalidProjection)?;
            return Ok(DialogueMemoryProjection {
                schema_version: WEB_SCHEMA_V1,
                current_turn: memory.current_turn,
                remembered_referents: memory.remembered_referents,
                turns_bound: memory.turns_bound,
            });
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = source;
            Err(GatewayError::Unavailable)
        }
    }
}
