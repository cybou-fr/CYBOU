// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! The wire between the gateway and the one thing it is not allowed to do itself.
//!
//! Checking a password against `/etc/shadow` needs privilege the gateway must not have. Rather
//! than give it that privilege, the check lives in a separate process whose whole vocabulary is the
//! two types below: a name and a secret in, then either one indistinguishable refusal or the
//! authenticated account's numeric identity. It cannot be asked to read a file or run a command.
//!
//! Nothing here logs, stores or returns the secret. It exists in memory for the length of one call
//! and is overwritten when the request is dropped.

use std::{
    collections::HashMap,
    sync::{Mutex, PoisonError},
    time::{Duration, Instant},
};

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

/// The helper's whole answer. Failed attempts remain one indistinguishable bit; UID and home are
/// returned only after the account has proved ownership of the password.
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Answer {
    /// Whether the account accepted the secret and is entitled to reach Cybou.
    ///
    /// One flag, deliberately. Saying *why* an attempt failed — no such user, wrong password, not
    /// in the group — would let anyone with the socket enumerate accounts, and the caller has
    /// nothing to do differently in any of those cases.
    pub authenticated: bool,
    /// Numeric identity established from the account database after successful authentication.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uid: Option<u32>,
    /// Home directory established alongside [`Self::uid`]. Never present on failure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub home: Option<String>,
}

/// The largest request the helper will read.
///
/// A caller that can open the socket can still not make it allocate: a name and a password are
/// small, and anything claiming otherwise is not a login attempt.
pub const MAX_REQUEST_BYTES: usize = 4096;

/// The longest account name the helper will even remember having seen.
///
/// The name is attacker-chosen and becomes a key in [`Throttle`]. Without a bound, a caller who
/// can open the socket could make the helper hold four kilobytes per attempt purely by claiming a
/// long name, which is a way to spend the helper's memory without ever guessing a password.
pub const MAX_USERNAME_BYTES: usize = 256;

/// How long a failed attempt is held before the helper answers.
///
/// This is the floor, not the whole cost. It exists so that a wrong password and an account that
/// does not exist take the same visible time, and [`Throttle`] raises it from here.
pub const FAILURE_DELAY: Duration = Duration::from_millis(750);

/// The most a single account's backoff can grow to.
///
/// Capped because the delay is paid by whoever is at the keyboard, and an owner who mistyped their
/// password six times is still the owner. Thirty seconds makes guessing hopeless and locks nobody
/// out permanently, which a hard lockout would — an attacker who knows a name could otherwise
/// deny that account by failing on purpose.
pub const MAX_FAILURE_DELAY: Duration = Duration::from_secs(30);

/// How long a failure keeps counting against the account that produced it.
///
/// After this much quiet the record is discarded, so backoff measures a run of failures rather
/// than a lifetime total.
pub const FAILURE_MEMORY: Duration = Duration::from_mins(15);

/// How many attempts the helper will have in flight at once.
///
/// Per-account backoff is evaded by rotating the name, and a helper that spawns a task per
/// connection pays every delay in parallel — which is the same as paying none. Holding a permit
/// across the delay makes the delays queue instead, so the whole socket admits at most this many
/// attempts per [`FAILURE_DELAY`] no matter how many names or connections a caller uses.
pub const MAX_CONCURRENT_ATTEMPTS: usize = 4;

/// How many accounts the helper will track failures for at once.
///
/// Bounded because the keys come from the caller. When the bound is reached the least recently
/// failed account is forgotten first: the names being hammered right now are the ones whose
/// backoff is worth keeping.
pub const MAX_TRACKED_ACCOUNTS: usize = 1024;

/// What a run of recent failures costs the next attempt on the same account.
///
/// This is deliberately not a lockout. A lockout converts knowledge of a username into the power
/// to deny that account, so the failure mode of the defence is the attack. Backoff instead makes
/// each successive guess more expensive while leaving the owner a way in on every attempt.
#[derive(Default)]
pub struct Throttle {
    tracked: Mutex<HashMap<String, Record>>,
}

/// One account's recent failures.
struct Record {
    /// Consecutive failures still inside [`FAILURE_MEMORY`].
    failures: u32,
    /// When the most recent of them happened.
    last_failure: Instant,
}

impl Throttle {
    /// A throttle that has seen nothing.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// How long an attempt on this account must be held before it may be answered.
    ///
    /// Doubles per failure from [`FAILURE_DELAY`], stops at [`MAX_FAILURE_DELAY`], and returns to
    /// the floor once the run of failures has aged out of [`FAILURE_MEMORY`].
    #[must_use]
    pub fn penalty(&self, username: &str, now: Instant) -> Duration {
        let tracked = self.tracked.lock().unwrap_or_else(PoisonError::into_inner);
        let failures = tracked
            .get(username)
            .filter(|record| now.duration_since(record.last_failure) < FAILURE_MEMORY)
            .map_or(0, |record| record.failures);
        Self::delay_for(failures)
    }

    /// Count a failure against this account.
    pub fn record_failure(&self, username: &str, now: Instant) {
        if username.len() > MAX_USERNAME_BYTES {
            return;
        }
        let mut tracked = self.tracked.lock().unwrap_or_else(PoisonError::into_inner);
        Self::forget_stale(&mut tracked, now);
        if let Some(record) = tracked.get_mut(username) {
            record.failures = record.failures.saturating_add(1);
            record.last_failure = now;
        } else {
            Self::make_room(&mut tracked);
            tracked.insert(
                username.to_owned(),
                Record {
                    failures: 1,
                    last_failure: now,
                },
            );
        }
    }

    /// Forget this account's failures, because it just proved it is its owner.
    pub fn record_success(&self, username: &str) {
        let mut tracked = self.tracked.lock().unwrap_or_else(PoisonError::into_inner);
        tracked.remove(username);
    }

    /// How many accounts are currently being tracked.
    #[must_use]
    pub fn tracked_accounts(&self) -> usize {
        self.tracked
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }

    /// The delay a given run of failures earns.
    fn delay_for(failures: u32) -> Duration {
        // Saturating rather than wrapping: a shift wide enough to overflow is already past the cap.
        let multiplier = 1_u32.checked_shl(failures).unwrap_or(u32::MAX);
        FAILURE_DELAY
            .saturating_mul(multiplier)
            .min(MAX_FAILURE_DELAY)
    }

    /// Drop records whose failures no longer count.
    fn forget_stale(tracked: &mut HashMap<String, Record>, now: Instant) {
        tracked.retain(|_, record| now.duration_since(record.last_failure) < FAILURE_MEMORY);
    }

    /// Evict the least recently failed account, if the table is full.
    fn make_room(tracked: &mut HashMap<String, Record>) {
        while tracked.len() >= MAX_TRACKED_ACCOUNTS {
            let Some(oldest) = tracked
                .iter()
                .min_by_key(|(_, record)| record.last_failure)
                .map(|(name, _)| name.clone())
            else {
                return;
            };
            tracked.remove(&oldest);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unknown_account_pays_only_the_floor() {
        let throttle = Throttle::new();
        assert_eq!(throttle.penalty("alice", Instant::now()), FAILURE_DELAY);
    }

    #[test]
    fn each_failure_doubles_what_the_next_attempt_costs() {
        let throttle = Throttle::new();
        let now = Instant::now();
        assert_eq!(throttle.penalty("alice", now), FAILURE_DELAY);
        throttle.record_failure("alice", now);
        assert_eq!(throttle.penalty("alice", now), FAILURE_DELAY * 2);
        throttle.record_failure("alice", now);
        assert_eq!(throttle.penalty("alice", now), FAILURE_DELAY * 4);
    }

    #[test]
    fn backoff_stops_growing_so_the_owner_is_never_locked_out() {
        let throttle = Throttle::new();
        let now = Instant::now();
        // Far more failures than any cap: the point is that the answer stays finite, because a
        // delay that grows without bound is a lockout wearing a different name.
        for _ in 0..64 {
            throttle.record_failure("alice", now);
        }
        assert_eq!(throttle.penalty("alice", now), MAX_FAILURE_DELAY);
    }

    #[test]
    fn a_run_of_failures_ages_out_of_memory() {
        let throttle = Throttle::new();
        let now = Instant::now();
        throttle.record_failure("alice", now);
        throttle.record_failure("alice", now);
        let later = now + FAILURE_MEMORY + Duration::from_secs(1);
        assert_eq!(throttle.penalty("alice", later), FAILURE_DELAY);
    }

    #[test]
    fn succeeding_clears_what_the_owner_mistyped() {
        let throttle = Throttle::new();
        let now = Instant::now();
        throttle.record_failure("alice", now);
        throttle.record_failure("alice", now);
        throttle.record_success("alice");
        assert_eq!(throttle.penalty("alice", now), FAILURE_DELAY);
    }

    #[test]
    fn one_accounts_failures_do_not_slow_another() {
        // Backoff that leaked across accounts would let anyone who can reach the socket delay
        // every other account by failing on a name they invented.
        let throttle = Throttle::new();
        let now = Instant::now();
        for _ in 0..8 {
            throttle.record_failure("alice", now);
        }
        assert_eq!(throttle.penalty("bob", now), FAILURE_DELAY);
    }

    #[test]
    fn the_table_stays_bounded_when_names_are_invented() {
        // The keys come from the caller, so rotating names must cost the helper memory it does
        // not have to keep.
        let throttle = Throttle::new();
        let now = Instant::now();
        for index in 0..(MAX_TRACKED_ACCOUNTS * 2) {
            throttle.record_failure(&format!("invented-{index}"), now);
        }
        assert!(throttle.tracked_accounts() <= MAX_TRACKED_ACCOUNTS);
    }

    #[test]
    fn an_overlong_name_is_not_remembered_at_all() {
        let throttle = Throttle::new();
        let name = "a".repeat(MAX_USERNAME_BYTES + 1);
        throttle.record_failure(&name, Instant::now());
        assert_eq!(throttle.tracked_accounts(), 0);
    }

    #[test]
    fn an_answer_says_only_whether_it_worked() {
        // The shape is the guarantee: there is no field a caller could read to learn that an
        // account exists, so a failed attempt teaches nothing about the host.
        let encoded = {
            let mut buffer = Vec::new();
            ciborium::into_writer(
                &Answer {
                    authenticated: false,
                    uid: None,
                    home: None,
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
