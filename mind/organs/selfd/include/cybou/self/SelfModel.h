// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/events/EventStore.h"
#include "cybou/identity/Identity.h"
#include "cybou/intentions/Intentions.h"
#include "cybou/predictor/Predictor.h"

namespace cybou {

struct SelfReport {
    QDateTime taken;

    qint64 ageInDays{0};
    quint64 sessions{0};
    QString architectureVersion;

    int openIntentions{0};
    qint64 oldestObligationDays{0};

    QList<Calibration> calibrations;
    int settledPredictions{0};

    quint64 contributions{0};
    bool journalIntact{true};
    quint64 firstBrokenAt{0};

    /// What the integrity check actually established, not merely whether it passed.
    ///
    /// A full rechain and a check anchored at a checkpoint both leave journalIntact true, but they
    /// are not the same evidence: the second trusts a prefix it did not re-examine. Carrying the
    /// status keeps self-assessment from reporting the weaker claim as the stronger one.
    VerificationStatus verification{VerificationStatus::FullyVerified};
    quint64 verifiedFrom{0};

    bool isValid() const { return taken.isValid(); }
};

QString narrateSelfReport(const SelfReport &report);

class SelfModel
{
public:
    SelfModel(
        EventStore *events,
        Identity *identity,
        Intentions *intentions,
        Predictor *predictor);

    SelfReport assess(const QUuid &causeId);
    SelfReport measure() const;

    QString narrate(const SelfReport &report) const
    {
        return narrateSelfReport(report);
    }

    QString lastError() const { return m_lastError; }

private:

    EventStore *m_events;
    Identity *m_identity;
    Intentions *m_intentions;
    Predictor *m_predictor;
    QString m_lastError;
};

} // namespace cybou
