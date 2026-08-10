// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/eventd/EventService.h"

#include "cybou/events/EnvelopeCodec.h"
#include "cybou/events/EventStore.h"

#include <QCborMap>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonDocument>
#include <QJsonObject>
#include <QRegularExpression>
#include <QSaveFile>

namespace cybou {

namespace {

QByteArray submitReply(quint64 sequence, const QString &error)
{
    QCborMap map;
    map.insert(QStringLiteral("sequence"), QString::number(sequence));
    map.insert(QStringLiteral("error"), error);
    return map.toCborValue().toCbor();
}

bool validConsumerId(const QString &consumerId)
{
    static const QRegularExpression pattern(
        QStringLiteral("^[a-z0-9][a-z0-9.-]{0,63}$"));
    return pattern.match(consumerId).hasMatch();
}

} // namespace

EventService::EventService(const QString &journalPath, QObject *parent)
    : QObject(parent)
    , m_journal(journalPath, QStringLiteral("cybou-eventd"))
    , m_offsetsPath(QFileInfo(journalPath).dir().filePath(
          QStringLiteral("consumer-offsets.json")))
{
    m_offsetsReady = loadOffsets();
    connect(
        &m_journal,
        &EventStore::accepted,
        this,
        [this](const CognitiveEnvelope &envelope, quint64 sequence) {
            Q_EMIT Accepted(
                EnvelopeCodec::encode(envelope),
                static_cast<qulonglong>(sequence));
        });
}

QString EventService::startupError() const
{
    return !m_journal.isOpen() ? m_journal.lastError() : m_offsetsError;
}

bool EventService::loadOffsets()
{
    m_offsetsError.clear();
    if (!QFile::exists(m_offsetsPath)) return true;
    QFile file(m_offsetsPath);
    if (!file.open(QIODevice::ReadOnly)) {
        m_offsetsError = QStringLiteral("cannot read consumer offsets: %1").arg(file.errorString());
        return false;
    }
    QJsonParseError parseError;
    const QJsonDocument document = QJsonDocument::fromJson(file.readAll(), &parseError);
    if (parseError.error != QJsonParseError::NoError || !document.isObject()
        || document.object().value(QStringLiteral("version")).toInt() != 1
        || !document.object().value(QStringLiteral("offsets")).isObject()) {
        m_offsetsError = QStringLiteral("invalid consumer-offset state");
        return false;
    }
    const quint64 head = m_journal.count();
    QMap<QString, quint64> loaded;
    const QJsonObject offsets = document.object().value(QStringLiteral("offsets")).toObject();
    for (auto it = offsets.begin(); it != offsets.end(); ++it) {
        bool ok = false;
        const quint64 offset = it.value().toString().toULongLong(&ok);
        if (!validConsumerId(it.key()) || !ok || offset > head) {
            m_offsetsError = QStringLiteral("invalid consumer offset for %1").arg(it.key());
            return false;
        }
        loaded.insert(it.key(), offset);
    }
    m_offsets = loaded;
    return true;
}

bool EventService::saveOffsets(const QMap<QString, quint64> &offsets)
{
    QJsonObject encoded;
    for (auto it = offsets.cbegin(); it != offsets.cend(); ++it)
        encoded.insert(it.key(), QString::number(it.value()));
    QJsonObject root;
    root.insert(QStringLiteral("version"), 1);
    root.insert(QStringLiteral("offsets"), encoded);
    QSaveFile file(m_offsetsPath);
    if (!file.open(QIODevice::WriteOnly)
        || file.write(QJsonDocument(root).toJson(QJsonDocument::Compact)) < 0
        || !file.commit()) {
        m_offsetsError = QStringLiteral("cannot persist consumer offsets: %1")
                             .arg(file.errorString());
        return false;
    }
    m_offsetsError.clear();
    return true;
}

bool EventService::Ready() const
{
    return m_journal.isOpen();
}

int EventService::SchemaVersion() const
{
    return m_journal.databaseSchemaVersion();
}

QByteArray EventService::Submit(const QByteArray &encodedEnvelope)
{
    QString decodeError;
    const auto envelope =
        EnvelopeCodec::decode(encodedEnvelope, &decodeError);
    if (!envelope) {
        return submitReply(
            0,
            QStringLiteral("invalid Event1 proposal: %1").arg(decodeError));
    }

    const quint64 sequence = m_journal.append(*envelope);
    return submitReply(
        sequence,
        sequence == 0 ? m_journal.lastError() : QString());
}

qulonglong EventService::Count() const
{
    return static_cast<qulonglong>(m_journal.count());
}

QByteArray EventService::Head() const
{
    return m_journal.head();
}

qulonglong EventService::Verify() const
{
    return static_cast<qulonglong>(m_journal.verify());
}

bool EventService::EnsureConsumer(const QString &consumerId, qulonglong initialOffset)
{
    if (!m_offsetsReady || !validConsumerId(consumerId)
        || initialOffset > m_journal.count()) return false;
    if (m_offsets.contains(consumerId)) return true;
    QMap<QString, quint64> candidate = m_offsets;
    candidate.insert(consumerId, initialOffset);
    if (!saveOffsets(candidate)) return false;
    m_offsets = candidate;
    return true;
}

bool EventService::AdvanceConsumer(const QString &consumerId, qulonglong offset)
{
    if (!m_offsetsReady || !m_offsets.contains(consumerId)
        || offset < m_offsets.value(consumerId) || offset > m_journal.count()) return false;
    if (offset == m_offsets.value(consumerId)) return true;
    QMap<QString, quint64> candidate = m_offsets;
    candidate[consumerId] = offset;
    if (!saveOffsets(candidate)) return false;
    m_offsets = candidate;
    return true;
}

QByteArray EventService::ConsumerBacklog(const QString &consumerId) const
{
    QCborMap result;
    result.insert(QStringLiteral("consumerId"), consumerId);
    const bool registered = validConsumerId(consumerId) && m_offsets.contains(consumerId);
    result.insert(QStringLiteral("registered"), registered);
    const quint64 head = m_journal.count();
    const quint64 offset = registered ? m_offsets.value(consumerId) : 0;
    result.insert(QStringLiteral("head"), QString::number(head));
    result.insert(QStringLiteral("offset"), QString::number(offset));
    quint64 backlog = registered ? head - offset : 0;
    // Consolidation must not count its own output as new input, so its backlog excludes
    // contributions carrying that capability scope. This used to decode one envelope per row of
    // backlog, which put an unbounded per-call cost on the process that owns the only write path -
    // the backlog grows with the biography, and any caller in the session could hold the writer
    // busy by asking repeatedly. One aggregate query answers the same question.
    if (registered && consumerId == QStringLiteral("lifecycle.consolidation")) {
        backlog = m_journal.countAfterExcludingCapability(offset, consumerId);
    }
    result.insert(QStringLiteral("backlog"), QString::number(backlog));
    return result.toCborValue().toCbor();
}

QByteArray EventService::Recent(int limit) const
{
    return EnvelopeCodec::encodeList(m_journal.recent(limit));
}

QByteArray EventService::Episode(const QString &correlationId) const
{
    return EnvelopeCodec::encodeList(
        m_journal.episode(QUuid::fromString(correlationId)));
}

QByteArray EventService::AtSequence(qulonglong sequence) const
{
    const auto envelope = m_journal.atSequence(sequence);
    return envelope ? EnvelopeCodec::encode(*envelope) : QByteArray();
}

bool EventService::Contains(const QString &messageId) const
{
    return m_journal.contains(QUuid::fromString(messageId));
}

QByteArray EventService::Contribution(const QString &messageId) const
{
    const auto envelope =
        m_journal.contribution(QUuid::fromString(messageId));
    return envelope ? EnvelopeCodec::encode(*envelope) : QByteArray();
}

QByteArray EventService::EvidenceFor(const QString &messageId) const
{
    return EnvelopeCodec::encodeUuidList(
        m_journal.evidenceFor(QUuid::fromString(messageId)));
}

bool EventService::HasOutcomeFor(
    const QString &causeId,
    const QString &originOrgan) const
{
    return m_journal.hasOutcomeFor(
        QUuid::fromString(causeId),
        originOrgan);
}

} // namespace cybou
