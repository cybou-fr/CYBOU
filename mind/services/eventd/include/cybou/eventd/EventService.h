// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/storage/Journal.h"

#include <QDBusContext>
#include <QHash>
#include <QMap>
#include <QObject>

namespace cybou {

/// Organ identities that only the corresponding Mind process may claim as a contribution origin.
///
/// A contribution's `originOrgan` is provenance: it answers who brought this into the biography.
/// Nothing outside eventd can check that claim, so eventd has to. Without this, any process in the
/// user session can submit a contribution attributed to predictord, presenced, or a future
/// perception adapter, and nothing downstream would be able to tell.
QStringList reservedOrganIdentities();

/// D-Bus boundary for the single production Journal owner.
class EventService
    : public QObject
    , protected QDBusContext
{
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.cybou.Mind.Event1")

public:
    explicit EventService(const QString &journalPath, QObject *parent = nullptr);

    bool isReady() const { return m_journal.isOpen() && m_offsetsReady; }
    QString startupError() const;

public Q_SLOTS:
    bool Ready() const;
    int SchemaVersion() const;

    QByteArray Submit(const QByteArray &encodedEnvelope);

    qulonglong Count() const;
    QByteArray Head() const;
    qulonglong Verify() const;
    bool EnsureConsumer(const QString &consumerId, qulonglong initialOffset);
    bool AdvanceConsumer(const QString &consumerId, qulonglong offset);
    QByteArray ConsumerBacklog(const QString &consumerId) const;

    QByteArray Recent(int limit) const;

    /// One page of contributions after `afterSequence`, oldest first.
    ///
    /// The paged counterpart to Recent. Recent(0) returns the entire biography in one reply, which
    /// is how organs used to rebuild their state; at scale that is seconds of work and hundreds of
    /// megabytes across the bus, paid separately by each organ on every start. Replay lets a caller
    /// resume from a cursor and bound what it asks for.
    ///
    /// Answers a CBOR map: from, to, head, hasMore, envelopes.
    QByteArray Replay(qulonglong afterSequence, int limit) const;
    QByteArray Episode(const QString &correlationId) const;
    QByteArray AtSequence(qulonglong sequence) const;

    bool Contains(const QString &messageId) const;
    QByteArray Contribution(const QString &messageId) const;
    QByteArray EvidenceFor(const QString &messageId) const;
    bool HasOutcomeFor(
        const QString &causeId,
        const QString &originOrgan) const;

Q_SIGNALS:
    void Accepted(const QByteArray &encodedEnvelope, qulonglong sequence);

private:
    bool loadOffsets();
    bool saveOffsets(const QMap<QString, quint64> &offsets);

    /// Organ identity of the current D-Bus caller, or an empty string when the caller is not one of
    /// the Mind organs. Resolved from the calling process's executable and cached per connection.
    QString callerOrganIdentity() const;

    /// Whether `claimedOrigin` is one this caller is entitled to use.
    bool originIsAuthentic(const QString &claimedOrigin) const;

    Journal m_journal;
    QString m_offsetsPath;
    QMap<QString, quint64> m_offsets;
    bool m_offsetsReady{false};
    QString m_offsetsError;

    /// Sender unique name to resolved organ identity. A unique bus name belongs to one connection
    /// for its whole lifetime and is never reused, so this cannot go stale in a way that would
    /// admit an impostor; it only avoids asking the bus for the same process on every submission.
    mutable QHash<QString, QString> m_resolvedCallers;
};

} // namespace cybou
