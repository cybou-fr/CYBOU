// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/eventd/EventService.h"

#include "cybou/events/EnvelopeCodec.h"
#include "cybou/events/EventStore.h"

#include <QCborMap>

namespace cybou {

namespace {

QByteArray submitReply(quint64 sequence, const QString &error)
{
    QCborMap map;
    map.insert(QStringLiteral("sequence"), QString::number(sequence));
    map.insert(QStringLiteral("error"), error);
    return map.toCborValue().toCbor();
}

} // namespace

EventService::EventService(const QString &journalPath, QObject *parent)
    : QObject(parent)
    , m_journal(journalPath, QStringLiteral("cybou-eventd"))
{
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
