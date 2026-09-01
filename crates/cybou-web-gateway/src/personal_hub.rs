// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Provider and governor for the Personal Desktop Pack (Mail, Calendar, Notes, Contacts).

use cybou_protocol::personal::{
    CalendarEventRecord, ContactRecord, MailAccountRecord, MailFolderKind, MailMessageRecord,
    NoteRecord,
};
use cybou_web_contracts::{
    CalendarProjection, ContactsProjection, CreateCalendarEventRequest, CreateContactRequest,
    CreateNoteRequest, MailProjection, NotesProjection, SendMailRequest, UpdateNoteRequest,
    WEB_SCHEMA_V1,
};
use rusqlite::{Connection, OptionalExtension as _, params};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, RwLock};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::state::GatewayError;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct PersonalStore {
    #[serde(default)]
    accounts: Vec<MailAccountRecord>,
    #[serde(default)]
    messages: Vec<MailMessageRecord>,
    #[serde(default)]
    calendar_events: Vec<CalendarEventRecord>,
    #[serde(default)]
    notes: Vec<NoteRecord>,
    #[serde(default)]
    contacts: Vec<ContactRecord>,
}

/// Hub managing personal email accounts, calendar schedules, notes, and contacts.
pub struct PersonalHub {
    database: Option<Mutex<Connection>>,
    database_failed: bool,
    volatile: RwLock<HashMap<u32, PersonalStore>>,
}

impl Default for PersonalHub {
    fn default() -> Self {
        Self::new()
    }
}

impl PersonalHub {
    /// Create a new `PersonalHub` with storage loaded from default store path if available.
    #[must_use]
    pub fn new() -> Self {
        let store_path = Self::default_store_path();
        Self::with_optional_store(store_path)
    }

    /// Determine the default store path from environment or standard Linux location.
    #[must_use]
    pub fn default_store_path() -> Option<PathBuf> {
        if let Ok(path) = std::env::var("CYBOU_PERSONAL_STORE") {
            return Some(PathBuf::from(path));
        }
        #[cfg(target_os = "linux")]
        {
            let candidate = PathBuf::from("/var/lib/cybou/personal-store.sqlite3");
            if candidate.parent().is_some_and(std::path::Path::exists) {
                return Some(candidate);
            }
        }
        None
    }

    /// Construct `PersonalHub` with an optional backing store path.
    #[must_use]
    pub fn with_optional_store(store_path: Option<PathBuf>) -> Self {
        let requested_database = store_path.is_some();
        let database = store_path.and_then(|path| {
            let connection = Connection::open(path).ok()?;
            connection
                .execute_batch(
                    "PRAGMA journal_mode=WAL;
                     PRAGMA foreign_keys=ON;
                     CREATE TABLE IF NOT EXISTS personal_stores (
                         uid INTEGER PRIMARY KEY NOT NULL,
                         payload TEXT NOT NULL,
                         updated_at INTEGER NOT NULL
                     );",
                )
                .ok()?;
            Some(Mutex::new(connection))
        });
        Self {
            database_failed: requested_database && database.is_none(),
            database,
            volatile: RwLock::new(HashMap::new()),
        }
    }

    fn read_store(&self, uid: u32) -> Result<PersonalStore, GatewayError> {
        if self.database_failed {
            return Err(GatewayError::Internal);
        }
        let Some(database) = &self.database else {
            return Ok(self
                .volatile
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .get(&uid)
                .cloned()
                .unwrap_or_default());
        };
        let database = database.lock().map_err(|_| GatewayError::Internal)?;
        let payload: Option<String> = database
            .query_row(
                "SELECT payload FROM personal_stores WHERE uid=?1",
                [uid],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| GatewayError::Internal)?;
        payload.map_or_else(
            || Ok(PersonalStore::default()),
            |payload| serde_json::from_str(&payload).map_err(|_| GatewayError::Internal),
        )
    }

    fn mutate<T>(
        &self,
        uid: u32,
        change: impl FnOnce(&mut PersonalStore) -> Result<T, GatewayError>,
    ) -> Result<T, GatewayError> {
        if self.database_failed {
            return Err(GatewayError::Internal);
        }
        let Some(database) = &self.database else {
            let mut stores = self
                .volatile
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            return change(stores.entry(uid).or_default());
        };
        let mut database = database.lock().map_err(|_| GatewayError::Internal)?;
        let transaction = database.transaction().map_err(|_| GatewayError::Internal)?;
        let payload: Option<String> = transaction
            .query_row(
                "SELECT payload FROM personal_stores WHERE uid=?1",
                [uid],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| GatewayError::Internal)?;
        let mut store = payload.map_or_else(
            || Ok(PersonalStore::default()),
            |payload| serde_json::from_str(&payload).map_err(|_| GatewayError::Internal),
        )?;
        let answer = change(&mut store)?;
        let payload = serde_json::to_string(&store).map_err(|_| GatewayError::Internal)?;
        transaction
            .execute(
                "INSERT INTO personal_stores(uid,payload,updated_at) VALUES(?1,?2,?3)
                 ON CONFLICT(uid) DO UPDATE SET payload=excluded.payload, updated_at=excluded.updated_at",
                params![uid, payload, OffsetDateTime::now_utc().unix_timestamp()],
            )
            .map_err(|_| GatewayError::Internal)?;
        transaction.commit().map_err(|_| GatewayError::Internal)?;
        Ok(answer)
    }

    /// Retrieve email accounts and messages.
    #[must_use]
    pub fn get_mail(
        &self,
        uid: u32,
        selected_account_id: Option<String>,
        selected_folder: Option<MailFolderKind>,
    ) -> Result<MailProjection, GatewayError> {
        let store = self.read_store(uid)?;

        let active_account_id = selected_account_id.unwrap_or_default();
        let active_folder = selected_folder.unwrap_or(MailFolderKind::Inbox);

        let filtered_messages: Vec<MailMessageRecord> = if active_account_id.is_empty() {
            store.messages.clone()
        } else {
            store
                .messages
                .iter()
                .filter(|m| m.account_id == active_account_id && m.folder == active_folder)
                .cloned()
                .collect()
        };

        Ok(MailProjection {
            schema_version: WEB_SCHEMA_V1,
            accounts: store.accounts,
            messages: filtered_messages,
            active_account_id,
            active_folder,
        })
    }

    /// Send an email message.
    pub fn send_mail(
        &self,
        _uid: u32,
        _req: SendMailRequest,
    ) -> Result<MailMessageRecord, GatewayError> {
        Err(GatewayError::Refused)
    }

    /// Retrieve calendar events.
    #[must_use]
    pub fn get_calendar(&self, uid: u32) -> Result<CalendarProjection, GatewayError> {
        Ok(CalendarProjection {
            schema_version: WEB_SCHEMA_V1,
            events: self.read_store(uid)?.calendar_events,
        })
    }

    /// Create a calendar event.
    pub fn create_calendar_event(
        &self,
        uid: u32,
        req: CreateCalendarEventRequest,
    ) -> Result<CalendarEventRecord, GatewayError> {
        let event = CalendarEventRecord {
            id: format!("cal-{}", Uuid::new_v4()),
            title: req.title,
            description: req.description,
            start_time: req.start_time,
            end_time: req.end_time,
            is_all_day: req.is_all_day,
            location: req.location,
            attendees: req.attendees,
            color_category: req.color_category,
            referenced_subject: req.referenced_subject,
        };

        self.mutate(uid, |store| {
            store.calendar_events.push(event.clone());
            Ok(())
        })?;
        Ok(event)
    }

    /// Retrieve notes.
    #[must_use]
    pub fn get_notes(&self, uid: u32) -> Result<NotesProjection, GatewayError> {
        Ok(NotesProjection {
            schema_version: WEB_SCHEMA_V1,
            notes: self.read_store(uid)?.notes,
        })
    }

    /// Create a new note.
    pub fn create_note(
        &self,
        uid: u32,
        req: CreateNoteRequest,
    ) -> Result<NoteRecord, GatewayError> {
        let now_str = OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default();

        let note = NoteRecord {
            id: format!("note-{}", Uuid::new_v4()),
            title: req.title,
            content_markdown: req.content_markdown,
            tags: req.tags,
            updated_at: now_str,
            is_pinned: req.is_pinned,
            referenced_subject: req.referenced_subject,
        };

        self.mutate(uid, |store| {
            store.notes.push(note.clone());
            Ok(())
        })?;
        Ok(note)
    }

    /// Update an existing note.
    pub fn update_note(
        &self,
        uid: u32,
        req: UpdateNoteRequest,
    ) -> Result<NoteRecord, GatewayError> {
        self.mutate(uid, |store| {
            let note = store
                .notes
                .iter_mut()
                .find(|n| n.id == req.id)
                .ok_or(GatewayError::NotFound)?;
            note.title = req.title;
            note.content_markdown = req.content_markdown;
            note.tags = req.tags;
            note.is_pinned = req.is_pinned;
            note.updated_at = OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default();
            Ok(note.clone())
        })
    }

    /// Retrieve contacts directory.
    #[must_use]
    pub fn get_contacts(&self, uid: u32) -> Result<ContactsProjection, GatewayError> {
        Ok(ContactsProjection {
            schema_version: WEB_SCHEMA_V1,
            contacts: self.read_store(uid)?.contacts,
        })
    }

    /// Create a new contact.
    pub fn create_contact(
        &self,
        uid: u32,
        req: CreateContactRequest,
    ) -> Result<ContactRecord, GatewayError> {
        let contact = ContactRecord {
            id: format!("cnt-{}", Uuid::new_v4()),
            name: req.name,
            email: req.email,
            role: req.role,
            organization: req.organization,
            phone: req.phone,
            tags: req.tags,
            notes: req.notes,
            referenced_subject: req.referenced_subject,
        };

        self.mutate(uid, |store| {
            store.contacts.push(contact.clone());
            Ok(())
        })?;
        Ok(contact)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn personal_hub_persists_and_reloads() {
        let tmp_dir = std::env::temp_dir().join(format!("cybou_test_personal_{}", Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&tmp_dir);
        let store_path = tmp_dir.join("personal.json");

        let hub = PersonalHub::with_optional_store(Some(store_path.clone()));
        let note = hub
            .create_note(
                1000,
                CreateNoteRequest {
                    title: "Test Note".to_owned(),
                    content_markdown: "Hello World".to_owned(),
                    tags: vec!["test".to_owned()],
                    is_pinned: false,
                    referenced_subject: None,
                },
            )
            .expect("created note");

        assert_eq!(note.title, "Test Note");
        assert_eq!(hub.get_notes(1000).expect("notes").notes.len(), 1);

        // Reload in new hub instance
        let reloaded = PersonalHub::with_optional_store(Some(store_path));
        let notes = reloaded.get_notes(1000).expect("notes").notes;
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].title, "Test Note");

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn personal_records_are_partitioned_by_authenticated_uid() {
        let hub = PersonalHub::with_optional_store(None);
        hub.create_note(
            1000,
            CreateNoteRequest {
                title: "Alice only".to_owned(),
                content_markdown: "private".to_owned(),
                tags: Vec::new(),
                is_pinned: false,
                referenced_subject: None,
            },
        )
        .expect("Alice writes");

        assert_eq!(hub.get_notes(1000).expect("Alice reads").notes.len(), 1);
        assert!(hub.get_notes(1001).expect("Bob reads").notes.is_empty());
    }
}
