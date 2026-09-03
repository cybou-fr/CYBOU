// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Gateway transport to an authenticated user's own Personal Core owner.

use std::path::PathBuf;

use async_trait::async_trait;
use cybou_personald::{Request, Response};
use cybou_protocol::personal::{
    CalendarEventRecord, ContactRecord, MailFolderKind, MailMessageRecord, NoteRecord,
};
use cybou_web_contracts::{
    CalendarProjection, ContactsProjection, CreateCalendarEventRequest, CreateContactRequest,
    CreateNoteRequest, MailProjection, NotesProjection, SendMailRequest, UpdateNoteRequest,
};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

use crate::state::{GatewayError, PersonalSource};

const RESPONSE_MAX_BYTES: u64 = 8 * 1024 * 1024;
const ROUND_TRIP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// Per-UID Unix-socket client for Personal Core owners.
pub struct SocketPersonal {
    socket_directory: PathBuf,
}

impl SocketPersonal {
    /// Address owners as `<directory>/<uid>/personal.sock`.
    #[must_use]
    pub fn in_directory(directory: impl Into<PathBuf>) -> Self {
        Self {
            socket_directory: directory.into(),
        }
    }

    async fn ask(&self, uid: u32, request: &Request) -> Result<Response, GatewayError> {
        let socket = self
            .socket_directory
            .join(uid.to_string())
            .join("personal.sock");
        let exchange = async {
            let mut stream = tokio::net::UnixStream::connect(socket)
                .await
                .map_err(|_| GatewayError::Unavailable)?;
            let mut encoded = Vec::new();
            ciborium::into_writer(request, &mut encoded).map_err(|_| GatewayError::Unavailable)?;
            stream
                .write_all(&encoded)
                .await
                .map_err(|_| GatewayError::Unavailable)?;
            stream
                .shutdown()
                .await
                .map_err(|_| GatewayError::Unavailable)?;
            let mut answer = Vec::new();
            (&mut stream)
                .take(RESPONSE_MAX_BYTES + 1)
                .read_to_end(&mut answer)
                .await
                .map_err(|_| GatewayError::Unavailable)?;
            if u64::try_from(answer.len()).is_ok_and(|len| len > RESPONSE_MAX_BYTES) {
                return Err(GatewayError::InvalidProjection);
            }
            ciborium::from_reader(answer.as_slice()).map_err(|_| GatewayError::InvalidProjection)
        };
        tokio::time::timeout(ROUND_TRIP_TIMEOUT, exchange)
            .await
            .map_err(|_| GatewayError::Timeout)?
    }
}

/// Translate the owner's refusals without turning one into another.
fn refusal(response: &Response) -> GatewayError {
    match *response {
        Response::NotFound => GatewayError::NotFound,
        Response::Refused => GatewayError::Refused,
        Response::Failed => GatewayError::Internal,
        _ => GatewayError::InvalidProjection,
    }
}

#[async_trait]
impl PersonalSource for SocketPersonal {
    async fn get_mail(
        &self,
        uid: u32,
        account_id: Option<String>,
        folder: Option<MailFolderKind>,
    ) -> Result<MailProjection, GatewayError> {
        match self.ask(uid, &Request::Mail { account_id, folder }).await? {
            Response::Mail(projection) => Ok(*projection),
            other => Err(refusal(&other)),
        }
    }

    async fn send_mail(
        &self,
        uid: u32,
        request: SendMailRequest,
    ) -> Result<MailMessageRecord, GatewayError> {
        Err(refusal(
            &self.ask(uid, &Request::SendMail(Box::new(request))).await?,
        ))
    }

    async fn get_calendar(&self, uid: u32) -> Result<CalendarProjection, GatewayError> {
        match self.ask(uid, &Request::Calendar).await? {
            Response::Calendar(projection) => Ok(*projection),
            other => Err(refusal(&other)),
        }
    }

    async fn create_calendar_event(
        &self,
        uid: u32,
        request: CreateCalendarEventRequest,
    ) -> Result<CalendarEventRecord, GatewayError> {
        match self
            .ask(uid, &Request::CreateCalendarEvent(Box::new(request)))
            .await?
        {
            Response::Event(event) => Ok(*event),
            other => Err(refusal(&other)),
        }
    }

    async fn get_notes(&self, uid: u32) -> Result<NotesProjection, GatewayError> {
        match self.ask(uid, &Request::Notes).await? {
            Response::Notes(projection) => Ok(*projection),
            other => Err(refusal(&other)),
        }
    }

    async fn create_note(
        &self,
        uid: u32,
        request: CreateNoteRequest,
    ) -> Result<NoteRecord, GatewayError> {
        match self
            .ask(uid, &Request::CreateNote(Box::new(request)))
            .await?
        {
            Response::Note(note) => Ok(*note),
            other => Err(refusal(&other)),
        }
    }

    async fn update_note(
        &self,
        uid: u32,
        request: UpdateNoteRequest,
    ) -> Result<NoteRecord, GatewayError> {
        match self
            .ask(uid, &Request::UpdateNote(Box::new(request)))
            .await?
        {
            Response::Note(note) => Ok(*note),
            other => Err(refusal(&other)),
        }
    }

    async fn get_contacts(&self, uid: u32) -> Result<ContactsProjection, GatewayError> {
        match self.ask(uid, &Request::Contacts).await? {
            Response::Contacts(projection) => Ok(*projection),
            other => Err(refusal(&other)),
        }
    }

    async fn create_contact(
        &self,
        uid: u32,
        request: CreateContactRequest,
    ) -> Result<ContactRecord, GatewayError> {
        match self
            .ask(uid, &Request::CreateContact(Box::new(request)))
            .await?
        {
            Response::Contact(contact) => Ok(*contact),
            other => Err(refusal(&other)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SocketPersonal;
    use crate::state::{GatewayError, PersonalSource};

    #[tokio::test]
    async fn an_absent_owner_is_unavailable_rather_than_an_empty_mailbox() {
        let directory =
            std::env::temp_dir().join(format!("cybou_personal_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).expect("directory");
        let client = SocketPersonal::in_directory(&directory);
        assert!(matches!(
            client.get_notes(1000).await,
            Err(GatewayError::Unavailable)
        ));
        std::fs::remove_dir_all(&directory).expect("remove temporary directory");
    }
}
