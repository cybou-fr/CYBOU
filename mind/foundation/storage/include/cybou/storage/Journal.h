// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT
//
// The biography. Append-only, and the append-only part is enforced rather than promised:
// every row carries the hash of the previous one, so a rewrite anywhere in the past breaks
// the chain from that point forward and `verify()` says where.
//
// docs/14-mind-architecture.md: old events are never corrected. A weakened hypothesis is a
// later event, so the mistake stays visible.

#pragma once

#include "cybou/protocol/CognitiveEnvelope.h"

#include <QSqlDatabase>
#include <QString>

namespace cybou {

class Journal
{
public:
    /// `path` is the SQLite file. Opening creates the schema if absent.
    explicit Journal(const QString &path, const QString &connectionName = QString());
    ~Journal();

    Journal(const Journal &) = delete;
    Journal &operator=(const Journal &) = delete;

    bool isOpen() const;
    QString lastError() const;

    /// Appends one contribution and returns its sequence number, or 0 on failure.
    /// Invalid envelopes are refused: the journal is the one place where a malformed
    /// contribution would be permanent.
    quint64 append(const CognitiveEnvelope &envelope);

    /// Number of contributions recorded.
    quint64 count() const;

    /// The hash of the most recent row - the head of the chain.
    QByteArray head() const;

    /// Walks the whole chain. Returns 0 when intact, otherwise the sequence number of the
    /// first row whose stored hash does not match its recomputed one.
    quint64 verify() const;

    /// Reads back contributions, newest first. `limit` of 0 means all.
    QList<CognitiveEnvelope> recent(int limit = 50) const;

    /// Everything belonging to one cognitive episode, oldest first, so a chain of reasoning
    /// can be replayed in the order it happened.
    QList<CognitiveEnvelope> episode(const QUuid &correlationId) const;

private:
    QByteArray rowHash(quint64 seq, const CognitiveEnvelope &e, const QByteArray &prev) const;
    bool ensureSchema();

    QSqlDatabase m_db;
    QString m_connectionName;
    QString m_lastError;
};

} // namespace cybou
