// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/crypto/KeyStore.h"
#include "cybou/storage/Journal.h"

#include <QDateTime>
#include <QList>
#include <QUuid>

namespace cybou {

/// What one sweep did, in enough detail to tell a full pass from a truncated one.
struct SweepReport {
    int expiredFound{0};
    int erased{0};
    int dependentsErased{0};
    int resumed{0};

    /// Targets whose erasure could not be completed. Named rather than counted: a failed erasure
    /// is a specific record that is still there, and a caller told only "three failed" cannot act.
    QList<QUuid> failed;

    /// Whether the sweep reached the end of what had expired.
    ///
    /// False when the budget ran out with work remaining. Partial is not empty: a sweep that
    /// stopped early and reported success would be indistinguishable from a system with nothing
    /// left to forget, which is the one reading that must never be wrong.
    bool complete{false};

    bool isClean() const { return failed.isEmpty(); }
};

/// Acts on what retention already decided.
///
/// ADR-0028 made `retainUntil` a durable fact about a record. This is what makes it consequential.
/// Nothing here decides how long anything should be kept -- that was settled when the contribution
/// was appended, and re-deciding it at expiry time would make retention a matter of whenever the
/// sweep happened to run.
///
/// Deliberately not inside Journal. A store that erased on its own schedule would be a store whose
/// contents depend on when it was last opened.
class RetentionSweep
{
public:
    RetentionSweep(Journal &journal, KeyStore &keys);

    /// Erase everything whose retention window closed before `instant`, up to a budget.
    ///
    /// Incomplete erasures are resumed first. A crash between requesting and applying leaves work
    /// that only this pass can finish, and doing new work ahead of it would let a half-erased
    /// record survive every sweep that ever ran.
    SweepReport sweep(const QDateTime &instant, int budget = 64);

    QString lastError() const { return m_lastError; }

private:
    /// The three-step protocol for one target: durable intent, key destruction, redaction.
    bool eraseOne(const QUuid &target, bool alreadyRequested);

    Journal &m_journal;
    KeyStore &m_keys;
    QString m_lastError;
};

} // namespace cybou
