// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT

#include "cybou/workspace/Workspace.h"

#include <QSet>

#include <algorithm>
#include <cmath>

namespace cybou {

namespace {

/// How long it takes a contribution to lose half its pull. Short enough that the moment moves
/// on by itself, long enough to survive a slow chain of organs answering each other.
constexpr double kHalfLifeSeconds = 120.0;

} // namespace

double attentionWeight(ContributionKind kind)
{
    switch (kind) {
    case ContributionKind::NeedSignal:
    case ContributionKind::Objection:
        // Both interrupt: something is wrong, or someone disagrees. Ignoring these is how a
        // system ends up confidently doing the wrong thing.
        return 3.0;
    case ContributionKind::Decision:
    case ContributionKind::Intention:
        return 2.0;
    case ContributionKind::Outcome:
    case ContributionKind::SelfAssessment:
    case ContributionKind::AttentionCandidate:
        return 1.5;
    case ContributionKind::Prediction:
    case ContributionKind::PlanProposal:
    case ContributionKind::Hypothesis:
    case ContributionKind::BeliefRevision:
        return 1.0;
    default:
        // Observations, recalls, learning: the background hum.
        return 0.5;
    }
}

QStringList Coalition::organs() const
{
    QStringList result;
    for (const auto &e : members) {
        if (!e.originOrgan.isEmpty() && !result.contains(e.originOrgan)) {
            result.append(e.originOrgan);
        }
    }
    return result;
}

Workspace::Workspace(Journal *journal, int capacity, QObject *parent)
    : QObject(parent)
    , m_journal(journal)
    , m_capacity(std::max(1, capacity))
{
}

bool Workspace::publish(const CognitiveEnvelope &envelope)
{
    if (!m_journal || m_journal->append(envelope) == 0) {
        return false;
    }

    m_moment.prepend(envelope);
    while (m_moment.size() > m_capacity) {
        // Out of the moment, still in the journal. This is the only kind of forgetting the
        // system does, and it is recoverable by reading.
        m_moment.removeLast();
    }

    Q_EMIT contributed(envelope);
    reevaluateFocus();
    return true;
}

void Workspace::rehydrate()
{
    if (!m_journal) {
        return;
    }
    m_moment = m_journal->recent(m_capacity); // already newest first
    reevaluateFocus();
}

double Workspace::salienceOf(const Coalition &coalition, const QDateTime &now) const
{
    double total = 0.0;
    for (const auto &e : coalition.members) {
        const double ageSeconds = std::max(0.0, e.wallTime.msecsTo(now) / 1000.0);
        const double recency = std::pow(0.5, ageSeconds / kHalfLifeSeconds);
        total += attentionWeight(e.kind) * e.confidence * recency;
    }

    // Corroboration: a concern several organs independently touched outranks one organ talking
    // to itself at the same volume. Square root so it helps without dominating.
    const int voices = coalition.organs().size();
    return total * std::sqrt(static_cast<double>(std::max(1, voices)));
}

QList<Coalition> Workspace::coalitions(const QDateTime &nowOrNull) const
{
    const QDateTime now = nowOrNull.isValid() ? nowOrNull : QDateTime::currentDateTimeUtc();

    QList<Coalition> result;
    QHash<QUuid, int> indexOf;

    // m_moment is newest first; build members oldest first so a coalition reads as a story.
    for (auto it = m_moment.crbegin(); it != m_moment.crend(); ++it) {
        const QUuid key = it->correlationId.isNull() ? it->messageId : it->correlationId;
        if (!indexOf.contains(key)) {
            Coalition c;
            c.correlationId = key;
            indexOf.insert(key, result.size());
            result.append(c);
        }
        Coalition &c = result[indexOf.value(key)];
        c.members.append(*it);
        if (!c.latest.isValid() || it->wallTime > c.latest) {
            c.latest = it->wallTime;
        }
    }

    for (Coalition &c : result) {
        c.salience = salienceOf(c, now);
    }

    std::stable_sort(result.begin(), result.end(), [](const Coalition &a, const Coalition &b) {
        if (a.salience != b.salience) {
            return a.salience > b.salience;
        }
        return a.latest > b.latest; // a tie goes to whatever spoke most recently
    });
    return result;
}

Coalition Workspace::focus(const QDateTime &now) const
{
    const auto all = coalitions(now);
    return all.isEmpty() ? Coalition{} : all.first();
}

void Workspace::reevaluateFocus()
{
    const Coalition current = focus();
    if (!current.isValid() || current.correlationId == m_lastFocus) {
        return;
    }
    m_lastFocus = current.correlationId;
    Q_EMIT focusChanged(current);
}

} // namespace cybou
