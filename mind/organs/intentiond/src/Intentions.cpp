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

    if (!catchUp()) {
        return {};
    }

    // Already exactly the open set, in acceptance order. Nothing to filter and nothing to sort.
    return m_open;
}

// One chronological pass, paged, resumed from a cursor.
//
// The two-pass version was justified by the claim that an Outcome closing an Intention can only be
// recognised once that Intention has been seen, so a single pass would meet some Outcomes first.
// Causation makes that impossible: an Outcome names the Intention it closes, and that Intention was
// accepted before it, so in chronological order the Intention has always been seen already. The
// justification contradicted the guarantee written directly beneath it.
//
// That same guarantee is what makes the cursor safe. Because closures never precede what they
// close, state accumulated up to a sequence stays correct as later pages arrive; nothing already
// read can be invalidated by something read afterwards.
bool Intentions::catchUp() const
{
    if (!m_events) {
        m_lastError = QStringLiteral("intentions need a journal to read");
        return false;
    }

    constexpr int kPageSize = 1000;
    for (;;) {
        const ContributionPage page = m_events->after(m_cursor, kPageSize);
        if (!page.ok) {
            m_lastError = QStringLiteral("could not read contributions after %1").arg(m_cursor);
            return false;
        }
        if (page.envelopes.isEmpty()) {
            return true;
        }

        for (const CognitiveEnvelope &e : page.envelopes) {
            if (e.kind == ContributionKind::Intention) {
                const QCborMap payload = QCborValue::fromCbor(e.payloadCbor).toMap();
                Intention i;
                i.id = e.messageId;
                i.description = payload[QStringLiteral("description")].toString();
                i.trigger = payload[QStringLiteral("trigger")].toString();
                i.formed = e.wallTime;
                m_open.append(i);
                m_openIds.insert(e.messageId);
                continue;
            }
            if (e.kind == ContributionKind::Outcome
                && e.originOrgan == QLatin1String("intentiond")
                && m_openIds.remove(e.causationId)) {
                // Removing is what keeps a read proportional to what is open rather than to
                // everything ever committed to. The membership check above is also what stops an
                // Outcome for something never seen from removing anything.
                const auto closed = std::find_if(
                    m_open.begin(), m_open.end(), [&e](const Intention &candidate) {
                        return candidate.id == e.causationId;
                    });
                if (closed != m_open.end()) {
                    m_open.erase(closed);
                }
            }
        }

        if (page.lastSequence <= m_cursor) {
            m_lastError = QStringLiteral("the journal did not advance past %1").arg(m_cursor);
            return false;
        }
        m_cursor = page.lastSequence;

        if (!page.hasMore) {
            return true;
        }
    }
}

} // namespace cybou
