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

    // One chronological pass, paged.
    //
    // This replaces two passes justified by the claim that an Outcome closing an Intention can only
    // be recognised once that Intention has been seen, so a single pass would meet some Outcomes
    // first. Causation makes that impossible: an Outcome names the Intention it closes, and the
    // Intention was accepted before it, so in chronological order the Intention has always been
    // seen by the time its Outcome arrives. The justification contradicted the guarantee written
    // directly beneath it.
    //
    // So intentions are collected in order as they appear and closures are recorded alongside; the
    // filter at the end walks the intentions, not the biography.
    QList<Intention> formed;
    QSet<QUuid> formedIds;
    QSet<QUuid> closed;

    const bool replayed = m_events->replayAll([&](const CognitiveEnvelope &e) {
        if (e.kind == ContributionKind::Intention) {
            const QCborMap payload = QCborValue::fromCbor(e.payloadCbor).toMap();
            Intention i;
            i.id = e.messageId;
            i.description = payload[QStringLiteral("description")].toString();
            i.trigger = payload[QStringLiteral("trigger")].toString();
            i.formed = e.wallTime;
            formed.append(i);
            formedIds.insert(e.messageId);
            return;
        }
        if (e.kind == ContributionKind::Outcome
            && e.originOrgan == QLatin1String("intentiond")
            && formedIds.contains(e.causationId)) {
            closed.insert(e.causationId);
        }
    });

    // A replay that failed halfway would leave commitments looking open because the Outcome that
    // closed them was never read. Reporting a partial answer as the open set is worse than
    // reporting none, so this fails closed.
    if (!replayed) {
        return {};
    }

    for (const Intention &intention : formed) {
        if (!closed.contains(intention.id)) {
            result.append(intention);
        }
    }

    // Oldest first, as replayAll yields. The recent(0) version ended with a reverse because that
    // call yields newest first; reversing here would invert the order Presence shows commitments in.
    return result;
}

} // namespace cybou
