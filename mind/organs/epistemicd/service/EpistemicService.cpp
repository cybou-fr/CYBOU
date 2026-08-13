// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "EpistemicService.h"

#include "cybou/fabric/FabricCodec.h"

#include <algorithm>

#include <QCborMap>
#include <QCborValue>
#include <QVariantList>
#include <QVariantMap>
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

// Every other organ answers in the versioned fabric envelope, and this one answered in bare CBOR.
// Nothing had consumed it yet, so nothing caught it - a projection nobody reads can be encoded any
// way at all and still look correct. Presence is the first reader, and this is the cheapest moment
// the wire could have been corrected.
QVariantMap encodeClaim(const EpistemicClaim &claim)
{
    QVariantMap map;
    map.insert(
        QStringLiteral("contributionId"), claim.contributionId.toString(QUuid::WithoutBraces));
    map.insert(QStringLiteral("sourceId"), claim.sourceId);
    map.insert(QStringLiteral("provenance"), claim.provenance);
    map.insert(QStringLiteral("value"), claim.value.toVariant());
    map.insert(
        QStringLiteral("acquiredAt"), claim.acquiredAt.toUTC().toString(Qt::ISODateWithMs));
    map.insert(
        QStringLiteral("freshUntil"), claim.freshUntil.toUTC().toString(Qt::ISODateWithMs));
    map.insert(QStringLiteral("status"), epistemicStatusToString(claim.status));
    return map;
}

// The current projection only.
//
// `superseded` grows for the life of the Journal: every real change a source reports files the
// previous reading there. Presence reads this on every Snapshot, so returning it inline would trade
// the full-Journal scan that P7.3 removed for an ever-growing reply - the same unbounded cost in a
// different place, and one that only becomes visible once there are more sources than one.
//
// What was superseded is still knowledge and is not discarded; it is asked for by the page, through
// KnowledgeHistory, by the rare caller that wants to know why Mind changed its mind.
QVariantMap encodeKnowledge(const SubjectKnowledge &knowledge)
{
    QVariantList current;
    for (const EpistemicClaim &claim : knowledge.current) {
        current.append(encodeClaim(claim));
    }

    QVariantMap map;
    map.insert(QStringLiteral("subject"), knowledge.subject);
    map.insert(QStringLiteral("status"), epistemicStatusToString(knowledge.status));
    map.insert(QStringLiteral("current"), current);
    map.insert(
        QStringLiteral("supersededCount"), static_cast<qulonglong>(knowledge.superseded.size()));
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
    QVariantList all;
    for (const SubjectKnowledge &knowledge :
         m_projection.knowledgeAt(QDateTime::currentDateTimeUtc())) {
        all.append(encodeKnowledge(knowledge));
    }
    return FabricCodec::encode(all);
}

QByteArray EpistemicService::KnowledgeOf(const QString &subject) const
{
    return FabricCodec::encode(
        encodeKnowledge(m_projection.knowledgeOf(subject, QDateTime::currentDateTimeUtc())));
}

QByteArray EpistemicService::KnowledgeHistory(
    const QString &subject, qulonglong afterIndex, int limit) const
{
    // Capped like Event1's Replay, and for the same reason: a caller asking for everything would
    // otherwise decide how much memory this organ and the bus have to find.
    constexpr int kMaxPage = 1000;
    const int page = limit <= 0 || limit > kMaxPage ? kMaxPage : limit;

    const SubjectKnowledge knowledge =
        m_projection.knowledgeOf(subject, QDateTime::currentDateTimeUtc());

    QVariantList superseded;
    const int total = knowledge.superseded.size();
    const int from = static_cast<int>(std::min<qulonglong>(afterIndex, total));
    const int to = std::min(from + page, total);
    for (int i = from; i < to; ++i) {
        superseded.append(encodeClaim(knowledge.superseded.at(i)));
    }

    QVariantMap map;
    map.insert(QStringLiteral("subject"), subject);
    map.insert(QStringLiteral("from"), static_cast<qulonglong>(from));
    map.insert(QStringLiteral("to"), static_cast<qulonglong>(to));
    map.insert(QStringLiteral("total"), static_cast<qulonglong>(total));
    map.insert(QStringLiteral("hasMore"), to < total);
    map.insert(QStringLiteral("superseded"), superseded);
    return FabricCodec::encode(map);
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

    // A gap means something was accepted that never reached us. Reading it now keeps the projection
    // a function of the whole history rather than of what happened to be delivered.
    //
    // Whether that read succeeded is the whole point, and this used to discard it. A failed
    // catch-up leaves the cursor where it was, so the announced sequence still looked admissible:
    // admitting it moved the cursor past contributions that had never been read, and nothing
    // downstream could ever discover them. That is the same defect catchUp() refuses one page at a
    // time - treating an unread stretch of history as read - reintroduced at the caller.
    //
    // So a gap that cannot be closed means this announcement is not admitted at all. Staying behind
    // is recoverable: the next catch-up, or the next announcement, reads from the same cursor.
    // Skipping is not.
    if (sequence > m_cursor + 1 && !catchUp()) {
        return;
    }

    if (sequence <= m_cursor) {
        return;
    }

    // Catch-up may have succeeded and still not reached this sequence - the contribution can be
    // committed but not yet visible to the reader that just ran. Admitting it here would leave the
    // same hole for a different reason.
    if (sequence != m_cursor + 1) {
        m_lastError =
            QStringLiteral("a gap below %1 is still unread; not admitting it").arg(sequence);
        return;
    }

    // Catch-up may have succeeded and still not reached this sequence - the contribution can be
    // committed but not yet visible to the reader that just ran. Admitting it here would leave the
    // same hole for a different reason.
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

    // A cursor that will not parse is not a cursor of zero. Falling back to zero alongside a
    // restored projection would claim nothing had been admitted while holding a projection that
    // says otherwise, and the replay that followed would re-admit everything against it.
    bool cursorParsed = false;
    const qulonglong cursor =
        map.value(QStringLiteral("cursor")).toString().toULongLong(&cursorParsed);
    if (!cursorParsed) {
        return false;
    }

    // Applied together or not at all. Half of this pair is worse than neither: a cursor without its
    // projection claims history was admitted that was not.
    m_projection = restored;
    m_cursor = cursor;
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
