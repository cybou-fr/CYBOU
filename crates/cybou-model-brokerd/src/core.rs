// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Choosing who answers, holding them to the budget, and attributing what comes back.

use std::sync::Mutex;

use cybou_protocol::model::{
    ModelIdentity, ModelRequest, ModelResult, ModelRoute, ModelTask, RouteRefused, WithoutAModel,
    admissible, attributable_to,
};

use crate::worker::{Worker, WorkerFailed};

/// One backend, and the route it is reachable by.
pub struct Registered {
    /// How this worker may be reached and what it may receive.
    pub route: ModelRoute,
    /// The worker itself.
    pub worker: Box<dyn Worker>,
}

/// Why the broker did not answer.
#[derive(Clone, Debug, PartialEq)]
pub enum BrokerRefused {
    /// This installation has no model that does this.
    ///
    /// Carries what happens instead, so a caller is told what to do rather than only that it cannot
    /// have what it asked for. ADR-0021: `NoModel` is a configuration, and a configuration that
    /// answers "no" without saying "and here is the deterministic thing that does it" would make it
    /// feel like a fault.
    NoModelFor {
        /// The task nothing here answers.
        task: ModelTask,
        /// What happens on this installation instead.
        instead: WithoutAModel,
    },
    /// Every route that does this task refused the request, for these reasons.
    ///
    /// Plural on purpose. One route refusing because the request may not leave the device and
    /// another because it carries too much are different problems, and collapsing them into "no
    /// route available" leaves an operator with nothing to act on.
    EveryRouteRefused {
        /// Each route that was considered, and why it would not do.
        reasons: Vec<(String, RouteRefused)>,
    },
    /// The worker that was chosen could not answer.
    WorkerFailed {
        /// Which one.
        provider: String,
        /// Why.
        failure: WorkerFailed,
    },
    /// The worker answered as a model that is not the one it declared.
    ///
    /// Refused rather than passed on with a warning. An answer attributed to an artifact that did
    /// not produce it is worse than no answer: the surface a person uses to ask which model told
    /// them something would confidently name the wrong one, and nothing downstream could tell.
    NotAttributable {
        /// What the registered manifest said would run.
        declared: Box<ModelIdentity>,
        /// What answered.
        answered: Box<ModelIdentity>,
    },
}

impl core::fmt::Display for BrokerRefused {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NoModelFor { task, instead } => match instead {
                WithoutAModel::Deterministic { instead } => write!(
                    formatter,
                    "no model does {task:?} here; {instead} does it instead"
                ),
                WithoutAModel::Unavailable { feature } => write!(
                    formatter,
                    "no model does {task:?} here, so {feature} is not available on this machine"
                ),
            },
            Self::EveryRouteRefused { reasons } => {
                write!(formatter, "{} route(s) refused this request", reasons.len())
            }
            Self::WorkerFailed { provider, failure } => write!(formatter, "{provider}: {failure}"),
            Self::NotAttributable { declared, answered } => write!(
                formatter,
                "answered by {}/{} but {}/{} was registered",
                answered.family, answered.revision, declared.family, declared.revision
            ),
        }
    }
}

impl core::error::Error for BrokerRefused {}

/// What the broker did with one request, for the record.
#[derive(Clone, Debug, PartialEq)]
pub struct Attempt {
    /// The request.
    pub request_id: uuid::Uuid,
    /// What was asked.
    pub task: ModelTask,
    /// Which route answered, if any did.
    pub answered_by: Option<String>,
    /// Whether it left the device.
    pub crossed_a_boundary: bool,
}

/// The faculty: it selects, it enforces, it attributes, and it owns nothing else.
///
/// It holds no biography, reads no Journal, touches no filesystem, authorizes nothing and executes
/// nothing. That is a claim about the code and it is checkable: this crate depends on the protocol
/// and the fabric, and on no organ. `scripts/validate-organ-layering.py` fails if that changes.
#[derive(Default)]
pub struct BrokerCore {
    workers: Vec<Registered>,
    /// The last few attempts, so the faculty can say what it has been doing.
    ///
    /// Bounded, and holding no input or output — a broker that kept prompts would be a second
    /// memory with different rules, which is the thing ADR-0029 spent a whole decision refusing.
    attempts: Mutex<Vec<Attempt>>,
}

/// How many attempts the faculty remembers.
const REMEMBERED_ATTEMPTS: usize = 64;

impl BrokerCore {
    /// A broker with no models. The ordinary state of a fresh installation.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            workers: Vec::new(),
            attempts: Mutex::new(Vec::new()),
        }
    }

    /// Register a backend.
    pub fn register(&mut self, route: ModelRoute, worker: Box<dyn Worker>) {
        self.workers.push(Registered { route, worker });
    }

    /// Whether anything at all can answer.
    #[must_use]
    pub fn has_a_model(&self) -> bool {
        !self.workers.is_empty()
    }

    /// The tasks this installation can currently answer.
    #[must_use]
    pub fn answerable_tasks(&self) -> Vec<ModelTask> {
        let mut tasks: Vec<ModelTask> = self
            .workers
            .iter()
            .flat_map(|registered| registered.route.tasks.iter().copied())
            .collect();
        tasks.sort_unstable();
        tasks.dedup();
        tasks
    }

    /// What the faculty has been doing lately.
    #[must_use]
    pub fn recent_attempts(&self) -> Vec<Attempt> {
        self.attempts
            .lock()
            .map(|held| held.clone())
            .unwrap_or_default()
    }

    /// Answer a request, or refuse it with a reason somebody can act on.
    ///
    /// Routes are considered in the order they were registered, so the same installation and the
    /// same request always choose the same worker. A broker that picked by measured latency would
    /// answer differently on a busy machine, and two answers a person could not compare is a worse
    /// outcome than a slower one.
    ///
    /// # Errors
    ///
    /// Returns [`BrokerRefused`] naming which of the four things went wrong: nothing here does this
    /// task, every route that does refused, the chosen worker failed, or what answered was not what
    /// was registered.
    pub fn submit(&self, request: &ModelRequest) -> Result<ModelResult, BrokerRefused> {
        let candidates: Vec<&Registered> = self
            .workers
            .iter()
            .filter(|registered| registered.route.tasks.contains(&request.task))
            .collect();

        if candidates.is_empty() {
            // Distinct from "every route refused". Nothing here does this at all, which is a fact
            // about the installation rather than about this request, and the caller's remedy is a
            // different one: install something, or use what does it instead.
            return Err(BrokerRefused::NoModelFor {
                task: request.task,
                instead: request.task.without_a_model(),
            });
        }

        let mut reasons = Vec::new();
        for registered in candidates {
            match admissible(&registered.route, request) {
                Ok(()) => return self.ask(registered, request),
                Err(refused) => reasons.push((registered.route.provider.clone(), refused)),
            }
        }
        Err(BrokerRefused::EveryRouteRefused { reasons })
    }

    /// Put the request to one worker and check what comes back is what was registered.
    fn ask(
        &self,
        registered: &Registered,
        request: &ModelRequest,
    ) -> Result<ModelResult, BrokerRefused> {
        let manifest = registered.worker.manifest();
        let output =
            registered
                .worker
                .answer(request)
                .map_err(|failure| BrokerRefused::WorkerFailed {
                    provider: registered.route.provider.clone(),
                    failure,
                })?;

        let result = ModelResult {
            request_id: request.request_id,
            answered_by: manifest.identity.clone(),
            output,
            input_tokens: request.input_tokens,
            output_tokens: 0,
            elapsed_ms: 0,
        };

        // Belt and braces: the identity was taken from the manifest a line ago, so this can only
        // fail if a worker's manifest changed between the two. That is exactly the case worth
        // catching — a worker that swaps its artifact under the broker is how an answer comes to be
        // attributed to something that never ran.
        if !attributable_to(&result, manifest) {
            return Err(BrokerRefused::NotAttributable {
                declared: Box::new(manifest.identity.clone()),
                answered: Box::new(result.answered_by),
            });
        }

        self.note(Attempt {
            request_id: request.request_id,
            task: request.task,
            answered_by: Some(registered.route.provider.clone()),
            crossed_a_boundary: registered.route.external_boundary,
        });
        Ok(result)
    }

    /// Remember one attempt, bounded.
    fn note(&self, attempt: Attempt) {
        if let Ok(mut attempts) = self.attempts.lock() {
            attempts.push(attempt);
            while attempts.len() > REMEMBERED_ATTEMPTS {
                attempts.remove(0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use cybou_protocol::model::{
        ModelInput, ModelManifest, ModelOutput, ModelRequirements, ModelTask,
    };
    use uuid::Uuid;

    use super::*;

    /// A worker that answers, so the broker can be exercised without an inference runtime.
    struct Stub {
        manifest: ModelManifest,
        answer: Result<ModelOutput, WorkerFailed>,
    }

    impl Worker for Stub {
        fn manifest(&self) -> &ModelManifest {
            &self.manifest
        }

        fn answer(&self, _request: &ModelRequest) -> Result<ModelOutput, WorkerFailed> {
            self.answer.clone()
        }
    }

    fn identity(byte: u8) -> ModelIdentity {
        ModelIdentity {
            family: "stub".to_owned(),
            revision: "1".to_owned(),
            artifact_sha256: [byte; 32],
            quantization: None,
            backend: "test".to_owned(),
            template_version: 1,
        }
    }

    fn manifest(byte: u8) -> ModelManifest {
        ModelManifest {
            model_id: "stub".to_owned(),
            identity: identity(byte),
            tasks: vec![ModelTask::InterpretActV1],
            license: "MIT".to_owned(),
            languages: vec!["en".to_owned()],
            min_ram_mb: 1,
            context_limit: 8192,
        }
    }

    fn route(provider: &str, external: bool) -> ModelRoute {
        ModelRoute {
            provider: provider.to_owned(),
            external_boundary: external,
            sensitivity_ceiling: 3,
            tasks: vec![ModelTask::InterpretActV1],
            context_limit: 8192,
        }
    }

    fn wording() -> ModelOutput {
        ModelOutput::CandidateAct {
            kind: "verify".to_owned(),
            subject: "the journal".to_owned(),
        }
    }

    fn stub(byte: u8, answer: Result<ModelOutput, WorkerFailed>) -> Box<dyn Worker> {
        Box::new(Stub {
            manifest: manifest(byte),
            answer,
        })
    }

    fn request() -> ModelRequest {
        ModelRequest {
            request_id: Uuid::from_u128(1),
            consumer: "meaningd".to_owned(),
            task: ModelTask::InterpretActV1,
            delivery: Uuid::from_u128(2),
            requirements: ModelRequirements {
                local_only: true,
                sensitivity_ceiling: 3,
                max_latency_ms: 4000,
                max_input_tokens: 4096,
                max_output_tokens: 512,
                max_ram_mb: 8192,
            },
            input: ModelInput::Utterance {
                text: "всё ли нормально с журналом?".to_owned(),
            },
            carries_sensitivity: 0,
            input_tokens: 32,
        }
    }

    fn broker_with(workers: Vec<(ModelRoute, Box<dyn Worker>)>) -> BrokerCore {
        let mut broker = BrokerCore::new();
        for (route, worker) in workers {
            broker.register(route, worker);
        }
        broker
    }

    #[test]
    fn an_installation_with_no_model_says_what_happens_instead() {
        // ADR-0021. A faculty that answered "unavailable" and stopped would make NoModel feel like
        // a fault; the caller's actual position is that something deterministic already does this.
        let broker = BrokerCore::new();
        assert!(!broker.has_a_model());

        let refusal = broker.submit(&request()).expect_err("nothing is installed");
        match refusal {
            BrokerRefused::NoModelFor { task, instead } => {
                assert_eq!(task, ModelTask::InterpretActV1);
                assert!(matches!(instead, WithoutAModel::Deterministic { .. }));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn nothing_here_doing_a_task_is_a_different_refusal_from_every_route_refusing() {
        // Different facts with different remedies: install something, versus fix the request or the
        // routing table. One reason for both would leave an operator with nowhere to start.
        let broker = broker_with(vec![(route("local", false), stub(0xAA, Ok(wording())))]);

        let mut summarising = request();
        summarising.task = ModelTask::SummarizeEvidenceV1;
        assert!(matches!(
            broker.submit(&summarising),
            Err(BrokerRefused::NoModelFor { .. })
        ));

        let mut too_much = request();
        too_much.carries_sensitivity = 9;
        assert!(matches!(
            broker.submit(&too_much),
            Err(BrokerRefused::EveryRouteRefused { .. })
        ));
    }

    #[test]
    fn a_request_that_may_not_leave_the_device_is_not_answered_by_something_that_does() {
        // MB1 and MB3 at the point they are actually enforced. The only route that does this task
        // is external, and the request is local-only.
        let broker = broker_with(vec![(route("remote", true), stub(0xAA, Ok(wording())))]);
        match broker.submit(&request()) {
            Err(BrokerRefused::EveryRouteRefused { reasons }) => {
                assert_eq!(reasons.len(), 1);
                assert_eq!(reasons[0].1, RouteRefused::LeavesTheDevice);
            }
            other => panic!("{other:?}"),
        }
        assert!(
            broker.recent_attempts().is_empty(),
            "a refused request was recorded as an attempt"
        );
    }

    #[test]
    fn the_first_admissible_route_answers_and_the_same_one_always_does() {
        // Deterministic selection. A broker that picked by measured latency would answer
        // differently on a busy machine, and two answers a person cannot compare is worse than a
        // slower one.
        let broker = broker_with(vec![
            (route("remote", true), stub(0xAA, Ok(wording()))),
            (route("local-a", false), stub(0xBB, Ok(wording()))),
            (route("local-b", false), stub(0xCC, Ok(wording()))),
        ]);

        for _ in 0..8 {
            let result = broker
                .submit(&request())
                .expect("a local route is admissible");
            assert_eq!(result.answered_by.artifact_sha256, [0xBB; 32]);
        }
    }

    #[test]
    fn a_worker_that_cannot_answer_is_a_capability_deficit_and_not_a_broken_broker() {
        // MB6. The faculty says which provider failed and why, and remains able to answer the next
        // request. Nothing about identity, biography or policy is involved in a model being down.
        let broker = broker_with(vec![(
            route("local-down", false),
            stub(0xAA, Err(WorkerFailed::NotReady)),
        )]);

        match broker.submit(&request()) {
            Err(BrokerRefused::WorkerFailed { provider, failure }) => {
                assert_eq!(provider, "local-down");
                assert_eq!(failure, WorkerFailed::NotReady);
            }
            other => panic!("{other:?}"),
        }
        assert!(broker.has_a_model(), "a model being down uninstalled it");
    }

    #[test]
    fn an_answer_is_attributed_to_the_artifact_that_was_registered() {
        // MB4. The digest travels out with the answer, so "which model told me this?" has an exact
        // answer rather than the name of whatever was configured.
        let broker = broker_with(vec![(route("local", false), stub(0xAA, Ok(wording())))]);
        let result = broker.submit(&request()).expect("admissible");
        assert_eq!(result.answered_by.artifact_sha256, [0xAA; 32]);
        assert_eq!(result.request_id, Uuid::from_u128(1));
    }

    #[test]
    fn what_the_faculty_remembers_is_that_it_answered_and_never_what_it_was_asked() {
        // A broker that kept prompts would be a second memory with different rules — the thing
        // ADR-0029 spent a whole decision refusing. What it keeps is enough to say what it has been
        // doing and not enough to reconstruct anything anyone said.
        let broker = broker_with(vec![(route("local", false), stub(0xAA, Ok(wording())))]);
        broker.submit(&request()).expect("admissible");

        let attempts = broker.recent_attempts();
        assert_eq!(attempts.len(), 1);
        assert_eq!(attempts[0].answered_by.as_deref(), Some("local"));
        assert!(!attempts[0].crossed_a_boundary);

        let recorded = format!("{attempts:?}");
        assert!(
            !recorded.contains("журнал"),
            "the faculty kept what it was asked: {recorded}"
        );
    }

    #[test]
    fn what_it_remembers_is_bounded() {
        let broker = broker_with(vec![(route("local", false), stub(0xAA, Ok(wording())))]);
        for index in 0..(REMEMBERED_ATTEMPTS * 3) {
            let mut one = request();
            one.request_id = Uuid::from_u128(index as u128);
            broker.submit(&one).expect("admissible");
        }
        assert_eq!(broker.recent_attempts().len(), REMEMBERED_ATTEMPTS);
    }

    #[test]
    fn the_faculty_can_say_which_tasks_this_installation_can_answer() {
        // What a surface needs to stop offering a feature that cannot work here, rather than
        // offering it and failing when somebody uses it.
        let empty = BrokerCore::new();
        assert!(empty.answerable_tasks().is_empty());

        let broker = broker_with(vec![
            (route("a", false), stub(0xAA, Ok(wording()))),
            (route("b", false), stub(0xBB, Ok(wording()))),
        ]);
        assert_eq!(broker.answerable_tasks(), vec![ModelTask::InterpretActV1]);
    }

    #[test]
    fn every_route_that_refused_says_why_it_did() {
        // Plural on purpose: one route refusing for distance and another for sensitivity are
        // different problems, and "no route available" leaves nothing to act on.
        let mut careful = route("local-careful", false);
        careful.sensitivity_ceiling = 0;
        let broker = broker_with(vec![
            (route("remote", true), stub(0xAA, Ok(wording()))),
            (careful, stub(0xBB, Ok(wording()))),
        ]);

        let mut carrying = request();
        carrying.carries_sensitivity = 2;
        match broker.submit(&carrying) {
            Err(BrokerRefused::EveryRouteRefused { reasons }) => {
                assert_eq!(reasons.len(), 2);
                assert_eq!(reasons[0].1, RouteRefused::LeavesTheDevice);
                assert_eq!(
                    reasons[1].1,
                    RouteRefused::AboveRouteCeiling {
                        carried: 2,
                        ceiling: 0,
                    }
                );
            }
            other => panic!("{other:?}"),
        }
    }
}
