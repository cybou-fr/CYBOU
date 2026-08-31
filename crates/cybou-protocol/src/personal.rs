// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Domain models for the CYBOU Personal Desktop Pack: Mail, Calendar, Notes, Contacts.
//!
//! Every personal desktop entity adheres to the CYBOU Cognitive Contract: it can optionally
//! associate directly with a [`SubjectRef`] (such as a local agent session, background operation,
//! system service, or project host path) to bridge personal productivity with host cognition.

use crate::subject::SubjectRef;
use serde::{Deserialize, Serialize};

/// Folder kind for personal electronic mailboxes.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MailFolderKind {
    /// Inbound messages.
    Inbox,
    /// Outbound dispatched messages.
    Sent,
    /// Unfinished drafts.
    Drafts,
    /// Long-term archived messages.
    Archive,
    /// Deleted messages awaiting purge.
    Trash,
}

/// A configured personal email account.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MailAccountRecord {
    /// Unique account ID.
    pub id: String,
    /// Human-friendly display label (e.g. `Work IMAP`, `Personal Proton`).
    pub name: String,
    /// Full email address.
    pub email: String,
    /// Server hostname / provider.
    pub server: String,
    /// Unread messages count.
    pub unread_count: usize,
}

/// A single email message or thread header.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MailMessageRecord {
    /// Unique message identifier.
    pub id: String,
    /// Associated account ID.
    pub account_id: String,
    /// Current mailbox folder.
    pub folder: MailFolderKind,
    /// Sender address and display name.
    pub from: String,
    /// Recipient addresses.
    pub to: Vec<String>,
    /// Subject line.
    pub subject: String,
    /// Plaintext preview snippet.
    pub preview: String,
    /// Full message body (markdown or text).
    pub body: String,
    /// ISO 8601 creation or arrival timestamp.
    pub timestamp: String,
    /// Whether this message is unread.
    pub is_unread: bool,
    /// Whether this message is flagged / starred.
    pub is_starred: bool,
    /// Optional cognitive anchor to a system subject (e.g. Agent capsule or Operation).
    pub referenced_subject: Option<SubjectRef>,
}

/// A calendar event entry.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarEventRecord {
    /// Unique event identifier.
    pub id: String,
    /// Event title.
    pub title: String,
    /// Event description.
    pub description: String,
    /// ISO 8601 start timestamp.
    pub start_time: String,
    /// ISO 8601 end timestamp.
    pub end_time: String,
    /// Whether this event spans the entire day.
    pub is_all_day: bool,
    /// Location or meeting URL.
    pub location: Option<String>,
    /// Participant names or email addresses.
    pub attendees: Vec<String>,
    /// Accent color category (e.g. `indigo`, `emerald`, `amber`, `rose`).
    pub color_category: String,
    /// Optional cognitive anchor to a system subject (e.g. Agent capsule or Operation).
    pub referenced_subject: Option<SubjectRef>,
}

/// A personal Markdown note or knowledge snippet.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteRecord {
    /// Unique note identifier.
    pub id: String,
    /// Note title.
    pub title: String,
    /// Markdown content.
    pub content_markdown: String,
    /// Organizational tag labels.
    pub tags: Vec<String>,
    /// ISO 8601 last modified timestamp.
    pub updated_at: String,
    /// Whether this note is pinned at the top.
    pub is_pinned: bool,
    /// Optional cognitive anchor to a system subject.
    pub referenced_subject: Option<SubjectRef>,
}

/// An address book contact record.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactRecord {
    /// Unique contact identifier.
    pub id: String,
    /// Full name.
    pub name: String,
    /// Primary email address.
    pub email: String,
    /// Professional role or title.
    pub role: String,
    /// Company or organization.
    pub organization: String,
    /// Telephone number.
    pub phone: Option<String>,
    /// Category tags.
    pub tags: Vec<String>,
    /// Freeform notes.
    pub notes: String,
    /// Optional cognitive anchor to an agent or project subject.
    pub referenced_subject: Option<SubjectRef>,
}
