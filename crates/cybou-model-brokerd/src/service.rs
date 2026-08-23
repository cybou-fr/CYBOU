// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! D-Bus `org.cybou.Faculty.ModelBroker1` service implementation on zbus.

// The `#[interface]` expansion emits part of its dispatch surface with the attribute's own span,
// which an `allow` on the impl block cannot reach. Every handler written here is documented.
#![allow(missing_docs)]

use std::sync::Arc;

use cybou_protocol::model::ModelRequest;
use zbus::interface;

use crate::BrokerCore;

/// D-Bus service exporting `org.cybou.Faculty.ModelBroker1`.
pub struct ModelBroker1Service {
    core: Arc<BrokerCore>,
}

impl ModelBroker1Service {
    /// Create a new handler around a broker.
    #[must_use]
    pub fn new(core: Arc<BrokerCore>) -> Self {
        Self { core }
    }
}

#[allow(
    clippy::unused_async,
    reason = "zbus dispatches every exported handler as a future"
)]
#[interface(name = "org.cybou.Faculty.ModelBroker1")]
impl ModelBroker1Service {
    /// Whether this faculty is ready to be asked.
    ///
    /// True with no model installed. Readiness is about the faculty, not about whether a model
    /// happens to be present: an installation with none is a supported configuration, and reporting
    /// it as not-ready would make a valid deployment look broken to every control plane.
    async fn ready(&self) -> bool {
        true
    }

    /// Overall health.
    ///
    /// A faculty with no model is healthy, because nothing is wrong with it. What it cannot do is
    /// reported through `AnswerableTasks`, where a surface can act on it, rather than through a
    /// health state that would put a permanent warning on a working machine.
    async fn health(&self) -> String {
        "healthy".to_string()
    }

    /// Last error diagnostic.
    async fn last_error(&self) -> String {
        String::new()
    }

    /// Whether any model at all is installed.
    async fn has_a_model(&self) -> bool {
        self.core.has_a_model()
    }

    /// The tasks this installation can answer, as CBOR.
    ///
    /// So a surface can stop offering a feature that cannot work here, rather than offering it and
    /// failing when somebody uses it.
    async fn answerable_tasks(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        let _ = ciborium::into_writer(&self.core.answerable_tasks(), &mut buf);
        buf
    }

    /// Put one request, returning a CBOR `ModelResult` or a reason it was refused.
    ///
    /// The boolean is separate from the payload deliberately. A refusal arriving as an empty-looking
    /// success is how a caller comes to render "the model said nothing" for something the broker
    /// never asked anybody.
    async fn submit(&self, request: Vec<u8>) -> (bool, Vec<u8>, String) {
        let Ok(request) = ciborium::from_reader::<ModelRequest, _>(request.as_slice()) else {
            return (false, Vec::new(), "unreadable request".to_owned());
        };
        match self.core.submit(&request) {
            Ok(result) => {
                let mut buf = Vec::new();
                let _ = ciborium::into_writer(&result, &mut buf);
                (true, buf, String::new())
            }
            Err(refused) => (false, Vec::new(), refused.to_string()),
        }
    }
}
