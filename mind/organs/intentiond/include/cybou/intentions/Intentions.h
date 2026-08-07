// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/storage/Journal.h"

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
    explicit Intentions(Journal *journal);

    /// Forms an intention from a contribution already present in the journal.
    QUuid form(const QString &description, const QString &trigger, const QUuid &causeId);

    bool close(const QUuid &intentionId, Resolution resolution, const QString &note = QString());

    QList<Intention> open() const;

    QString lastError() const { return m_lastError; }

private:
    Journal *m_journal;
    QString m_lastError;
};

QString resolutionToString(Resolution r);

} // namespace cybou
