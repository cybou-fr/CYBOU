// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT
//
// Standing goals that outlive the process that formed them.
//
// There is no separate state file here, on purpose. An intention *is* a contribution in the
// journal, and closing one *is* an Outcome that names it as its cause. Open intentions are
// therefore derived, not stored twice - so the list can never drift from the biography, and
// "why did you think you owed me this?" is answerable by reading the chain.

#pragma once

#include "cybou/storage/Journal.h"

#include <QString>

namespace cybou {

struct Intention {
    QUuid id;
    QString description;
    /// What has to happen for this to be satisfiable - free text in the alpha, a condition
    /// later. Recorded so the reason survives even when the intention does not.
    QString trigger;
    QDateTime formed;
};

enum class Resolution : quint8 {
    Fulfilled,
    Abandoned,
    /// The reason it existed no longer applies. Not a failure, and worth distinguishing from
    /// abandonment when accuracy is measured later.
    Obsolete,
};

class Intentions
{
public:
    explicit Intentions(Journal *journal);

    /// Forms an intention and returns its id, or a null uuid on failure.
    QUuid form(const QString &description, const QString &trigger = QString());

    /// Records that an intention ended, and why. The Outcome names the intention as its
    /// cause, which is what removes it from the open list.
    bool close(const QUuid &intentionId, Resolution resolution, const QString &note = QString());

    /// Intentions with no Outcome naming them. This is the answer to "what do I still owe?"
    /// after a reboot, and it is computed from the journal every time rather than cached.
    QList<Intention> open() const;

    QString lastError() const { return m_lastError; }

private:
    Journal *m_journal;
    QString m_lastError;
};

QString resolutionToString(Resolution r);

} // namespace cybou
