// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/meaning/CognitiveAct.h"

#include <algorithm>

namespace cybou {

QString actKindToString(ActKind kind)
{
    switch (kind) {
    case ActKind::Ask:
        return QStringLiteral("ask");
    case ActKind::Inform:
        return QStringLiteral("inform");
    case ActKind::Request:
        return QStringLiteral("request");
    case ActKind::Correct:
        return QStringLiteral("correct");
    case ActKind::Confirm:
        return QStringLiteral("confirm");
    case ActKind::Reject:
        return QStringLiteral("reject");
    }
    return QStringLiteral("unknown");
}

void ReferenceResolution::resolveIfUnambiguous(double margin)
{
    m_resolved.clear();
    if (candidates.isEmpty()) {
        return;
    }

    QList<ReferenceCandidate> ranked = candidates;
    std::stable_sort(
        ranked.begin(), ranked.end(), [](const ReferenceCandidate &a, const ReferenceCandidate &b) {
            return a.score > b.score;
        });

    if (ranked.size() == 1) {
        m_resolved = ranked.first().entityId;
        return;
    }

    // A leader that is not clearly ahead is not a resolution. Two servers at 0.52 and 0.48 are a
    // question to ask, not an answer to act on.
    if (ranked.at(0).score - ranked.at(1).score >= margin) {
        m_resolved = ranked.first().entityId;
    }
}

bool ReferenceResolution::resolveByPerson(const QString &entityId)
{
    // Only among the candidates that were actually surfaced. Accepting anything at all would let a
    // correction introduce a target the interpretation never considered, which is a different act.
    for (const ReferenceCandidate &candidate : candidates) {
        if (candidate.entityId == entityId) {
            m_resolved = entityId;
            return true;
        }
    }
    return false;
}

bool CognitiveAct::isFullyResolved() const
{
    for (const ReferenceResolution &reference : references) {
        if (!reference.isResolved()) {
            return false;
        }
    }
    return true;
}

} // namespace cybou
