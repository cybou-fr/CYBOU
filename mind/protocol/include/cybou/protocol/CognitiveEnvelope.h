// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include <QByteArray>
#include <QDateTime>
#include <QList>
#include <QMetaType>
#include <QString>
#include <QUuid>

namespace cybou {

inline constexpr quint16 kLegacyEnvelopeSchemaVersion = 1;
inline constexpr quint16 kCurrentEnvelopeSchemaVersion = 2;

/// Schema 3 adds the protection descriptor: whether a payload is sealed, and under which opaque key
/// domain and epoch.
///
/// Not the default. An envelope declares schema 3 only when it carries protection, so an ordinary
/// contribution keeps the schema-2 canonical form and every hash already written stays exactly as it
/// was. ADR-0028 fixes that the canonical field set is selected by schema version rather than
/// extended in place, and this is the first version to exercise that rule.
inline constexpr quint16 kProtectedEnvelopeSchemaVersion = 3;

enum class ContributionKind : quint16 {
    Observation = 1,
    BeliefRevision,
    Hypothesis,
    MemoryRecall,
    NeedSignal,
    AttentionCandidate,
    Prediction,
    PlanProposal,
    Objection,
    Decision,
    Intention,
    Outcome,
    SelfAssessment,
    Learning,

    /// The two halves of an erasure, from ADR-0028.
    ///
    /// They are separate kinds because they are separate durable facts. `ErasureRequested` records
    /// an intent before anything irreversible happens; `ErasureApplied` records that it did. A
    /// request with no matching application is the only state a crash can leave behind, and it is
    /// resumable precisely because it claims nothing about what was destroyed.
    ///
    /// Appended rather than inserted: the numeric value of every existing kind is part of every
    /// hash already written.
    ErasureRequested,
    ErasureApplied,

    /// A durable record that context was supplied to a named consumer, from ADR-0030.
    ///
    /// Its own kind because it is its own fact: what Mind disclosed, to whom, and with what
    /// provenance. It commits to a digest of what was released rather than copying it, so the
    /// Journal does not become a second store of the material the disclosure was already the risk
    /// of.
    ///
    /// Appended rather than inserted, like the erasure kinds: the numeric value of every existing
    /// kind is part of every hash already written.
    ContextDisclosed,
};

/// How long a contribution may exist at all.
///
/// A separate axis from `PrivacyClass`, and deliberately so. Privacy asks *who may see this*;
/// retention asks *how long may this exist*. The answers do not correlate - an identity fact may be
/// highly private and needed for years, while public telemetry is worthless after ten minutes - and
/// forcing them onto one ordering would make every future classification argument a fight about the
/// wrong axis.
enum class RetentionClass : quint8 {
    Ephemeral = 0,
    Short,
    Standard,
    Long,
    Permanent,
};

enum class PrivacyClass : quint8 {
    Local = 0,
    Node,
    Household,
    Public,
};

constexpr PrivacyClass mostRestrictive(PrivacyClass a, PrivacyClass b) noexcept
{
    return static_cast<quint8>(a) < static_cast<quint8>(b) ? a : b;
}

constexpr bool isRootKind(ContributionKind kind) noexcept
{
    // `ContextDisclosed` is rooted for the same reason an observation is: it records something
    // that happened outside the Journal, and there is no prior contribution that caused it. The
    // request it answers is not a contribution, and citing the disclosed material as evidence
    // would require the recorder to know that material before the record exists -- which is the
    // ordering ADR-0030's Release exchange is built to prevent.
    //
    // Its provenance is therefore in its payload: a digest committing to exactly what was
    // released, verifiable afterwards against what the consumer received.
    return kind == ContributionKind::Observation || kind == ContributionKind::ContextDisclosed;
}

/// Whether a kind is a storage operation rather than a thought.
///
/// ADR-0028 forbids `Event1.Submit` from accepting these: destroying biography must never be
/// reachable by the same call that records a thought about it. A proposal is not permission to
/// execute, and this is the earliest point that rule can be enforced.
constexpr bool isErasureKind(ContributionKind kind) noexcept
{
    return kind == ContributionKind::ErasureRequested
        || kind == ContributionKind::ErasureApplied;
}

QString kindToString(ContributionKind kind);
QString privacyToString(PrivacyClass privacy);
PrivacyClass privacyFromString(const QString &text);

struct CognitiveEnvelope {
    quint16 schemaVersion{kCurrentEnvelopeSchemaVersion};

    QUuid messageId;
    QUuid correlationId;
    QUuid causationId;

    QString originOrgan;
    QString originNode;

    ContributionKind kind{ContributionKind::Observation};

    QDateTime wallTime;
    quint64 monotonicTime{0};
    quint64 logicalClock{0};

    double confidence{1.0};

    QList<QUuid> evidence;

    /// The payload as stored: plaintext for an ordinary contribution, `nonce ‖ ciphertext ‖ tag` for
    /// a sealed one. What it means is decided by `protection`, never guessed from its contents.
    QByteArray payloadCbor;

    PrivacyClass privacy{PrivacyClass::Local};
    QString capabilityScope;

    /// How the payload is protected, if it is.
    ///
    /// The key domain is an opaque UUID and epoch, never a name: a domain called `medical` or
    /// `location` would leak the category of an erased payload through metadata that survives
    /// erasure, which for many subjects is most of what there was to hide.
    ///
    /// Present only on schema-3 envelopes, and part of the non-erasable metadata: which key sealed a
    /// payload is a fact about the record that must stay verifiable after the payload is gone.
    /// When this contribution stops being allowed to exist, as an absolute instant.
    ///
    /// The class alone is a pointer into a policy that will change: if `Short` means seven days
    /// today and a day next year, every record written under the old meaning silently acquires the
    /// new one, and retention stops being a fact about the contribution. The resolved instant is
    /// what governs; the class and policy version are recorded so a later reader can see how it was
    /// arrived at.
    ///
    /// Null means unbounded, which is what every contribution written before retention existed
    /// carries. Absence is not a short lifetime.
    RetentionClass retentionClass{RetentionClass::Standard};
    quint16 retentionPolicyVersion{0};
    QDateTime retainUntil;

    struct Protection {
        bool sealed{false};
        QUuid keyDomainId;
        quint32 keyEpoch{0};

        bool isValid() const { return !sealed || !keyDomainId.isNull(); }
    };
    Protection protection;

    /// Structural validation only. Journal::append validates that referenced contributions
    /// already exist and that privacy is not weakened.
    bool isValid() const;

    /// The earliest instant among this contribution and everything it was derived from.
    ///
    /// Retention propagates by the same discipline as privacy and through the same code path:
    /// privacy takes the most restrictive class among references, retention takes the earliest
    /// expiry. A conclusion may not outlive the evidence it rests on, or erasing the evidence on
    /// expiry would leave the conclusion restating it.
    QDateTime derivedRetainUntil(const QList<QDateTime> &referenceRetainUntil) const;

    PrivacyClass derivedPrivacy(const QList<PrivacyClass> &evidencePrivacy) const;
};

} // namespace cybou

// Journal::accepted and Workspace::contributed are typed runtime signals. Registration here keeps
// the envelope usable by QSignalSpy and future queued transports without making protocol depend
// on any presentation type.
Q_DECLARE_METATYPE(cybou::CognitiveEnvelope)
