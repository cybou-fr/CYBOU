// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT

#include "cybou/intentions/Intentions.h"

#include <QCborMap>
#include <QCborValue>
#include <QSet>

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

Intentions::Intentions(Journal *journal)
    : m_journal(journal)
{
}

QUuid Intentions::form(const QString &description, const QString &trigger)
{
    if (!m_journal || description.isEmpty()) {
        m_lastError = QStringLiteral("an intention needs a journal and a description");
        return {};
    }

    CognitiveEnvelope e;
    e.messageId = QUuid::createUuid();
    // Its own episode: everything that later happens because of this intention hangs off it.
    e.correlationId = e.messageId;
    e.originOrgan = QStringLiteral("intentiond");
    e.kind = ContributionKind::Intention;
    e.wallTime = QDateTime::currentDateTimeUtc();
    e.confidence = 1.0;
    e.privacy = PrivacyClass::Node;
    // An Intention is a root: it is formed, not derived, so it needs no causation. The type
    // check in CognitiveEnvelope::isValid would otherwise reject it, which is why evidence
    // carries the reason instead when there is one.

    QCborMap payload;
    payload[QStringLiteral("description")] = description;
    payload[QStringLiteral("trigger")] = trigger;
    e.payloadCbor = payload.toCborValue().toCbor();

    // Intention is not an Observation, so isValid() demands a cause or evidence. It names
    // itself: an intention is its own reason for existing until something supersedes it.
    e.causationId = e.messageId;

    if (m_journal->append(e) == 0) {
        m_lastError = m_journal->lastError();
        return {};
    }
    return e.messageId;
}

bool Intentions::close(const QUuid &intentionId, Resolution resolution, const QString &note)
{
    if (!m_journal || intentionId.isNull()) {
        m_lastError = QStringLiteral("closing needs a journal and an intention");
        return false;
    }

    CognitiveEnvelope e;
    e.messageId = QUuid::createUuid();
    e.correlationId = intentionId; // stays inside the intention's episode
    e.causationId = intentionId;   // this is what removes it from the open list
    e.originOrgan = QStringLiteral("intentiond");
    e.kind = ContributionKind::Outcome;
    e.wallTime = QDateTime::currentDateTimeUtc();
    e.confidence = 1.0;
    e.privacy = PrivacyClass::Node;
    e.evidence = {intentionId};

    QCborMap payload;
    payload[QStringLiteral("resolution")] = resolutionToString(resolution);
    payload[QStringLiteral("note")] = note;
    e.payloadCbor = payload.toCborValue().toCbor();

    if (m_journal->append(e) == 0) {
        m_lastError = m_journal->lastError();
        return false;
    }
    return true;
}

QList<Intention> Intentions::open() const
{
    QList<Intention> result;
    if (!m_journal) {
        return result;
    }

    // Read the whole biography once and fold it: intentions formed, minus intentions whose
    // outcome names them. Cheap while the journal is small; when it stops being small this is
    // the first place a projection table earns its keep.
    const auto all = m_journal->recent(0);

    QSet<QUuid> closed;
    for (const auto &e : all) {
        if (e.kind == ContributionKind::Outcome && !e.causationId.isNull()) {
            closed.insert(e.causationId);
        }
    }

    for (const auto &e : all) {
        if (e.kind != ContributionKind::Intention || closed.contains(e.messageId)) {
            continue;
        }
        const QCborMap payload = QCborValue::fromCbor(e.payloadCbor).toMap();
        Intention i;
        i.id = e.messageId;
        i.description = payload[QStringLiteral("description")].toString();
        i.trigger = payload[QStringLiteral("trigger")].toString();
        i.formed = e.wallTime;
        result.append(i);
    }

    // recent() is newest first; an obligation list reads better oldest first - the thing you
    // have owed longest is the thing you should see at the top.
    std::reverse(result.begin(), result.end());
    return result;
}

} // namespace cybou
