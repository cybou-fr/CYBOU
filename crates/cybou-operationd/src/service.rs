// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! D-Bus implementation of `org.cybou.Runtime.Operation1`.

#![allow(missing_docs)]

use cybou_protocol::operation::{OperationLogEntry, OperationRecord};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path, sync::Mutex as SyncMutex};
use tokio::sync::{Mutex, watch};
use uuid::Uuid;
use zbus::interface;

#[derive(Default)]
struct Store {
    operations: Vec<OperationRecord>,
    logs: HashMap<Uuid, Vec<OperationLogEntry>>,
    cancellation: HashMap<Uuid, watch::Sender<bool>>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum CancelResult {
    Cancelled,
    NotFound,
    Conflict,
    Refused,
}

/// Sole lifecycle owner for background operations.
pub struct Operation1Service {
    store: Mutex<Store>,
    database: Option<SyncMutex<Connection>>,
}

impl Operation1Service {
    #[must_use]
    pub fn new() -> Self {
        Self {
            store: Mutex::new(Store::default()),
            database: None,
        }
    }

    /// Open the configured durable database, creating its parent and schema when needed.
    pub fn durable_default() -> rusqlite::Result<Self> {
        let path = std::env::var("CYBOU_OPERATION_STORE")
            .unwrap_or_else(|_| "/var/lib/cybou/operations.sqlite3".to_owned());
        Self::with_database(path)
    }

    /// Open and restore a durable Operation1 database.
    pub fn with_database(path: impl AsRef<Path>) -> rusqlite::Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|_| rusqlite::Error::InvalidPath(parent.into()))?;
        }
        let connection = Connection::open(path)?;
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
        Ok(Self {
            store: Mutex::new(Store {
                operations,
                logs,
                cancellation: HashMap::new(),
            }),
            database: Some(SyncMutex::new(connection)),
        })
    }

    /// Register a real job together with the token its worker observes.
    pub async fn register(
        &self,
        record: OperationRecord,
    ) -> rusqlite::Result<watch::Receiver<bool>> {
        let (sender, receiver) = watch::channel(false);
        let mut store = self.store.lock().await;
        if let Some(database) = &self.database {
            let encoded =
                cybou_fabric::encode(&record).map_err(|_| rusqlite::Error::InvalidQuery)?;
            database.lock().expect("operation database").execute(
                "INSERT INTO operations(id, record) VALUES (?1, ?2)",
                params![record.id.to_string(), encoded],
            )?;
        }
        store.cancellation.insert(record.id, sender);
        store.operations.insert(0, record);
        Ok(receiver)
    }

    /// Append one durable worker log entry.
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
        let sequence = store
            .logs
            .get(&operation_id)
            .map_or(0_i64, |values| values.len() as i64);
        if let Some(database) = &self.database {
            let encoded =
                cybou_fabric::encode(&entry).map_err(|_| rusqlite::Error::InvalidQuery)?;
            database.lock().expect("operation database").execute(
                "INSERT INTO operation_logs(operation_id, sequence, entry) VALUES (?1, ?2, ?3)",
                params![operation_id.to_string(), sequence, encoded],
            )?;
        }
        store.logs.entry(operation_id).or_default().push(entry);
        Ok(())
    }

    /// Replace lifecycle/progress state reported by the worker, durably before projection.
    pub async fn update(&self, record: OperationRecord) -> rusqlite::Result<()> {
        let mut store = self.store.lock().await;
        let position = store
            .operations
            .iter()
            .position(|value| value.id == record.id)
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        if let Some(database) = &self.database {
            let encoded =
                cybou_fabric::encode(&record).map_err(|_| rusqlite::Error::InvalidQuery)?;
            let changed = database.lock().expect("operation database").execute(
                "UPDATE operations SET record = ?2 WHERE id = ?1",
                params![record.id.to_string(), encoded],
            )?;
            if changed != 1 {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
        }
        store.operations[position] = record;
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
        let store = self.store.lock().await;
        let Some(position) = store.operations.iter().position(|v| v.id == id) else {
            return cybou_fabric::encode(&CancelResult::NotFound).unwrap_or_default();
        };
        let operation = &store.operations[position];
        let result = if operation.state.is_terminal() {
            CancelResult::Conflict
        } else if !operation.cancellable {
            CancelResult::Refused
        } else if let Some(sender) = store.cancellation.get(&id) {
            if sender.send(true).is_ok() {
                CancelResult::Cancelled
            } else {
                CancelResult::Conflict
            }
        } else {
            CancelResult::Refused
        };
        // Operation1 requests cancellation; only the worker may later publish Cancelled.
        cybou_fabric::encode(&result).unwrap_or_default()
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
}
