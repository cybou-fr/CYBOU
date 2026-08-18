// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/meaning/CognitiveAct.h"

#include <QList>
#include <QString>
#include <QUuid>

namespace cybou {

/// What a response is for. Realization varies by language; the goal does not.
enum class ResponseGoal : quint8 {
    ExplainStatus = 0,
    Clarify,      ///< a reference could not be resolved and the person must choose
    Acknowledge,
    Refuse,
};

QString responseGoalToString(ResponseGoal goal);

/// One thing the response asserts, with the evidence behind it.
///
/// A claim carries its provenance because a sentence that cannot name where it came from is not
/// something Mind should be saying.
struct PlanClaim {
    QString subject;
    QString value;
    QList<QUuid> evidence;

    bool isValid() const { return !subject.isEmpty() && !value.isEmpty() && !evidence.isEmpty(); }
};

/// A response that exists semantically before it becomes prose, from ADR-0031.
///
/// The plan is the authority. Realization renders it into a language and may vary wording, order and
/// register freely -- but it has no channel through which to introduce a claim, because the only
/// claims it can see are these.
struct ResponsePlan {
    ResponseGoal goal{ResponseGoal::ExplainStatus};
    QList<PlanClaim> claims;

    /// References the person still has to settle. Present in the plan rather than dropped: an
    /// answer that quietly omitted what it could not resolve would read as complete.
    QList<ReferenceResolution> unresolved;

    /// Hedges the realization must not discard. Fluency that drops a qualification states something
    /// stronger than Mind meant.
    QStringList qualifications;

    SensitivityClass sensitivity{kUnclassifiedSensitivity};

    bool isValid() const;
};

/// Which surface language a plan is rendered into.
enum class Language : quint8 { English = 0, Russian };

/// Render a plan into prose.
///
/// Takes the plan and nothing else -- no journal, no context, no free text from a caller. That is
/// how ADR-0031's C6 is enforced: a renderer cannot add an authoritative claim that is absent from
/// the plan, because there is no argument through which one could arrive.
QString realize(const ResponsePlan &plan, Language language);

/// A plan that answers an act, built from what was resolved and what was not.
///
/// An act whose references are unresolved produces a `Clarify` plan carrying the candidates, never
/// an answer about a guessed target.
ResponsePlan planFor(const CognitiveAct &act, const QList<PlanClaim> &claims);

} // namespace cybou
