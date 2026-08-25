// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! One prompt turn against an ACP agent: initialize, open a session, ask, and collect what came back.
//!
//! [`crate::AcpClient`] stops after the handshake on purpose — it discovers what an agent is without
//! ever giving it anything to do. This is the next thing, and it is a different kind of object: it
//! runs a turn, so it has to answer two questions the handshake never faced.
//!
//! ## What happens when the agent asks permission
//!
//! It is refused. Not deferred, not auto-approved — refused, and recorded.
//!
//! An ACP agent can send `session/request_permission` when it wants to do something it thinks needs
//! a person's consent. The reference clients auto-approve, which is how a demo is written and not how
//! a boundary is. In Cybou the question is already answered before it is asked:
//!
//! ```text
//! inside its capsule    the agent needs no permission and is never asked for one
//! outside its capsule   the answer is an ActionProposal, which a person decides
//! ```
//!
//! This client can reach neither. It cannot widen a capsule and it cannot raise a proposal, so the
//! only honest answer it has is *no*. Auto-approving here would put the decision in the one place it
//! must never be: the thing being bounded, asking the thing that is supposed to bound it, and getting
//! a yes from a default. So permission requests are cancelled, and every one of them is returned to
//! the caller — because an agent that keeps asking for something is a fact worth surfacing, and
//! silently refusing would have thrown it away.
//!
//! ## What ends a turn
//!
//! A deadline the caller supplies, not one this module keeps. A prompt can take minutes; a constant
//! here would be a second clock beside the lease's, and the two disagree the first time a session is
//! given four hours and this file believes in five minutes.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use agent_client_protocol::schema::{
    ProtocolVersion,
    v1::{
        ContentBlock, InitializeRequest, NewSessionRequest, PromptRequest,
        RequestPermissionOutcome, RequestPermissionRequest, RequestPermissionResponse,
        SessionNotification, SessionUpdate, TextContent,
    },
};
use agent_client_protocol::{AcpAgent, AcpAgentConfig, Agent, Client, ConnectionTo};
use serde::{Deserialize, Serialize};

use crate::client::AcpClientError;

/// Everything one prompt turn produced.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTurn {
    /// The session the agent opened for this turn.
    pub session_id: String,
    /// Why the turn ended, in the protocol's own words.
    pub stop_reason: String,
    /// The agent's message text, in order, with nothing else folded in.
    ///
    /// Thoughts and tool calls are deliberately not concatenated here. They are in `updates`, where
    /// they keep their kind: a surface that showed an agent's internal reasoning as its answer would
    /// be presenting a draft as a conclusion.
    pub message: String,
    /// Every session update the agent sent, in order, exactly as it sent it.
    ///
    /// Kept whole rather than projected into a Cybou vocabulary. This is the seam a live session
    /// surface reads, and a projection written before there is a surface to read it would be a guess
    /// about what that surface needs, baked into the one place the original is still available.
    pub updates: Vec<serde_json::Value>,
    /// What the agent asked permission for, and was refused.
    pub refused_permissions: Vec<String>,
}

impl AgentTurn {
    /// Whether the agent ended its own turn rather than being stopped or cut off.
    #[must_use]
    pub fn ended_by_the_agent(&self) -> bool {
        self.stop_reason == "end_turn"
    }
}

/// A client that runs one prompt turn against an ACP agent.
#[derive(Clone, Copy, Debug)]
pub struct AcpSession {
    turn_deadline: Duration,
}

impl AcpSession {
    /// A client that gives one turn this long.
    ///
    /// The caller's deadline, because the caller is the one holding the lease. A session with four
    /// hours left and a prompt bounded by a constant would be stopped by the constant.
    #[must_use]
    pub const fn within(turn_deadline: Duration) -> Self {
        Self { turn_deadline }
    }

    /// Initialize an agent, open a session on `workspace`, send one prompt and collect the turn.
    ///
    /// The command must already be a capsule entrypoint. This drives a protocol; it is not a sandbox,
    /// and handing it a bare agent binary would run that agent on the host.
    ///
    /// # Errors
    ///
    /// Returns [`AcpClientError`] when the process cannot start, the agent selects a protocol version
    /// this client does not speak, the wire exchange fails, or the turn does not finish before the
    /// deadline.
    pub async fn one_turn(
        &self,
        process: AcpAgentConfig,
        workspace: PathBuf,
        prompt: &str,
    ) -> Result<AgentTurn, AcpClientError> {
        let collected = Arc::new(Mutex::new(AgentTurn::default()));
        let notified = Arc::clone(&collected);
        let refused = Arc::clone(&collected);
        let prompt = prompt.to_owned();

        let exchange = Client
            .builder()
            .name("cybou-acp")
            .on_receive_notification(
                async move |notification: SessionNotification, _cx| {
                    record(&notified, &notification.update);
                    Ok(())
                },
                agent_client_protocol::on_receive_notification!(),
            )
            .on_receive_request(
                async move |request: RequestPermissionRequest, responder, _connection| {
                    // Refused, and written down. See this module's header: the thing being bounded
                    // does not get to ask the thing bounding it for a wider grant.
                    if let Ok(mut turn) = refused.lock() {
                        turn.refused_permissions.push(describe(&request));
                    }
                    responder.respond(RequestPermissionResponse::new(
                        RequestPermissionOutcome::Cancelled,
                    ))
                },
                agent_client_protocol::on_receive_request!(),
            )
            .connect_with(
                AcpAgent::new(process),
                async move |connection: ConnectionTo<Agent>| {
                    let initialized = connection
                        .send_request(InitializeRequest::new(ProtocolVersion::V1))
                        .block_task()
                        .await?;
                    if initialized.protocol_version != ProtocolVersion::V1 {
                        // Returned as a protocol failure rather than carried on with. An agent that
                        // answered in a version this client does not speak has not agreed to
                        // anything, and proceeding would be guessing at what it meant.
                        return Err(agent_client_protocol::Error::internal_error());
                    }

                    let session = connection
                        .send_request(NewSessionRequest::new(workspace))
                        .block_task()
                        .await?;
                    let session_id = session.session_id;

                    let answered = connection
                        .send_request(PromptRequest::new(
                            session_id.clone(),
                            vec![ContentBlock::Text(TextContent::new(prompt))],
                        ))
                        .block_task()
                        .await?;
                    Ok((session_id.to_string(), answered.stop_reason))
                },
            );

        let (session_id, stop_reason) = tokio::time::timeout(self.turn_deadline, exchange)
            .await
            .map_err(|_| AcpClientError::Timeout)?
            .map_err(|error| AcpClientError::Protocol(error.to_string()))?;

        let mut turn = collected
            .lock()
            .map_err(|_| AcpClientError::Protocol("the update collector was poisoned".to_owned()))?
            .clone();
        turn.session_id = session_id;
        serde_json::to_value(stop_reason)?
            .as_str()
            .unwrap_or("unknown")
            .clone_into(&mut turn.stop_reason);
        Ok(turn)
    }
}

/// Keep one update whole, and take the agent's own words out of it.
fn record(collected: &Arc<Mutex<AgentTurn>>, update: &SessionUpdate) {
    let Ok(mut turn) = collected.lock() else {
        return;
    };
    if let Ok(value) = serde_json::to_value(update) {
        turn.updates.push(value);
    }
    // Only the agent's message. A thought is not an answer, and a surface that concatenated the two
    // would present a draft as a conclusion.
    if let SessionUpdate::AgentMessageChunk(chunk) = update
        && let ContentBlock::Text(text) = &chunk.content
    {
        turn.message.push_str(&text.text);
    }
}

/// What the agent wanted, in enough detail for a person to recognise it.
fn describe(request: &RequestPermissionRequest) -> String {
    serde_json::to_value(&request.tool_call)
        .ok()
        .and_then(|value| {
            value
                .get("title")
                .or_else(|| value.get("toolCallId"))
                .and_then(|title| title.as_str().map(ToOwned::to_owned))
        })
        .unwrap_or_else(|| "an unnamed tool call".to_owned())
}
