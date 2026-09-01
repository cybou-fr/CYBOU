// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! The `org.cybou.Runtime.Agent1` surface: launch, hold and stop agent sessions.
//!
//! ## Launch accepts a selection, never authority
//!
//! `Launch` takes a profile id, agent, workspace, offered model class and initial prompt. It takes no
//! memory, CPU, task, lifetime, host, spending or token bound. The launcher reads the root-owned
//! profile catalogue itself and derives every grant from it, so a reachable caller chooses among
//! offers and cannot manufacture a `CapsuleGrant`.
//!
//! Host admission happens here, under the same registry lock that inserts the launching session.
//! This is the owner boundary the CLI could not provide: two simultaneous callers cannot both be
//! told the same remaining capacity is theirs.
//!
//! ## Stopping is stopping units, not asking an agent
//!
//! `Stop` runs the session's teardown. It does not send anything to the agent and does not wait for
//! it to agree — the capsule is a cgroup with a kill switch, and a boundary made of requests is not
//! one.
//!
//! ## A session leaves the registry when it is gone, not when it was asked to go
//!
//! The order matters and the earlier version had it wrong. It took the session out of the registry
//! first and tore it down afterwards, so a capsule that refused to die was a capsule this owner had
//! forgotten: absent from every listing, with no surface offering to stop it, and an agent still
//! working inside it. The one failure a person could not recover from was the one reported as
//! success.
//!
//! So: the reason is fixed, the units are terminated, the host is asked whether they are actually
//! gone, and only then is the session forgotten. An ending that cannot be confirmed leaves the
//! session listed and answers `false`, which is the truthful thing to tell a caller whose request
//! did not take effect.
//!
//! The reason is fixed **before** the teardown, because a session torn down first and labelled
//! afterwards could be marked expired if the clock ran out in between, which would replace a
//! person's decision with a timer. A confirmed ending releases its live reservation and leaves a
//! bounded final view that a person can still read.

#![allow(missing_docs)]

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use cybou_capsule::KernelCapsuleSpec;
use cybou_fabric::encode;
use cybou_protocol::agent::LaunchRequest;
use time::OffsetDateTime;
use uuid::Uuid;
use zbus::{fdo, interface};

use crate::registry::SessionRegistry;
use crate::session::SessionEnd;
use crate::{HostCapacity, LiveSession, SessionPlan, session::Session, view::Ledger};

/// One request after the operator's profile has supplied every bound and the launch has been
/// planned, but before anything has touched the host.
pub struct PreparedLaunch {
    /// The complete owner-derived session plan.
    pub plan: SessionPlan,
    /// The kernel boundary compiled from the same lease.
    pub spec: KernelCapsuleSpec,
    /// The initial task, which is content rather than authority.
    pub prompt: String,
}

/// The host-dependent half of launching.
///
/// Profile reading and planning live behind this trait as well as execution so tests can prove the
/// D-Bus owner's admission and lifecycle decisions without starting service-manager units.
pub trait Launcher: Send + Sync {
    /// Resolve the caller's selection only through operator-approved profiles.
    ///
    /// # Errors
    ///
    /// Returns an error string if profile resolution or boundary admission fails.
    fn prepare(
        &self,
        request: &LaunchRequest,
        now: OffsetDateTime,
    ) -> Result<PreparedLaunch, String>;

    /// Begin the already-admitted launch and arrange its lifecycle updates.
    ///
    /// # Errors
    ///
    /// Returns an error string if process spawning or capsule initialization fails.
    fn start(
        &self,
        prepared: PreparedLaunch,
        registry: Arc<Mutex<SessionRegistry>>,
    ) -> Result<(), String>;

    /// Return the catalogue of operator-approved profiles and host readiness.
    ///
    /// # Errors
    ///
    /// Returns an error string if profile catalogues cannot be read.
    fn offers(&self) -> Result<cybou_protocol::agent::AgentOffersResponse, String> {
        Ok(cybou_protocol::agent::AgentOffersResponse::default())
    }
}

/// What the surface needs in order to end a session on the host.
///
/// A trait rather than a direct call, so the decision to run teardown stays testable without a
/// service manager, and so this file holds no policy about how a unit is stopped.
pub trait Teardown: Send + Sync {
    /// End everything one session put on the host, and say whether it ended.
    ///
    /// The answer is not decoration. A session removed from the registry on the strength of having
    /// *asked* would be a capsule still running that this owner has forgotten — no listing showing
    /// it, no surface offering to stop it, and an agent working on inside. So the caller is told,
    /// and an unproven ending keeps the session where a person can still see it.
    fn tear_down(&self, plan: &crate::plan::SessionPlan) -> Ended;
}

/// Host boundary that changes and verifies one capsule's cgroup freeze state.
pub trait KernelControl: Send + Sync {
    /// Apply the requested state and return only after the kernel reading agrees.
    fn set_freeze_state(&self, unit: &str, state: crate::telemetry::FreezeState) -> bool;

    /// Independently read whether the requested freeze state still holds.
    fn freeze_state_is(&self, unit: &str, state: crate::telemetry::FreezeState) -> bool;

    /// Stop the egress broker and return only after systemd reports it inactive.
    fn revoke_egress(&self, unit: &str) -> bool;

    /// Stop the model gateway and prove its socket and bearer disappeared with its runtime dir.
    fn revoke_model(&self, unit: &str, artifacts: &[std::path::PathBuf]) -> bool;
}

struct SystemdKernelControl;

impl KernelControl for SystemdKernelControl {
    fn set_freeze_state(&self, unit: &str, state: crate::telemetry::FreezeState) -> bool {
        crate::telemetry::set_freeze_state(unit, state)
    }

    fn freeze_state_is(&self, unit: &str, state: crate::telemetry::FreezeState) -> bool {
        crate::telemetry::freeze_state_is(unit, state)
    }

    fn revoke_egress(&self, unit: &str) -> bool {
        crate::telemetry::revoke_egress(unit)
    }

    fn revoke_model(&self, unit: &str, artifacts: &[std::path::PathBuf]) -> bool {
        crate::telemetry::revoke_model(unit, artifacts)
    }
}

/// Whether a session's units are actually gone.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Ended {
    /// Every unit this session started is confirmed inactive.
    Confirmed,
    /// Something is still running, or could not be asked about.
    ///
    /// Not a failure to report and forget. It is the case where the truthful thing is to keep
    /// showing the session, because it is still there.
    Unproven,
}

/// Process-owned Agent1 dispatch surface.
pub struct Agent1Service {
    registry: Arc<Mutex<SessionRegistry>>,
    teardown: Arc<dyn Teardown>,
    kernel_control: Arc<dyn KernelControl>,
    control_gates: Mutex<HashMap<Uuid, Arc<tokio::sync::Mutex<()>>>>,
    launch: Option<(HostCapacity, Arc<dyn Launcher>)>,
}

impl Agent1Service {
    /// Serve this registry.
    #[must_use]
    pub fn new(registry: Arc<Mutex<SessionRegistry>>, teardown: Arc<dyn Teardown>) -> Self {
        Self {
            registry,
            teardown,
            kernel_control: Arc::new(SystemdKernelControl),
            control_gates: Mutex::new(HashMap::new()),
            launch: None,
        }
    }

    /// Serve this registry and own new launches against one bounded host capacity.
    #[must_use]
    pub fn with_launch(
        registry: Arc<Mutex<SessionRegistry>>,
        teardown: Arc<dyn Teardown>,
        capacity: HostCapacity,
        launcher: Arc<dyn Launcher>,
    ) -> Self {
        Self {
            registry,
            teardown,
            kernel_control: Arc::new(SystemdKernelControl),
            control_gates: Mutex::new(HashMap::new()),
            launch: Some((capacity, launcher)),
        }
    }

    #[cfg(test)]
    fn with_kernel_control(mut self, kernel_control: Arc<dyn KernelControl>) -> Self {
        self.kernel_control = kernel_control;
        self
    }

    fn control_gate(&self, capsule_id: Uuid) -> fdo::Result<Arc<tokio::sync::Mutex<()>>> {
        let mut gates = self.control_gates.lock().map_err(|_| {
            fdo::Error::Failed("the capsule control gates are unavailable".to_owned())
        })?;
        Ok(Arc::clone(
            gates
                .entry(capsule_id)
                .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(()))),
        ))
    }

    async fn change_freeze_state(
        &self,
        capsule_id: Uuid,
        state: crate::telemetry::FreezeState,
    ) -> fdo::Result<bool> {
        let gate = self.control_gate(capsule_id)?;
        let _control = gate.lock().await;
        let unit = {
            let registry = self.registry.lock().map_err(|_| {
                fdo::Error::Failed("the session registry is unavailable".to_owned())
            })?;
            let Some(live) = registry.get(capsule_id) else {
                return Ok(false);
            };
            if matches!(
                live.session.state(),
                &crate::session::SessionState::Quarantined
            ) {
                return Err(fdo::Error::Failed(
                    "quarantine cannot be released without restoring its revoked boundaries"
                        .to_owned(),
                ));
            }
            format!("{}.service", live.plan.capsule_unit)
        };
        let control = Arc::clone(&self.kernel_control);
        let controlled_unit = unit.clone();
        let established =
            tokio::task::spawn_blocking(move || control.set_freeze_state(&controlled_unit, state))
                .await
                .unwrap_or(false);
        if !established {
            return Err(fdo::Error::Failed(
                "the requested cgroup freeze state was not established".to_owned(),
            ));
        }

        let control = Arc::clone(&self.kernel_control);
        let verified = tokio::task::spawn_blocking(move || control.freeze_state_is(&unit, state))
            .await
            .unwrap_or(false);
        if !verified {
            return Err(fdo::Error::Failed(
                "the cgroup freeze state changed before it could be published".to_owned(),
            ));
        }

        let mut registry = self
            .registry
            .lock()
            .map_err(|_| fdo::Error::Failed("the session registry is unavailable".to_owned()))?;
        let Some(live) = registry.get_mut(capsule_id) else {
            return Ok(false);
        };
        match state {
            crate::telemetry::FreezeState::Frozen => live.session.pause(),
            crate::telemetry::FreezeState::Running => live.session.running(),
        }
        .map_err(|error| fdo::Error::Failed(error.to_string()))?;
        Ok(true)
    }

    async fn quarantine(&self, capsule_id: Uuid) -> fdo::Result<bool> {
        let gate = self.control_gate(capsule_id)?;
        let _control = gate.lock().await;
        let (capsule_unit, egress_unit, model_boundary) = {
            let registry = self.registry.lock().map_err(|_| {
                fdo::Error::Failed("the session registry is unavailable".to_owned())
            })?;
            let Some(live) = registry.get(capsule_id) else {
                return Ok(false);
            };
            (
                format!("{}.service", live.plan.capsule_unit),
                (!live.plan.hosts.is_empty()).then(|| format!("{}.service", live.plan.egress_unit)),
                live.plan.gateway_unit.as_ref().map(|unit| {
                    let artifacts: Vec<std::path::PathBuf> = [
                        live.plan.model_socket.clone(),
                        live.plan.model_token.clone(),
                    ]
                    .into_iter()
                    .flatten()
                    .collect();
                    (unit.clone(), artifacts)
                }),
            )
        };

        let control = Arc::clone(&self.kernel_control);
        let controlled_unit = capsule_unit.clone();
        let frozen = tokio::task::spawn_blocking(move || {
            control.set_freeze_state(&controlled_unit, crate::telemetry::FreezeState::Frozen)
        })
        .await
        .unwrap_or(false);
        if !frozen {
            return Err(fdo::Error::Failed(
                "quarantine did not establish the cgroup freeze".to_owned(),
            ));
        }

        let egress_revoked = if let Some(unit) = egress_unit {
            let control = Arc::clone(&self.kernel_control);
            tokio::task::spawn_blocking(move || control.revoke_egress(&unit))
                .await
                .unwrap_or(false)
        } else {
            true
        };

        let model_revoked = if let Some((unit, artifacts)) = model_boundary {
            let control = Arc::clone(&self.kernel_control);
            tokio::task::spawn_blocking(move || control.revoke_model(&unit, &artifacts))
                .await
                .unwrap_or(false)
        } else {
            true
        };

        let control = Arc::clone(&self.kernel_control);
        let still_frozen = tokio::task::spawn_blocking(move || {
            control.freeze_state_is(&capsule_unit, crate::telemetry::FreezeState::Frozen)
        })
        .await
        .unwrap_or(false);

        let mut registry = self
            .registry
            .lock()
            .map_err(|_| fdo::Error::Failed("the session registry is unavailable".to_owned()))?;
        let Some(live) = registry.get_mut(capsule_id) else {
            return Ok(false);
        };
        if egress_revoked && model_revoked && still_frozen {
            live.session
                .quarantine()
                .map_err(|error| fdo::Error::Failed(error.to_string()))?;
            Ok(true)
        } else if still_frozen {
            live.session
                .pause()
                .map_err(|error| fdo::Error::Failed(error.to_string()))?;
            Err(fdo::Error::Failed(
                match (egress_revoked, model_revoked) {
                    (false, false) => {
                        "capsule frozen, but network and model revocation were not established"
                    }
                    (false, true) => "capsule frozen, but network revocation was not established",
                    (true, false) => "capsule frozen, but model revocation was not established",
                    (true, true) => unreachable!("both boundaries were established"),
                }
                .to_owned(),
            ))
        } else {
            Err(fdo::Error::Failed(
                "the capsule was no longer frozen before quarantine could be published".to_owned(),
            ))
        }
    }
}

#[allow(clippy::unused_async, reason = "zbus handlers are futures")]
#[interface(name = "org.cybou.Runtime.Agent1")]
impl Agent1Service {
    async fn ready(&self) -> bool {
        true
    }

    /// Return the catalogue of offered profiles and runtime readiness.
    async fn offers(&self) -> fdo::Result<Vec<u8>> {
        let response = match &self.launch {
            Some((capacity, launcher)) => {
                let mut res = launcher.offers().map_err(fdo::Error::Failed)?;
                res.capacity_bounded = capacity.is_bounded();
                res
            }
            None => cybou_protocol::agent::AgentOffersResponse {
                profiles: Vec::new(),
                profiles_state: "not-configured".to_owned(),
                capacity_state: "not-configured".to_owned(),
                provider_state: "not-configured".to_owned(),
                capacity_bounded: false,
                provider_connected: false,
            },
        };
        cybou_fabric::encode(&response).map_err(|error| fdo::Error::Failed(error.to_string()))
    }

    /// Launch one agent under an operator-approved profile.
    async fn launch(&self, request: Vec<u8>) -> fdo::Result<Vec<u8>> {
        let request: LaunchRequest = cybou_fabric::decode(&request)
            .map_err(|error| fdo::Error::InvalidArgs(error.to_string()))?;
        let (capacity, launcher) = self.launch.as_ref().ok_or_else(|| {
            fdo::Error::AccessDenied("this owner was not configured to launch sessions".to_owned())
        })?;
        if !capacity.is_bounded() {
            return Err(fdo::Error::AccessDenied(
                "Agent1 refuses reachable launches without a bounded host capacity".to_owned(),
            ));
        }

        let now = OffsetDateTime::now_utc();
        let prepared = launcher
            .prepare(&request, now)
            .map_err(fdo::Error::AccessDenied)?;
        let capsule_id = prepared.plan.lease.grant().capsule_id;
        let session = Session::launching(capsule_id, now);
        let view = crate::view::of(&session, &prepared.plan, Ledger::Elsewhere);
        let live = LiveSession {
            plan: prepared.plan.clone(),
            session,
            ledger: Ledger::Elsewhere,
        };

        self.registry
            .lock()
            .map_err(|_| fdo::Error::Failed("the session registry is unavailable".to_owned()))?
            .admit(*capacity, live)
            .map_err(|why| fdo::Error::LimitsExceeded(why.to_string()))?;

        if let Err(why) = launcher.start(prepared, Arc::clone(&self.registry)) {
            if let Ok(mut registry) = self.registry.lock() {
                registry.take(capsule_id);
            }
            return Err(fdo::Error::Failed(why));
        }

        encode(&view).map_err(|error| fdo::Error::Failed(error.to_string()))
    }

    /// Every session this owner holds, as a surface should show it.
    async fn sessions(&self) -> fdo::Result<Vec<u8>> {
        let views = self
            .registry
            .lock()
            .map_err(|_| fdo::Error::Failed("the session registry is unavailable".to_owned()))?
            .views();
        encode(&views).map_err(|error| fdo::Error::Failed(error.to_string()))
    }

    /// One session, by the capsule identity that is also its own.
    async fn session(&self, capsule_id: String) -> fdo::Result<Vec<u8>> {
        let capsule_id = identity(&capsule_id)?;
        let registry = self
            .registry
            .lock()
            .map_err(|_| fdo::Error::Failed("the session registry is unavailable".to_owned()))?;
        let view = registry
            .views()
            .into_iter()
            .find(|view| view.capsule_id == capsule_id)
            .ok_or_else(|| fdo::Error::FileNotFound("no such session".to_owned()))?;
        encode(&view).map_err(|error| fdo::Error::Failed(error.to_string()))
    }

    /// End one session now.
    ///
    /// Idempotent in the way that matters: a second Stop on a session this owner no longer holds
    /// returns `false` without another teardown. `false` can also mean that teardown was unproven;
    /// callers that need to distinguish those outcomes must read the canonical session listing.
    async fn stop(&self, capsule_id: String) -> fdo::Result<bool> {
        let capsule_id = identity(&capsule_id)?;
        let gate = self.control_gate(capsule_id)?;
        let control = gate.lock().await;
        let now = OffsetDateTime::now_utc();

        // Marked as ending while it is still held, so a listing taken during the teardown shows a
        // session on its way out rather than one that is fine.
        let plan = {
            let mut registry = self.registry.lock().map_err(|_| {
                fdo::Error::Failed("the session registry is unavailable".to_owned())
            })?;
            let Some(live) = registry.get_mut(capsule_id) else {
                return Ok(false);
            };
            live.session.begin_ending(SessionEnd::Stopped);
            live.plan.clone()
        };

        if self.teardown.tear_down(&plan) == Ended::Unproven {
            // Still there. Keeping it listed is the whole point: a forgotten capsule is one nobody
            // can be shown and nobody can be offered a way to stop.
            return Ok(false);
        }

        let mut registry = self
            .registry
            .lock()
            .map_err(|_| fdo::Error::Failed("the session registry is unavailable".to_owned()))?;
        registry.finish(capsule_id, now);
        drop(registry);
        drop(control);
        if let Ok(mut gates) = self.control_gates.lock() {
            gates.remove(&capsule_id);
        }
        Ok(true)
    }

    /// Stop a live session, refusing controls whose kernel effects are not yet established.
    async fn action(&self, capsule_id: String, action: String) -> fdo::Result<bool> {
        let cap_id = identity(&capsule_id)?;
        let action: cybou_protocol::agent::CapsuleAction =
            serde_json::from_str(&format!("\"{action}\""))
                .map_err(|error| fdo::Error::InvalidArgs(error.to_string()))?;

        match action {
            cybou_protocol::agent::CapsuleAction::Stop => self.stop(capsule_id).await,
            cybou_protocol::agent::CapsuleAction::Freeze => {
                self.change_freeze_state(cap_id, crate::telemetry::FreezeState::Frozen)
                    .await
            }
            cybou_protocol::agent::CapsuleAction::Resume => {
                self.change_freeze_state(cap_id, crate::telemetry::FreezeState::Running)
                    .await
            }
            cybou_protocol::agent::CapsuleAction::Quarantine => self.quarantine(cap_id).await,
        }
    }

    /// Get live telemetry snapshot for a capsule session.
    async fn telemetry(&self, capsule_id: String) -> fdo::Result<Vec<u8>> {
        let cap_id = identity(&capsule_id)?;
        let (view, capsule_unit) = {
            let registry = self.registry.lock().map_err(|_| {
                fdo::Error::Failed("the session registry is unavailable".to_owned())
            })?;
            let view = registry
                .views()
                .into_iter()
                .find(|v| v.capsule_id == cap_id)
                .ok_or_else(|| fdo::Error::FileNotFound("no such session".to_owned()))?;
            let capsule_unit = registry
                .get(cap_id)
                .map(|live| format!("{}.service", live.plan.capsule_unit));
            (view, capsule_unit)
        };
        let readings = if let Some(unit) = capsule_unit {
            tokio::task::spawn_blocking(move || crate::telemetry::read_unit(&unit))
                .await
                .unwrap_or_default()
        } else {
            crate::telemetry::CgroupReadings::default()
        };
        let observed_at = OffsetDateTime::now_utc();

        let telemetry = cybou_protocol::agent::CapsuleTelemetryRecord {
            capsule_id: cap_id,
            standing: view.standing,
            pids_count: metric(readings.process_count, observed_at),
            pids_current: metric(readings.pids_current, observed_at),
            pids_max: metric(readings.pids_max, observed_at),
            memory_used_mib: metric(
                readings
                    .memory_current_bytes
                    .map(|bytes| bytes / 1024 / 1024),
                observed_at,
            ),
            memory_max_mib: metric(
                readings.memory_max_bytes.map(|bytes| bytes / 1024 / 1024),
                observed_at,
            ),
            cpu_usage_pct: cybou_protocol::agent::AgentMetric::unavailable(),
            cpu_usage_usec: metric(readings.cpu_usage_usec, observed_at),
            egress_requests_count: cybou_protocol::agent::AgentMetric::unavailable(),
            egress_denied_count: cybou_protocol::agent::AgentMetric::unavailable(),
            files_modified_count: cybou_protocol::agent::AgentMetric::unavailable(),
            tokens_in: cybou_protocol::agent::AgentMetric::unavailable(),
            tokens_out: cybou_protocol::agent::AgentMetric::unavailable(),
            active_tool: cybou_protocol::agent::AgentMetric::unavailable(),
            recent_activity: cybou_protocol::agent::AgentMetric::unavailable(),
        };

        encode(&telemetry).map_err(|error| fdo::Error::Failed(error.to_string()))
    }
}

fn metric<T>(
    value: Option<T>,
    observed_at: OffsetDateTime,
) -> cybou_protocol::agent::AgentMetric<T> {
    value.map_or_else(cybou_protocol::agent::AgentMetric::unavailable, |value| {
        cybou_protocol::agent::AgentMetric::known(value, observed_at)
    })
}

fn identity(value: &str) -> fdo::Result<Uuid> {
    Uuid::parse_str(value)
        .map_err(|_| fdo::Error::InvalidArgs("a session is named by its capsule UUID".to_owned()))
}

#[cfg(test)]
mod tests {
    use std::{
        sync::atomic::{AtomicU8, AtomicUsize, Ordering},
        time::Duration as StdDuration,
    };

    use cybou_capsule::{
        CapabilityProfile, LeaseRequest, ModelGrant, NetworkGrant, ResourceBudget, SpendPolicy,
        Workspace, compile, issue_lease,
    };
    use time::Duration;

    use super::*;
    use crate::plan::{Ceilings, Launch, SessionPlan, plan};
    use crate::registry::LiveSession;
    use crate::session::Session;

    #[derive(Default)]
    struct CountingTeardown(AtomicUsize);

    impl Teardown for CountingTeardown {
        fn tear_down(&self, _plan: &SessionPlan) -> Ended {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ended::Confirmed
        }
    }

    /// A host where the capsule will not die.
    #[derive(Default)]
    struct StubbornTeardown(AtomicUsize);

    impl Teardown for StubbornTeardown {
        fn tear_down(&self, _plan: &SessionPlan) -> Ended {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ended::Unproven
        }
    }

    #[derive(Default)]
    struct FakeLauncher {
        prepared: AtomicUsize,
        started: AtomicUsize,
    }

    impl Launcher for FakeLauncher {
        fn prepare(
            &self,
            request: &LaunchRequest,
            _now: OffsetDateTime,
        ) -> Result<PreparedLaunch, String> {
            if request.profile != "sandboxed-autonomous" {
                return Err("the profile is not offered".to_owned());
            }
            let ordinal = self.prepared.fetch_add(1, Ordering::SeqCst) as u128;
            let live = held_for(Uuid::from_u128(CAPSULE.as_u128() + ordinal));
            Ok(PreparedLaunch {
                spec: compile(live.plan.lease.grant()).map_err(|error| error.to_string())?,
                plan: live.plan,
                prompt: request.prompt.clone(),
            })
        }

        fn start(
            &self,
            _prepared: PreparedLaunch,
            _registry: Arc<Mutex<SessionRegistry>>,
        ) -> Result<(), String> {
            self.started.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    struct RefusingStarter(FakeLauncher);

    impl Launcher for RefusingStarter {
        fn prepare(
            &self,
            request: &LaunchRequest,
            now: OffsetDateTime,
        ) -> Result<PreparedLaunch, String> {
            self.0.prepare(request, now)
        }

        fn start(
            &self,
            _prepared: PreparedLaunch,
            _registry: Arc<Mutex<SessionRegistry>>,
        ) -> Result<(), String> {
            Err("the launch task could not be owned".to_owned())
        }
    }

    struct RecordingControl {
        establishes: bool,
        revokes_egress: bool,
        revokes_model: bool,
        calls: Mutex<Vec<crate::telemetry::FreezeState>>,
    }

    impl RecordingControl {
        fn establishing() -> Self {
            Self {
                establishes: true,
                revokes_egress: true,
                revokes_model: true,
                calls: Mutex::new(Vec::new()),
            }
        }
    }

    impl KernelControl for RecordingControl {
        fn set_freeze_state(&self, _unit: &str, state: crate::telemetry::FreezeState) -> bool {
            self.calls.lock().expect("calls").push(state);
            self.establishes
        }

        fn freeze_state_is(&self, _unit: &str, _state: crate::telemetry::FreezeState) -> bool {
            self.establishes
        }

        fn revoke_egress(&self, _unit: &str) -> bool {
            self.revokes_egress
        }

        fn revoke_model(&self, _unit: &str, _artifacts: &[std::path::PathBuf]) -> bool {
            self.revokes_model
        }
    }

    struct ConcurrentControl {
        active: AtomicUsize,
        max_active: AtomicUsize,
        state: AtomicU8,
    }

    impl ConcurrentControl {
        fn running() -> Self {
            Self {
                active: AtomicUsize::new(0),
                max_active: AtomicUsize::new(0),
                state: AtomicU8::new(0),
            }
        }

        fn enter(&self) {
            let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
            self.max_active.fetch_max(active, Ordering::SeqCst);
            std::thread::sleep(StdDuration::from_millis(25));
        }

        fn leave(&self) {
            self.active.fetch_sub(1, Ordering::SeqCst);
        }
    }

    impl KernelControl for ConcurrentControl {
        fn set_freeze_state(&self, _unit: &str, state: crate::telemetry::FreezeState) -> bool {
            self.enter();
            self.state.store(
                u8::from(matches!(state, crate::telemetry::FreezeState::Frozen)),
                Ordering::SeqCst,
            );
            self.leave();
            true
        }

        fn freeze_state_is(&self, _unit: &str, state: crate::telemetry::FreezeState) -> bool {
            self.enter();
            let established = self.state.load(Ordering::SeqCst)
                == u8::from(matches!(state, crate::telemetry::FreezeState::Frozen));
            self.leave();
            established
        }

        fn revoke_egress(&self, _unit: &str) -> bool {
            true
        }

        fn revoke_model(&self, _unit: &str, _artifacts: &[std::path::PathBuf]) -> bool {
            true
        }
    }

    fn at(offset: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_787_000_000 + offset).expect("a fixed instant")
    }

    const CAPSULE: Uuid = Uuid::from_u128(0xf001);

    fn held() -> LiveSession {
        held_for(CAPSULE)
    }

    fn held_for(capsule_id: Uuid) -> LiveSession {
        let mut profile = CapabilityProfile::bounded(
            "sandboxed-autonomous",
            ResourceBudget {
                memory_mib: 4096,
                cpus: 2,
                tasks_max: 512,
                lifetime: Duration::hours(4),
            },
        )
        .expect("a valid profile");
        profile.network = NetworkGrant::to(&["github.com"]);
        profile.model = Some(ModelGrant {
            class: "Strong".to_owned(),
            spend: SpendPolicy::Capped(100),
        });
        profile.may_execute = true;
        let lease = issue_lease(
            LeaseRequest {
                selected_profile: profile,
                capsule_id,
                agent: "opencode".to_owned(),
                workspace: Workspace::at("/srv/project"),
            },
            at(0),
        )
        .expect("a lease is issued");

        let plan = plan(
            &Launch {
                lease,
                task_id: Uuid::from_u128(0xf002),
                ceilings: Ceilings {
                    token_limit: 1000,
                    max_output_tokens: 32,
                    sensitivity: 1,
                },
            },
            at(1),
        )
        .expect("a plan");

        let mut session = Session::launching(capsule_id, at(0));
        session.running().expect("it came up");
        LiveSession {
            plan,
            session,
            ledger: crate::view::Ledger::Elsewhere,
        }
    }

    fn serving() -> (
        Agent1Service,
        Arc<CountingTeardown>,
        Arc<Mutex<SessionRegistry>>,
    ) {
        let mut registry = SessionRegistry::new();
        registry.insert(held());
        let registry = Arc::new(Mutex::new(registry));
        let teardown = Arc::new(CountingTeardown::default());
        (
            Agent1Service::new(
                Arc::clone(&registry),
                Arc::clone(&teardown) as Arc<dyn Teardown>,
            ),
            teardown,
            registry,
        )
    }

    #[tokio::test]
    async fn stopping_a_session_tears_it_down_once_and_lists_its_final_view() {
        let (service, teardown, registry) = serving();
        assert_eq!(registry.lock().expect("held").len(), 1);

        assert!(service.stop(CAPSULE.to_string()).await.expect("stops"));
        assert_eq!(teardown.0.load(Ordering::SeqCst), 1);
        assert!(
            registry.lock().expect("held").is_empty(),
            "the registry answers what is running"
        );
        let views = registry.lock().expect("held").views();
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].standing, crate::view::Standing::Ended);
        assert_eq!(
            views[0].ended_because.as_deref(),
            Some("the session was stopped")
        );
        assert!(views[0].ended_at.is_some());
    }

    #[tokio::test]
    async fn stopping_something_already_gone_tears_nothing_down_again() {
        // A second Stop is not an error worth reporting differently: in both cases the session is
        // over by the time the caller is answered, and tearing down twice would run unit stops
        // against names that now belong to nothing.
        let (service, teardown, _) = serving();
        assert!(service.stop(CAPSULE.to_string()).await.expect("stops"));
        assert!(!service.stop(CAPSULE.to_string()).await.expect("answers"));

        assert_eq!(teardown.0.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn quarantine_requires_both_freeze_and_egress_revocation() {
        let (service, _, registry) = serving();
        let control = Arc::new(RecordingControl::establishing());
        let service = service.with_kernel_control(control);

        assert!(
            service
                .action(CAPSULE.to_string(), "quarantine".to_owned())
                .await
                .expect("both boundaries were established")
        );
        assert_eq!(
            registry.lock().expect("held").views()[0].standing,
            crate::view::Standing::Quarantined,
        );
    }

    #[tokio::test]
    async fn partial_quarantine_is_reported_as_paused_not_quarantined() {
        let (service, _, registry) = serving();
        let control = Arc::new(RecordingControl {
            establishes: true,
            revokes_egress: false,
            revokes_model: true,
            calls: Mutex::new(Vec::new()),
        });
        let service = service.with_kernel_control(control);

        service
            .action(CAPSULE.to_string(), "quarantine".to_owned())
            .await
            .expect_err("egress revocation was not established");
        assert_eq!(
            registry.lock().expect("held").views()[0].standing,
            crate::view::Standing::Paused,
        );
    }

    #[tokio::test]
    async fn quarantine_requires_model_revocation_too() {
        let (service, _, registry) = serving();
        let control = Arc::new(RecordingControl {
            establishes: true,
            revokes_egress: true,
            revokes_model: false,
            calls: Mutex::new(Vec::new()),
        });
        let service = service.with_kernel_control(control);

        let error = service
            .action(CAPSULE.to_string(), "quarantine".to_owned())
            .await
            .expect_err("a live model boundary prevents quarantine");
        assert!(error.to_string().contains("model revocation"));
        assert_eq!(
            registry.lock().expect("held").views()[0].standing,
            crate::view::Standing::Paused,
        );
    }

    #[tokio::test]
    async fn freeze_and_resume_are_projected_only_after_kernel_confirmation() {
        let (service, _, registry) = serving();
        let control = Arc::new(RecordingControl::establishing());
        let service = service.with_kernel_control(Arc::clone(&control) as Arc<dyn KernelControl>);

        assert!(
            service
                .action(CAPSULE.to_string(), "freeze".to_owned())
                .await
                .expect("freeze is confirmed")
        );
        assert_eq!(
            registry.lock().expect("held").views()[0].standing,
            crate::view::Standing::Paused
        );
        assert!(
            service
                .action(CAPSULE.to_string(), "resume".to_owned())
                .await
                .expect("thaw is confirmed")
        );
        assert_eq!(
            registry.lock().expect("held").views()[0].standing,
            crate::view::Standing::Running
        );
        assert_eq!(
            *control.calls.lock().expect("calls"),
            vec![
                crate::telemetry::FreezeState::Frozen,
                crate::telemetry::FreezeState::Running
            ]
        );
    }

    #[tokio::test]
    async fn concurrent_controls_are_serialized_per_capsule() {
        let (service, _, registry) = serving();
        let control = Arc::new(ConcurrentControl::running());
        let service = service.with_kernel_control(Arc::clone(&control) as Arc<dyn KernelControl>);

        let (frozen, running) = tokio::join!(
            service.action(CAPSULE.to_string(), "freeze".to_owned()),
            service.action(CAPSULE.to_string(), "resume".to_owned()),
        );

        assert!(frozen.expect("freeze is confirmed"));
        assert!(running.expect("resume is confirmed"));
        assert_eq!(
            control.max_active.load(Ordering::SeqCst),
            1,
            "no two physical transitions or final kernel reads may overlap"
        );
        assert_eq!(control.state.load(Ordering::SeqCst), 0);
        assert_eq!(
            registry.lock().expect("held").views()[0].standing,
            crate::view::Standing::Running,
            "the registry follows the last serialized kernel transition"
        );
    }

    #[tokio::test]
    async fn unconfirmed_freeze_does_not_change_the_registry_projection() {
        let (service, _, registry) = serving();
        let control = Arc::new(RecordingControl {
            establishes: false,
            revokes_egress: false,
            revokes_model: false,
            calls: Mutex::new(Vec::new()),
        });
        let service = service.with_kernel_control(control);

        service
            .action(CAPSULE.to_string(), "freeze".to_owned())
            .await
            .expect_err("an unconfirmed freeze is a failure");
        assert_eq!(
            registry.lock().expect("held").views()[0].standing,
            crate::view::Standing::Running
        );
    }

    #[tokio::test]
    async fn unread_runtime_metrics_are_unavailable_not_copied_from_the_grant() {
        let (service, _, _) = serving();
        let encoded = service
            .telemetry(CAPSULE.to_string())
            .await
            .expect("the owner answers");
        let telemetry: cybou_protocol::agent::CapsuleTelemetryRecord =
            cybou_fabric::decode(&encoded).expect("typed telemetry");

        assert_eq!(
            telemetry.memory_used_mib.state,
            cybou_protocol::agent::AgentMetricState::Unavailable
        );
        assert_eq!(telemetry.memory_used_mib.value, None);
        assert_eq!(
            telemetry.pids_count.state,
            cybou_protocol::agent::AgentMetricState::Unavailable
        );
        assert_eq!(telemetry.pids_count.value, None);
        assert_eq!(
            telemetry.memory_max_mib.state,
            cybou_protocol::agent::AgentMetricState::Unavailable
        );
        assert_eq!(telemetry.memory_max_mib.value, None);
        assert_eq!(telemetry.pids_max.value, None);
    }

    #[tokio::test]
    async fn a_session_that_would_not_die_stays_listed_and_the_caller_is_told() {
        // The failure a person could not recover from, previously reported as success. A capsule
        // removed from the registry on the strength of having been asked to stop is one nobody can
        // be shown and nobody can be offered a way to stop.
        let mut registry = SessionRegistry::new();
        registry.insert(held());
        let registry = Arc::new(Mutex::new(registry));
        let teardown = Arc::new(StubbornTeardown::default());
        let service = Agent1Service::new(
            Arc::clone(&registry),
            Arc::clone(&teardown) as Arc<dyn Teardown>,
        );

        assert!(
            !service.stop(CAPSULE.to_string()).await.expect("answers"),
            "an ending that was not confirmed is not a stop"
        );
        assert_eq!(teardown.0.load(Ordering::SeqCst), 1);
        assert_eq!(
            registry.lock().expect("held").len(),
            1,
            "the session is still there, so it is still listed"
        );

        // And it shows as on its way out rather than as fine, because that is what it is.
        let views = registry.lock().expect("held").views();
        assert_eq!(views[0].standing, crate::view::Standing::Ending);
        assert_eq!(
            views[0].ended_because.as_deref(),
            Some("the session was stopped")
        );
    }

    #[tokio::test]
    async fn a_session_is_named_by_its_capsule_and_nothing_else_is_accepted() {
        let (service, teardown, _) = serving();
        assert!(service.stop("the-session".to_owned()).await.is_err());
        assert!(service.session("the-session".to_owned()).await.is_err());
        assert_eq!(teardown.0.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn a_listing_holds_grants_and_never_ledgers() {
        // This owner holds no ledger: the model gateway charges its own copy of every lease. A
        // listing reporting nought would state, of every session on it, that nothing was spent.
        let (service, _, _) = serving();
        let encoded = service.sessions().await.expect("a listing");
        let views: Vec<crate::view::SessionView> = cybou_fabric::decode(&encoded).expect("decodes");

        assert_eq!(views.len(), 1);
        assert_eq!(views[0].capsule_id, CAPSULE);
        match views[0].spend {
            Some(crate::view::SpendView::Capped { spent, .. }) => assert_eq!(spent, None),
            other => panic!("a capped grant showed as {other:?}"),
        }
    }

    fn launch_request() -> LaunchRequest {
        LaunchRequest {
            profile: "sandboxed-autonomous".to_owned(),
            agent: "opencode".to_owned(),
            workspace: "/srv/project".to_owned(),
            model_class: Some("Strong".to_owned()),
            prompt: "inspect this workspace".to_owned(),
        }
    }

    fn bounded_capacity(max_sessions: u32) -> HostCapacity {
        HostCapacity {
            max_sessions,
            memory_mib: 8192,
            cpus: 4,
            tasks_max: 1024,
            spend_units: 200,
        }
    }

    #[tokio::test]
    async fn launch_admits_and_starts_one_owner_derived_session() {
        let registry = Arc::new(Mutex::new(SessionRegistry::new()));
        let launcher = Arc::new(FakeLauncher::default());
        let service = Agent1Service::with_launch(
            Arc::clone(&registry),
            Arc::new(CountingTeardown::default()),
            bounded_capacity(1),
            Arc::clone(&launcher) as Arc<dyn Launcher>,
        );

        let encoded = service
            .launch(encode(&launch_request()).expect("request encodes"))
            .await
            .expect("admitted");
        let view: crate::view::SessionView = cybou_fabric::decode(&encoded).expect("view decodes");

        assert_eq!(view.capsule_id, CAPSULE);
        assert_eq!(view.standing, crate::view::Standing::Launching);
        assert_eq!(registry.lock().expect("held").len(), 1);
        assert_eq!(launcher.started.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn two_launches_cannot_both_take_the_last_host_slot() {
        let registry = Arc::new(Mutex::new(SessionRegistry::new()));
        let launcher = Arc::new(FakeLauncher::default());
        let service = Agent1Service::with_launch(
            Arc::clone(&registry),
            Arc::new(CountingTeardown::default()),
            bounded_capacity(1),
            Arc::clone(&launcher) as Arc<dyn Launcher>,
        );
        let request = encode(&launch_request()).expect("request encodes");

        assert!(service.launch(request.clone()).await.is_ok());
        assert!(service.launch(request).await.is_err());
        assert_eq!(registry.lock().expect("held").len(), 1);
        assert_eq!(launcher.prepared.load(Ordering::SeqCst), 2);
        assert_eq!(launcher.started.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn reachable_launch_refuses_an_unbounded_host() {
        let registry = Arc::new(Mutex::new(SessionRegistry::new()));
        let launcher = Arc::new(FakeLauncher::default());
        let service = Agent1Service::with_launch(
            Arc::clone(&registry),
            Arc::new(CountingTeardown::default()),
            HostCapacity::unbounded(),
            Arc::clone(&launcher) as Arc<dyn Launcher>,
        );

        assert!(
            service
                .launch(encode(&launch_request()).expect("request encodes"))
                .await
                .is_err()
        );
        assert!(registry.lock().expect("held").is_empty());
        assert_eq!(launcher.prepared.load(Ordering::SeqCst), 0);
        assert_eq!(launcher.started.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn an_immediate_start_failure_releases_its_capacity() {
        let registry = Arc::new(Mutex::new(SessionRegistry::new()));
        let service = Agent1Service::with_launch(
            Arc::clone(&registry),
            Arc::new(CountingTeardown::default()),
            bounded_capacity(1),
            Arc::new(RefusingStarter(FakeLauncher::default())),
        );

        assert!(
            service
                .launch(encode(&launch_request()).expect("request encodes"))
                .await
                .is_err()
        );
        assert!(registry.lock().expect("held").is_empty());
    }
}
