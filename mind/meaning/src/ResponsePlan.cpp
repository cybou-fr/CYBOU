// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/meaning/ResponsePlan.h"

namespace cybou {

QString responseGoalToString(ResponseGoal goal)
{
    switch (goal) {
    case ResponseGoal::ExplainStatus:
        return QStringLiteral("explain-status");
    case ResponseGoal::Clarify:
        return QStringLiteral("clarify");
    case ResponseGoal::Acknowledge:
        return QStringLiteral("acknowledge");
    case ResponseGoal::Refuse:
        return QStringLiteral("refuse");
    }
    return QStringLiteral("unknown");
}

bool ResponsePlan::isValid() const
{
    for (const PlanClaim &claim : claims) {
        if (!claim.isValid()) {
            return false;
        }
    }

    // A clarification with nothing to clarify, or an explanation with nothing to explain, is a plan
    // that would realize into a sentence saying nothing while looking like an answer.
    if (goal == ResponseGoal::Clarify) {
        return !unresolved.isEmpty();
    }
    if (goal == ResponseGoal::ExplainStatus) {
        return !claims.isEmpty();
    }
    return true;
}

ResponsePlan planFor(const CognitiveAct &act, const QList<PlanClaim> &claims)
{
    ResponsePlan plan;
    plan.sensitivity = act.sensitivity;

    // Anything unresolved becomes a clarification rather than an answer. This is where C2 stops
    // being an assertion about acts and becomes the thing a person actually reads.
    for (const ReferenceResolution &reference : act.references) {
        if (!reference.isResolved()) {
            plan.unresolved.append(reference);
        }
    }

    if (!plan.unresolved.isEmpty()) {
        plan.goal = ResponseGoal::Clarify;
        return plan;
    }

    plan.goal = ResponseGoal::ExplainStatus;
    plan.claims = claims;
    return plan;
}

namespace {

QString joinCandidates(const ReferenceResolution &reference, const QString &separator)
{
    QStringList names;
    for (const ReferenceCandidate &candidate : reference.candidates) {
        names.append(candidate.entityId);
    }
    return names.join(separator);
}

} // namespace

QString realize(const ResponsePlan &plan, Language language)
{
    const bool ru = language == Language::Russian;
    QStringList lines;

    if (plan.goal == ResponseGoal::Clarify) {
        for (const ReferenceResolution &reference : plan.unresolved) {
            // The candidates are named, because "which one did you mean?" without the options is a
            // question the person cannot answer.
            lines.append(ru ? QStringLiteral("Уточните, что имеется в виду под «%1»: %2.")
                                  .arg(reference.surfaceForm, joinCandidates(reference, QStringLiteral(" или ")))
                            : QStringLiteral("Which did you mean by \"%1\": %2?")
                                  .arg(reference.surfaceForm, joinCandidates(reference, QStringLiteral(" or "))));
        }
    }

    for (const PlanClaim &claim : plan.claims) {
        lines.append(ru ? QStringLiteral("%1: %2.").arg(claim.subject, claim.value)
                        : QStringLiteral("%1 is %2.").arg(claim.subject, claim.value));
    }

    // Qualifications are rendered, never summarised away. Dropping a hedge for fluency states
    // something stronger than the plan does.
    for (const QString &qualification : plan.qualifications) {
        lines.append(ru ? QStringLiteral("Оговорка: %1.").arg(qualification)
                        : QStringLiteral("Note: %1.").arg(qualification));
    }

    return lines.join(QStringLiteral(" "));
}

} // namespace cybou
