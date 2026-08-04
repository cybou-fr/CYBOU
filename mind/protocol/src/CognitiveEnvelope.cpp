// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT

#include "cybou/protocol/CognitiveEnvelope.h"

namespace cybou {

QString kindToString(ContributionKind kind)
{
    switch (kind) {
    case ContributionKind::Observation:        return QStringLiteral("observation");
    case ContributionKind::BeliefRevision:     return QStringLiteral("belief-revision");
    case ContributionKind::Hypothesis:         return QStringLiteral("hypothesis");
    case ContributionKind::MemoryRecall:       return QStringLiteral("memory-recall");
    case ContributionKind::NeedSignal:         return QStringLiteral("need-signal");
    case ContributionKind::AttentionCandidate: return QStringLiteral("attention-candidate");
    case ContributionKind::Prediction:         return QStringLiteral("prediction");
    case ContributionKind::PlanProposal:       return QStringLiteral("plan-proposal");
    case ContributionKind::Objection:          return QStringLiteral("objection");
    case ContributionKind::Decision:           return QStringLiteral("decision");
    case ContributionKind::Intention:          return QStringLiteral("intention");
    case ContributionKind::Outcome:            return QStringLiteral("outcome");
    case ContributionKind::SelfAssessment:     return QStringLiteral("self-assessment");
    case ContributionKind::Learning:           return QStringLiteral("learning");
    }
    return QStringLiteral("unknown");
}

QString privacyToString(PrivacyClass privacy)
{
    switch (privacy) {
    case PrivacyClass::Local:     return QStringLiteral("local");
    case PrivacyClass::Node:      return QStringLiteral("node");
    case PrivacyClass::Household: return QStringLiteral("household");
    case PrivacyClass::Public:    return QStringLiteral("public");
    }
    return QStringLiteral("local");
}

PrivacyClass privacyFromString(const QString &text)
{
    if (text == QLatin1String("public")) {
        return PrivacyClass::Public;
    }
    if (text == QLatin1String("household")) {
        return PrivacyClass::Household;
    }
    if (text == QLatin1String("node")) {
        return PrivacyClass::Node;
    }
    // Everything else, including an empty or unrecognised value: fail closed.
    return PrivacyClass::Local;
}

bool CognitiveEnvelope::isValid() const
{
    if (messageId.isNull() || originOrgan.isEmpty() || !wallTime.isValid()) {
        return false;
    }
    // A contribution that is not a root observation must say what caused it. Without this the
    // causal graph silently grows orphans, and an explanation cannot be traced.
    if (kind != ContributionKind::Observation && causationId.isNull() && evidence.isEmpty()) {
        return false;
    }
    return confidence >= 0.0 && confidence <= 1.0;
}

PrivacyClass CognitiveEnvelope::derivedPrivacy(const QList<PrivacyClass> &evidencePrivacy) const
{
    PrivacyClass result = privacy;
    for (const PrivacyClass p : evidencePrivacy) {
        result = mostRestrictive(result, p);
    }
    return result;
}

} // namespace cybou
