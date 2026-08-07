// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include <QByteArray>
#include <QDateTime>
#include <QList>
#include <QString>
#include <QUuid>

namespace cybou {

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

/// In protocol v1 only a direct Observation may enter the biography without a prior basis.
constexpr bool isRootKind(ContributionKind kind) noexcept
{
    return kind == ContributionKind::Observation;
}

QString kindToString(ContributionKind kind);
QString privacyToString(PrivacyClass privacy);
PrivacyClass privacyFromString(const QString &text);

struct CognitiveEnvelope {
    quint16 schemaVersion{1};

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
