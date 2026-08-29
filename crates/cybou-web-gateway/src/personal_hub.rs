// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Provider and governor for the Personal Desktop Pack (Mail, Calendar, Notes, Contacts).

use std::sync::RwLock;
use cybou_protocol::personal::{
    CalendarEventRecord, ContactRecord, MailAccountRecord, MailFolderKind, MailMessageRecord, NoteRecord,
};
use cybou_web_contracts::{
    CalendarProjection, ContactsProjection, CreateCalendarEventRequest, CreateContactRequest,
    CreateNoteRequest, MailProjection, NotesProjection, SendMailRequest, UpdateNoteRequest,
    WEB_SCHEMA_V1,
};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::state::GatewayError;

/// Hub managing personal email accounts, calendar schedules, notes, and contacts.
pub struct PersonalHub {
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
    /// Create a new `PersonalHub` with empty initial state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            accounts: RwLock::new(Vec::new()),
            messages: RwLock::new(Vec::new()),
            calendar_events: RwLock::new(Vec::new()),
            notes: RwLock::new(Vec::new()),
            contacts: RwLock::new(Vec::new()),
        }
    }

    /// Retrieve email accounts and messages.
    #[must_use]
    pub fn get_mail(&self, selected_account_id: Option<String>, selected_folder: Option<MailFolderKind>) -> MailProjection {
        let accounts = self.accounts.read().unwrap_or_else(|e| e.into_inner()).clone();
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
        let events = self.calendar_events.read().unwrap_or_else(|e| e.into_inner()).clone();

        CalendarProjection {
            schema_version: WEB_SCHEMA_V1,
            events,
        }
    }

    /// Create a calendar event.
    pub fn create_calendar_event(&self, req: CreateCalendarEventRequest) -> Result<CalendarEventRecord, GatewayError> {
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

        let mut events = self.calendar_events.write().unwrap_or_else(|e| e.into_inner());
        events.push(event.clone());
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

        let mut notes = self.notes.write().unwrap_or_else(|e| e.into_inner());
        notes.push(note.clone());
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

        Ok(note.clone())
    }

    /// Retrieve contacts directory.
    #[must_use]
    pub fn get_contacts(&self) -> ContactsProjection {
        let contacts = self.contacts.read().unwrap_or_else(|e| e.into_inner()).clone();

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

        let mut contacts = self.contacts.write().unwrap_or_else(|e| e.into_inner());
        contacts.push(contact.clone());
        Ok(contact)
    }
}
