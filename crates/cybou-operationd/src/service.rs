// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! D-Bus implementation of `org.cybou.Runtime.Operation1`.

#![allow(missing_docs)]

use cybou_protocol::{
    action::Proposer,
    agent::{SessionView, Standing},
    operation::{
        OperationKind, OperationLogEntry, OperationProgress, OperationRecord, OperationState,
    },
    subject::SubjectRef,
};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::{Arc, Mutex as SyncMutex},
};
use time::OffsetDateTime;
use tokio::sync::{Mutex, watch};
use uuid::Uuid;
use zbus::interface;

const MAX_TERMINAL_OPERATIONS: usize = 100;
const MAX_LOG_ENTRIES_PER_OPERATION: usize = 500;

#[derive(Default)]
struct Store {
    operations: Vec<OperationRecord>,
    logs: HashMap<Uuid, Vec<OperationLogEntry>>,
    cancellation: HashMap<Uuid, watch::Sender<bool>>,
    cancellation_requested: HashSet<Uuid>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum CancelResult {
    Cancelled,
    NotFound,
    Conflict,
    Refused,
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

    fn database_lock(
        database: &SyncMutex<Connection>,
    ) -> rusqlite::Result<std::sync::MutexGuard<'_, Connection>> {
        database.lock().map_err(|_| rusqlite::Error::InvalidQuery)
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
                let record =
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
        if let Some(database) = &self.database {
            let encoded =
                cybou_fabric::encode(&record).map_err(|_| rusqlite::Error::InvalidQuery)?;
            let mut connection = Self::database_lock(database)?;
            let transaction = connection.transaction()?;
            transaction.execute(
                "INSERT INTO operations(id, record) VALUES (?1, ?2)",
                params![record.id.to_string(), encoded],
            )?;
            transaction.execute(
                "DELETE FROM cancellation_requests WHERE operation_id = ?1",
                [record.id.to_string()],
            )?;
            Self::delete_history(&transaction, &removed)?;
            transaction.commit()?;
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

    fn persist_cancellation_request(&self, id: Uuid) -> rusqlite::Result<()> {
        if let Some(database) = &self.database {
            Self::database_lock(database)?.execute(
                "INSERT OR IGNORE INTO cancellation_requests(operation_id) VALUES (?1)",
                [id.to_string()],
            )?;
        }
        Ok(())
    }

    async fn request_local_cancellation(&self, id: Uuid) -> rusqlite::Result<()> {
        self.persist_cancellation_request(id)?;
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
        if let Some(database) = &self.database {
            let encoded =
                cybou_fabric::encode(&entry).map_err(|_| rusqlite::Error::InvalidQuery)?;
            let mut connection = Self::database_lock(database)?;
            let transaction = connection.transaction()?;
            let sequence: i64 = transaction.query_row(
                "SELECT COALESCE(MAX(sequence), -1) + 1 FROM operation_logs WHERE operation_id = ?1",
                [operation_id.to_string()],
                |row| row.get(0),
            )?;
            transaction.execute(
                "INSERT INTO operation_logs(operation_id, sequence, entry) VALUES (?1, ?2, ?3)",
                params![operation_id.to_string(), sequence, encoded],
            )?;
            transaction.execute(
                "DELETE FROM operation_logs
                 WHERE operation_id = ?1 AND sequence NOT IN (
                     SELECT sequence FROM operation_logs
                     WHERE operation_id = ?1 ORDER BY sequence DESC LIMIT 500
                 )",
                [operation_id.to_string()],
            )?;
            transaction.commit()?;
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
        if let Some(database) = &self.database {
            let encoded =
                cybou_fabric::encode(&record).map_err(|_| rusqlite::Error::InvalidQuery)?;
            let mut connection = Self::database_lock(database)?;
            let transaction = connection.transaction()?;
            let changed = transaction.execute(
                "UPDATE operations SET record = ?2 WHERE id = ?1",
                params![record.id.to_string(), encoded],
            )?;
            if changed != 1 {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
            if record.state.is_terminal() {
                transaction.execute(
                    "DELETE FROM cancellation_requests WHERE operation_id = ?1",
                    [record.id.to_string()],
                )?;
            }
            Self::delete_history(&transaction, &removed)?;
            transaction.commit()?;
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
        let sessions: Vec<SessionView> =
            cybou_fabric::decode(&encoded).map_err(|e| e.to_string())?;
        for session in sessions {
            self.reconcile_agent(session)
                .await
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    async fn reconcile_agent(&self, session: SessionView) -> rusqlite::Result<()> {
        let id = Uuid::new_v5(&Uuid::NAMESPACE_OID, session.capsule_id.as_bytes());
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
            started_at: session.started_at,
            updated_at: OffsetDateTime::now_utc(),
            finished_at: session.ended_at,
        };
        let exists = self
            .store
            .lock()
            .await
            .operations
            .iter()
            .any(|value| value.id == id);
        if exists {
            self.update(record).await
        } else {
            self.register_external(record).await
        }
    }

    async fn register_external(&self, record: OperationRecord) -> rusqlite::Result<()> {
        let mut store = self.store.lock().await;
        let mut operations = store.operations.clone();
        if !operations.iter().any(|value| value.id == record.id) {
            operations.insert(0, record.clone());
        }
        let removed = Self::bounded_history(&mut operations);
        if let Some(database) = &self.database {
            let encoded =
                cybou_fabric::encode(&record).map_err(|_| rusqlite::Error::InvalidQuery)?;
            let mut connection = Self::database_lock(database)?;
            let transaction = connection.transaction()?;
            transaction.execute(
                "INSERT OR IGNORE INTO operations(id, record) VALUES (?1, ?2)",
                params![record.id.to_string(), encoded],
            )?;
            Self::delete_history(&transaction, &removed)?;
            transaction.commit()?;
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
            return cybou_fabric::encode(&CancelResult::NotFound).unwrap_or_default();
        };
        let (operation, sender) = {
            let store = self.store.lock().await;
            let Some(operation) = store.operations.iter().find(|v| v.id == id).cloned() else {
                return cybou_fabric::encode(&CancelResult::NotFound).unwrap_or_default();
            };
            (operation, store.cancellation.get(&id).cloned())
        };
        let result = if operation.state.is_terminal() {
            CancelResult::Conflict
        } else if !operation.cancellable {
            CancelResult::Refused
        } else if let Some(sender) = sender {
            if self.request_local_cancellation(id).await.is_ok() {
                // A dropped receiver does not lose the request: it is durable and the worker gets
                // it immediately when it reattaches.
                let _ = sender.send(true);
                CancelResult::Cancelled
            } else {
                CancelResult::Conflict
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
                let mut cancelled = operation;
                cancelled.state = OperationState::Cancelled;
                cancelled.cancellable = false;
                cancelled.updated_at = OffsetDateTime::now_utc();
                cancelled.finished_at = Some(cancelled.updated_at);
                if self.update(cancelled).await.is_ok() {
                    CancelResult::Cancelled
                } else {
                    CancelResult::Conflict
                }
            } else {
                CancelResult::Refused
            }
        } else if self.request_local_cancellation(id).await.is_ok() {
            CancelResult::Cancelled
        } else {
            CancelResult::Conflict
        };
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
                started_at: now,
                updated_at: now,
                finished_at: None,
            })
            .await
            .expect("register operation");
        let encoded = service.cancel(&id.to_string()).await;
        assert!(matches!(
            cybou_fabric::decode::<CancelResult>(&encoded),
            Ok(CancelResult::Cancelled)
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
            cybou_fabric::decode::<CancelResult>(&cancel),
            Ok(CancelResult::Conflict)
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
            cybou_fabric::decode::<CancelResult>(&encoded),
            Ok(CancelResult::Cancelled)
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
