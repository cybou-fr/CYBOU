// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/protocol/CognitiveEnvelope.h"

#include <QSqlDatabase>
#include <QSqlQuery>
#include <QString>

#include <optional>

namespace cybou {

class Journal
{
public:
    explicit Journal(const QString &path, const QString &connectionName = QString());
    ~Journal();

    Journal(const Journal &) = delete;
    Journal &operator=(const Journal &) = delete;

    bool isOpen() const;
    QString lastError() const;

    quint64 append(const CognitiveEnvelope &envelope);

    quint64 count() const;
    QByteArray head() const;
    quint64 verify() const;

    QList<CognitiveEnvelope> recent(int limit = 50) const;
    QList<CognitiveEnvelope> episode(const QUuid &correlationId) const;

    bool contains(const QUuid &messageId) const;
    std::optional<CognitiveEnvelope> contribution(const QUuid &messageId) const;
    bool hasOutcomeFor(const QUuid &causeId, const QString &originOrgan = QString()) const;

private:
    QByteArray rowHash(quint64 seq, const CognitiveEnvelope &e, const QByteArray &prev) const;
    bool ensureSchema();
    std::optional<CognitiveEnvelope> readOne(QSqlQuery &query) const;

    QSqlDatabase m_db;
    QString m_connectionName;
    QString m_lastError;
};

} // namespace cybou
