// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/events/EventStore.h"

#include <QSet>
#include <QString>

namespace cybou {

struct Intention {
    QUuid id;
    QString description;
    QString trigger;
    QDateTime formed;
};

enum class Resolution : quint8 {
    Fulfilled,
    Abandoned,
    Obsolete,
};

class Intentions
{
public:
    explicit Intentions(EventStore *journal);

    /// Forms an intention from a contribution already present in the journal.
    QUuid form(const QString &description, const QString &trigger, const QUuid &causeId);

    bool close(const QUuid &intentionId, Resolution resolution, const QString &note = QString());

    QList<Intention> open() const;

    QString lastError() const { return m_lastError; }

private:
    /// Take in everything accepted since the cursor.
    ///
    /// Every call to open() used to replay the whole biography, so asking what Mind is committed to
    /// cost the length of its life - and Presence asks on every Snapshot. This makes a read cost the
    /// contributions that arrived since the last one.
    ///
    /// Failing here is failing closed, for the reason the replay always was: an unread Outcome
    /// leaves the commitment it closed looking open, and a stale obligation presented as current is
    /// worse than admitting the answer could not be assembled.
    bool catchUp() const;

    EventStore *m_events;
    mutable QString m_lastError;

    // Derived state, rebuilt from the Journal and never authoritative over it. Mutable because
    // answering may require reading what has been accepted since the last question, which changes
    // how much has been read rather than what is true.
    //
    // Only what is still open, in the order it was accepted, because that is the order Presence
    // shows obligations in.
    //
    // An earlier version kept every intention ever formed and filtered the closed ones out on each
    // call. That made a read cost the number of commitments a life has ever had rather than the
    // number it currently carries - the same shape as the Journal replay it had just replaced, one
    // level up. Closing removes here instead, so what is not open is not carried.
    //
    // m_openIds exists because an Outcome names its Intention: without it, closing would search the
    // list, and a long-lived Mind closes as often as it opens.
    mutable QList<Intention> m_open;
    mutable QSet<QUuid> m_openIds;
    mutable quint64 m_cursor{0};
};

QString resolutionToString(Resolution r);

} // namespace cybou
