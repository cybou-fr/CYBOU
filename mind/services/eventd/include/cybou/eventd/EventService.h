// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/storage/Journal.h"

#include <QObject>

namespace cybou {

/// D-Bus boundary for the single production Journal owner.
class EventService : public QObject
{
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.cybou.Mind.Event1")

public:
    explicit EventService(const QString &journalPath, QObject *parent = nullptr);

    bool isReady() const { return m_journal.isOpen(); }
    QString startupError() const { return m_journal.lastError(); }

public Q_SLOTS:
    bool Ready() const;
    int SchemaVersion() const;

    QByteArray Submit(const QByteArray &encodedEnvelope);

    qulonglong Count() const;
    QByteArray Head() const;
    qulonglong Verify() const;

    QByteArray Recent(int limit) const;
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
    Journal m_journal;
};

} // namespace cybou
