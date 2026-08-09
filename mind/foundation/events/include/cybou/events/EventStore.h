// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/protocol/CognitiveEnvelope.h"

#include <QObject>

#include <optional>

namespace cybou {

/// Transport-neutral view of the durable cognitive event store.
///
/// M2 Journal implements this locally. M3 EventClient implements it over D-Bus. Organs depend on
/// this contract rather than on SQLite/Journal, which makes eventd the production persistence
/// boundary without rewriting organ semantics.
class EventStore : public QObject
{
    Q_OBJECT

public:
    explicit EventStore(QObject *parent = nullptr)
        : QObject(parent)
    {
    }

    ~EventStore() override = default;

    EventStore(const EventStore &) = delete;
    EventStore &operator=(const EventStore &) = delete;

    virtual bool isOpen() const = 0;
    virtual QString lastError() const = 0;
    virtual int databaseSchemaVersion() const = 0;

    virtual quint64 append(const CognitiveEnvelope &envelope) = 0;

    virtual quint64 count() const = 0;
    virtual QByteArray head() const = 0;
    virtual quint64 verify() const = 0;

    virtual QList<CognitiveEnvelope> recent(int limit = 50) const = 0;
    virtual QList<CognitiveEnvelope> episode(const QUuid &correlationId) const = 0;
    virtual std::optional<CognitiveEnvelope> atSequence(quint64 sequence) const = 0;

    virtual bool contains(const QUuid &messageId) const = 0;
    virtual std::optional<CognitiveEnvelope> contribution(const QUuid &messageId) const = 0;
    virtual QList<QUuid> evidenceFor(const QUuid &messageId) const = 0;
    virtual bool hasOutcomeFor(
        const QUuid &causeId,
        const QString &originOrgan = QString()) const = 0;

Q_SIGNALS:
    /// Durable acceptance boundary. A proposal that did not commit must never appear here.
    void accepted(const CognitiveEnvelope &envelope, quint64 sequence);
};

} // namespace cybou
