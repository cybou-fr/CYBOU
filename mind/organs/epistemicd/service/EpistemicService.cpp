// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "EpistemicService.h"

#include <QCborArray>
#include <QCborMap>
#include <QDateTime>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QSaveFile>

namespace cybou {

namespace {

inline constexpr quint16 kCheckpointSchemaVersion = 1;

// One page at a time, so a long biography is never held in memory at once and never crosses the bus
// in a single reply. Matches the server-side cap; asking for more would simply be trimmed.
inline constexpr int kReplayPageSize = 1000;

QCborMap encodeClaim(const EpistemicClaim &claim)
{
    QCborMap map;
    map.insert(QStringLiteral("sourceId"), claim.sourceId);
    map.insert(QStringLiteral("value"), claim.value);
    map.insert(
        QStringLiteral("acquiredAt"), claim.acquiredAt.toUTC().toString(Qt::ISODateWithMs));
    map.insert(
        QStringLiteral("freshUntil"), claim.freshUntil.toUTC().toString(Qt::ISODateWithMs));
    map.insert(QStringLiteral("status"), epistemicStatusToString(claim.status));
    return map;
}

QCborMap encodeKnowledge(const SubjectKnowledge &knowledge)
{
    QCborArray current;
    for (const EpistemicClaim &claim : knowledge.current) {
        current.append(encodeClaim(claim));
    }
    QCborArray superseded;
    for (const EpistemicClaim &claim : knowledge.superseded) {
        superseded.append(encodeClaim(claim));
    }

    QCborMap map;
    map.insert(QStringLiteral("subject"), knowledge.subject);
    map.insert(QStringLiteral("status"), epistemicStatusToString(knowledge.status));
    map.insert(QStringLiteral("current"), current);
    map.insert(QStringLiteral("superseded"), superseded);
    return map;
}

} // namespace

EpistemicService::EpistemicService(
    EventStore *events,
    QString checkpointPath,
    QObject *parent)
    : QObject(parent)
    , m_events(events)
    , m_checkpointPath(std::move(checkpointPath))
{
    if (!m_events || !m_events->isOpen()) {
        m_startupError = QStringLiteral("epistemicd has no journal to derive from");
        return;
    }

    // A checkpoint that will not load is not an error. Rebuilding from the Journal is always
    // available and always correct, so the only cost is the replay that follows.
    load();

    if (!catchUp()) {
        m_startupError = m_lastError;
        return;
    }
    m_ready = true;
}

bool EpistemicService::Ready() const
{
    return m_ready;
}

// Being behind is not being unhealthy. This organ's health is whether it can derive at all, which
// means having a journal to read; how much of it has been read is what Cursor reports.
QString EpistemicService::Health() const
{
    return m_ready ? QStringLiteral("healthy") : QStringLiteral("unavailable");
}

QString EpistemicService::LastError() const
{
    return m_lastError;
}

qulonglong EpistemicService::Cursor() const
{
    return static_cast<qulonglong>(m_cursor);
}

QByteArray EpistemicService::Knowledge() const
{
    QCborArray all;
    for (const SubjectKnowledge &knowledge :
         m_projection.knowledgeAt(QDateTime::currentDateTimeUtc())) {
        all.append(encodeKnowledge(knowledge));
    }
    return all.toCborValue().toCbor();
}

QByteArray EpistemicService::KnowledgeOf(const QString &subject) const
{
    return encodeKnowledge(
               m_projection.knowledgeOf(subject, QDateTime::currentDateTimeUtc()))
        .toCborValue()
        .toCbor();
}

bool EpistemicService::catchUp()
{
    if (!m_events || !m_events->isOpen()) {
        m_lastError = QStringLiteral("the journal is unavailable");
        return false;
    }

    const quint64 startedAt = m_cursor;
    for (;;) {
        const ContributionPage page = m_events->after(m_cursor, kReplayPageSize);
        if (!page.ok) {
            // A failed page is not the end of history. Stopping here and calling it caught up would
            // leave a gap that nothing downstream could discover, so the cursor keeps whatever it
            // reached and the failure is reported.
            m_lastError = QStringLiteral("could not read contributions after %1").arg(m_cursor);
            return false;
        }
        if (page.envelopes.isEmpty()) {
            break;
        }

        for (const CognitiveEnvelope &envelope : page.envelopes) {
            m_projection.admit(envelope);
        }
        if (page.lastSequence <= m_cursor) {
            m_lastError = QStringLiteral("the journal did not advance past %1").arg(m_cursor);
            return false;
        }
        m_cursor = page.lastSequence;

        if (!page.hasMore) {
            break;
        }
    }

    m_lastError.clear();
    if (m_cursor != startedAt) {
        persist();
        Q_EMIT Changed();
    }
    return true;
}

void EpistemicService::admitAccepted(const CognitiveEnvelope &envelope, quint64 sequence)
{
    // Announcements can arrive out of order relative to what a catch-up already read, and one that
    // is already behind the cursor has been admitted. Re-admitting would be harmless - admission is
    // idempotent - but moving the cursor backwards would not be.
    if (sequence <= m_cursor) {
        return;
    }

    // A gap means something was accepted that never reached us. Reading it now is cheap and keeps
    // the projection a function of the whole history rather than of what happened to be delivered.
    if (sequence > m_cursor + 1) {
        catchUp();
    }

    if (sequence <= m_cursor) {
        return;
    }

    m_projection.admit(envelope);
    m_cursor = sequence;
    persist();
    Q_EMIT Changed();
}

bool EpistemicService::load()
{
    QFile file(m_checkpointPath);
    if (!file.exists() || !file.open(QIODevice::ReadOnly)) {
        return false;
    }

    const QCborValue root = QCborValue::fromCbor(file.readAll());
    if (!root.isMap()
        || root.toMap().value(QStringLiteral("schemaVersion")).toInteger(-1)
            != kCheckpointSchemaVersion) {
        return false;
    }

    const QCborMap map = root.toMap();
    EpistemicProjection restored;
    if (!restored.restore(map.value(QStringLiteral("projection")).toByteArray())) {
        return false;
    }

    // Applied together or not at all. Half of this pair is worse than neither: a cursor without its
    // projection claims history was admitted that was not.
    m_projection = restored;
    m_cursor = map.value(QStringLiteral("cursor")).toString().toULongLong();
    return true;
}

void EpistemicService::persist()
{
    QDir().mkpath(QFileInfo(m_checkpointPath).absolutePath());

    QCborMap root;
    root.insert(QStringLiteral("schemaVersion"), kCheckpointSchemaVersion);
    root.insert(QStringLiteral("cursor"), QString::number(m_cursor));
    root.insert(QStringLiteral("projection"), m_projection.snapshot());

    QSaveFile file(m_checkpointPath);
    if (!file.open(QIODevice::WriteOnly)) {
        return;
    }
    if (file.write(root.toCborValue().toCbor()) < 0) {
        return;
    }
    // A checkpoint that fails to write costs a replay next start and nothing else, so it is
    // deliberately not an error the caller has to handle.
    file.commit();
}

} // namespace cybou
