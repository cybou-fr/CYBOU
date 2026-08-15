// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/storage/RetentionSweep.h"

namespace cybou {

RetentionSweep::RetentionSweep(Journal &journal, KeyStore &keys)
    : m_journal(journal)
    , m_keys(keys)
{
}

bool RetentionSweep::eraseOne(const QUuid &target, bool alreadyRequested)
{
    // Step one: durable intent. Skipped only when a previous pass already recorded it, because a
    // second request for the same target would claim a second erasure had been asked for.
    if (!alreadyRequested && m_journal.requestErasure(target, QStringLiteral("retention-expiry")) == 0) {
        m_lastError = m_journal.lastError();
        return false;
    }

    // Step two: key destruction, idempotent. A target that was never sealed has no key, and that
    // is not a failure -- an unsealed payload is erased by redaction alone.
    m_keys.destroyKeyFor(target);

    // Step three: redaction and the epoch bump, in one transaction.
    if (!m_journal.applyErasure(target)) {
        m_lastError = m_journal.lastError();
        return false;
    }
    return true;
}

SweepReport RetentionSweep::sweep(const QDateTime &instant, int budget)
{
    SweepReport report;
    m_lastError.clear();

    if (!instant.isValid() || budget <= 0) {
        m_lastError = QStringLiteral("a sweep needs a valid instant and a positive budget");
        return report;
    }

    int remaining = budget;

    // Resume before starting anything new. An interrupted erasure is the one state a crash can
    // leave, and a sweep that always preferred fresh work would step over it every time.
    for (const QUuid &pending : m_journal.incompleteErasures()) {
        if (remaining <= 0) {
            return report;
        }
        --remaining;
        if (eraseOne(pending, true)) {
            ++report.resumed;
        } else {
            report.failed.append(pending);
        }
    }

    // One page. `expiredBefore` excludes anything already requested, so the next call resumes where
    // this one stopped rather than re-reading what it just erased.
    if (remaining <= 0) {
        return report;
    }

    const int page = remaining;
    const QList<QUuid> expired = m_journal.expiredBefore(instant, page);
    report.expiredFound = expired.size();

    for (const QUuid &target : expired) {
        if (remaining <= 0) {
            return report;
        }

        // Descendants first. Erasing the payload and leaving what was derived from it would keep
        // the reasoning that restates the record Mind was asked to forget.
        const QList<QUuid> dependents = m_journal.retentionDependents(target);
        for (const QUuid &dependent : dependents) {
            if (dependent == target) {
                continue;
            }
            if (remaining <= 0) {
                return report;
            }
            --remaining;
            if (eraseOne(dependent, false)) {
                ++report.dependentsErased;
            } else {
                report.failed.append(dependent);
            }
        }

        if (remaining <= 0) {
            return report;
        }
        --remaining;
        if (eraseOne(target, false)) {
            ++report.erased;
        } else {
            report.failed.append(target);
        }
    }

    // Complete means the page came back short of its own limit: had more expired, the query would
    // have filled it. Comparing against `budget` instead would call a sweep complete whenever
    // resuming had already eaten part of the budget.
    report.complete = expired.size() < page;
    return report;
}

} // namespace cybou
