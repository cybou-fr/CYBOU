// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT
//
// What the system can truthfully say about itself.
//
// This is the organ most at risk of becoming decoration, so it is built under one rule taken
// straight from ADR-0003: every field below is measured, and every measurement is traceable to
// a contribution in the journal. There is no overall "health score" and no mood, because
// neither could be derived from anything. If a value cannot be computed, it is absent - not
// estimated, not defaulted to something reassuring.

#pragma once

#include "cybou/identity/Identity.h"
#include "cybou/intentions/Intentions.h"
#include "cybou/predictor/Predictor.h"
#include "cybou/storage/Journal.h"

namespace cybou {

struct SelfReport {
    QDateTime taken;

    // Continuity - from identityd.
    qint64 ageInDays{0};
    quint64 sessions{0};
    QString architectureVersion;

    // Obligation - from intentiond.
    int openIntentions{0};
    /// Age of the oldest unmet intention. The number that should make someone uncomfortable.
    qint64 oldestObligationDays{0};

    // Accuracy - from predictord, one entry per subject it has been tested on.
    QList<Calibration> calibrations;
    int settledPredictions{0};

    // Integrity - from the journal itself.
    quint64 contributions{0};
    bool journalIntact{true};
    /// Sequence number of the first broken row, 0 when intact.
    quint64 firstBrokenAt{0};

    bool isValid() const { return taken.isValid(); }
};

class SelfModel
{
public:
    /// Every dependency is required. A self-model assembled from part of the system would be
    /// quietly wrong about the rest of it, which is worse than not existing.
    SelfModel(Journal *journal, Identity *identity, Intentions *intentions, Predictor *predictor);

    /// Measures the current state and records it as a SelfAssessment. Returns an invalid report
    /// if it could not be written - a self-assessment that was not remembered did not happen.
    SelfReport assess();

    /// The same measurement without recording it, for surfaces that poll.
    SelfReport measure() const;

    /// A sentence built only from measured values, for the Presence panel. Says less when it
    /// knows less rather than padding with generalities.
    QString narrate(const SelfReport &report) const;

    QString lastError() const { return m_lastError; }

private:
    /// Subjects predictord has actually been tested on, in first-seen order.
    QStringList testedSubjects() const;

    Journal *m_journal;
    Identity *m_identity;
    Intentions *m_intentions;
    Predictor *m_predictor;
    QString m_lastError;
};

} // namespace cybou
