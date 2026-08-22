// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

//! Journal admission rules, ported from the predecessor without a storage dependency.
//!
//! The predecessor decides what may enter the Journal in two places: `CognitiveEnvelope::isValid`
//! answers what an envelope must be on its own, and `Journal::appendWithinTransaction` answers
//! what it must be given the contributions it names. Only the second needs a database, and only
//! to *read* four facts per reference. Keeping the rules here rather than in `cybou-storage`
//! means they are protocol semantics that a writer applies, not behavior that a particular
//! storage engine happens to have — and it lets them be tested where no `SQLite` exists.
//!
//! Nothing here writes, hashes, or assigns a sequence. A caller that gets [`Admitted`] has been
//! told the contribution is admissible, not that it was accepted: durability is still the writer's
//! to establish, and acceptance is still published only after a commit returns.

use uuid::Uuid;

use crate::canonical::CanonicalEnvelope;

/// Legacy envelope schema. Readable, never writable.
pub const ENVELOPE_SCHEMA_LEGACY: u16 = 1;
/// Ordinary envelope schema.
pub const ENVELOPE_SCHEMA_CURRENT: u16 = 2;
/// Adds the payload protection descriptor.
pub const ENVELOPE_SCHEMA_PROTECTED: u16 = 3;
/// Adds the sensitivity axis.
pub const ENVELOPE_SCHEMA_CLASSIFIED: u16 = 4;

/// Frozen numeric contribution kinds. The numeric value of every kind is part of every hash
/// already written, so kinds are appended and never renumbered.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum Kind {
    /// Something observed outside the Journal.
    Observation = 1,
    /// A revision of an existing belief.
    BeliefRevision = 2,
    /// A proposed explanation.
    Hypothesis = 3,
    /// A recalled prior contribution.
    MemoryRecall = 4,
    /// A signalled need.
    NeedSignal = 5,
    /// A candidate for attention.
    AttentionCandidate = 6,
    /// A prediction with a later outcome.
    Prediction = 7,
    /// A proposed plan.
    PlanProposal = 8,
    /// An objection to a proposal.
    Objection = 9,
    /// A decision taken.
    Decision = 10,
    /// An intention formed.
    Intention = 11,
    /// The terminal result of a causal contribution.
    Outcome = 12,
    /// An assessment of the system by itself.
    SelfAssessment = 13,
    /// Something learned.
    Learning = 14,
    /// Durable intent to erase, before anything irreversible happens.
    ErasureRequested = 15,
    /// Durable record that an erasure was applied.
    ErasureApplied = 16,
    /// A durable record that context was disclosed to a named consumer.
    ContextDisclosed = 17,
}

impl Kind {
    /// The kind for a frozen numeric value, or `None` for an unknown one.
    ///
    /// Unknown is not "probably an Observation". A writer that guessed would admit a row whose
    /// kind no rule in this module can reason about.
    #[must_use]
    pub fn from_u16(value: u16) -> Option<Self> {
        Some(match value {
            1 => Self::Observation,
            2 => Self::BeliefRevision,
            3 => Self::Hypothesis,
            4 => Self::MemoryRecall,
            5 => Self::NeedSignal,
            6 => Self::AttentionCandidate,
            7 => Self::Prediction,
            8 => Self::PlanProposal,
            9 => Self::Objection,
            10 => Self::Decision,
            11 => Self::Intention,
            12 => Self::Outcome,
            13 => Self::SelfAssessment,
            14 => Self::Learning,
            15 => Self::ErasureRequested,
            16 => Self::ErasureApplied,
            17 => Self::ContextDisclosed,
            _ => return None,
        })
    }

    /// Whether this kind records something that happened outside the Journal and therefore has no
    /// prior contribution to cite.
    #[must_use]
    pub fn is_root(self) -> bool {
        matches!(self, Self::Observation | Self::ContextDisclosed)
    }

    /// Whether this kind is a storage operation rather than a thought.
    ///
    /// ADR-0028 forbids `Event1.Submit` from accepting these: destroying biography must never be
    /// reachable by the same call that records a thought about it.
    #[must_use]
    pub fn is_erasure(self) -> bool {
        matches!(self, Self::ErasureRequested | Self::ErasureApplied)
    }
}

/// Where a contribution may exist. Ordered from most to least restrictive.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Privacy {
    /// This device only.
    Local = 0,
    /// This node.
    Node = 1,
    /// The household.
    Household = 2,
    /// Unrestricted.
    Public = 3,
}

impl Privacy {
    /// The privacy class for a frozen numeric value, or `None` for an unknown one.
    #[must_use]
    pub fn from_u8(value: u8) -> Option<Self> {
        Some(match value {
            0 => Self::Local,
            1 => Self::Node,
            2 => Self::Household,
            3 => Self::Public,
            _ => return None,
        })
    }

    /// The more restrictive of two classes.
    #[must_use]
    pub fn most_restrictive(self, other: Self) -> Self {
        if (self as u8) < (other as u8) {
            self
        } else {
            other
        }
    }
}

/// Who may be shown a contribution. Ordered from least to most sensitive.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Sensitivity {
    /// No particular exposure concern.
    Ordinary = 0,
    /// About the person, and theirs to release.
    Personal = 1,
    /// Harmful if disclosed, even to a trusted consumer.
    Sensitive = 2,
    /// Disclosure is the harm.
    Secret = 3,
    /// Confers access; never a deliberate training target.
    Credential = 4,
}

impl Sensitivity {
    /// The sensitivity class for a frozen numeric value, or `None` for an unknown one.
    #[must_use]
    pub fn from_u8(value: u8) -> Option<Self> {
        Some(match value {
            0 => Self::Ordinary,
            1 => Self::Personal,
            2 => Self::Sensitive,
            3 => Self::Secret,
            4 => Self::Credential,
            _ => return None,
        })
    }

    /// The more sensitive of two classifications.
    #[must_use]
    pub fn most_sensitive(self, other: Self) -> Self {
        if (self as u8) > (other as u8) {
            self
        } else {
            other
        }
    }

    /// Whether a classification may ever be used as a supervised training target.
    ///
    /// ADR-0033's A9 as a closed rule over the type rather than a policy flag, because a flag can
    /// be cleared by whoever is doing the training.
    #[must_use]
    pub fn may_be_training_target(self) -> bool {
        !matches!(self, Self::Secret | Self::Credential)
    }
}

/// The four facts a writer must read back about a contribution this one names.
///
/// Deliberately not the referenced envelope. Admission needs exactly these, and passing the whole
/// envelope would invite a future rule to reach for its payload — which for a sealed or erased
/// reference does not exist.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReferenceFacts {
    /// Privacy class of the referenced contribution.
    pub privacy: Privacy,
    /// Its expiry as UTC epoch milliseconds, or zero for unbounded.
    pub retain_until_ms: u64,
    /// Its sensitivity class.
    pub sensitivity: Sensitivity,
}

/// What the writer read back for everything this contribution names.
///
/// The caller resolves the causal and evidence identities and reports what it found; a reference
/// it could not resolve is reported as absent rather than skipped, because a contribution that
/// silently lost a reference would be admitted as if it never had one.
#[derive(Clone, Debug, Default)]
pub struct Resolved {
    /// Facts for the causal predecessor, or `None` when the envelope declares no causation.
    pub causation: Option<Option<ReferenceFacts>>,
    /// Facts for each evidence identity in declaration order; `None` for one that does not exist.
    pub evidence: Vec<Option<ReferenceFacts>>,
    /// Whether the causal contribution already has a terminal `Outcome`.
    pub causation_has_outcome: bool,
    /// Whether the message identity is already present in the Journal.
    pub message_id_exists: bool,
}

/// Why a contribution may not enter the Journal.
///
/// Every variant is a refusal, never a correction. The predecessor refuses rather than clamping a
/// weakened privacy or an over-long retention on purpose: the envelope's declaration is the
/// contract, and quietly correcting it would leave the caller believing something the Journal does
/// not.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum Rejection {
    /// The envelope declares a schema version this build does not know.
    #[error("unknown envelope schema {0}")]
    UnknownSchema(u16),
    /// Legacy schema 1 envelopes are readable but may not be written.
    #[error("new contributions must use envelope schema v2, v3 or v4")]
    LegacySchema,
    /// The numeric kind is not one this build can reason about.
    #[error("unknown contribution kind {0}")]
    UnknownKind(u16),
    /// The numeric privacy class is not one this build can reason about.
    #[error("unknown privacy class {0}")]
    UnknownPrivacy(u8),
    /// The numeric sensitivity class is not one this build can reason about.
    #[error("unknown sensitivity class {0}")]
    UnknownSensitivity(u8),
    /// A sealed payload was declared on a schema that carries no protection descriptor.
    #[error("a sealed payload requires envelope schema v3 or v4")]
    SealedWithoutProtectedSchema,
    /// A sealed payload named no key domain, so nothing could ever interpret its descriptor.
    #[error("a sealed payload must name its key domain")]
    SealedWithoutKeyDomain,
    /// The contribution or its correlation has no stable identity.
    #[error("a contribution must carry a message and correlation identity")]
    MissingIdentity,
    /// No process owner claimed authorship.
    #[error("a contribution must name its origin organ")]
    MissingOriginOrgan,
    /// Confidence was not a finite value within the closed unit interval.
    #[error("confidence must be a finite value between 0 and 1")]
    ConfidenceOutOfRange,
    /// A contribution cannot be its own cause.
    #[error("a contribution cannot cause itself")]
    SelfCausation,
    /// Evidence was nil, repeated, or restated the contribution's own or causal identity.
    #[error("evidence must be distinct, non-nil, and not the contribution or its cause")]
    MalformedEvidence,
    /// A root kind cited a cause or evidence it cannot have.
    #[error("a root contribution cannot cite a cause or evidence")]
    RootWithReferences,
    /// A derived kind cited neither a cause nor evidence.
    #[error("a derived contribution must cite a cause or evidence")]
    DerivedWithoutReferences,
    /// The message identity is already present.
    #[error("messageId already exists")]
    DuplicateMessageId,
    /// The declared cause is not in the Journal.
    #[error("causal contribution does not exist")]
    MissingCausation,
    /// A declared evidence identity is not in the Journal.
    #[error("evidence contribution does not exist")]
    MissingEvidence,
    /// The contribution declared a weaker privacy class than something it rests on.
    #[error("contribution privacy is weaker than its references")]
    PrivacyWeakerThanReferences,
    /// The contribution declared a later expiry than something it rests on.
    #[error("contribution outlives the retention of its references")]
    OutlivesReferences,
    /// The contribution declared a weaker sensitivity class than something it rests on.
    #[error("contribution is less sensitive than its references")]
    LessSensitiveThanReferences,
    /// The causal contribution already reached a terminal outcome.
    #[error("the causal contribution already has a terminal Outcome")]
    CauseAlreadyConcluded,
}

/// The admissible facts a writer needs after the rules have passed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Admitted {
    /// Decoded contribution kind.
    pub kind: Kind,
    /// Decoded privacy class.
    pub privacy: Privacy,
    /// Decoded sensitivity class.
    pub sensitivity: Sensitivity,
}

/// Structural admissibility, decided without reading the Journal.
///
/// This is the predecessor's `isValid` plus the write-time schema restriction. It is separated so
/// a caller can refuse a malformed envelope before opening a transaction or resolving a single
/// reference — a refusal that costs nothing should not cost a database round trip.
///
/// # Errors
///
/// Returns the first rule the envelope fails, in the predecessor's evaluation order.
pub fn check_structure(envelope: &CanonicalEnvelope) -> Result<Admitted, Rejection> {
    match envelope.schema_version {
        ENVELOPE_SCHEMA_LEGACY => return Err(Rejection::LegacySchema),
        ENVELOPE_SCHEMA_CURRENT | ENVELOPE_SCHEMA_PROTECTED | ENVELOPE_SCHEMA_CLASSIFIED => {}
        other => return Err(Rejection::UnknownSchema(other)),
    }

    let kind = Kind::from_u16(envelope.kind).ok_or(Rejection::UnknownKind(envelope.kind))?;
    let privacy =
        Privacy::from_u8(envelope.privacy).ok_or(Rejection::UnknownPrivacy(envelope.privacy))?;
    let sensitivity = Sensitivity::from_u8(envelope.sensitivity)
        .ok_or(Rejection::UnknownSensitivity(envelope.sensitivity))?;

    if envelope.sealed {
        if envelope.schema_version != ENVELOPE_SCHEMA_PROTECTED
            && envelope.schema_version != ENVELOPE_SCHEMA_CLASSIFIED
        {
            return Err(Rejection::SealedWithoutProtectedSchema);
        }
        if envelope.key_domain_id.is_nil() {
            return Err(Rejection::SealedWithoutKeyDomain);
        }
    }

    if envelope.message_id.is_nil() || envelope.correlation_id.is_nil() {
        return Err(Rejection::MissingIdentity);
    }
    if envelope.origin_organ.trim().is_empty() {
        return Err(Rejection::MissingOriginOrgan);
    }
    if !envelope.confidence.is_finite() || !(0.0..=1.0).contains(&envelope.confidence) {
        return Err(Rejection::ConfidenceOutOfRange);
    }
    if envelope.causation_id == envelope.message_id {
        return Err(Rejection::SelfCausation);
    }

    let mut seen: Vec<Uuid> = Vec::with_capacity(envelope.evidence.len());
    for id in &envelope.evidence {
        if id.is_nil()
            || *id == envelope.message_id
            || *id == envelope.causation_id
            || seen.contains(id)
        {
            return Err(Rejection::MalformedEvidence);
        }
        seen.push(*id);
    }

    let cites_references = !envelope.causation_id.is_nil() || !envelope.evidence.is_empty();
    if kind.is_root() {
        if cites_references {
            return Err(Rejection::RootWithReferences);
        }
    } else if !cites_references {
        return Err(Rejection::DerivedWithoutReferences);
    }

    Ok(Admitted {
        kind,
        privacy,
        sensitivity,
    })
}

/// The earliest expiry among a contribution and everything it was derived from.
///
/// Zero is unbounded rather than immediate, so it never wins the comparison. That asymmetry
/// matters: treating "no lifetime recorded" as "expires now" would make every contribution derived
/// from an older record expire the moment it was written.
#[must_use]
pub fn derived_retain_until_ms(declared: u64, references: &[u64]) -> u64 {
    let mut earliest = declared;
    for &reference in references {
        if reference == 0 {
            continue;
        }
        if earliest == 0 || reference < earliest {
            earliest = reference;
        }
    }
    earliest
}

/// Full admissibility, given what the writer read back for every named reference.
///
/// # Errors
///
/// Returns the first rule the contribution fails. Structural rules are evaluated first, so a
/// malformed envelope is refused for being malformed rather than for a reference it should never
/// have been allowed to declare.
pub fn check_admission(
    envelope: &CanonicalEnvelope,
    resolved: &Resolved,
) -> Result<Admitted, Rejection> {
    let admitted = check_structure(envelope)?;

    if resolved.message_id_exists {
        return Err(Rejection::DuplicateMessageId);
    }

    let mut references: Vec<ReferenceFacts> = Vec::new();
    if !envelope.causation_id.is_nil() {
        let facts = resolved
            .causation
            .ok_or(Rejection::MissingCausation)?
            .ok_or(Rejection::MissingCausation)?;
        references.push(facts);
    }
    if resolved.evidence.len() != envelope.evidence.len() {
        return Err(Rejection::MissingEvidence);
    }
    for facts in &resolved.evidence {
        references.push(facts.ok_or(Rejection::MissingEvidence)?);
    }

    let derived_privacy = references.iter().fold(admitted.privacy, |acc, facts| {
        acc.most_restrictive(facts.privacy)
    });
    if derived_privacy != admitted.privacy {
        return Err(Rejection::PrivacyWeakerThanReferences);
    }

    // Erasure records are exempt, and must be: they name the target as their cause, so the rule
    // would have every erasure inherit the retention of the thing it erased and expire with it. A
    // journal that forgot that it had forgotten something could not answer the one question an
    // erasure protocol exists to answer.
    if !admitted.kind.is_erasure() {
        let expiries: Vec<u64> = references.iter().map(|f| f.retain_until_ms).collect();
        if derived_retain_until_ms(envelope.retain_until_ms, &expiries) != envelope.retain_until_ms
        {
            return Err(Rejection::OutlivesReferences);
        }
    }

    // Checked only for schema 4, because earlier envelopes do not carry the field and would all
    // declare Ordinary against evidence read back as unclassified.
    if envelope.schema_version == ENVELOPE_SCHEMA_CLASSIFIED {
        let derived = references.iter().fold(admitted.sensitivity, |acc, facts| {
            acc.most_sensitive(facts.sensitivity)
        });
        if derived != admitted.sensitivity {
            return Err(Rejection::LessSensitiveThanReferences);
        }
    }

    if admitted.kind == Kind::Outcome && resolved.causation_has_outcome {
        return Err(Rejection::CauseAlreadyConcluded);
    }

    Ok(admitted)
}

#[cfg(test)]
mod tests {
    use super::{
        Admitted, BackupState, ENVELOPE_SCHEMA_CLASSIFIED, ENVELOPE_SCHEMA_PROTECTED, Kind,
        Privacy, ReferenceFacts, Rejection, Resolved, Sensitivity, check_admission,
        check_structure, derived_retain_until_ms,
    };
    use crate::canonical::CanonicalEnvelope;
    use uuid::Uuid;

    #[test]
    fn silence_about_backups_is_not_a_declaration_that_there_are_none() {
        // The whole point. A deployment that has said nothing cannot be reported as having
        // forgotten completely, because nobody established that no copy exists.
        let state = BackupState::from_rotation(None, 1_000, 2_000);
        assert_eq!(state, BackupState::Unknown);
        assert!(!state.reached_every_copy());
    }

    #[test]
    fn a_deployment_that_keeps_no_backups_says_so() {
        let state = BackupState::from_rotation(Some(0), 1_000, 1_000);
        assert_eq!(state, BackupState::NoneDeclared);
        assert!(state.reached_every_copy());
    }

    #[test]
    fn an_erasure_is_incomplete_while_an_older_copy_could_still_exist() {
        // Seven days of rotation, erased now: for the next seven days a backup taken before the
        // erasure may still hold the key that was destroyed.
        let erased_at = 1_000_000_000_000;
        let state = BackupState::from_rotation(Some(7), erased_at, erased_at);
        match state {
            BackupState::PendingRotation { complete_after_ms } => {
                assert_eq!(complete_after_ms, erased_at + 7 * 86_400_000);
            }
            other => panic!("expected a pending rotation, got {other:?}"),
        }
        assert!(!state.reached_every_copy());
    }

    #[test]
    fn it_becomes_complete_only_once_the_rotation_has_actually_passed() {
        let erased_at = 1_000_000_000_000;
        let window = 7 * 86_400_000;
        assert!(
            !BackupState::from_rotation(Some(7), erased_at, erased_at + window - 1)
                .reached_every_copy()
        );
        assert_eq!(
            BackupState::from_rotation(Some(7), erased_at, erased_at + window),
            BackupState::Complete
        );
    }

    #[test]
    fn a_rotation_long_enough_to_overflow_does_not_wrap_into_completeness() {
        // A deployment declaring an absurd rotation must not come out the other side reported as
        // having forgotten everything.
        let state = BackupState::from_rotation(Some(u32::MAX), i64::MAX - 1, i64::MAX);
        assert!(!state.reached_every_copy(), "{state:?}");
    }

    #[test]
    fn every_state_has_a_frozen_spelling() {
        for state in [
            BackupState::NoneDeclared,
            BackupState::PendingRotation {
                complete_after_ms: 0,
            },
            BackupState::Complete,
            BackupState::Unknown,
        ] {
            assert!(!state.name().is_empty());
        }
    }

    fn uuid(byte: u8) -> Uuid {
        Uuid::from_bytes([byte; 16])
    }

    fn observation() -> CanonicalEnvelope {
        CanonicalEnvelope {
            schema_version: 2,
            message_id: uuid(1),
            correlation_id: uuid(2),
            causation_id: Uuid::nil(),
            origin_organ: "perceptiond".into(),
            origin_node: String::new(),
            kind: Kind::Observation as u16,
            wall_time_ms: 1_760_000_000_000,
            monotonic_time: 42,
            logical_clock: 7,
            confidence: 1.0,
            evidence: Vec::new(),
            payload: vec![0xa0],
            privacy: Privacy::Local as u8,
            capability_scope: String::new(),
            sealed: false,
            key_domain_id: Uuid::nil(),
            key_epoch: 0,
            retention_class: 2,
            retention_policy_version: 1,
            retain_until_ms: 0,
            sensitivity: Sensitivity::Ordinary as u8,
        }
    }

    fn derived_from(cause: Uuid) -> CanonicalEnvelope {
        let mut envelope = observation();
        envelope.message_id = uuid(9);
        envelope.kind = Kind::Learning as u16;
        envelope.causation_id = cause;
        envelope
    }

    fn present(facts: ReferenceFacts) -> Resolved {
        Resolved {
            causation: Some(Some(facts)),
            ..Resolved::default()
        }
    }

    fn ordinary_local() -> ReferenceFacts {
        ReferenceFacts {
            privacy: Privacy::Local,
            retain_until_ms: 0,
            sensitivity: Sensitivity::Ordinary,
        }
    }

    #[test]
    fn a_well_formed_root_observation_is_admissible() {
        assert_eq!(
            check_structure(&observation()),
            Ok(Admitted {
                kind: Kind::Observation,
                privacy: Privacy::Local,
                sensitivity: Sensitivity::Ordinary,
            })
        );
    }

    #[test]
    fn legacy_envelopes_are_readable_but_never_writable() {
        let mut envelope = observation();
        envelope.schema_version = 1;
        assert_eq!(check_structure(&envelope), Err(Rejection::LegacySchema));
    }

    #[test]
    fn an_unknown_kind_is_refused_rather_than_guessed() {
        let mut envelope = observation();
        envelope.kind = 9999;
        assert_eq!(
            check_structure(&envelope),
            Err(Rejection::UnknownKind(9999))
        );
    }

    #[test]
    fn a_root_kind_cannot_cite_a_cause() {
        let mut envelope = observation();
        envelope.causation_id = uuid(3);
        assert_eq!(
            check_structure(&envelope),
            Err(Rejection::RootWithReferences)
        );
    }

    #[test]
    fn a_derived_kind_must_cite_something() {
        let mut envelope = observation();
        envelope.kind = Kind::Learning as u16;
        assert_eq!(
            check_structure(&envelope),
            Err(Rejection::DerivedWithoutReferences)
        );
    }

    #[test]
    fn evidence_cannot_repeat_or_restate_the_contribution() {
        let mut envelope = derived_from(uuid(3));
        envelope.evidence = vec![uuid(4), uuid(4)];
        assert_eq!(
            check_structure(&envelope),
            Err(Rejection::MalformedEvidence)
        );

        envelope.evidence = vec![envelope.message_id];
        assert_eq!(
            check_structure(&envelope),
            Err(Rejection::MalformedEvidence)
        );

        envelope.evidence = vec![envelope.causation_id];
        assert_eq!(
            check_structure(&envelope),
            Err(Rejection::MalformedEvidence)
        );
    }

    #[test]
    fn confidence_outside_the_unit_interval_is_refused() {
        let mut envelope = observation();
        envelope.confidence = f64::NAN;
        assert_eq!(
            check_structure(&envelope),
            Err(Rejection::ConfidenceOutOfRange)
        );
        envelope.confidence = 1.000_001;
        assert_eq!(
            check_structure(&envelope),
            Err(Rejection::ConfidenceOutOfRange)
        );
    }

    #[test]
    fn a_sealed_payload_needs_a_protected_schema_and_a_key_domain() {
        let mut envelope = observation();
        envelope.sealed = true;
        envelope.key_domain_id = uuid(5);
        assert_eq!(
            check_structure(&envelope),
            Err(Rejection::SealedWithoutProtectedSchema)
        );

        envelope.schema_version = ENVELOPE_SCHEMA_PROTECTED;
        envelope.key_domain_id = Uuid::nil();
        assert_eq!(
            check_structure(&envelope),
            Err(Rejection::SealedWithoutKeyDomain)
        );
    }

    #[test]
    fn a_missing_reference_is_refused_rather_than_ignored() {
        let envelope = derived_from(uuid(3));
        let resolved = Resolved {
            causation: Some(None),
            ..Resolved::default()
        };
        assert_eq!(
            check_admission(&envelope, &resolved),
            Err(Rejection::MissingCausation)
        );

        let mut with_evidence = envelope.clone();
        with_evidence.evidence = vec![uuid(4)];
        let resolved = Resolved {
            causation: Some(Some(ordinary_local())),
            evidence: vec![None],
            ..Resolved::default()
        };
        assert_eq!(
            check_admission(&with_evidence, &resolved),
            Err(Rejection::MissingEvidence)
        );
    }

    #[test]
    fn a_short_evidence_resolution_cannot_pass_as_a_complete_one() {
        let mut envelope = derived_from(uuid(3));
        envelope.evidence = vec![uuid(4), uuid(5)];
        let resolved = Resolved {
            causation: Some(Some(ordinary_local())),
            evidence: vec![Some(ordinary_local())],
            ..Resolved::default()
        };
        assert_eq!(
            check_admission(&envelope, &resolved),
            Err(Rejection::MissingEvidence)
        );
    }

    #[test]
    fn privacy_may_not_be_weakened_by_a_derived_contribution() {
        let mut envelope = derived_from(uuid(3));
        envelope.privacy = Privacy::Public as u8;
        let resolved = present(ordinary_local());
        assert_eq!(
            check_admission(&envelope, &resolved),
            Err(Rejection::PrivacyWeakerThanReferences)
        );
    }

    #[test]
    fn a_conclusion_may_not_outlive_its_evidence() {
        let mut envelope = derived_from(uuid(3));
        envelope.retain_until_ms = 2_000;
        let resolved = present(ReferenceFacts {
            retain_until_ms: 1_000,
            ..ordinary_local()
        });
        assert_eq!(
            check_admission(&envelope, &resolved),
            Err(Rejection::OutlivesReferences)
        );
    }

    #[test]
    fn an_erasure_record_does_not_expire_with_what_it_erased() {
        let mut envelope = derived_from(uuid(3));
        envelope.kind = Kind::ErasureApplied as u16;
        envelope.retain_until_ms = 0;
        let resolved = present(ReferenceFacts {
            retain_until_ms: 1_000,
            ..ordinary_local()
        });
        assert!(check_admission(&envelope, &resolved).is_ok());
    }

    #[test]
    fn unbounded_retention_never_wins_the_earliest_comparison() {
        assert_eq!(derived_retain_until_ms(0, &[0, 0]), 0);
        assert_eq!(derived_retain_until_ms(0, &[5_000]), 5_000);
        assert_eq!(derived_retain_until_ms(9_000, &[0]), 9_000);
        assert_eq!(derived_retain_until_ms(9_000, &[5_000]), 5_000);
    }

    #[test]
    fn sensitivity_is_checked_only_for_the_schema_that_carries_it() {
        let mut envelope = derived_from(uuid(3));
        let sensitive_reference = present(ReferenceFacts {
            sensitivity: Sensitivity::Secret,
            ..ordinary_local()
        });

        // Schema 2 carries no sensitivity field; every row would read back as Ordinary against a
        // reference the predecessor stored as Personal, so the rule does not apply.
        assert!(check_admission(&envelope, &sensitive_reference).is_ok());

        envelope.schema_version = ENVELOPE_SCHEMA_CLASSIFIED;
        assert_eq!(
            check_admission(&envelope, &sensitive_reference),
            Err(Rejection::LessSensitiveThanReferences)
        );

        envelope.sensitivity = Sensitivity::Secret as u8;
        assert!(check_admission(&envelope, &sensitive_reference).is_ok());
    }

    #[test]
    fn a_cause_reaches_a_terminal_outcome_once() {
        let mut envelope = derived_from(uuid(3));
        envelope.kind = Kind::Outcome as u16;
        let resolved = Resolved {
            causation: Some(Some(ordinary_local())),
            causation_has_outcome: true,
            ..Resolved::default()
        };
        assert_eq!(
            check_admission(&envelope, &resolved),
            Err(Rejection::CauseAlreadyConcluded)
        );

        let resolved = present(ordinary_local());
        assert!(check_admission(&envelope, &resolved).is_ok());
    }

    #[test]
    fn a_repeated_message_identity_is_refused() {
        let resolved = Resolved {
            message_id_exists: true,
            ..Resolved::default()
        };
        assert_eq!(
            check_admission(&observation(), &resolved),
            Err(Rejection::DuplicateMessageId)
        );
    }

    #[test]
    fn a_credential_is_never_a_training_target() {
        assert!(!Sensitivity::Credential.may_be_training_target());
        assert!(!Sensitivity::Secret.may_be_training_target());
        assert!(Sensitivity::Sensitive.may_be_training_target());
    }
}

/// How far an erasure reached, on the axis erasure cannot reach by itself.
///
/// Destroying a key and redacting a payload reaches the live database and every future backup. It
/// does not reach a backup already taken: a copy made before the erasure, together with a recovery
/// root that still unwraps the key captured in it, defeats the erasure for that record. Backup
/// rotation is therefore part of the retention guarantee rather than an operational detail, and a
/// deployment keeping backups indefinitely has weakened erasure to the age of its oldest one
/// (ADR-0028, E11 and E12).
///
/// This is a state and not a boolean because "erased" is too binary to be honest. A person asking
/// whether something was forgotten must not be told "yes, completely" while a copy that can still
/// be recovered is in rotation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackupState {
    /// Nothing outside the live database holds a copy, because this deployment declares no backups.
    ///
    /// A declaration, not an observation: the system cannot see what somebody copied elsewhere. It
    /// says what the deployment stated about itself, which is the most it can honestly say.
    NoneDeclared,
    /// A backup taken before this erasure may still be in rotation, and may still hold the key.
    ///
    /// `complete_after_ms` is the instant at which the declared rotation will have discarded every
    /// copy that predates the erasure. Until then the erasure is real for the live database and
    /// incomplete for the deployment as a whole.
    PendingRotation {
        /// Unix milliseconds after which no pre-erasure copy remains inside the declared rotation.
        complete_after_ms: i64,
    },
    /// Every copy that predated the erasure has left the declared rotation.
    Complete,
    /// This deployment has not said whether it keeps backups, so nothing can be claimed.
    ///
    /// The default, and deliberately the unreassuring one. Silence about backups is not evidence
    /// that none exist, and an erasure that reported completeness on the strength of nobody having
    /// mentioned a copy would be stating what nobody established.
    Unknown,
}

impl BackupState {
    /// The state a declared rotation implies for an erasure at this instant.
    ///
    /// `rotation_days` is what the deployment declared it keeps. `None` means it declared nothing,
    /// which is not the same as declaring none.
    #[must_use]
    pub const fn from_rotation(rotation_days: Option<u32>, erased_at_ms: i64, now_ms: i64) -> Self {
        let Some(days) = rotation_days else {
            return Self::Unknown;
        };
        if days == 0 {
            return Self::NoneDeclared;
        }
        let window_ms = (days as i64).saturating_mul(86_400_000);
        // A sum that did not fit is a calculation that could not be made, not a window that has
        // passed. Saturating here and then comparing would turn "we cannot say" into "it is done",
        // which is the one direction this type exists to prevent.
        let Some(complete_after_ms) = erased_at_ms.checked_add(window_ms) else {
            return Self::PendingRotation {
                complete_after_ms: i64::MAX,
            };
        };
        if now_ms >= complete_after_ms {
            Self::Complete
        } else {
            Self::PendingRotation { complete_after_ms }
        }
    }

    /// The frozen spelling this state is recorded and reported under.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::NoneDeclared => "no-backups-declared",
            Self::PendingRotation { .. } => "pending-rotation",
            Self::Complete => "complete",
            Self::Unknown => "unknown",
        }
    }

    /// Whether the erasure can be said to have reached every copy this deployment knows of.
    ///
    /// Only one state answers yes on its own, and one more answers yes because the rotation has
    /// since passed. `Unknown` never does.
    #[must_use]
    pub const fn reached_every_copy(self) -> bool {
        matches!(self, Self::NoneDeclared | Self::Complete)
    }
}

/// Why something was erased.
///
/// A closed set, never free text. An erasure record is permanent, so a free-text reason would let
/// the thing being forgotten be restated in the one place that can never be erased: "remove the
/// record of diagnosis X" defeats the erasure it requests. A typed reason says why without saying
/// what.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum ErasureReason {
    /// A person asked for it.
    UserRequested = 1,
    /// The retention this contribution was accepted under has elapsed.
    RetentionExpired = 2,
    /// Consent that made the recording legitimate was taken back.
    ConsentWithdrawn = 3,
    /// A policy the contribution was accepted under has changed.
    PolicyChange = 4,
    /// The source that produced it is no longer trusted to have produced it.
    SourceRevoked = 5,
}

impl ErasureReason {
    /// The reason for a frozen numeric value, or `None` for one this build does not know.
    ///
    /// Unknown is not "probably a user request". An erasure whose reason cannot be read is one
    /// nobody can later account for, and accounting for it is the whole purpose of recording it.
    #[must_use]
    pub const fn from_u16(value: u16) -> Option<Self> {
        Some(match value {
            1 => Self::UserRequested,
            2 => Self::RetentionExpired,
            3 => Self::ConsentWithdrawn,
            4 => Self::PolicyChange,
            5 => Self::SourceRevoked,
            _ => return None,
        })
    }

    /// The reason for the spelling a caller uses on the wire.
    #[must_use]
    pub fn from_name(value: &str) -> Option<Self> {
        Some(match value {
            "user-requested" => Self::UserRequested,
            "retention-expired" => Self::RetentionExpired,
            "consent-withdrawn" => Self::ConsentWithdrawn,
            "policy-change" => Self::PolicyChange,
            "source-revoked" => Self::SourceRevoked,
            _ => return None,
        })
    }

    /// How this reason is spelled on the wire.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::UserRequested => "user-requested",
            Self::RetentionExpired => "retention-expired",
            Self::ConsentWithdrawn => "consent-withdrawn",
            Self::PolicyChange => "policy-change",
            Self::SourceRevoked => "source-revoked",
        }
    }
}

#[cfg(test)]
mod erasure_reason_tests {
    use super::ErasureReason;

    #[test]
    fn a_reason_this_build_does_not_know_is_not_guessed_at() {
        assert_eq!(ErasureReason::from_u16(0), None);
        assert_eq!(ErasureReason::from_u16(6), None);
        assert_eq!(ErasureReason::from_name("because I said so"), None);
        assert_eq!(ErasureReason::from_name(""), None);
    }

    #[test]
    fn every_reason_round_trips_through_its_wire_spelling() {
        for reason in [
            ErasureReason::UserRequested,
            ErasureReason::RetentionExpired,
            ErasureReason::ConsentWithdrawn,
            ErasureReason::PolicyChange,
            ErasureReason::SourceRevoked,
        ] {
            assert_eq!(ErasureReason::from_name(reason.name()), Some(reason));
            assert_eq!(ErasureReason::from_u16(reason as u16), Some(reason));
        }
    }
}
