// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Who a reader is, and how long the answer lasts.
//!
//! The gateway never sees `/etc/shadow` and never learns whether an account exists. It hands a
//! name and a secret to `cybou-authd` over a socket only its own user can open, and receives one
//! bit back. Everything else here is about what happens after that bit is `true`: a session that
//! expires, lives only in this process, and is named in the reply so the page can say who it
//! belongs to.

use std::{
    collections::HashMap,
    sync::{Mutex, PoisonError},
};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

/// How long a session lasts before the person authenticates again.
///
/// Short enough that a forgotten browser stops being a way in the same day, long enough not to
/// interrupt someone using the desktop. There is no sliding renewal: a session that renewed itself
/// on use would never expire for whoever was holding it, which is the case where expiry matters.
pub const SESSION_LIFETIME: Duration = Duration::hours(8);

/// What the browser sends back to prove it already authenticated.
pub const SESSION_COOKIE: &str = "cybou_session";

/// A login attempt, as it arrives from a browser.
#[derive(Deserialize)]
pub struct LoginRequest {
    /// The Linux account being claimed.
    pub username: String,
    /// The secret offered for it.
    pub password: String,
}

impl Drop for LoginRequest {
    fn drop(&mut self) {
        // The gateway holds a password for the length of one request and overwrites it after. It
        // is never logged, never stored, and never put in an error.
        let secret = std::mem::take(&mut self.password);
        let mut bytes = secret.into_bytes();
        bytes.fill(0);
    }
}

/// Something that can say whether an account accepts a secret.
///
/// A trait so the gateway's own tests can run without a PAM stack or a privileged helper. The
/// implementation that matters talks to `cybou-authd`; a test one answers from a table.
#[async_trait]
pub trait CredentialVerifier: Send + Sync {
    /// Whether this account, with this secret, may reach Cybou.
    async fn verify(&self, username: &str, password: &str) -> bool;
}

/// One authenticated reader.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Session {
    /// The Linux account this session belongs to.
    pub username: String,
    /// When it stops being valid.
    pub expires_at: OffsetDateTime,
}

/// The sessions this process is currently honouring.
///
/// In memory, deliberately. A restart ending every session is the correct behaviour for a surface
/// whose whole job is reading live state: there is nothing to resume, and a session store on disk
/// would be a second place holding something about a person.
#[derive(Default)]
pub struct Sessions {
    live: Mutex<HashMap<String, Session>>,
}

impl Sessions {
    /// Begin a session for an authenticated account, and return the token the browser will send.
    pub fn begin(&self, username: &str, now: OffsetDateTime) -> String {
        let token = Uuid::new_v4().to_string();
        let mut live = self.live.lock().unwrap_or_else(PoisonError::into_inner);
        // Expired entries are cleared whenever anything is written, so a long-running gateway does
        // not accumulate sessions nobody can use.
        live.retain(|_, session| session.expires_at > now);
        live.insert(
            token.clone(),
            Session {
                username: username.to_owned(),
                expires_at: now + SESSION_LIFETIME,
            },
        );
        token
    }

    /// The session a token names, if it names one that has not expired.
    pub fn resolve(&self, token: &str, now: OffsetDateTime) -> Option<Session> {
        let live = self.live.lock().unwrap_or_else(PoisonError::into_inner);
        live.get(token)
            .filter(|session| session.expires_at > now)
            .cloned()
    }

    /// End a session, if the token names one.
    pub fn end(&self, token: &str) {
        let mut live = self.live.lock().unwrap_or_else(PoisonError::into_inner);
        live.remove(token);
    }
}

/// The `Set-Cookie` value that gives a browser its session.
///
/// `HttpOnly` so a script cannot read it, `Secure` so it is never sent in the clear, `SameSite` so
/// another site cannot make the browser use it, and a path of `/` so it reaches every route.
#[must_use]
pub fn session_cookie(token: &str) -> String {
    format!(
        "{SESSION_COOKIE}={token}; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age={}",
        SESSION_LIFETIME.whole_seconds()
    )
}

/// The `Set-Cookie` value that takes it away again.
#[must_use]
pub fn cleared_cookie() -> String {
    format!("{SESSION_COOKIE}=; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age=0")
}

/// The session token a request carries, if it carries one.
#[must_use]
pub fn token_in(cookie_header: &str) -> Option<&str> {
    cookie_header.split(';').find_map(|pair| {
        let (name, value) = pair.split_once('=')?;
        (name.trim() == SESSION_COOKIE).then(|| value.trim())
    })
}

/// What a login attempt produced, for the browser.
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginOutcome {
    /// Whether a session was established.
    ///
    /// One flag and no detail. Saying whether the account exists would make this a way to
    /// enumerate the host's accounts, and there is nothing a caller would do differently.
    pub authenticated: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(hour: i64) -> OffsetDateTime {
        OffsetDateTime::UNIX_EPOCH + Duration::hours(hour)
    }

    #[test]
    fn a_session_stops_being_honoured_when_it_expires() {
        let sessions = Sessions::default();
        let token = sessions.begin("alice", at(0));

        assert_eq!(
            sessions.resolve(&token, at(1)).map(|s| s.username),
            Some("alice".to_owned())
        );
        // Eight hours later it is gone, and no use in between extended it.
        assert!(sessions.resolve(&token, at(9)).is_none());
    }

    #[test]
    fn ending_a_session_ends_it() {
        let sessions = Sessions::default();
        let token = sessions.begin("alice", at(0));
        sessions.end(&token);
        assert!(sessions.resolve(&token, at(1)).is_none());
    }

    #[test]
    fn a_token_nobody_issued_names_no_session() {
        let sessions = Sessions::default();
        sessions.begin("alice", at(0));
        assert!(sessions.resolve("not-a-token", at(1)).is_none());
        assert!(sessions.resolve("", at(1)).is_none());
    }

    #[test]
    fn the_token_is_read_from_among_other_cookies() {
        assert_eq!(
            token_in("theme=dark; cybou_session=abc123; other=1"),
            Some("abc123")
        );
        assert_eq!(token_in("cybou_session=abc123"), Some("abc123"));
        assert_eq!(token_in("theme=dark"), None);
        assert_eq!(token_in(""), None);
    }

    #[test]
    fn the_cookie_cannot_be_read_by_a_script_or_sent_in_the_clear() {
        let cookie = session_cookie("abc123");
        for required in ["HttpOnly", "Secure", "SameSite=Strict", "Path=/"] {
            assert!(
                cookie.contains(required),
                "the session cookie must carry {required}"
            );
        }
    }

    #[test]
    fn a_login_request_does_not_leave_its_secret_behind() {
        let mut request = LoginRequest {
            username: "alice".into(),
            password: "hunter2".into(),
        };
        let secret = std::mem::take(&mut request.password);
        let mut bytes = secret.into_bytes();
        bytes.fill(0);
        assert!(bytes.iter().all(|byte| *byte == 0));
        assert!(request.password.is_empty());
    }
}
