// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/events/EventStore.h"

#include <QDBusConnection>
#include <QDBusMessage>

namespace cybou {

/// Synchronous Event1 client used by the current in-process organs.
///
/// The transport is intentionally hidden behind EventStore. M4 can move an organ into its own
/// process without changing its domain API.
class EventClient : public EventStore
{
    Q_OBJECT

public:
    explicit EventClient(QObject *parent = nullptr);
    ~EventClient() override = default;

    bool isOpen() const override;
    bool isOpen(int timeoutMs) const;
    QString lastError() const override { return m_lastError; }
    int databaseSchemaVersion() const override;

    quint64 append(const CognitiveEnvelope &envelope) override;
    quint64 append(const CognitiveEnvelope &envelope, int timeoutMs);

    quint64 count() const override;
    quint64 count(int timeoutMs) const;
    QByteArray head() const override;
    quint64 verify() const override;
    bool ensureConsumer(const QString &consumerId, quint64 initialOffset = 0) const;
    bool advanceConsumer(const QString &consumerId, quint64 offset) const;
    std::optional<quint64> consumerBacklog(const QString &consumerId) const;

    QList<CognitiveEnvelope> recent(int limit = 50) const override;
    QList<CognitiveEnvelope> recent(int limit, int timeoutMs) const;
    ContributionPage after(quint64 afterSequence, int limit) const override;
    ContributionPage after(quint64 afterSequence, int limit, int timeoutMs) const;
    QList<CognitiveEnvelope> episode(const QUuid &correlationId) const override;
    std::optional<CognitiveEnvelope> atSequence(quint64 sequence) const override;

    bool contains(const QUuid &messageId) const override;
    std::optional<CognitiveEnvelope> contribution(const QUuid &messageId) const override;
    QList<QUuid> evidenceFor(const QUuid &messageId) const override;
    bool hasOutcomeFor(
        const QUuid &causeId,
        const QString &originOrgan = QString()) const override;

private Q_SLOTS:
    void onAccepted(const QByteArray &encodedEnvelope, qulonglong sequence);

private:
    QDBusMessage call(
        const QString &method,
        const QVariantList &arguments = QVariantList(),
        int timeoutMs = -1) const;

    QByteArray callBytes(
        const QString &method,
        const QVariantList &arguments = QVariantList(),
        int timeoutMs = -1) const;

    mutable QString m_lastError;
    QDBusConnection m_bus;
};

} // namespace cybou
