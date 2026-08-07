// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/events/EventStore.h"

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
    EventStore *m_events;
    QString m_lastError;
};

QString resolutionToString(Resolution r);

} // namespace cybou
