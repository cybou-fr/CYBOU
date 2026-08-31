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
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::RwLock;
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
    store_path: Option<PathBuf>,
    accounts: RwLock<Vec<MailAccountRecord>>,
    messages: RwLock<Vec<MailMessageRecord>>,
    calendar_events: RwLock<Vec<CalendarEventRecord>>,
    notes: RwLock<Vec<NoteRecord>>,
    contacts: RwLock<Vec<ContactRecord>>,
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
            let candidate = PathBuf::from("/var/lib/cybou/personal-store.json");
            if candidate.parent().is_some_and(|p| p.exists()) {
                return Some(candidate);
            }
        }
        None
    }

    /// Construct `PersonalHub` with an optional backing store path.
    #[must_use]
    pub fn with_optional_store(store_path: Option<PathBuf>) -> Self {
        let mut loaded = PersonalStore::default();
        if let Some(ref path) = store_path {
            if let Ok(bytes) = std::fs::read(path) {
                if let Ok(parsed) = serde_json::from_slice::<PersonalStore>(&bytes) {
                    loaded = parsed;
                }
            }
        }

        Self {
            store_path,
            accounts: RwLock::new(loaded.accounts),
            messages: RwLock::new(loaded.messages),
            calendar_events: RwLock::new(loaded.calendar_events),
            notes: RwLock::new(loaded.notes),
            contacts: RwLock::new(loaded.contacts),
        }
    }

    fn persist(&self) {
        let Some(ref path) = self.store_path else {
            return;
        };

        let store = PersonalStore {
            accounts: self
                .accounts
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .clone(),
            messages: self
                .messages
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .clone(),
            calendar_events: self
                .calendar_events
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .clone(),
            notes: self.notes.read().unwrap_or_else(|e| e.into_inner()).clone(),
            contacts: self
                .contacts
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .clone(),
        };

        if let Ok(json_bytes) = serde_json::to_vec_pretty(&store) {
            let tmp_path = path.with_extension("tmp");
            if std::fs::write(&tmp_path, json_bytes).is_ok() {
                let _ = std::fs::rename(tmp_path, path);
            }
        }
    }

    /// Retrieve email accounts and messages.
    #[must_use]
    pub fn get_mail(
        &self,
        selected_account_id: Option<String>,
        selected_folder: Option<MailFolderKind>,
    ) -> MailProjection {
        let accounts = self
            .accounts
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let all_messages = self.messages.read().unwrap_or_else(|e| e.into_inner());

        let active_account_id = selected_account_id.unwrap_or_default();
        let active_folder = selected_folder.unwrap_or(MailFolderKind::Inbox);

        let filtered_messages: Vec<MailMessageRecord> = if active_account_id.is_empty() {
            all_messages.clone()
        } else {
            all_messages
                .iter()
                .filter(|m| m.account_id == active_account_id && m.folder == active_folder)
                .cloned()
                .collect()
        };

        MailProjection {
            schema_version: WEB_SCHEMA_V1,
            accounts,
            messages: filtered_messages,
            active_account_id,
            active_folder,
        }
    }

    /// Send an email message.
    pub fn send_mail(&self, _req: SendMailRequest) -> Result<MailMessageRecord, GatewayError> {
        Err(GatewayError::Refused)
    }

    /// Retrieve calendar events.
    #[must_use]
    pub fn get_calendar(&self) -> CalendarProjection {
        let events = self
            .calendar_events
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone();

        CalendarProjection {
            schema_version: WEB_SCHEMA_V1,
            events,
        }
    }

    /// Create a calendar event.
    pub fn create_calendar_event(
        &self,
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

        {
            let mut events = self
                .calendar_events
                .write()
                .unwrap_or_else(|e| e.into_inner());
            events.push(event.clone());
        }
        self.persist();
        Ok(event)
    }

    /// Retrieve notes.
    #[must_use]
    pub fn get_notes(&self) -> NotesProjection {
        let notes = self.notes.read().unwrap_or_else(|e| e.into_inner()).clone();

        NotesProjection {
            schema_version: WEB_SCHEMA_V1,
            notes,
        }
    }

    /// Create a new note.
    pub fn create_note(&self, req: CreateNoteRequest) -> Result<NoteRecord, GatewayError> {
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

        {
            let mut notes = self.notes.write().unwrap_or_else(|e| e.into_inner());
            notes.push(note.clone());
        }
        self.persist();
        Ok(note)
    }

    /// Update an existing note.
    pub fn update_note(&self, req: UpdateNoteRequest) -> Result<NoteRecord, GatewayError> {
        let mut notes = self.notes.write().unwrap_or_else(|e| e.into_inner());
        let note = notes
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

        let cloned = note.clone();
        drop(notes);
        self.persist();
        Ok(cloned)
    }

    /// Retrieve contacts directory.
    #[must_use]
    pub fn get_contacts(&self) -> ContactsProjection {
        let contacts = self
            .contacts
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone();

        ContactsProjection {
            schema_version: WEB_SCHEMA_V1,
            contacts,
        }
    }

    /// Create a new contact.
    pub fn create_contact(&self, req: CreateContactRequest) -> Result<ContactRecord, GatewayError> {
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

        {
            let mut contacts = self.contacts.write().unwrap_or_else(|e| e.into_inner());
            contacts.push(contact.clone());
        }
        self.persist();
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
            .create_note(CreateNoteRequest {
                title: "Test Note".to_owned(),
                content_markdown: "Hello World".to_owned(),
                tags: vec!["test".to_owned()],
                is_pinned: false,
                referenced_subject: None,
            })
            .expect("created note");

        assert_eq!(note.title, "Test Note");
        assert_eq!(hub.get_notes().notes.len(), 1);

        // Reload in new hub instance
        let reloaded = PersonalHub::with_optional_store(Some(store_path));
        let notes = reloaded.get_notes().notes;
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].title, "Test Note");

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }
}
