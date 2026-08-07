// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT

#include "cybou/workspace/Workspace.h"

#include <algorithm>
#include <cmath>

namespace cybou {

namespace {

constexpr double kHalfLifeSeconds = 120.0;

} // namespace

double attentionWeight(ContributionKind kind)
{
    switch (kind) {
    case ContributionKind::NeedSignal:
    case ContributionKind::Objection:
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
        return 0.5;
    }
}

QStringList Coalition::organs() const
{
    QStringList result;
    for (const auto &envelope : members) {
        if (!envelope.originOrgan.isEmpty()
            && !result.contains(envelope.originOrgan)) {
            result.append(envelope.originOrgan);
        }
    }
    return result;
}

Workspace::Workspace(
    EventStore *events,
    int capacity,
    QObject *parent)
    : QObject(parent)
    , m_events(events)
    , m_capacity(std::max(1, capacity))
{
    if (m_events) {
        connect(
            m_events,
            &EventStore::accepted,
            this,
            [this](const CognitiveEnvelope &envelope, quint64) {
                accept(envelope);
            });
    }
}

bool Workspace::publish(const CognitiveEnvelope &envelope)
{
    return m_events && m_events->append(envelope) != 0;
}

void Workspace::accept(const CognitiveEnvelope &envelope)
{
    if (envelope.messageId.isNull()) {
        return;
    }

    for (const CognitiveEnvelope &current : m_moment) {
        if (current.messageId == envelope.messageId) {
            return;
        }
    }

    m_moment.prepend(envelope);
    while (m_moment.size() > m_capacity) {
        m_moment.removeLast();
    }

    Q_EMIT contributed(envelope);
    reevaluateFocus();
}

void Workspace::rehydrate()
{
    if (!m_events) {
        return;
    }

    m_moment = m_events->recent(m_capacity);
    reevaluateFocus();
}

double Workspace::salienceOf(
    const Coalition &coalition,
    const QDateTime &now) const
{
    double total = 0.0;
    for (const auto &envelope : coalition.members) {
        const double ageSeconds =
            std::max(0.0, envelope.wallTime.msecsTo(now) / 1000.0);
        const double recency =
            std::pow(0.5, ageSeconds / kHalfLifeSeconds);
        total +=
            attentionWeight(envelope.kind) * envelope.confidence * recency;
    }

    const int voices = coalition.organs().size();
    return total * std::sqrt(static_cast<double>(std::max(1, voices)));
}

QList<Coalition> Workspace::coalitions(const QDateTime &nowOrNull) const
{
    const QDateTime now =
        nowOrNull.isValid() ? nowOrNull : QDateTime::currentDateTimeUtc();

    QList<Coalition> result;
    QHash<QUuid, int> indexOf;

    for (auto it = m_moment.crbegin(); it != m_moment.crend(); ++it) {
        const QUuid key =
            it->correlationId.isNull() ? it->messageId : it->correlationId;

        if (!indexOf.contains(key)) {
            Coalition coalition;
            coalition.correlationId = key;
            indexOf.insert(key, result.size());
            result.append(coalition);
        }

        Coalition &coalition = result[indexOf.value(key)];
        coalition.members.append(*it);
        if (!coalition.latest.isValid() || it->wallTime > coalition.latest) {
            coalition.latest = it->wallTime;
        }
    }

    for (Coalition &coalition : result) {
        coalition.salience = salienceOf(coalition, now);
    }

    std::stable_sort(
        result.begin(),
        result.end(),
        [](const Coalition &a, const Coalition &b) {
            if (a.salience != b.salience) {
                return a.salience > b.salience;
            }
            return a.latest > b.latest;
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

MomentState Workspace::momentState() const
{
    MomentState state;
    const Coalition current = focus();
    if (!current.isValid()) {
        return state;
    }

    state.focus = current.correlationId;
    state.salience = current.salience;
    state.organs = current.organs();
    return state;
}

} // namespace cybou
