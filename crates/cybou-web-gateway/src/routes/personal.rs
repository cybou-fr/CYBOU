// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! HTTP endpoints for the Personal Desktop Pack: Mail, Calendar, Notes, Contacts.

use axum::{
    Json,
    extract::{Query, State},
};
use cybou_protocol::personal::{
    CalendarEventRecord, ContactRecord, MailFolderKind, MailMessageRecord, NoteRecord,
};
use cybou_web_contracts::{
    CalendarProjection, ContactsProjection, CreateCalendarEventRequest, CreateContactRequest,
    CreateNoteRequest, MailProjection, NotesProjection, SendMailRequest, UpdateNoteRequest,
};
use serde::Deserialize;

use crate::state::{GatewayError, GatewayState};

/// Query parameters for filtering mailbox messages.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MailQuery {
    /// Account identifier to filter messages by.
    pub account_id: Option<String>,
    /// Mail folder filter (Inbox, Sent, Drafts, Trash, Archive).
    pub folder: Option<MailFolderKind>,
}

/// GET `/api/v1/personal/mail`
pub async fn get_mail(
    State(state): State<GatewayState>,
    Query(query): Query<MailQuery>,
) -> Result<Json<MailProjection>, GatewayError> {
    Ok(Json(state.personal.get_mail(query.account_id, query.folder)))
}

/// POST `/api/v1/personal/mail/send`
pub async fn send_mail(
    State(state): State<GatewayState>,
    Json(request): Json<SendMailRequest>,
) -> Result<Json<MailMessageRecord>, GatewayError> {
    let msg = state.personal.send_mail(request)?;
    Ok(Json(msg))
}

/// GET `/api/v1/personal/calendar`
pub async fn get_calendar(
    State(state): State<GatewayState>,
) -> Result<Json<CalendarProjection>, GatewayError> {
    Ok(Json(state.personal.get_calendar()))
}

/// POST `/api/v1/personal/calendar/events`
pub async fn create_calendar_event(
    State(state): State<GatewayState>,
    Json(request): Json<CreateCalendarEventRequest>,
) -> Result<Json<CalendarEventRecord>, GatewayError> {
    let event = state.personal.create_calendar_event(request)?;
    Ok(Json(event))
}

/// GET `/api/v1/personal/notes`
pub async fn get_notes(
    State(state): State<GatewayState>,
) -> Result<Json<NotesProjection>, GatewayError> {
    Ok(Json(state.personal.get_notes()))
}

/// POST `/api/v1/personal/notes`
pub async fn create_note(
    State(state): State<GatewayState>,
    Json(request): Json<CreateNoteRequest>,
) -> Result<Json<NoteRecord>, GatewayError> {
    let note = state.personal.create_note(request)?;
    Ok(Json(note))
}

/// POST `/api/v1/personal/notes/update`
pub async fn update_note(
    State(state): State<GatewayState>,
    Json(request): Json<UpdateNoteRequest>,
) -> Result<Json<NoteRecord>, GatewayError> {
    let note = state.personal.update_note(request)?;
    Ok(Json(note))
}

/// GET `/api/v1/personal/contacts`
pub async fn get_contacts(
    State(state): State<GatewayState>,
) -> Result<Json<ContactsProjection>, GatewayError> {
    Ok(Json(state.personal.get_contacts()))
}

/// POST `/api/v1/personal/contacts`
pub async fn create_contact(
    State(state): State<GatewayState>,
    Json(request): Json<CreateContactRequest>,
) -> Result<Json<ContactRecord>, GatewayError> {
    let contact = state.personal.create_contact(request)?;
    Ok(Json(contact))
}

#[cfg(test)]
mod tests {
    use crate::personal_hub::PersonalHub;
    use cybou_protocol::personal::MailFolderKind;
    use cybou_web_contracts::{
        CreateCalendarEventRequest, CreateContactRequest, CreateNoteRequest, SendMailRequest,
        UpdateNoteRequest,
    };

    #[test]
    fn personal_hub_manages_mail_calendar_notes_contacts() {
        let hub = PersonalHub::new();

        // Mail tests
        let mail = hub.get_mail(None, Some(MailFolderKind::Inbox));
        assert!(mail.accounts.is_empty());
        assert!(mail.messages.is_empty());

        let sent = hub.send_mail(SendMailRequest {
            account_id: "acc-1".to_owned(),
            to: vec!["colleague@cybou.local".to_owned()],
            subject: "Test Subject".to_owned(),
            body: "Test Body".to_owned(),
            referenced_subject: None,
        });
        assert!(sent.is_err());

        // Calendar tests
        let cal = hub.get_calendar();
        assert!(cal.events.is_empty());
        let event = hub.create_calendar_event(CreateCalendarEventRequest {
            title: "Sprint Planning".to_owned(),
            description: "Plan Milestone 7".to_owned(),
            start_time: "2026-08-30T10:00:00Z".to_owned(),
            end_time: "2026-08-30T11:30:00Z".to_owned(),
            is_all_day: false,
            location: None,
            attendees: vec!["operator@cybou.local".to_owned()],
            color_category: "emerald".to_owned(),
            referenced_subject: None,
        });
        assert!(event.is_ok());
        let cal_after = hub.get_calendar();
        assert_eq!(cal_after.events.len(), 1);

        // Notes tests
        let notes = hub.get_notes();
        assert!(notes.notes.is_empty());
        let new_note = hub.create_note(CreateNoteRequest {
            title: "New Ideas".to_owned(),
            content_markdown: "Ideas list".to_owned(),
            tags: vec!["ideas".to_owned()],
            is_pinned: false,
            referenced_subject: None,
        });
        assert!(new_note.is_ok());
        let note_id = new_note.unwrap().id;
        let updated = hub.update_note(UpdateNoteRequest {
            id: note_id,
            title: "Updated Ideas".to_owned(),
            content_markdown: "Updated ideas list".to_owned(),
            tags: vec!["ideas".to_owned(), "v2".to_owned()],
            is_pinned: true,
        });
        assert!(updated.is_ok());
        let notes_after = hub.get_notes();
        assert_eq!(notes_after.notes.len(), 1);

        // Contacts tests
        let contacts = hub.get_contacts();
        assert!(contacts.contacts.is_empty());
        let new_cnt = hub.create_contact(CreateContactRequest {
            name: "Alice Cooper".to_owned(),
            email: "alice@cybou.net".to_owned(),
            role: "Security Analyst".to_owned(),
            organization: "CYBOU SecOps".to_owned(),
            phone: None,
            tags: vec!["security".to_owned()],
            notes: "Internal security review".to_owned(),
            referenced_subject: None,
        });
        assert!(new_cnt.is_ok());
        let contacts_after = hub.get_contacts();
        assert_eq!(contacts_after.contacts.len(), 1);
    }
}
