// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT
//
// The shared language of the Cybou Mind: typed cognitive contributions, never free text.
//
// docs/14-mind-architecture.md is the reference. Two rules are enforced by the types rather
// than by convention:
//
//   1. A contribution carries its causation and its evidence, so any claim can be traced back
//      to the observations it rests on.
//   2. PrivacyClass is a closed enum with a fail-closed default, because a string is a comment
//      until something refuses to act on it.

#pragma once

#include <QByteArray>
#include <QDateTime>
#include <QList>
#include <QString>
#include <QUuid>

namespace cybou {

/// What kind of contribution this is. The alpha uses a subset; the rest are declared now so
/// the wire format does not change when organs arrive (docs/14).
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
};

/// Ordered from most restrictive to least. Local is the default on purpose: an absent or
/// unrecognised class must never mean "shareable".
enum class PrivacyClass : quint8 {
    Local = 0,
    Node,
    Household,
    Public,
};

/// A contribution's class is at least as restrictive as the most restrictive of its evidence.
/// Without this, privacy leaks through generalisation - a summary of local sensor data would
/// become publishable simply by passing through consolidation.
constexpr PrivacyClass mostRestrictive(PrivacyClass a, PrivacyClass b) noexcept
{
    return (static_cast<quint8>(a) < static_cast<quint8>(b)) ? a : b;
}

QString kindToString(ContributionKind kind);
QString privacyToString(PrivacyClass privacy);

/// Named kindToString/privacyToString rather than toString: QCOMPARE finds a toString
/// overload by ADL for its failure messages and requires char*, so a QString-returning
/// toString in this namespace breaks every test that compares these types.
///
/// Parse back from the journal. Unknown input yields Local for privacy - fail closed.
PrivacyClass privacyFromString(const QString &text);

/// The envelope every organ publishes and every organ can read.
struct CognitiveEnvelope {
    quint16 schemaVersion{1};

    QUuid messageId;
    /// Binds a whole cognitive episode - a build, a reboot, an investigation.
    QUuid correlationId;
    /// The contribution that directly produced this one. Null for a root observation.
    QUuid causationId;

    QString originOrgan;
    QString originNode;

    ContributionKind kind{ContributionKind::Observation};

    QDateTime wallTime;
    /// Monotonic milliseconds since boot: wall time can jump, this cannot.
    quint64 monotonicTime{0};
    /// Restores order across nodes without trusting synchronised clocks.
    quint64 logicalClock{0};

    /// How sure the originating organ is. 1.0 for a direct observation.
    double confidence{1.0};

    QList<QUuid> evidence;
    QByteArray payloadCbor;

    PrivacyClass privacy{PrivacyClass::Local};
    QString capabilityScope;

    /// True when the envelope can be written to the journal and read back meaningfully.
    bool isValid() const;

    /// Privacy derived from evidence, per the rule above. Callers pass the classes of the
    /// contributions named in `evidence`.
    PrivacyClass derivedPrivacy(const QList<PrivacyClass> &evidencePrivacy) const;
};

} // namespace cybou
