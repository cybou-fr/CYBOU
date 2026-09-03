// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! The owner of one Linux account's Personal Core: mail, calendar, notes and contacts.
//!
//! The gateway partitioned this data by numeric UID, which stopped two accounts reading each
//! other's records but left every account's records inside one process that no account owns. This
//! owner runs *as* the person. There is no UID field anywhere below, because the process identity
//! is the partition: a request that reaches this socket can only ever be answered from the store of
//! the user this process runs as, and there is no code path that could name another.

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
use std::{path::Path, sync::Mutex, time::Duration};
use time::OffsetDateTime;
use uuid::Uuid;

/// Largest request this owner will read before refusing.
pub const MAX_REQUEST_BYTES: u64 = 256 * 1024;

/// How long one request may take to arrive.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// What the gateway may ask this user's owner for.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Request {
    /// Mail accounts and the messages of one folder.
    Mail {
        /// Account to filter by, or empty for everything this owner holds.
        account_id: Option<String>,
        /// Folder to filter by; the inbox when absent.
        folder: Option<MailFolderKind>,
    },
    /// Send a message.
    SendMail(Box<SendMailRequest>),
    /// Calendar events.
    Calendar,
    /// Create one calendar event.
    CreateCalendarEvent(Box<CreateCalendarEventRequest>),
    /// Notes.
    Notes,
    /// Create one note.
    CreateNote(Box<CreateNoteRequest>),
    /// Replace the content of one existing note.
    UpdateNote(Box<UpdateNoteRequest>),
    /// Contacts.
    Contacts,
    /// Create one contact.
    CreateContact(Box<CreateContactRequest>),
}

/// What this user's owner answers.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Response {
    /// A mail projection.
    Mail(Box<MailProjection>),
    /// A calendar projection.
    Calendar(Box<CalendarProjection>),
    /// A notes projection.
    Notes(Box<NotesProjection>),
    /// A contacts projection.
    Contacts(Box<ContactsProjection>),
    /// One created or updated note.
    Note(Box<NoteRecord>),
    /// One created calendar event.
    Event(Box<CalendarEventRecord>),
    /// One created contact.
    Contact(Box<ContactRecord>),
    /// The named record does not exist.
    NotFound,
    /// This owner will not do that.
    Refused,
    /// The store could not be read or written; nothing was changed.
    Failed,
}

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

/// One Linux account's personal records.
pub struct Owner {
    database: Mutex<Connection>,
}

impl Owner {
    /// Open, creating the schema when needed.
    ///
    /// # Errors
    ///
    /// Returns an error when the database cannot be opened or its schema cannot be created.
    pub fn open(path: impl AsRef<Path>) -> rusqlite::Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|_| rusqlite::Error::InvalidPath(parent.into()))?;
        }
        let connection = Connection::open(path)?;
        connection.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=FULL;
             CREATE TABLE IF NOT EXISTS personal (
                 id INTEGER PRIMARY KEY NOT NULL CHECK (id = 0),
                 payload TEXT NOT NULL,
                 updated_at INTEGER NOT NULL
             );",
        )?;
        Ok(Self {
            database: Mutex::new(connection),
        })
    }

    fn read(&self) -> Result<PersonalStore, Response> {
        let database = self.database.lock().map_err(|_| Response::Failed)?;
        let payload: Option<String> = database
            .query_row("SELECT payload FROM personal WHERE id = 0", [], |row| {
                row.get(0)
            })
            .optional()
            .map_err(|_| Response::Failed)?;
        payload.map_or_else(
            || Ok(PersonalStore::default()),
            |payload| serde_json::from_str(&payload).map_err(|_| Response::Failed),
        )
    }

    fn mutate<T>(
        &self,
        change: impl FnOnce(&mut PersonalStore) -> Result<T, Response>,
    ) -> Result<T, Response> {
        let mut database = self.database.lock().map_err(|_| Response::Failed)?;
        let transaction = database.transaction().map_err(|_| Response::Failed)?;
        let payload: Option<String> = transaction
            .query_row("SELECT payload FROM personal WHERE id = 0", [], |row| {
                row.get(0)
            })
            .optional()
            .map_err(|_| Response::Failed)?;
        let mut store = payload.map_or_else(
            || Ok(PersonalStore::default()),
            |payload| serde_json::from_str(&payload).map_err(|_| Response::Failed),
        )?;
        let answer = change(&mut store)?;
        let payload = serde_json::to_string(&store).map_err(|_| Response::Failed)?;
        transaction
            .execute(
                "INSERT INTO personal(id, payload, updated_at) VALUES(0, ?1, ?2)
                 ON CONFLICT(id) DO UPDATE SET payload = excluded.payload,
                                               updated_at = excluded.updated_at",
                params![payload, OffsetDateTime::now_utc().unix_timestamp()],
            )
            .map_err(|_| Response::Failed)?;
        transaction.commit().map_err(|_| Response::Failed)?;
        Ok(answer)
    }

    fn now() -> String {
        OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default()
    }

    /// Answer one request from this account's own records.
    #[must_use]
    pub fn answer(&self, request: Request) -> Response {
        match self.answer_inner(request) {
            Ok(response) | Err(response) => response,
        }
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one match arm per request; splitting it would hide the surface"
    )]
    fn answer_inner(&self, request: Request) -> Result<Response, Response> {
        match request {
            Request::Mail { account_id, folder } => {
                let store = self.read()?;
                let active_account_id = account_id.unwrap_or_default();
                let active_folder = folder.unwrap_or(MailFolderKind::Inbox);
                let messages = if active_account_id.is_empty() {
                    store.messages.clone()
                } else {
                    store
                        .messages
                        .iter()
                        .filter(|message| {
                            message.account_id == active_account_id
                                && message.folder == active_folder
                        })
                        .cloned()
                        .collect()
                };
                Ok(Response::Mail(Box::new(MailProjection {
                    schema_version: WEB_SCHEMA_V1,
                    accounts: store.accounts,
                    messages,
                    active_account_id,
                    active_folder,
                })))
            }
            // No provider carries this anywhere. Recording a "Sent" message would make the desktop
            // show a delivery that never happened.
            Request::SendMail(_) => Ok(Response::Refused),
            Request::Calendar => Ok(Response::Calendar(Box::new(CalendarProjection {
                schema_version: WEB_SCHEMA_V1,
                events: self.read()?.calendar_events,
            }))),
            Request::CreateCalendarEvent(request) => {
                let event = CalendarEventRecord {
                    id: format!("cal-{}", Uuid::new_v4()),
                    title: request.title,
                    description: request.description,
                    start_time: request.start_time,
                    end_time: request.end_time,
                    is_all_day: request.is_all_day,
                    location: request.location,
                    attendees: request.attendees,
                    color_category: request.color_category,
                    referenced_subject: request.referenced_subject,
                };
                self.mutate(|store| {
                    store.calendar_events.push(event.clone());
                    Ok(())
                })?;
                Ok(Response::Event(Box::new(event)))
            }
            Request::Notes => Ok(Response::Notes(Box::new(NotesProjection {
                schema_version: WEB_SCHEMA_V1,
                notes: self.read()?.notes,
            }))),
            Request::CreateNote(request) => {
                let note = NoteRecord {
                    id: format!("note-{}", Uuid::new_v4()),
                    title: request.title,
                    content_markdown: request.content_markdown,
                    tags: request.tags,
                    updated_at: Self::now(),
                    is_pinned: request.is_pinned,
                    referenced_subject: request.referenced_subject,
                };
                self.mutate(|store| {
                    store.notes.push(note.clone());
                    Ok(())
                })?;
                Ok(Response::Note(Box::new(note)))
            }
            Request::UpdateNote(request) => {
                let updated = self.mutate(|store| {
                    let note = store
                        .notes
                        .iter_mut()
                        .find(|note| note.id == request.id)
                        .ok_or(Response::NotFound)?;
                    note.title = request.title;
                    note.content_markdown = request.content_markdown;
                    note.tags = request.tags;
                    note.is_pinned = request.is_pinned;
                    note.updated_at = Self::now();
                    Ok(note.clone())
                })?;
                Ok(Response::Note(Box::new(updated)))
            }
            Request::Contacts => Ok(Response::Contacts(Box::new(ContactsProjection {
                schema_version: WEB_SCHEMA_V1,
                contacts: self.read()?.contacts,
            }))),
            Request::CreateContact(request) => {
                let contact = ContactRecord {
                    id: format!("cnt-{}", Uuid::new_v4()),
                    name: request.name,
                    email: request.email,
                    role: request.role,
                    organization: request.organization,
                    phone: request.phone,
                    tags: request.tags,
                    notes: request.notes,
                    referenced_subject: request.referenced_subject,
                };
                self.mutate(|store| {
                    store.contacts.push(contact.clone());
                    Ok(())
                })?;
                Ok(Response::Contact(Box::new(contact)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Owner, Request, Response};
    use cybou_web_contracts::{CreateNoteRequest, UpdateNoteRequest};

    fn owner(directory: &std::path::Path) -> Owner {
        Owner::open(directory.join("personal.sqlite3")).expect("open owner store")
    }

    fn note(title: &str) -> Request {
        Request::CreateNote(Box::new(CreateNoteRequest {
            title: title.to_owned(),
            content_markdown: "private".to_owned(),
            tags: Vec::new(),
            is_pinned: false,
            referenced_subject: None,
        }))
    }

    #[test]
    fn one_owner_holds_only_its_own_records() {
        let root = std::env::temp_dir().join(format!("cybou_personald_{}", uuid::Uuid::new_v4()));
        let alice = root.join("1000");
        let bob = root.join("1001");
        std::fs::create_dir_all(&alice).expect("alice directory");
        std::fs::create_dir_all(&bob).expect("bob directory");

        let alice_owner = owner(&alice);
        assert!(matches!(
            alice_owner.answer(note("Alice only")),
            Response::Note(_)
        ));

        // There is no request that could ask Bob's owner for Alice's note: the store is the
        // process, and the process is the person.
        let Response::Notes(bobs) = owner(&bob).answer(Request::Notes) else {
            panic!("notes projection");
        };
        assert!(bobs.notes.is_empty());
        let Response::Notes(hers) = alice_owner.answer(Request::Notes) else {
            panic!("notes projection");
        };
        assert_eq!(hers.notes.len(), 1);

        std::fs::remove_dir_all(&root).expect("remove temporary directory");
    }

    #[test]
    fn records_survive_the_owner_restarting() {
        let directory =
            std::env::temp_dir().join(format!("cybou_personald_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).expect("directory");
        assert!(matches!(
            owner(&directory).answer(note("Durable")),
            Response::Note(_)
        ));
        let Response::Notes(notes) = owner(&directory).answer(Request::Notes) else {
            panic!("notes projection");
        };
        assert_eq!(notes.notes.len(), 1);
        assert_eq!(notes.notes[0].title, "Durable");
        std::fs::remove_dir_all(&directory).expect("remove temporary directory");
    }

    #[test]
    fn updating_a_note_that_does_not_exist_changes_nothing() {
        let directory =
            std::env::temp_dir().join(format!("cybou_personald_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).expect("directory");
        let owner = owner(&directory);
        assert!(matches!(
            owner.answer(Request::UpdateNote(Box::new(UpdateNoteRequest {
                id: "note-absent".to_owned(),
                title: "Invented".to_owned(),
                content_markdown: String::new(),
                tags: Vec::new(),
                is_pinned: false,
            }))),
            Response::NotFound
        ));
        let Response::Notes(notes) = owner.answer(Request::Notes) else {
            panic!("notes projection");
        };
        assert!(notes.notes.is_empty());
        std::fs::remove_dir_all(&directory).expect("remove temporary directory");
    }
}
