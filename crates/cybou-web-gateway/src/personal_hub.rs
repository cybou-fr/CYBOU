// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Thread-safe in-memory provider and governor for the Personal Desktop Pack (Mail, Calendar, Notes, Contacts).

use std::sync::RwLock;
use cybou_protocol::personal::{
    CalendarEventRecord, ContactRecord, MailAccountRecord, MailFolderKind, MailMessageRecord,
    NoteRecord,
};
use cybou_protocol::subject::SubjectRef;
use cybou_web_contracts::{
    CalendarProjection, ContactsProjection, CreateCalendarEventRequest, CreateContactRequest,
    CreateNoteRequest, MailProjection, NotesProjection, SendMailRequest, UpdateNoteRequest,
    WEB_SCHEMA_V1,
};

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
    /// Create a new `PersonalHub` populated with initial cognitive-connected items.
    #[must_use]
    pub fn new() -> Self {
        let default_accounts = vec![
            MailAccountRecord {
                id: "acc-cybou-primary".to_owned(),
                name: "CYBOU Work (IMAP)".to_owned(),
                email: "operator@cybou.local".to_owned(),
                server: "imap.cybou.local".to_owned(),
                unread_count: 2,
            },
            MailAccountRecord {
                id: "acc-personal-vault".to_owned(),
                name: "Encrypted Personal".to_owned(),
                email: "user@vault.sec".to_owned(),
                server: "vault.sec".to_owned(),
                unread_count: 0,
            },
        ];

        let default_messages = vec![
            MailMessageRecord {
                id: "msg-01".to_owned(),
                account_id: "acc-cybou-primary".to_owned(),
                folder: MailFolderKind::Inbox,
                from: "System Security <security@cybou.local>".to_owned(),
                to: vec!["operator@cybou.local".to_owned()],
                subject: "Weekly Sandbox & Landlock Confinement Audit".to_owned(),
                preview: "Confinement report: 0 policy violations detected across active capsules...".to_owned(),
                body: "## Confinement Status Report\n\nAll OpenCode capsules ran under strict Landlock v3 and Seccomp-BPF rules. No unauthorized egress or path traversal attempts detected.".to_owned(),
                timestamp: "2026-08-28T21:40:00Z".to_owned(),
                is_unread: true,
                is_starred: true,
                referenced_subject: Some(SubjectRef::Service {
                    name: "cybou-capsule.service".to_owned(),
                    node_id: None,
                }),
            },
            MailMessageRecord {
                id: "msg-02".to_owned(),
                account_id: "acc-cybou-primary".to_owned(),
                folder: MailFolderKind::Inbox,
                from: "Dr. Elena Rostova <elena.rostova@cybou.net>".to_owned(),
                to: vec!["operator@cybou.local".to_owned()],
                subject: "Living Canvas 2D Spatial Layout Synchronization".to_owned(),
                preview: "The new monotonic z-index and deck collapse invariants are fully verified...".to_owned(),
                body: "Hi Operator,\n\nThe layout engine invariants (L1-L14) have been tested and verified across multi-card clusters. Spatial navigation and non-spatial outline views are in sync.\n\nBest,\nElena".to_owned(),
                timestamp: "2026-08-28T20:15:00Z".to_owned(),
                is_unread: true,
                is_starred: false,
                referenced_subject: None,
            },
            MailMessageRecord {
                id: "msg-03".to_owned(),
                account_id: "acc-cybou-primary".to_owned(),
                folder: MailFolderKind::Sent,
                from: "operator@cybou.local".to_owned(),
                to: vec!["elena.rostova@cybou.net".to_owned()],
                subject: "Re: Living Canvas 2D Spatial Layout Synchronization".to_owned(),
                preview: "Great work. Proceeding with Milestone 6 personal desktop suite...".to_owned(),
                body: "Elena,\n\nProceeding with personal pack integration (Mail, Calendar, Notes, Contacts) with deep cognitive linking.\n\nOperator".to_owned(),
                timestamp: "2026-08-28T20:30:00Z".to_owned(),
                is_unread: false,
                is_starred: false,
                referenced_subject: None,
            },
        ];

        let default_calendar_events = vec![
            CalendarEventRecord {
                id: "evt-01".to_owned(),
                title: "Cognitive Architecture Review".to_owned(),
                description: "Review Action1 causality graph and Mind client subscriptions".to_owned(),
                start_time: "2026-08-29T09:00:00Z".to_owned(),
                end_time: "2026-08-29T10:30:00Z".to_owned(),
                is_all_day: false,
                location: Some("Studio Alpha (Online)".to_owned()),
                attendees: vec!["operator@cybou.local".to_owned(), "elena.rostova@cybou.net".to_owned()],
                color_category: "indigo".to_owned(),
                referenced_subject: Some(SubjectRef::Service {
                    name: "cybou-web-gateway.service".to_owned(),
                    node_id: None,
                }),
            },
            CalendarEventRecord {
                id: "evt-02".to_owned(),
                title: "Automated Borg Vault Snapshot".to_owned(),
                description: "Scheduled daily incremental deduplication".to_owned(),
                start_time: "2026-08-29T20:00:00Z".to_owned(),
                end_time: "2026-08-29T20:15:00Z".to_owned(),
                is_all_day: false,
                location: Some("Host Backup Daemon".to_owned()),
                attendees: vec!["system".to_owned()],
                color_category: "emerald".to_owned(),
                referenced_subject: None,
            },
        ];

        let default_notes = vec![
            NoteRecord {
                id: "note-01".to_owned(),
                title: "Living Canvas Invariants & Principles".to_owned(),
                content_markdown: "# Living Canvas Invariants\n\n- Infinite 2D space with sub-pixel camera pan/zoom\n- No window overlaps when auto-arranging\n- Decks & Clusters form composable workspaces\n- Every subsystem links to a `SubjectRef`".to_owned(),
                tags: vec!["architecture".to_owned(), "canvas".to_owned(), "invariants".to_owned()],
                updated_at: "2026-08-28T22:00:00Z".to_owned(),
                is_pinned: true,
                referenced_subject: None,
            },
            NoteRecord {
                id: "note-02".to_owned(),
                title: "Security Sandboxing Checklist".to_owned(),
                content_markdown: "- [x] Landlock LSM filesystem restrictions\n- [x] Bubblewrap mount namespaces\n- [x] Seccomp-BPF syscall filters\n- [x] Governed egress socket proxy".to_owned(),
                tags: vec!["security".to_owned(), "sandboxing".to_owned()],
                updated_at: "2026-08-28T22:30:00Z".to_owned(),
                is_pinned: false,
                referenced_subject: Some(SubjectRef::Service {
                    name: "cybou-capsule.service".to_owned(),
                    node_id: None,
                }),
            },
        ];

        let default_contacts = vec![
            ContactRecord {
                id: "cnt-01".to_owned(),
                name: "Dr. Elena Rostova".to_owned(),
                email: "elena.rostova@cybou.net".to_owned(),
                role: "Cognitive Systems Architect".to_owned(),
                organization: "DeepMind / CYBOU Labs".to_owned(),
                phone: Some("+1-555-0199".to_owned()),
                tags: vec!["core-team".to_owned(), "architecture".to_owned()],
                notes: "Primary contributor to Action1 causality and continuous epistemic models.".to_owned(),
                referenced_subject: None,
            },
            ContactRecord {
                id: "cnt-02".to_owned(),
                name: "Alexey Vanev".to_owned(),
                email: "alexey.v@cybou.net".to_owned(),
                role: "Kernel & Security Engineer".to_owned(),
                organization: "CYBOU Security".to_owned(),
                phone: Some("+1-555-0144".to_owned()),
                tags: vec!["security".to_owned(), "kernel".to_owned()],
                notes: "Maintainer of Landlock and Seccomp capsule security policies.".to_owned(),
                referenced_subject: None,
            },
        ];

        Self {
            accounts: RwLock::new(default_accounts),
            messages: RwLock::new(default_messages),
            calendar_events: RwLock::new(default_calendar_events),
            notes: RwLock::new(default_notes),
            contacts: RwLock::new(default_contacts),
        }
    }

    /// Retrieve personal email projection for a given account and folder.
    #[must_use]
    pub fn get_mail(&self, account_id: Option<String>, folder: Option<MailFolderKind>) -> MailProjection {
        let accounts = self.accounts.read().expect("read accounts").clone();
        let target_acc = account_id.unwrap_or_else(|| accounts.first().map(|a| a.id.clone()).unwrap_or_default());
        let target_folder = folder.unwrap_or(MailFolderKind::Inbox);

        let all_messages = self.messages.read().expect("read messages");
        let filtered: Vec<MailMessageRecord> = all_messages
            .iter()
            .filter(|m| m.account_id == target_acc && m.folder == target_folder)
            .cloned()
            .collect();

        MailProjection {
            schema_version: WEB_SCHEMA_V1,
            accounts,
            messages: filtered,
            active_account_id: target_acc,
            active_folder: target_folder,
        }
    }

    /// Send a new outgoing email.
    pub fn send_mail(&self, req: SendMailRequest) -> Result<MailMessageRecord, GatewayError> {
        let mut messages = self.messages.write().expect("write messages");
        let id = format!("msg-{:04}", messages.len() + 1);
        let preview = if req.body.len() > 80 {
            format!("{}...", &req.body[..77])
        } else {
            req.body.clone()
        };
        let record = MailMessageRecord {
            id,
            account_id: req.account_id,
            folder: MailFolderKind::Sent,
            from: "operator@cybou.local".to_owned(),
            to: req.to,
            subject: req.subject,
            preview,
            body: req.body,
            timestamp: "2026-08-28T23:30:00Z".to_owned(),
            is_unread: false,
            is_starred: false,
            referenced_subject: req.referenced_subject,
        };
        messages.push(record.clone());
        Ok(record)
    }

    /// Get all scheduled calendar events.
    #[must_use]
    pub fn get_calendar(&self) -> CalendarProjection {
        let events = self.calendar_events.read().expect("read events").clone();
        CalendarProjection {
            schema_version: WEB_SCHEMA_V1,
            events,
        }
    }

    /// Create a new calendar event.
    pub fn create_calendar_event(&self, req: CreateCalendarEventRequest) -> Result<CalendarEventRecord, GatewayError> {
        let mut events = self.calendar_events.write().expect("write events");
        let id = format!("evt-{:03}", events.len() + 1);
        let record = CalendarEventRecord {
            id,
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
        events.push(record.clone());
        Ok(record)
    }

    /// Get all saved personal notes.
    #[must_use]
    pub fn get_notes(&self) -> NotesProjection {
        let notes = self.notes.read().expect("read notes").clone();
        NotesProjection {
            schema_version: WEB_SCHEMA_V1,
            notes,
        }
    }

    /// Create a new note.
    pub fn create_note(&self, req: CreateNoteRequest) -> Result<NoteRecord, GatewayError> {
        let mut notes = self.notes.write().expect("write notes");
        let id = format!("note-{:03}", notes.len() + 1);
        let record = NoteRecord {
            id,
            title: req.title,
            content_markdown: req.content_markdown,
            tags: req.tags,
            updated_at: "2026-08-28T23:35:00Z".to_owned(),
            is_pinned: req.is_pinned,
            referenced_subject: req.referenced_subject,
        };
        notes.push(record.clone());
        Ok(record)
    }

    /// Update an existing note.
    pub fn update_note(&self, req: UpdateNoteRequest) -> Result<NoteRecord, GatewayError> {
        let mut notes = self.notes.write().expect("write notes");
        let found = notes.iter_mut().find(|n| n.id == req.id).ok_or(GatewayError::NotFound)?;
        found.title = req.title;
        found.content_markdown = req.content_markdown;
        found.tags = req.tags;
        found.is_pinned = req.is_pinned;
        found.updated_at = "2026-08-28T23:36:00Z".to_owned();
        Ok(found.clone())
    }

    /// Get all address book contacts.
    #[must_use]
    pub fn get_contacts(&self) -> ContactsProjection {
        let contacts = self.contacts.read().expect("read contacts").clone();
        ContactsProjection {
            schema_version: WEB_SCHEMA_V1,
            contacts,
        }
    }

    /// Create a new address book contact.
    pub fn create_contact(&self, req: CreateContactRequest) -> Result<ContactRecord, GatewayError> {
        let mut contacts = self.contacts.write().expect("write contacts");
        let id = format!("cnt-{:03}", contacts.len() + 1);
        let record = ContactRecord {
            id,
            name: req.name,
            email: req.email,
            role: req.role,
            organization: req.organization,
            phone: req.phone,
            tags: req.tags,
            notes: req.notes,
            referenced_subject: req.referenced_subject,
        };
        contacts.push(record.clone());
        Ok(record)
    }
}
