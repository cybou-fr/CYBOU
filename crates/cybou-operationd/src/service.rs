// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! D-Bus implementation of `org.cybou.Runtime.Operation1`.

#![allow(missing_docs)]

use cybou_protocol::{
    action::{ActionRecord, AttemptReport, Proposer},
    agent::{SessionView, Standing},
    operation::{
        CancelOutcome, ObservationState, OperationKind, OperationLogEntry, OperationProgress,
        OperationRecord, OperationState,
    },
    subject::SubjectRef,
};
use rusqlite::{Connection, params};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::{Arc, Mutex as SyncMutex},
};
use time::OffsetDateTime;
use tokio::sync::{Mutex, watch};
use uuid::Uuid;
use zbus::interface;

/// Owners that establish operations this one only reflects.
const AGENT1: &str = "Agent1";
const ACTION1: &str = "Action1";

const MAX_TERMINAL_OPERATIONS: usize = 100;
const MAX_LOG_ENTRIES_PER_OPERATION: usize = 500;

#[derive(Default)]
struct Store {
    operations: Vec<OperationRecord>,
    logs: HashMap<Uuid, Vec<OperationLogEntry>>,
    cancellation: HashMap<Uuid, watch::Sender<bool>>,
    cancellation_requested: HashSet<Uuid>,
}

/// Sole lifecycle owner for background operations.
#[derive(Clone)]
pub struct Operation1Service {
    store: Arc<Mutex<Store>>,
    database: Option<Arc<SyncMutex<Connection>>>,
}

impl Operation1Service {
    fn bounded_history(operations: &mut Vec<OperationRecord>) -> Vec<Uuid> {
        let mut terminal_count = 0_usize;
        let mut removed = Vec::new();
        operations.retain(|operation| {
            if !operation.state.is_terminal() {
                return true;
            }
            terminal_count += 1;
            if terminal_count <= MAX_TERMINAL_OPERATIONS {
                true
            } else {
                removed.push(operation.id);
                false
            }
        });
        removed
    }

    fn delete_history(
        transaction: &rusqlite::Transaction<'_>,
        removed: &[Uuid],
    ) -> rusqlite::Result<()> {
        for id in removed {
            transaction.execute(
                "DELETE FROM operation_logs WHERE operation_id = ?1",
                [id.to_string()],
            )?;
            transaction.execute("DELETE FROM operations WHERE id = ?1", [id.to_string()])?;
            transaction.execute(
                "DELETE FROM cancellation_requests WHERE operation_id = ?1",
                [id.to_string()],
            )?;
        }
        Ok(())
    }

    /// Run one durable unit of work off the async executor.
    ///
    /// These transactions run with `synchronous=FULL`, so they wait on the disk. Doing that on a
    /// runtime worker would stall every other caller of this owner while one record is committed.
    async fn durably<F>(&self, work: F) -> rusqlite::Result<()>
    where
        F: FnOnce(&mut Connection) -> rusqlite::Result<()> + Send + 'static,
    {
        let Some(database) = self.database.clone() else {
            return Ok(());
        };
        tokio::task::spawn_blocking(move || {
            let mut connection = database.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;
            work(&mut connection)
        })
        .await
        .map_err(|_| rusqlite::Error::InvalidQuery)?
    }

    #[must_use]
    pub fn new() -> Self {
        Self {
            store: Arc::new(Mutex::new(Store::default())),
            database: None,
        }
    }

    /// Open the configured durable database, creating its parent and schema when needed.
    ///
    /// # Errors
    ///
    /// Returns an error when the database path, schema, or stored records cannot be opened or
    /// decoded.
    pub fn durable_default() -> rusqlite::Result<Self> {
        let path = std::env::var("CYBOU_OPERATION_STORE")
            .unwrap_or_else(|_| "/var/lib/cybou/operations.sqlite3".to_owned());
        Self::with_database(path)
    }

    /// Open and restore a durable Operation1 database.
    ///
    /// # Errors
    ///
    /// Returns an error when the directory/database cannot be created or existing records are not
    /// valid fabric values.
    pub fn with_database(path: impl AsRef<Path>) -> rusqlite::Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|_| rusqlite::Error::InvalidPath(parent.into()))?;
        }
        let mut connection = Connection::open(path)?;
        connection.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=FULL;
             CREATE TABLE IF NOT EXISTS operations (
                 id TEXT PRIMARY KEY NOT NULL,
                 record BLOB NOT NULL
             );
             CREATE TABLE IF NOT EXISTS operation_logs (
                 operation_id TEXT NOT NULL,
                 sequence INTEGER NOT NULL,
                 entry BLOB NOT NULL,
                 PRIMARY KEY(operation_id, sequence)
             );
             CREATE TABLE IF NOT EXISTS cancellation_requests (
                 operation_id TEXT PRIMARY KEY NOT NULL
             );
             DELETE FROM operation_logs
             WHERE (operation_id, sequence) IN (
                 SELECT operation_id, sequence FROM (
                     SELECT operation_id, sequence,
                            ROW_NUMBER() OVER (
                                PARTITION BY operation_id ORDER BY sequence DESC
                            ) AS retention_rank
                     FROM operation_logs
                 ) WHERE retention_rank > 500
             );",
        )?;
        let mut operations = Vec::new();
        {
            let mut statement =
                connection.prepare("SELECT record FROM operations ORDER BY rowid DESC")?;
            let rows = statement.query_map([], |row| row.get::<_, Vec<u8>>(0))?;
            for row in rows {
                let record: OperationRecord =
                    cybou_fabric::decode(&row?).map_err(|_| rusqlite::Error::InvalidQuery)?;
                operations.push(record);
            }
        }
        let mut logs: HashMap<Uuid, Vec<OperationLogEntry>> = HashMap::new();
        {
            let mut statement = connection
                .prepare("SELECT operation_id, entry FROM operation_logs ORDER BY sequence")?;
            let rows = statement.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
            })?;
            for row in rows {
                let (id, encoded) = row?;
                let id = Uuid::parse_str(&id).map_err(|_| rusqlite::Error::InvalidQuery)?;
                let entry =
                    cybou_fabric::decode(&encoded).map_err(|_| rusqlite::Error::InvalidQuery)?;
                logs.entry(id).or_default().push(entry);
            }
        }
        let mut cancellation_requested = HashSet::new();
        {
            let mut statement =
                connection.prepare("SELECT operation_id FROM cancellation_requests")?;
            let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
            for row in rows {
                let id = Uuid::parse_str(&row?).map_err(|_| rusqlite::Error::InvalidQuery)?;
                cancellation_requested.insert(id);
            }
        }
        for operation in &mut operations {
            if !operation.state.is_terminal() {
                // A restored record is a memory of the last publication, not a live observation.
                operation.observation = ObservationState::Stale;
            }
        }
        let removed = Self::bounded_history(&mut operations);
        if !removed.is_empty() {
            let transaction = connection.transaction()?;
            Self::delete_history(&transaction, &removed)?;
            transaction.commit()?;
            for id in removed {
                logs.remove(&id);
            }
        }
        Ok(Self {
            store: Arc::new(Mutex::new(Store {
                operations,
                logs,
                cancellation: HashMap::new(),
                cancellation_requested,
            })),
            database: Some(Arc::new(SyncMutex::new(connection))),
        })
    }

    /// Register a real job together with the token its worker observes.
    ///
    /// # Errors
    ///
    /// Returns an error without changing memory when the durable transaction cannot commit.
    pub async fn register(
        &self,
        record: OperationRecord,
    ) -> rusqlite::Result<watch::Receiver<bool>> {
        let (sender, receiver) = watch::channel(false);
        let mut store = self.store.lock().await;
        let mut operations = store.operations.clone();
        operations.insert(0, record.clone());
        let removed = Self::bounded_history(&mut operations);
        {
            let encoded =
                cybou_fabric::encode(&record).map_err(|_| rusqlite::Error::InvalidQuery)?;
            let id = record.id.to_string();
            let removed = removed.clone();
            self.durably(move |connection| {
                let transaction = connection.transaction()?;
                transaction.execute(
                    "INSERT INTO operations(id, record) VALUES (?1, ?2)",
                    params![id, encoded],
                )?;
                transaction.execute(
                    "DELETE FROM cancellation_requests WHERE operation_id = ?1",
                    [id],
                )?;
                Self::delete_history(&transaction, &removed)?;
                transaction.commit()
            })
            .await?;
        }
        store.cancellation.insert(record.id, sender);
        store.cancellation_requested.remove(&record.id);
        store.operations = operations;
        for id in removed {
            store.logs.remove(&id);
            store.cancellation.remove(&id);
            store.cancellation_requested.remove(&id);
        }
        Ok(receiver)
    }

    /// Reattach a restarted local worker to a restored, non-terminal operation.
    ///
    /// The returned token starts as requested when cancellation arrived while the worker was
    /// detached, so no request is lost across either process restart.
    ///
    /// # Errors
    ///
    /// Returns an error when the operation is absent, terminal, or not cancellable.
    pub async fn reattach(&self, id: Uuid) -> rusqlite::Result<watch::Receiver<bool>> {
        let mut store = self.store.lock().await;
        let operation = store
            .operations
            .iter()
            .find(|operation| operation.id == id)
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        if operation.state.is_terminal() || !operation.cancellable {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let requested = store.cancellation_requested.contains(&id);
        let (sender, receiver) = watch::channel(requested);
        store.cancellation.insert(id, sender);
        Ok(receiver)
    }

    async fn persist_cancellation_request(&self, id: Uuid) -> rusqlite::Result<()> {
        self.durably(move |connection| {
            connection
                .execute(
                    "INSERT OR IGNORE INTO cancellation_requests(operation_id) VALUES (?1)",
                    [id.to_string()],
                )
                .map(|_| ())
        })
        .await
    }

    /// Publish "a cancel request is in flight" without claiming the work has stopped.
    async fn mark_cancellation_requested(&self, id: Uuid) {
        let existing = self
            .store
            .lock()
            .await
            .operations
            .iter()
            .find(|value| value.id == id)
            .cloned();
        if let Some(mut record) = existing {
            if record.cancellation_requested || record.state.is_terminal() {
                return;
            }
            record.cancellation_requested = true;
            record.updated_at = OffsetDateTime::now_utc();
            let _ = self.update(record).await;
        }
    }

    async fn request_local_cancellation(&self, id: Uuid) -> rusqlite::Result<()> {
        self.persist_cancellation_request(id).await?;
        self.store.lock().await.cancellation_requested.insert(id);
        Ok(())
    }

    /// Append one durable worker log entry.
    ///
    /// # Errors
    ///
    /// Returns an error when the operation is unknown or the durable transaction cannot commit.
    pub async fn append_log(
        &self,
        operation_id: Uuid,
        entry: OperationLogEntry,
    ) -> rusqlite::Result<()> {
        let mut store = self.store.lock().await;
        if !store
            .operations
            .iter()
            .any(|value| value.id == operation_id)
        {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        let mut next = store.logs.get(&operation_id).cloned().unwrap_or_default();
        next.push(entry.clone());
        if next.len() > MAX_LOG_ENTRIES_PER_OPERATION {
            next.drain(..next.len() - MAX_LOG_ENTRIES_PER_OPERATION);
        }
        {
            let encoded =
                cybou_fabric::encode(&entry).map_err(|_| rusqlite::Error::InvalidQuery)?;
            let id = operation_id.to_string();
            self.durably(move |connection| {
                let transaction = connection.transaction()?;
                let sequence: i64 = transaction.query_row(
                    "SELECT COALESCE(MAX(sequence), -1) + 1 FROM operation_logs WHERE operation_id = ?1",
                    [&id],
                    |row| row.get(0),
                )?;
                transaction.execute(
                    "INSERT INTO operation_logs(operation_id, sequence, entry) VALUES (?1, ?2, ?3)",
                    params![id, sequence, encoded],
                )?;
                transaction.execute(
                    "DELETE FROM operation_logs
                     WHERE operation_id = ?1 AND sequence NOT IN (
                         SELECT sequence FROM operation_logs
                         WHERE operation_id = ?1 ORDER BY sequence DESC LIMIT 500
                     )",
                    [&id],
                )?;
                transaction.commit()
            })
            .await?;
        }
        store.logs.insert(operation_id, next);
        Ok(())
    }

    /// Replace lifecycle/progress state reported by the worker, durably before projection.
    ///
    /// # Errors
    ///
    /// Returns an error when the operation is unknown or the durable transaction cannot commit.
    pub async fn update(&self, record: OperationRecord) -> rusqlite::Result<()> {
        let mut store = self.store.lock().await;
        let position = store
            .operations
            .iter()
            .position(|value| value.id == record.id)
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        let mut operations = store.operations.clone();
        operations[position] = record.clone();
        let removed = Self::bounded_history(&mut operations);
        {
            let encoded =
                cybou_fabric::encode(&record).map_err(|_| rusqlite::Error::InvalidQuery)?;
            let id = record.id.to_string();
            let terminal = record.state.is_terminal();
            let removed = removed.clone();
            self.durably(move |connection| {
                let transaction = connection.transaction()?;
                let changed = transaction.execute(
                    "UPDATE operations SET record = ?2 WHERE id = ?1",
                    params![id, encoded],
                )?;
                if changed != 1 {
                    return Err(rusqlite::Error::QueryReturnedNoRows);
                }
                if terminal {
                    transaction.execute(
                        "DELETE FROM cancellation_requests WHERE operation_id = ?1",
                        [id],
                    )?;
                }
                Self::delete_history(&transaction, &removed)?;
                transaction.commit()
            })
            .await?;
        }
        store.operations = operations;
        if record.state.is_terminal() {
            store.cancellation.remove(&record.id);
            store.cancellation_requested.remove(&record.id);
        }
        for id in removed {
            store.logs.remove(&id);
            store.cancellation.remove(&id);
            store.cancellation_requested.remove(&id);
        }
        Ok(())
    }

    /// Reconcile the operation projection from Agent1's canonical sessions.
    ///
    /// # Errors
    ///
    /// Returns an error when Agent1 cannot be read or a reconciled record cannot be persisted.
    pub async fn reconcile_agents(&self) -> Result<(), String> {
        let sessions = match self.read_agent_sessions().await {
            Ok(sessions) => sessions,
            Err(error) => {
                // Agent1 itself cannot be read: say so instead of repeating the last projection.
                self.mark_established_by(AGENT1, ObservationState::Unavailable, None)
                    .await;
                return Err(error);
            }
        };
        let mut seen = HashSet::new();
        for session in sessions {
            seen.insert(Uuid::new_v5(
                &Uuid::NAMESPACE_OID,
                session.capsule_id.as_bytes(),
            ));
            self.reconcile_agent(session)
                .await
                .map_err(|e| e.to_string())?;
        }
        // Agent1 no longer establishes anything outside `seen`; a restored Running record must not
        // keep asserting a worker that the canonical owner cannot see.
        self.mark_established_by(AGENT1, ObservationState::Detached, Some(&seen))
            .await;
        Ok(())
    }

    /// Reconcile the operation projection from Action1's canonical lifecycle records.
    ///
    /// Only a record that has crossed the durable execution boundary is background work: a proposal
    /// still waiting on a decision is something asking for attention, not something running.
    ///
    /// # Errors
    ///
    /// Returns an error when Action1 cannot be read or a reconciled record cannot be persisted.
    pub async fn reconcile_actions(&self) -> Result<(), String> {
        let records = match self.read_action_records().await {
            Ok(records) => records,
            Err(error) => {
                self.mark_established_by(ACTION1, ObservationState::Unavailable, None)
                    .await;
                return Err(error);
            }
        };
        let mut seen = HashSet::new();
        for record in records {
            if record.execution_started.is_none() {
                continue;
            }
            let id = Self::action_operation_id(record.proposal.proposal_id);
            seen.insert(id);
            self.reconcile_action(id, record)
                .await
                .map_err(|e| e.to_string())?;
        }
        self.mark_established_by(ACTION1, ObservationState::Detached, Some(&seen))
            .await;
        Ok(())
    }

    async fn read_action_records(&self) -> Result<Vec<ActionRecord>, String> {
        let endpoint = cybou_fabric::ACTION;
        let encoded: Vec<u8> = zbus::Connection::session()
            .await
            .map_err(|e| e.to_string())?
            .call_method(
                Some(endpoint.service),
                endpoint.object_path,
                Some(endpoint.interface),
                "RecentRecords",
                &(),
            )
            .await
            .map_err(|e| e.to_string())?
            .body()
            .deserialize()
            .map_err(|e| e.to_string())?;
        cybou_fabric::decode(&encoded).map_err(|e| e.to_string())
    }

    /// One proposal is one operation, for as long as the record exists.
    fn action_operation_id(proposal_id: Uuid) -> Uuid {
        let namespace = Uuid::new_v5(&Uuid::NAMESPACE_OID, b"cybou.operation.action");
        Uuid::new_v5(&namespace, proposal_id.as_bytes())
    }

    /// The operation category for a typed action verb, without guessing at one it does not know.
    fn action_kind(operation: &str) -> OperationKind {
        match operation {
            "service.restart" => OperationKind::ServiceRestart,
            "service.stop" => OperationKind::ServiceStop,
            "package.install" => OperationKind::PackageInstall,
            // Upgrading is not removal, and the table has no separate category for it. Its own verb
            // is more honest than borrowing one that means something else.
            other => OperationKind::Custom(other.to_owned()),
        }
    }

    async fn reconcile_action(&self, id: Uuid, record: ActionRecord) -> rusqlite::Result<()> {
        let observed_at = OffsetDateTime::now_utc();
        let started = record
            .execution_started
            .as_ref()
            .ok_or(rusqlite::Error::InvalidQuery)?;
        // The attempt is what the executor says it did; the outcome is what the host saw afterwards.
        // Whether the work ran is the first question, so the lifecycle follows the attempt.
        let (state, observation, step) = match record.attempt.as_ref().map(|a| &a.report) {
            None => (
                OperationState::Running,
                ObservationState::Known,
                "Executing".to_owned(),
            ),
            Some(AttemptReport::Completed) => (
                OperationState::Completed,
                ObservationState::Known,
                if record.outcome.is_some() {
                    "Concluded".to_owned()
                } else {
                    "Carried out; awaiting independent observation".to_owned()
                },
            ),
            Some(AttemptReport::Failed { because }) => (
                OperationState::Failed {
                    error: because.clone(),
                },
                ObservationState::Known,
                "Failed".to_owned(),
            ),
            Some(AttemptReport::Refused { because }) => (
                OperationState::Refused {
                    because: because.clone(),
                },
                ObservationState::Known,
                "Declined before anything ran".to_owned(),
            ),
            // Action1 establishes that it began and that nobody knows how it ended. The lifecycle
            // stays what was last published, and the observation says why it stopped being visible.
            Some(AttemptReport::DidNotFinish) => (
                OperationState::Running,
                ObservationState::Detached,
                "Began; this host does not know how it ended".to_owned(),
            ),
        };
        let subject = started
            .target_resource
            .strip_prefix("systemd:")
            .map(|unit| SubjectRef::Service {
                name: unit.to_owned(),
                node_id: None,
            });
        let finished_at = record.attempt.as_ref().and_then(|attempt| attempt.ended_at);
        let operation = OperationRecord {
            id,
            kind: Self::action_kind(&started.operation),
            state,
            label: format!("{} on {}", started.operation, started.target_resource),
            initiator: record.proposal.proposed_by.clone(),
            subject,
            progress: OperationProgress {
                // Nothing between Action1 and the Body reports a fraction of an action done.
                percent: None,
                step,
                total_steps: None,
                current_step: None,
                detail: None,
            },
            // A permit that is already executing has no cancel. Offering a button that does nothing
            // is exactly the thing this owner exists to stop doing.
            cancellable: false,
            establisher: Some(ACTION1.to_owned()),
            cancellation_requested: false,
            observation,
            last_observed_at: Some(observed_at),
            started_at: started.started_at,
            updated_at: observed_at,
            finished_at,
        };
        let existing = self
            .store
            .lock()
            .await
            .operations
            .iter()
            .find(|value| value.id == id)
            .cloned();
        match existing {
            Some(current) if Self::only_freshness_differs(&current, &operation) => {
                self.touch_observation(id, observed_at).await;
                Ok(())
            }
            Some(_) => self.update(operation).await,
            None => self.register_external(operation).await,
        }
    }

    async fn read_agent_sessions(&self) -> Result<Vec<SessionView>, String> {
        let endpoint = cybou_fabric::AGENT;
        let encoded: Vec<u8> = zbus::Connection::session()
            .await
            .map_err(|e| e.to_string())?
            .call_method(
                Some(endpoint.service),
                endpoint.object_path,
                Some(endpoint.interface),
                "Sessions",
                &(),
            )
            .await
            .map_err(|e| e.to_string())?
            .body()
            .deserialize()
            .map_err(|e| e.to_string())?;
        cybou_fabric::decode(&encoded).map_err(|e| e.to_string())
    }

    /// Apply an observation verdict to one owner's live operations, excluding those just observed.
    ///
    /// Lifecycle state is left untouched: only the executing worker may publish a terminal state.
    pub async fn mark_established_by(
        &self,
        establisher: &str,
        observation: ObservationState,
        seen: Option<&HashSet<Uuid>>,
    ) {
        let stale: Vec<OperationRecord> = self
            .store
            .lock()
            .await
            .operations
            .iter()
            .filter(|record| {
                record.establisher.as_deref() == Some(establisher)
                    && !record.state.is_terminal()
                    && record.observation != observation
                    && seen.is_none_or(|seen| !seen.contains(&record.id))
            })
            .cloned()
            .collect();
        for mut record in stale {
            record.observation = observation;
            record.cancellable = false;
            record.updated_at = OffsetDateTime::now_utc();
            let _ = self.update(record).await;
        }
    }

    /// Whether two records differ only in when they were observed.
    fn only_freshness_differs(current: &OperationRecord, next: &OperationRecord) -> bool {
        let mut probe = next.clone();
        probe.updated_at = current.updated_at;
        probe.last_observed_at = current.last_observed_at;
        probe == *current
    }

    /// Refresh observation timestamps in memory only, so an unchanged agent costs no disk I/O.
    async fn touch_observation(&self, id: Uuid, observed_at: OffsetDateTime) {
        let mut store = self.store.lock().await;
        if let Some(record) = store.operations.iter_mut().find(|value| value.id == id) {
            record.updated_at = observed_at;
            record.last_observed_at = Some(observed_at);
        }
    }

    async fn reconcile_agent(&self, session: SessionView) -> rusqlite::Result<()> {
        let id = Uuid::new_v5(&Uuid::NAMESPACE_OID, session.capsule_id.as_bytes());
        let observed_at = OffsetDateTime::now_utc();
        let state = match session.standing {
            Standing::Ended
                if session
                    .task
                    .as_ref()
                    .and_then(|task| task.result.as_ref())
                    .is_some() =>
            {
                OperationState::Completed
            }
            Standing::Ended
                if session
                    .ended_because
                    .as_deref()
                    .is_some_and(|reason| reason.contains("stopped")) =>
            {
                OperationState::Cancelled
            }
            Standing::Ended => OperationState::Failed {
                error: session.ended_because.clone().unwrap_or_else(|| {
                    "Agent1 reported an end without a successful result".to_owned()
                }),
            },
            Standing::Ending
            | Standing::Launching
            | Standing::Running
            | Standing::Paused
            | Standing::Quarantined => OperationState::Running,
        };
        let record = OperationRecord {
            id,
            kind: OperationKind::AgentTask,
            state,
            label: format!("{} agent task", session.agent),
            initiator: Proposer::Agent {
                capsule_id: session.capsule_id,
                agent: session.agent.clone(),
            },
            subject: Some(SubjectRef::Agent {
                capsule_id: session.capsule_id.to_string(),
                agent_type: session.agent.clone(),
            }),
            progress: OperationProgress {
                percent: None,
                step: session.task.as_ref().map_or_else(
                    || format!("{:?}", session.standing),
                    |task| task.phase.clone(),
                ),
                total_steps: None,
                current_step: None,
                detail: None,
            },
            cancellable: session.is_live(),
            establisher: Some(AGENT1.to_owned()),
            cancellation_requested: false,
            observation: ObservationState::Known,
            last_observed_at: Some(observed_at),
            started_at: session.started_at,
            updated_at: observed_at,
            finished_at: session.ended_at,
        };
        let existing = self
            .store
            .lock()
            .await
            .operations
            .iter()
            .find(|value| value.id == id)
            .cloned();
        match existing {
            Some(current) => {
                let mut record = record;
                // A pending cancel request belongs to the owner, not to the Agent1 projection.
                record.cancellation_requested =
                    current.cancellation_requested && !record.state.is_terminal();
                if Self::only_freshness_differs(&current, &record) {
                    // Nothing semantic changed: refreshing memory is honest and costs no fsync.
                    self.touch_observation(id, observed_at).await;
                    Ok(())
                } else {
                    self.update(record).await
                }
            }
            None => self.register_external(record).await,
        }
    }

    async fn register_external(&self, record: OperationRecord) -> rusqlite::Result<()> {
        let mut store = self.store.lock().await;
        let mut operations = store.operations.clone();
        if !operations.iter().any(|value| value.id == record.id) {
            operations.insert(0, record.clone());
        }
        let removed = Self::bounded_history(&mut operations);
        {
            let encoded =
                cybou_fabric::encode(&record).map_err(|_| rusqlite::Error::InvalidQuery)?;
            let id = record.id.to_string();
            let removed = removed.clone();
            self.durably(move |connection| {
                let transaction = connection.transaction()?;
                transaction.execute(
                    "INSERT OR IGNORE INTO operations(id, record) VALUES (?1, ?2)",
                    params![id, encoded],
                )?;
                Self::delete_history(&transaction, &removed)?;
                transaction.commit()
            })
            .await?;
        }
        store.operations = operations;
        for id in removed {
            store.logs.remove(&id);
            store.cancellation.remove(&id);
            store.cancellation_requested.remove(&id);
        }
        Ok(())
    }
}

impl Default for Operation1Service {
    fn default() -> Self {
        Self::new()
    }
}

#[interface(name = "org.cybou.Runtime.Operation1")]
impl Operation1Service {
    async fn list(&self) -> Vec<u8> {
        let values = self.store.lock().await.operations.clone();
        cybou_fabric::encode(&values).unwrap_or_default()
    }

    async fn get(&self, id: &str) -> Vec<u8> {
        let Ok(id) = Uuid::parse_str(id) else {
            return Vec::new();
        };
        let value = self
            .store
            .lock()
            .await
            .operations
            .iter()
            .find(|v| v.id == id)
            .cloned();
        value
            .and_then(|v| cybou_fabric::encode(&v).ok())
            .unwrap_or_default()
    }

    async fn logs(&self, id: &str) -> Vec<u8> {
        let Ok(id) = Uuid::parse_str(id) else {
            return Vec::new();
        };
        let values = self
            .store
            .lock()
            .await
            .logs
            .get(&id)
            .cloned()
            .unwrap_or_default();
        cybou_fabric::encode(&values).unwrap_or_default()
    }

    async fn cancel(&self, id: &str) -> Vec<u8> {
        let Ok(id) = Uuid::parse_str(id) else {
            return cybou_fabric::encode(&CancelOutcome::NotFound).unwrap_or_default();
        };
        let (operation, sender) = {
            let store = self.store.lock().await;
            let Some(operation) = store.operations.iter().find(|v| v.id == id).cloned() else {
                return cybou_fabric::encode(&CancelOutcome::NotFound).unwrap_or_default();
            };
            (operation, store.cancellation.get(&id).cloned())
        };
        let result = if operation.state.is_terminal() {
            CancelOutcome::Conflict
        } else if !operation.cancellable {
            CancelOutcome::Refused
        } else if let Some(sender) = sender {
            if self.request_local_cancellation(id).await.is_ok() {
                // A dropped receiver does not lose the request: it is durable and the worker gets
                // it immediately when it reattaches.
                let _ = sender.send(true);
                CancelOutcome::CancellationAccepted
            } else {
                CancelOutcome::Conflict
            }
        } else if let Some(SubjectRef::Agent { capsule_id, .. }) = &operation.subject {
            let endpoint = cybou_fabric::AGENT;
            let stopped = match zbus::Connection::session().await {
                Ok(connection) => connection
                    .call_method(
                        Some(endpoint.service),
                        endpoint.object_path,
                        Some(endpoint.interface),
                        "Stop",
                        &(capsule_id.as_str(),),
                    )
                    .await
                    .ok()
                    .and_then(|reply| reply.body().deserialize::<bool>().ok())
                    .unwrap_or(false),
                Err(_) => false,
            };
            if stopped {
                // Agent1 confirmed teardown before returning, so the terminal state is observed.
                let mut cancelled = operation;
                cancelled.state = OperationState::Cancelled;
                cancelled.cancellable = false;
                cancelled.cancellation_requested = false;
                cancelled.observation = ObservationState::Known;
                cancelled.updated_at = OffsetDateTime::now_utc();
                cancelled.last_observed_at = Some(cancelled.updated_at);
                cancelled.finished_at = Some(cancelled.updated_at);
                if self.update(cancelled).await.is_ok() {
                    CancelOutcome::CancellationConfirmed
                } else {
                    CancelOutcome::Conflict
                }
            } else {
                CancelOutcome::Refused
            }
        } else if self.request_local_cancellation(id).await.is_ok() {
            CancelOutcome::CancellationAccepted
        } else {
            CancelOutcome::Conflict
        };
        if matches!(result, CancelOutcome::CancellationAccepted) {
            self.mark_cancellation_requested(id).await;
        }
        // Operation1 requests cancellation; only the worker may later publish Cancelled.
        cybou_fabric::encode(&result).unwrap_or_default()
    }

    /// Whether a detached worker has a durable cancellation request waiting for it.
    async fn cancellation_requested(&self, id: &str) -> bool {
        let Ok(id) = Uuid::parse_str(id) else {
            return false;
        };
        self.store.lock().await.cancellation_requested.contains(&id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cybou_protocol::{
        action::Proposer,
        operation::{OperationKind, OperationProgress, OperationState},
    };
    use time::OffsetDateTime;

    #[tokio::test]
    async fn cancel_signals_worker_without_falsely_finishing_record() {
        let service = Operation1Service::new();
        let id = Uuid::new_v4();
        let now = OffsetDateTime::now_utc();
        let mut token = service
            .register(OperationRecord {
                id,
                kind: OperationKind::IndexWorkspace,
                state: OperationState::Running,
                label: "index".into(),
                initiator: Proposer::Mind,
                subject: None,
                progress: OperationProgress::default(),
                cancellable: true,
                establisher: None,
                cancellation_requested: false,
                observation: cybou_protocol::operation::ObservationState::Known,
                last_observed_at: None,
                started_at: now,
                updated_at: now,
                finished_at: None,
            })
            .await
            .expect("register operation");
        let encoded = service.cancel(&id.to_string()).await;
        assert!(matches!(
            cybou_fabric::decode::<CancelOutcome>(&encoded),
            Ok(CancelOutcome::CancellationAccepted)
        ));
        token.changed().await.expect("cancellation signal");
        assert!(*token.borrow());
        assert_eq!(
            service.store.lock().await.operations[0].state,
            OperationState::Running
        );
    }

    #[tokio::test]
    async fn operations_survive_owner_restart_without_inventing_a_worker() {
        let directory = std::env::temp_dir().join(format!("cybou_operation_{}", Uuid::new_v4()));
        let path = directory.join("operations.sqlite3");
        let id = Uuid::new_v4();
        let now = OffsetDateTime::now_utc();
        let service = Operation1Service::with_database(&path).expect("open database");
        let _token = service
            .register(OperationRecord {
                id,
                kind: OperationKind::IndexWorkspace,
                state: OperationState::Running,
                label: "durable index".into(),
                initiator: Proposer::Mind,
                subject: None,
                progress: OperationProgress::default(),
                cancellable: true,
                establisher: None,
                cancellation_requested: false,
                observation: cybou_protocol::operation::ObservationState::Known,
                last_observed_at: None,
                started_at: now,
                updated_at: now,
                finished_at: None,
            })
            .await
            .expect("persist operation");
        service
            .append_log(
                id,
                OperationLogEntry {
                    timestamp: now,
                    stream: "system".into(),
                    text: "durable log".into(),
                },
            )
            .await
            .expect("persist log");
        let mut completed = service.store.lock().await.operations[0].clone();
        completed.state = OperationState::Completed;
        completed.finished_at = Some(now);
        service.update(completed).await.expect("persist completion");
        drop(service);

        let restored = Operation1Service::with_database(&path).expect("restore database");
        assert_eq!(restored.store.lock().await.operations[0].id, id);
        assert_eq!(
            restored.store.lock().await.operations[0].state,
            OperationState::Completed
        );
        assert_eq!(restored.store.lock().await.logs[&id][0].text, "durable log");
        let cancel = restored.cancel(&id.to_string()).await;
        assert!(matches!(
            cybou_fabric::decode::<CancelOutcome>(&cancel),
            Ok(CancelOutcome::Conflict)
        ));
        std::fs::remove_dir_all(directory).expect("remove database");
    }

    #[tokio::test]
    async fn detached_worker_reattaches_without_losing_a_cancellation_request() {
        let directory = std::env::temp_dir().join(format!("cybou_reattach_{}", Uuid::new_v4()));
        let path = directory.join("operations.sqlite3");
        let id = Uuid::new_v4();
        let now = OffsetDateTime::now_utc();
        let record = OperationRecord {
            id,
            kind: OperationKind::IndexWorkspace,
            state: OperationState::Running,
            label: "restartable index".into(),
            initiator: Proposer::Mind,
            subject: None,
            progress: OperationProgress::default(),
            cancellable: true,
            establisher: None,
            cancellation_requested: false,
            observation: cybou_protocol::operation::ObservationState::Known,
            last_observed_at: None,
            started_at: now,
            updated_at: now,
            finished_at: None,
        };
        let service = Operation1Service::with_database(&path).expect("open database");
        let _original_worker = service
            .register(record.clone())
            .await
            .expect("register worker");
        drop(service);

        let restored = Operation1Service::with_database(&path).expect("restore owner");
        let encoded = restored.cancel(&id.to_string()).await;
        assert!(matches!(
            cybou_fabric::decode::<CancelOutcome>(&encoded),
            Ok(CancelOutcome::CancellationAccepted)
        ));
        assert!(restored.cancellation_requested(&id.to_string()).await);
        drop(restored);

        let restored_again = Operation1Service::with_database(&path).expect("restore again");
        let token = restored_again.reattach(id).await.expect("reattach worker");
        assert!(*token.borrow(), "reattached worker missed cancellation");

        let mut cancelled = record;
        cancelled.state = OperationState::Cancelled;
        cancelled.cancellable = false;
        cancelled.updated_at = OffsetDateTime::now_utc();
        cancelled.finished_at = Some(cancelled.updated_at);
        restored_again
            .update(cancelled)
            .await
            .expect("worker publishes terminal state");
        assert!(!restored_again.cancellation_requested(&id.to_string()).await);
        drop(restored_again);

        let final_owner = Operation1Service::with_database(&path).expect("final restore");
        assert!(!final_owner.cancellation_requested(&id.to_string()).await);
        std::fs::remove_dir_all(directory).expect("remove database");
    }

    #[tokio::test]
    async fn agent_reconciliation_uses_owner_phase_and_never_simulates_percent() {
        let service = Operation1Service::new();
        let now = OffsetDateTime::now_utc();
        let capsule_id = Uuid::new_v4();
        service
            .reconcile_agent(SessionView {
                capsule_id,
                agent: "opencode".into(),
                profile: "bounded".into(),
                workspace: "/workspace".into(),
                standing: Standing::Running,
                ended_because: None,
                started_at: now,
                expires_at: now + time::Duration::hours(1),
                ended_at: None,
                model_class: None,
                spend: None,
                spend_observed_at: None,
                memory_mib: 512,
                cpus: 1,
                tasks_max: 64,
                hosts: Vec::new(),
                units: Vec::new(),
                task: Some(cybou_protocol::agent::AgentTaskView {
                    prompt: "inspect".into(),
                    phase: "Reading repository".into(),
                    result: None,
                    refused_permissions: Vec::new(),
                }),
            })
            .await
            .expect("reconcile Agent1 view");

        let store = service.store.lock().await;
        assert_eq!(store.operations.len(), 1);
        assert_eq!(store.operations[0].progress.step, "Reading repository");
        assert_eq!(store.operations[0].progress.percent, None);
        assert_eq!(
            store.operations[0].id,
            Uuid::new_v5(&Uuid::NAMESPACE_OID, capsule_id.as_bytes())
        );
    }

    fn live_agent_operation(id: Uuid, now: OffsetDateTime) -> OperationRecord {
        OperationRecord {
            id,
            kind: OperationKind::AgentTask,
            state: OperationState::Running,
            label: "opencode agent task".into(),
            initiator: Proposer::Mind,
            subject: None,
            progress: OperationProgress::default(),
            cancellable: true,
            establisher: Some(AGENT1.to_owned()),
            cancellation_requested: false,
            observation: ObservationState::Known,
            last_observed_at: Some(now),
            started_at: now,
            updated_at: now,
            finished_at: None,
        }
    }

    #[tokio::test]
    async fn accepted_cancellation_is_published_as_a_request_not_as_an_ending() {
        let service = Operation1Service::new();
        let id = Uuid::new_v4();
        let now = OffsetDateTime::now_utc();
        let _token = service
            .register(OperationRecord {
                id,
                kind: OperationKind::IndexWorkspace,
                state: OperationState::Running,
                label: "index".into(),
                initiator: Proposer::Mind,
                subject: None,
                progress: OperationProgress::default(),
                cancellable: true,
                establisher: None,
                cancellation_requested: false,
                observation: ObservationState::Known,
                last_observed_at: None,
                started_at: now,
                updated_at: now,
                finished_at: None,
            })
            .await
            .expect("register operation");

        let encoded = service.cancel(&id.to_string()).await;
        assert!(matches!(
            cybou_fabric::decode::<CancelOutcome>(&encoded),
            Ok(CancelOutcome::CancellationAccepted)
        ));

        let store = service.store.lock().await;
        let record = &store.operations[0];
        assert_eq!(record.state, OperationState::Running);
        assert!(record.cancellation_requested);
    }

    #[tokio::test]
    async fn a_vanished_agent_session_stops_claiming_a_worker_that_agent1_cannot_see() {
        let service = Operation1Service::new();
        let now = OffsetDateTime::now_utc();
        let vanished = Uuid::new_v4();
        let still_running = Uuid::new_v4();
        service
            .register_external(live_agent_operation(vanished, now))
            .await
            .expect("register vanished agent operation");
        service
            .register_external(live_agent_operation(still_running, now))
            .await
            .expect("register observed agent operation");

        let seen = HashSet::from([still_running]);
        service
            .mark_established_by(AGENT1, ObservationState::Detached, Some(&seen))
            .await;

        let store = service.store.lock().await;
        let detached = store
            .operations
            .iter()
            .find(|record| record.id == vanished)
            .expect("vanished operation retained");
        // Detached is an observation verdict: the lifecycle state stays whatever the worker last
        // published, because no one observed this work ending.
        assert_eq!(detached.observation, ObservationState::Detached);
        assert_eq!(detached.state, OperationState::Running);
        assert!(!detached.cancellable);
        let observed = store
            .operations
            .iter()
            .find(|record| record.id == still_running)
            .expect("observed operation retained");
        assert_eq!(observed.observation, ObservationState::Known);
    }

    #[tokio::test]
    async fn an_unchanged_agent_projection_refreshes_freshness_without_touching_disk() {
        let directory = std::env::temp_dir().join(format!("cybou_operation_{}", Uuid::new_v4()));
        let path = directory.join("operations.sqlite3");
        let service = Operation1Service::with_database(&path).expect("open database");
        let id = Uuid::new_v4();
        let now = OffsetDateTime::now_utc();
        let record = live_agent_operation(id, now);
        service
            .register_external(record.clone())
            .await
            .expect("register agent operation");

        let mut repeat = record.clone();
        repeat.updated_at = now + time::Duration::seconds(2);
        repeat.last_observed_at = Some(repeat.updated_at);
        assert!(Operation1Service::only_freshness_differs(&record, &repeat));

        service.touch_observation(id, repeat.updated_at).await;
        let store = service.store.lock().await;
        let refreshed = &store.operations[0];
        assert_eq!(refreshed.last_observed_at, Some(repeat.updated_at));
        assert_eq!(refreshed.observation, ObservationState::Known);
        drop(store);
        let _ = std::fs::remove_dir_all(&directory);
    }

    fn executing_action(report: Option<AttemptReport>) -> ActionRecord {
        use cybou_protocol::action::{
            ActionProposal, AuthorizationDecision, AuthorizationVerdict, ExecutionAttempt,
            ExecutionStarted, RiskLevel,
        };
        let proposal_id = Uuid::new_v4();
        let decision_id = Uuid::new_v4();
        let attempt_id = Uuid::new_v4();
        let now = OffsetDateTime::now_utc();
        ActionRecord {
            proposal: ActionProposal {
                proposal_id,
                proposed_by: Proposer::Mind,
                cause_id: None,
                intent: "recover the database".into(),
                operation: "service.restart".into(),
                target_resource: "systemd:postgresql.service".into(),
                parameters: Vec::new(),
                risk_level: RiskLevel::Medium,
                reversible: true,
                proposed_at: now,
            },
            checks: Vec::new(),
            decision: AuthorizationDecision {
                decision_id,
                proposal_id,
                verdict: AuthorizationVerdict::Granted,
                checked_capabilities: Vec::new(),
                decided_at: now,
            },
            permit_id: Some(Uuid::new_v4()),
            execution_started: Some(ExecutionStarted {
                attempt_id,
                proposal_id,
                decision_id,
                operation: "service.restart".into(),
                target_resource: "systemd:postgresql.service".into(),
                started_at: now,
            }),
            attempt: report.map(|report| ExecutionAttempt {
                attempt_id,
                proposal_id,
                decision_id,
                operation: "service.restart".into(),
                target_resource: "systemd:postgresql.service".into(),
                report,
                body_readings: Vec::new(),
                started_at: now,
                ended_at: Some(now),
            }),
            outcome: None,
        }
    }

    #[tokio::test]
    async fn an_executing_action_becomes_one_operation_that_offers_no_cancel() {
        let service = Operation1Service::new();
        let record = executing_action(None);
        let id = Operation1Service::action_operation_id(record.proposal.proposal_id);
        service
            .reconcile_action(id, record.clone())
            .await
            .expect("reconcile an executing action");

        let store = service.store.lock().await;
        let operation = &store.operations[0];
        assert_eq!(operation.state, OperationState::Running);
        assert_eq!(operation.kind, OperationKind::ServiceRestart);
        assert_eq!(operation.establisher.as_deref(), Some(ACTION1));
        // A permit that is already executing cannot be recalled, so no cancel is offered.
        assert!(!operation.cancellable);
        // Nothing between Action1 and the Body reports a fraction of an action done.
        assert_eq!(operation.progress.percent, None);
        assert_eq!(
            operation.subject,
            Some(SubjectRef::Service {
                name: "postgresql.service".to_owned(),
                node_id: None,
            })
        );
        // The identity is derived from the proposal, so the same action is the same operation.
        assert_eq!(
            operation.id,
            Operation1Service::action_operation_id(record.proposal.proposal_id)
        );
    }

    #[tokio::test]
    async fn a_refusal_is_not_recorded_as_a_failure_and_an_unknown_ending_is_not_recorded_at_all() {
        let service = Operation1Service::new();

        let refused = executing_action(Some(AttemptReport::Refused {
            because: "policy declined it".to_owned(),
        }));
        let refused_id = Operation1Service::action_operation_id(refused.proposal.proposal_id);
        service
            .reconcile_action(refused_id, refused)
            .await
            .expect("reconcile a refusal");

        let unfinished = executing_action(Some(AttemptReport::DidNotFinish));
        let unfinished_id = Operation1Service::action_operation_id(unfinished.proposal.proposal_id);
        service
            .reconcile_action(unfinished_id, unfinished)
            .await
            .expect("reconcile an unknown ending");

        let store = service.store.lock().await;
        let refused = store
            .operations
            .iter()
            .find(|record| record.id == refused_id)
            .expect("the refusal");
        // Nothing ran, so there is nothing to have failed.
        assert_eq!(
            refused.state,
            OperationState::Refused {
                because: "policy declined it".to_owned()
            }
        );

        let unfinished = store
            .operations
            .iter()
            .find(|record| record.id == unfinished_id)
            .expect("the unknown ending");
        // Something may well have happened, so it is neither completed nor failed; what is known is
        // that it can no longer be observed.
        assert_eq!(unfinished.state, OperationState::Running);
        assert_eq!(unfinished.observation, ObservationState::Detached);
    }

    #[tokio::test]
    async fn one_owner_going_quiet_does_not_detach_the_other_owner_s_operations() {
        let service = Operation1Service::new();
        let now = OffsetDateTime::now_utc();
        let agent = Uuid::new_v4();
        service
            .register_external(live_agent_operation(agent, now))
            .await
            .expect("register an agent operation");
        let action = executing_action(None);
        let action_id = Operation1Service::action_operation_id(action.proposal.proposal_id);
        service
            .reconcile_action(action_id, action)
            .await
            .expect("register an action operation");

        // Action1 cannot be read; Agent1 can.
        service
            .mark_established_by(ACTION1, ObservationState::Unavailable, None)
            .await;

        let store = service.store.lock().await;
        let agent = store
            .operations
            .iter()
            .find(|record| record.id == agent)
            .expect("the agent operation");
        assert_eq!(agent.observation, ObservationState::Known);
        let action = store
            .operations
            .iter()
            .find(|record| record.id == action_id)
            .expect("the action operation");
        assert_eq!(action.observation, ObservationState::Unavailable);
    }

    #[tokio::test]
    async fn retention_bounds_terminal_history_and_logs_but_never_active_work() {
        let service = Operation1Service::new();
        let now = OffsetDateTime::now_utc();
        for index in 0..=MAX_TERMINAL_OPERATIONS {
            let _ = service
                .register(OperationRecord {
                    id: Uuid::new_v4(),
                    kind: OperationKind::IndexWorkspace,
                    state: OperationState::Completed,
                    label: format!("completed {index}"),
                    initiator: Proposer::Mind,
                    subject: None,
                    progress: OperationProgress::default(),
                    cancellable: false,
                    establisher: None,
                    cancellation_requested: false,
                    observation: cybou_protocol::operation::ObservationState::Known,
                    last_observed_at: None,
                    started_at: now,
                    updated_at: now,
                    finished_at: Some(now),
                })
                .await
                .expect("register terminal history");
        }
        let active_id = Uuid::new_v4();
        let _ = service
            .register(OperationRecord {
                id: active_id,
                kind: OperationKind::IndexWorkspace,
                state: OperationState::Running,
                label: "active".into(),
                initiator: Proposer::Mind,
                subject: None,
                progress: OperationProgress::default(),
                cancellable: true,
                establisher: None,
                cancellation_requested: false,
                observation: cybou_protocol::operation::ObservationState::Known,
                last_observed_at: None,
                started_at: now,
                updated_at: now,
                finished_at: None,
            })
            .await
            .expect("register active work");
        for index in 0..=MAX_LOG_ENTRIES_PER_OPERATION {
            service
                .append_log(
                    active_id,
                    OperationLogEntry {
                        timestamp: now,
                        stream: "system".into(),
                        text: index.to_string(),
                    },
                )
                .await
                .expect("append bounded log");
        }

        let store = service.store.lock().await;
        assert_eq!(
            store
                .operations
                .iter()
                .filter(|operation| operation.state.is_terminal())
                .count(),
            MAX_TERMINAL_OPERATIONS
        );
        assert!(
            store
                .operations
                .iter()
                .any(|operation| operation.id == active_id)
        );
        assert_eq!(store.logs[&active_id].len(), MAX_LOG_ENTRIES_PER_OPERATION);
        assert_eq!(store.logs[&active_id][0].text, "1");
    }
}
