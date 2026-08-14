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
    return kind == ContributionKind::Observation;
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

    PrivacyClass derivedPrivacy(const QList<PrivacyClass> &evidencePrivacy) const;
};

} // namespace cybou

// Journal::accepted and Workspace::contributed are typed runtime signals. Registration here keeps
// the envelope usable by QSignalSpy and future queued transports without making protocol depend
// on any presentation type.
Q_DECLARE_METATYPE(cybou::CognitiveEnvelope)
