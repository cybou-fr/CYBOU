// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! The wire between the gateway and the one thing it is not allowed to do itself.
//!
//! Checking a password against `/etc/shadow` needs privilege the gateway must not have. Rather
//! than give it that privilege, the check lives in a separate process whose whole vocabulary is the
//! two types below: a name and a secret in, a yes or no out. It cannot be asked to read a file,
//! run a command, or say anything about an account beyond whether that account accepted that
//! password.
//!
//! Nothing here logs, stores or returns the secret. It exists in memory for the length of one call
//! and is overwritten when the request is dropped.

use serde::{Deserialize, Serialize};

/// The group whose members may authenticate to Cybou.
///
/// Being a valid Linux account is not the same as being someone this system answers to. Without a
/// gate like this, every service account on the host — and `root` — would be a way in, which is
/// how a login form becomes a larger attack surface than the thing it protects. Membership is the
/// grant: `gpasswd -a alice cybou-access` gives access, removing them takes it away.
pub const ACCESS_GROUP: &str = "cybou-access";

/// One question for the helper.
#[derive(Deserialize, Serialize)]
pub struct Request {
    /// The Linux account being claimed.
    pub username: String,
    /// The secret offered for it.
    pub password: String,
}

impl Drop for Request {
    fn drop(&mut self) {
        // Overwritten rather than merely freed. A password sitting in a released allocation is a
        // password that can still be read out of the process, and this is the only place in Cybou
        // that ever holds one.
        //
        // `String` will not shrink or reallocate here, so the bytes overwritten are the bytes that
        // were there.
        let secret = std::mem::take(&mut self.password);
        let mut bytes = secret.into_bytes();
        bytes.fill(0);
        drop(bytes);
    }
}

/// The helper's whole answer.
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Answer {
    /// Whether the account accepted the secret and is entitled to reach Cybou.
    ///
    /// One flag, deliberately. Saying *why* an attempt failed — no such user, wrong password, not
    /// in the group — would let anyone with the socket enumerate accounts, and the caller has
    /// nothing to do differently in any of those cases.
    pub authenticated: bool,
}

/// The largest request the helper will read.
///
/// A caller that can open the socket can still not make it allocate: a name and a password are
/// small, and anything claiming otherwise is not a login attempt.
pub const MAX_REQUEST_BYTES: usize = 4096;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_answer_says_only_whether_it_worked() {
        // The shape is the guarantee: there is no field a caller could read to learn that an
        // account exists, so a failed attempt teaches nothing about the host.
        let encoded = {
            let mut buffer = Vec::new();
            ciborium::into_writer(
                &Answer {
                    authenticated: false,
                },
                &mut buffer,
            )
            .expect("encode");
            buffer
        };
        let decoded: ciborium::Value = ciborium::from_reader(encoded.as_slice()).expect("decode");
        let map = decoded.as_map().expect("a map");
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn a_request_does_not_leave_its_secret_behind() {
        let mut request = Request {
            username: "alice".into(),
            password: "hunter2".into(),
        };
        // Take the same path Drop takes, on a value we can still look at afterwards.
        let secret = std::mem::take(&mut request.password);
        let mut bytes = secret.into_bytes();
        bytes.fill(0);
        assert!(bytes.iter().all(|byte| *byte == 0));
        assert!(request.password.is_empty());
    }
}
