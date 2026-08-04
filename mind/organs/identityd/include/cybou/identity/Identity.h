// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT
//
// Continuity of the subject.
//
// docs/14-mind-architecture.md: identity is not the database and not the id. It is the fact
// that the same subject persists across reboots and across architectural change, carrying its
// biography with it.
//
// This organ holds no memories and makes no decisions. It answers one question - "am I still
// the same?" - and records the answer in the journal so the claim itself has evidence.

#pragma once

#include "cybou/storage/Journal.h"

#include <QDateTime>
#include <QString>
#include <QUuid>

namespace cybou {

struct IdentityState {
    QUuid identityId;
    /// When this identity first existed. Never rewritten.
    QDateTime origin;
    /// How many times the system has come up as this identity.
    quint64 sessionCount{0};
    /// The architecture that last wrote this state, so a migration can be detected.
    QString architectureVersion;

    bool isValid() const { return !identityId.isNull() && origin.isValid(); }

    /// How long this identity has existed, in days. What "I have been here since" means.
    qint64 ageInDays() const;
};

class Identity
{
public:
    /// `statePath` is a small file beside the journal. The journal is the biography; this is
    /// only the pointer that says whose biography it is.
    Identity(const QString &statePath, Journal *journal);

    /// Loads existing state, or creates it on first run. Increments the session counter and
    /// writes one contribution to the journal either way, because "I woke up" is an event and
    /// "I was born" is a different event.
    ///
    /// Returns false only when the state cannot be persisted - continuity that is not written
    /// down is not continuity.
    bool beginSession();

    IdentityState state() const { return m_state; }

    /// True when this run created the identity rather than continuing one.
    bool wasBorn() const { return m_born; }

    QString lastError() const { return m_lastError; }

private:
    bool load();
    bool save() const;
    void record(ContributionKind kind, const QString &summary);

    QString m_statePath;
    Journal *m_journal;
    IdentityState m_state;
    bool m_born{false};
    QString m_lastError;
};

} // namespace cybou
