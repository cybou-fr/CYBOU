// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/intentions/Intentions.h"

#include <QCborMap>
#include <QCborValue>
#include <QSet>

#include <algorithm>

namespace cybou {

QString resolutionToString(Resolution r)
{
    switch (r) {
    case Resolution::Fulfilled: return QStringLiteral("fulfilled");
    case Resolution::Abandoned: return QStringLiteral("abandoned");
    case Resolution::Obsolete:  return QStringLiteral("obsolete");
    }
    return QStringLiteral("abandoned");
}

Intentions::Intentions(EventStore *journal)
    : m_events(journal)
{
}

QUuid Intentions::form(
    const QString &description, const QString &trigger, const QUuid &causeId)
{
    m_lastError.clear();

    if (!m_events || description.trimmed().isEmpty() || causeId.isNull()) {
        m_lastError = QStringLiteral(
            "an intention needs a journal, a description, and an existing cause");
        return {};
    }

    const auto cause = m_events->contribution(causeId);
    if (!cause) {
        m_lastError = QStringLiteral("the intention cause does not exist");
        return {};
    }

    CognitiveEnvelope e;
    e.messageId = QUuid::createUuid();
    e.correlationId = cause->correlationId;
    e.causationId = causeId;
    e.originOrgan = QStringLiteral("intentiond");
    e.originNode = cause->originNode;
    e.kind = ContributionKind::Intention;
    e.wallTime = QDateTime::currentDateTimeUtc();
    e.confidence = 1.0;
    e.privacy = cause->privacy;

    QCborMap payload;
    payload[QStringLiteral("description")] = description.trimmed();
    payload[QStringLiteral("trigger")] = trigger.trimmed();
    e.payloadCbor = payload.toCborValue().toCbor();

    if (m_events->append(e) == 0) {
        m_lastError = m_events->lastError();
        return {};
    }
    return e.messageId;
}

bool Intentions::close(
    const QUuid &intentionId, Resolution resolution, const QString &note)
{
    m_lastError.clear();

    if (!m_events || intentionId.isNull()) {
        m_lastError = QStringLiteral("closing needs a journal and an intention");
        return false;
    }

    const auto intention = m_events->contribution(intentionId);
    if (!intention) {
        m_lastError = QStringLiteral("the intention does not exist");
        return false;
    }
    if (intention->kind != ContributionKind::Intention) {
        m_lastError = QStringLiteral("the target contribution is not an intention");
        return false;
    }
    if (m_events->hasOutcomeFor(intentionId, QStringLiteral("intentiond"))) {
        m_lastError = QStringLiteral("the intention is already closed");
        return false;
    }

    CognitiveEnvelope e;
    e.messageId = QUuid::createUuid();
    e.correlationId = intention->correlationId;
    e.causationId = intentionId;
    e.originOrgan = QStringLiteral("intentiond");
    e.originNode = intention->originNode;
    e.kind = ContributionKind::Outcome;
    e.wallTime = QDateTime::currentDateTimeUtc();
    e.confidence = 1.0;
    e.privacy = intention->privacy;

    QCborMap payload;
    payload[QStringLiteral("resolution")] = resolutionToString(resolution);
    payload[QStringLiteral("note")] = note;
    e.payloadCbor = payload.toCborValue().toCbor();

    if (m_events->append(e) == 0) {
        m_lastError = m_events->lastError();
        return false;
    }
    return true;
}

QList<Intention> Intentions::open() const
{
    QList<Intention> result;
    if (!m_events) {
        return result;
    }

    // Replayed in pages rather than pulled in one reply. This used to be recent(0), which
    // materialises the entire biography and carries it across the bus in a single message - about
    // nine seconds and hundreds of megabytes at a million contributions, paid on every call.
    //
    // Two passes are still needed, because an Outcome closing an Intention can only be recognised
    // once that Intention has been seen, and a single chronological pass would meet some Outcomes
    // first. So the first pass collects what was closed and the second builds the open set. Both
    // are paged; neither holds the whole history.
    QSet<QUuid> intentionIds;
    QSet<QUuid> closed;
    const bool firstPass = m_events->replayAll([&](const CognitiveEnvelope &e) {
        if (e.kind == ContributionKind::Intention) {
            intentionIds.insert(e.messageId);
            return;
        }
        // The Intention is required to have been seen already. Causation guarantees it precedes its
        // Outcome, so in chronological order that check is always satisfiable here - which is
        // exactly why the two passes can be chronological at all.
        if (e.kind == ContributionKind::Outcome
            && e.originOrgan == QLatin1String("intentiond")
            && intentionIds.contains(e.causationId)) {
            closed.insert(e.causationId);
        }
    });

    // A replay that failed halfway would leave commitments looking open because the Outcome that
    // closed them was never read. Reporting a partial answer as the open set is worse than
    // reporting none, so this fails closed.
    if (!firstPass) {
        return result;
    }

    const bool secondPass = m_events->replayAll([&](const CognitiveEnvelope &e) {
        if (e.kind != ContributionKind::Intention || closed.contains(e.messageId)) {
            return;
        }

        const QCborMap payload = QCborValue::fromCbor(e.payloadCbor).toMap();
        Intention i;
        i.id = e.messageId;
        i.description = payload[QStringLiteral("description")].toString();
        i.trigger = payload[QStringLiteral("trigger")].toString();
        i.formed = e.wallTime;
        result.append(i);
    });

    if (!secondPass) {
        return {};
    }

    // No reverse here, unlike the recent(0) version. That call yields newest first and the result
    // had to be flipped to put oldest first; replayAll already yields oldest first, so reversing
    // would now invert the order Presence shows commitments in.
    return result;
}

} // namespace cybou
