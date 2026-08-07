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
    QByteArray payloadCbor;

    PrivacyClass privacy{PrivacyClass::Local};
    QString capabilityScope;

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
