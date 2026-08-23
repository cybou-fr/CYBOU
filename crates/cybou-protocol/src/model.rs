// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Asking a model for something, in a vocabulary that cannot ask it for authority (ADR-0021, ADR-0035).
//!
//! No inference runtime exists yet, and that is why this is worth writing now. A model runtime that
//! arrives before the shape of the request does gets whatever shape is convenient at the call site,
//! and every constraint the substrate spent its life establishing has to be re-imposed afterwards
//! against a working system. The types come first so the runtime has somewhere to land.
//!
//! ADR-0021's distinctions are normative and this module is built to make the wrong ones
//! unrepresentable rather than merely discouraged:
//!
//! ```text
//! model output      ≠ knowledge
//! model confidence  ≠ epistemic confidence
//! model proposal    ≠ authorization
//! ```
//!
//! So there is no variant of [`ModelOutput`] that asserts a fact and none that names an action. The
//! strongest thing a model can return is a candidate that something else has to accept — a
//! candidate act the deterministic parser did not produce, wording for a plan Mind already built,
//! a synthesis of passages that were already disclosed, or a desktop arrangement the desktop will
//! validate. A model cannot say "the disk is failing" through this interface, because there is no
//! field to say it in.
//!
//! ## `NoModel` is a configuration
//!
//! ADR-0021 says an installation may have no generative model at all, and that this is not a
//! degraded error state. A vocabulary can either hold that or quietly break it, and the way it
//! breaks it is by having tasks with no answer for the question "what happens without one?".
//! [`ModelTask::without_a_model`] forces every task to answer, and the answers are of exactly two
//! kinds: something deterministic already does this, or the feature is *absent*. Absent is not
//! degraded and not an error — it is a capability this installation does not have.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// What a model is being asked to do.
///
/// A closed set, versioned in the name. A task is a shape of request with a known input, a known
/// output and a known answer for what happens without a model — an open `String` task would be a
/// way to add all three without anybody reviewing them.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelTask {
    /// Propose a cognitive act for an utterance the deterministic vocabulary did not recognise.
    InterpretActV1,
    /// Word a plan Mind has already built.
    RealizeResponsePlanV1,
    /// Turn text into a vector for retrieval.
    EmbedTextV1,
    /// Reorder retrieved passages by how well they answer a query.
    RerankV1,
    /// Summarise passages that were already disclosed.
    SummarizeEvidenceV1,
    /// Propose an arrangement of the desktop.
    ProposeDesktopPlanV1,
    /// Propose an explanation for observed system behaviour.
    DiagnoseSystemV1,
}

/// Every task, so that a test can hold a property across all of them.
///
/// Written out rather than derived. A macro would keep this in step automatically and would also
/// mean a new task could be added without anybody deciding what it does on a machine with no model.
pub const ALL_MODEL_TASKS: &[ModelTask] = &[
    ModelTask::InterpretActV1,
    ModelTask::RealizeResponsePlanV1,
    ModelTask::EmbedTextV1,
    ModelTask::RerankV1,
    ModelTask::SummarizeEvidenceV1,
    ModelTask::ProposeDesktopPlanV1,
    ModelTask::DiagnoseSystemV1,
];

/// What happens for a task when the installation has no model.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WithoutAModel {
    /// Something deterministic already does this, and keeps doing it.
    ///
    /// The model is an improvement on an existing answer, never the only source of one. These are
    /// the tasks where installing a model changes how well Mind speaks, and removing it changes
    /// nothing about what Mind knows.
    Deterministic {
        /// What does it instead, named so the claim can be checked against the code.
        instead: &'static str,
    },
    /// The feature is absent.
    ///
    /// Not degraded, not an error, and not a stub returning something plausible: absent. A surface
    /// offering semantic search on a machine with no embedding model must say the machine cannot do
    /// it, because a search that silently falls back to matching filenames answers a different
    /// question than the one asked.
    Unavailable {
        /// What is not available, in the words a person would be told.
        feature: &'static str,
    },
}

impl ModelTask {
    /// What this task does on an installation with no model.
    ///
    /// Total by construction: adding a task without answering this does not compile.
    #[must_use]
    pub const fn without_a_model(self) -> WithoutAModel {
        match self {
            Self::InterpretActV1 => WithoutAModel::Deterministic {
                instead: "cybou_meaning::interpret, which refuses rather than guesses",
            },
            Self::RealizeResponsePlanV1 => WithoutAModel::Deterministic {
                instead: "cybou_meaning::realize, which is plainer and carries every qualification",
            },
            Self::EmbedTextV1 => WithoutAModel::Unavailable {
                feature: "semantic search",
            },
            Self::RerankV1 => WithoutAModel::Unavailable {
                feature: "reordering results by how well they answer",
            },
            Self::SummarizeEvidenceV1 => WithoutAModel::Unavailable {
                feature: "summaries of what was found",
            },
            Self::ProposeDesktopPlanV1 => WithoutAModel::Unavailable {
                feature: "arranging the desktop from a sentence",
            },
            Self::DiagnoseSystemV1 => WithoutAModel::Unavailable {
                feature: "explanations of system behaviour in prose",
            },
        }
    }

    /// Whether this task's answer is offered to a person as prose.
    ///
    /// The tasks where a fluent sentence could quietly acquire a claim, and so the ones whose
    /// output has to be checked against what Mind actually established before it is shown.
    #[must_use]
    pub const fn produces_prose(self) -> bool {
        matches!(
            self,
            Self::RealizeResponsePlanV1 | Self::SummarizeEvidenceV1 | Self::DiagnoseSystemV1
        )
    }
}

/// What a consumer needs of whatever answers.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelRequirements {
    /// Whether this may only be answered without leaving the device.
    pub local_only: bool,
    /// The most exposing thing this request is permitted to carry.
    pub sensitivity_ceiling: u8,
    /// How long the consumer will wait.
    pub max_latency_ms: u32,
    /// The most input tokens this request may carry.
    pub max_input_tokens: u32,
    /// The most output tokens the answer may run to.
    pub max_output_tokens: u32,
    /// The most resident memory the model may occupy.
    pub max_ram_mb: u32,
}

/// Where an answer would come from.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelRoute {
    /// Stable name of the worker or provider.
    pub provider: String,
    /// Whether reaching it leaves the device.
    pub external_boundary: bool,
    /// The most exposing class this route is permitted to receive.
    pub sensitivity_ceiling: u8,
    /// The tasks this route can answer.
    pub tasks: Vec<ModelTask>,
    /// The most input tokens it accepts.
    pub context_limit: u32,
}

/// Why a route may not answer a request.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RouteRefused {
    /// The request may not leave the device and this route does.
    LeavesTheDevice,
    /// The request carries something more exposing than the route may receive.
    AboveRouteCeiling {
        /// What the request carries.
        carried: u8,
        /// What the route may receive.
        ceiling: u8,
    },
    /// The request carries something more exposing than the consumer itself permitted.
    ///
    /// Separate from the route's ceiling because they are different failures with different
    /// culprits: one is a route chosen wrongly, the other is a request assembled wrongly, and a
    /// single reason would leave nobody able to tell which happened.
    AboveRequestCeiling {
        /// What the request carries.
        carried: u8,
        /// What the request itself declared as its limit.
        ceiling: u8,
    },
    /// This route does not do this task.
    DoesNotDoThisTask,
    /// The input is larger than the route will take.
    InputExceedsContext {
        /// What the request carries.
        tokens: u32,
        /// What the route accepts.
        limit: u32,
    },
}

/// What a model was handed.
///
/// Every variant is something that already crossed a disclosure boundary. There is no variant
/// holding a path, a query for a database, or a handle to anything — a model receives text that was
/// decided upon elsewhere, and cannot reach for more.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelInput {
    /// A sentence a person said, which the deterministic vocabulary did not recognise.
    Utterance {
        /// The sentence.
        text: String,
    },
    /// Text to be turned into a vector, or ranked, or summarised.
    Passages {
        /// The passages, in the order they were disclosed.
        passages: Vec<String>,
    },
    /// A plan Mind built, rendered plainly, to be worded better.
    ///
    /// The plain rendering rather than the plan, deliberately. A model handed the typed plan could
    /// read fields it was not meant to word; handed the sentences, the most it can do is rewrite
    /// them, and a rewrite that added a claim is detectable by comparing against what it was given.
    PlainRendering {
        /// What `realize` produced.
        rendered: String,
    },
}

/// What a model returned.
///
/// Every variant is a proposal. None asserts a fact, names an action, or carries a permission —
/// there is no field for any of those, which is the difference between a rule and a promise.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelOutput {
    /// A cognitive act the model thinks the sentence meant, for something else to accept or refuse.
    CandidateAct {
        /// The act family, by its frozen name.
        kind: String,
        /// What the model took the subject to be.
        subject: String,
    },
    /// Wording, to be checked against the plan it was meant to word.
    Wording {
        /// The prose.
        text: String,
    },
    /// A summary of passages that were already disclosed.
    Synthesis {
        /// The summary.
        text: String,
        /// Which of the supplied passages it drew on, by index.
        drew_on: Vec<u32>,
    },
    /// A vector.
    Embedding {
        /// The vector.
        vector: Vec<f32>,
    },
    /// An ordering of the supplied passages, by index, best first.
    Ranking {
        /// The order.
        order: Vec<u32>,
    },
    /// A proposed arrangement, named in a vocabulary the desktop validates.
    ///
    /// Strings rather than typed commands on purpose: this crate does not know what a desktop is,
    /// and a model that could construct the desktop's own command type would be constructing
    /// something the desktop is obliged to run. It parses them, and refuses what it cannot parse.
    DesktopPlanProposal {
        /// The proposed commands, in order.
        commands: Vec<String>,
    },
}

/// Which model answered, precisely enough to ask it again and get the same thing.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelIdentity {
    /// Family, as its publisher names it.
    pub family: String,
    /// Revision within the family.
    pub revision: String,
    /// SHA-256 of the artifact that ran.
    ///
    /// The field that makes attribution mean anything. A family and a revision name what somebody
    /// intended to install; only the digest says what actually answered.
    pub artifact_sha256: [u8; 32],
    /// How the weights were quantized, if they were.
    pub quantization: Option<String>,
    /// The runtime that executed it.
    pub backend: String,
    /// Which version of the prompt or template was used.
    ///
    /// Part of the identity of an answer, not an implementation detail: the same weights under a
    /// different template are a different thing to have asked.
    pub template_version: u32,
}

/// What one model artifact is and what it can be asked.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelManifest {
    /// Stable identifier for this artifact within the installation.
    pub model_id: String,
    /// What ran, or would run.
    pub identity: ModelIdentity,
    /// The tasks this artifact is declared to answer.
    pub tasks: Vec<ModelTask>,
    /// Its licence, as an SPDX identifier.
    pub license: String,
    /// The languages it is declared to handle.
    pub languages: Vec<String>,
    /// Resident memory it needs.
    pub min_ram_mb: u32,
    /// The most input tokens it accepts.
    pub context_limit: u32,
}

/// One request to a model.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelRequest {
    /// Unique identity of this request.
    pub request_id: Uuid,
    /// Who is asking, as the consumer is configured rather than as it calls itself.
    pub consumer: String,
    /// What is being asked.
    pub task: ModelTask,
    /// The disclosure this request's input was drawn from.
    ///
    /// Not optional. A model is a named consumer under ADR-0030, so everything it receives was
    /// supplied to it by a decision that is recorded — and a request that could omit this would be
    /// a way to hand a model context nobody recorded handing it.
    pub delivery: Uuid,
    /// What the consumer needs of whatever answers.
    pub requirements: ModelRequirements,
    /// What the model is handed.
    pub input: ModelInput,
    /// The most exposing thing the input carries.
    pub carries_sensitivity: u8,
    /// How many tokens the input is estimated at.
    pub input_tokens: u32,
}

/// What came back.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelResult {
    /// The request this answers.
    pub request_id: Uuid,
    /// Which model answered.
    pub answered_by: ModelIdentity,
    /// What it returned.
    pub output: ModelOutput,
    /// Input tokens actually consumed.
    pub input_tokens: u32,
    /// Output tokens actually produced.
    pub output_tokens: u32,
    /// How long it took.
    pub elapsed_ms: u32,
}

/// Whether this route may answer this request.
///
/// Checked in the order a reader would check it: whether it may leave the device at all, whether
/// the request is internally consistent, whether the route may receive what it carries, whether the
/// route does this task, and whether the input fits.
///
/// # Errors
///
/// Returns the first [`RouteRefused`] that applies.
pub fn admissible(route: &ModelRoute, request: &ModelRequest) -> Result<(), RouteRefused> {
    if request.requirements.local_only && route.external_boundary {
        return Err(RouteRefused::LeavesTheDevice);
    }
    if request.carries_sensitivity > request.requirements.sensitivity_ceiling {
        return Err(RouteRefused::AboveRequestCeiling {
            carried: request.carries_sensitivity,
            ceiling: request.requirements.sensitivity_ceiling,
        });
    }
    if request.carries_sensitivity > route.sensitivity_ceiling {
        return Err(RouteRefused::AboveRouteCeiling {
            carried: request.carries_sensitivity,
            ceiling: route.sensitivity_ceiling,
        });
    }
    if !route.tasks.contains(&request.task) {
        return Err(RouteRefused::DoesNotDoThisTask);
    }
    if request.input_tokens > route.context_limit
        || request.input_tokens > request.requirements.max_input_tokens
    {
        return Err(RouteRefused::InputExceedsContext {
            tokens: request.input_tokens,
            limit: route
                .context_limit
                .min(request.requirements.max_input_tokens),
        });
    }
    Ok(())
}

/// Whether a result may be attributed to the artifact that was selected.
///
/// The digest is compared, not the name. A worker that loaded a different file than the manifest
/// named would otherwise produce answers attributed to a model that never ran, and the attribution
/// surface — which exists so a person can ask "which model told me this?" — would confidently give
/// the wrong answer.
#[must_use]
pub fn attributable_to(result: &ModelResult, manifest: &ModelManifest) -> bool {
    result.answered_by.artifact_sha256 == manifest.identity.artifact_sha256
        && result.answered_by.template_version == manifest.identity.template_version
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: u8) -> [u8; 32] {
        [byte; 32]
    }

    fn identity(byte: u8) -> ModelIdentity {
        ModelIdentity {
            family: "ministral".to_owned(),
            revision: "3-3b".to_owned(),
            artifact_sha256: digest(byte),
            quantization: Some("Q4_K_M".to_owned()),
            backend: "llama.cpp".to_owned(),
            template_version: 1,
        }
    }

    fn manifest() -> ModelManifest {
        ModelManifest {
            model_id: "local.lite".to_owned(),
            identity: identity(0xAA),
            tasks: vec![ModelTask::InterpretActV1, ModelTask::RealizeResponsePlanV1],
            license: "Apache-2.0".to_owned(),
            languages: vec!["en".to_owned(), "ru".to_owned(), "fr".to_owned()],
            min_ram_mb: 4096,
            context_limit: 8192,
        }
    }

    fn local_route() -> ModelRoute {
        ModelRoute {
            provider: "local.llama".to_owned(),
            external_boundary: false,
            sensitivity_ceiling: 3,
            tasks: vec![ModelTask::InterpretActV1],
            context_limit: 8192,
        }
    }

    fn requirements() -> ModelRequirements {
        ModelRequirements {
            local_only: true,
            sensitivity_ceiling: 3,
            max_latency_ms: 4000,
            max_input_tokens: 4096,
            max_output_tokens: 512,
            max_ram_mb: 8192,
        }
    }

    fn request() -> ModelRequest {
        ModelRequest {
            request_id: Uuid::from_u128(1),
            consumer: "meaningd".to_owned(),
            task: ModelTask::InterpretActV1,
            delivery: Uuid::from_u128(2),
            requirements: requirements(),
            input: ModelInput::Utterance {
                text: "а можешь посмотреть, всё ли нормально с журналом?".to_owned(),
            },
            carries_sensitivity: 0,
            input_tokens: 32,
        }
    }

    #[test]
    fn every_task_says_what_happens_on_a_machine_with_no_model() {
        // ADR-0021: NoModel is a configuration, not a degraded error state. A vocabulary breaks
        // that by having a task with no answer here, and the break is invisible until somebody
        // installs Cybou on a machine that cannot run a model.
        for task in ALL_MODEL_TASKS {
            match task.without_a_model() {
                WithoutAModel::Deterministic { instead } => {
                    assert!(!instead.is_empty(), "{task:?}");
                }
                WithoutAModel::Unavailable { feature } => {
                    assert!(!feature.is_empty(), "{task:?}");
                }
            }
        }
    }

    #[test]
    fn the_two_things_mind_already_does_are_not_made_to_depend_on_a_model() {
        // Interpretation and realization exist deterministically today. If either became
        // `Unavailable`, installing a model would have become a prerequisite for Mind speaking at
        // all, which is the model-centric architecture ADR-0021 exists to refuse.
        assert!(matches!(
            ModelTask::InterpretActV1.without_a_model(),
            WithoutAModel::Deterministic { .. }
        ));
        assert!(matches!(
            ModelTask::RealizeResponsePlanV1.without_a_model(),
            WithoutAModel::Deterministic { .. }
        ));
    }

    #[test]
    fn nothing_a_model_can_return_asserts_a_fact_or_names_an_action() {
        // The rule stated where it can be checked rather than promised in a comment. Every variant
        // is a candidate for something else to accept: none carries a truth value, a permission, a
        // command to run, or a path to touch.
        //
        // This test is a reading of the type. It fails when somebody adds a variant, which is the
        // point: the addition should be a decision, not an edit.
        let outputs = [
            ModelOutput::CandidateAct {
                kind: "verify".to_owned(),
                subject: "the journal".to_owned(),
            },
            ModelOutput::Wording {
                text: String::new(),
            },
            ModelOutput::Synthesis {
                text: String::new(),
                drew_on: Vec::new(),
            },
            ModelOutput::Embedding { vector: Vec::new() },
            ModelOutput::Ranking { order: Vec::new() },
            ModelOutput::DesktopPlanProposal {
                commands: Vec::new(),
            },
        ];
        assert_eq!(
            outputs.len(),
            6,
            "a variant was added or removed; check that it is still a proposal and not an assertion"
        );
    }

    #[test]
    fn a_request_that_may_not_leave_the_device_cannot_be_answered_by_something_that_does() {
        let remote = ModelRoute {
            provider: "remote.somebody".to_owned(),
            external_boundary: true,
            sensitivity_ceiling: u8::MAX,
            tasks: vec![ModelTask::InterpretActV1],
            context_limit: 128_000,
        };
        assert_eq!(
            admissible(&remote, &request()),
            Err(RouteRefused::LeavesTheDevice)
        );
    }

    #[test]
    fn a_request_carrying_more_than_it_declared_is_refused_before_any_route_is_considered() {
        // The request is wrong, not the route. Reporting this as a route problem would send
        // somebody looking at the routing table for a bug in whoever assembled the input.
        let mut over = request();
        over.carries_sensitivity = 9;
        assert_eq!(
            admissible(&local_route(), &over),
            Err(RouteRefused::AboveRequestCeiling {
                carried: 9,
                ceiling: 3,
            })
        );
    }

    #[test]
    fn a_route_permitted_less_than_the_request_carries_is_refused_separately() {
        let mut careful = local_route();
        careful.sensitivity_ceiling = 0;
        let mut carrying = request();
        carrying.carries_sensitivity = 2;
        assert_eq!(
            admissible(&careful, &carrying),
            Err(RouteRefused::AboveRouteCeiling {
                carried: 2,
                ceiling: 0,
            })
        );
    }

    #[test]
    fn a_route_is_not_asked_for_something_it_does_not_do() {
        let mut summarising = request();
        summarising.task = ModelTask::SummarizeEvidenceV1;
        assert_eq!(
            admissible(&local_route(), &summarising),
            Err(RouteRefused::DoesNotDoThisTask)
        );
    }

    #[test]
    fn the_tighter_of_the_two_input_limits_is_the_one_that_binds() {
        // The consumer's budget and the route's context are different limits, and a request must
        // respect both. Reporting the route's limit when the consumer's was smaller would send
        // somebody to enlarge the wrong number.
        let mut large = request();
        large.input_tokens = 6000;
        assert_eq!(
            admissible(&local_route(), &large),
            Err(RouteRefused::InputExceedsContext {
                tokens: 6000,
                limit: 4096,
            })
        );
    }

    #[test]
    fn an_admissible_request_is_admitted() {
        // The control. Every test above passes on a function that refuses everything.
        assert_eq!(admissible(&local_route(), &request()), Ok(()));
    }

    #[test]
    fn an_answer_is_attributed_by_what_ran_and_not_by_what_was_named() {
        // A worker that loaded a different file than the manifest named would otherwise produce
        // answers attributed to a model that never ran, and the surface a person uses to ask which
        // model told them something would confidently give the wrong answer.
        let honest = ModelResult {
            request_id: Uuid::from_u128(1),
            answered_by: identity(0xAA),
            output: ModelOutput::Wording {
                text: "…".to_owned(),
            },
            input_tokens: 32,
            output_tokens: 8,
            elapsed_ms: 300,
        };
        assert!(attributable_to(&honest, &manifest()));

        let other_weights = ModelResult {
            answered_by: identity(0xBB),
            ..honest.clone()
        };
        assert!(!attributable_to(&other_weights, &manifest()));

        // Same weights, different template. Still a different thing to have asked.
        let other_template = ModelResult {
            answered_by: ModelIdentity {
                template_version: 2,
                ..identity(0xAA)
            },
            ..honest
        };
        assert!(!attributable_to(&other_template, &manifest()));
    }

    #[test]
    fn the_tasks_that_reach_a_person_as_prose_are_the_ones_marked_as_such() {
        // The set whose output can quietly acquire a claim, so the set whose output has to be
        // checked against what Mind established before anyone reads it.
        for task in ALL_MODEL_TASKS {
            let expected = matches!(
                task,
                ModelTask::RealizeResponsePlanV1
                    | ModelTask::SummarizeEvidenceV1
                    | ModelTask::DiagnoseSystemV1
            );
            assert_eq!(task.produces_prose(), expected, "{task:?}");
        }
    }

    #[test]
    fn a_request_and_its_result_survive_the_wire() {
        let mut encoded = Vec::new();
        ciborium::into_writer(&request(), &mut encoded).expect("a request encodes");
        let decoded: ModelRequest =
            ciborium::from_reader(encoded.as_slice()).expect("a request decodes");
        assert_eq!(decoded, request());
        assert_eq!(
            decoded.delivery,
            Uuid::from_u128(2),
            "a request must always name the disclosure its input came from"
        );
    }
}
