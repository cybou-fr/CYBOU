// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/identity/Identity.h"
#include "cybou/intentions/Intentions.h"
#include "cybou/predictor/Predictor.h"
#include "cybou/events/EventStore.h"

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

    bool isValid() const { return taken.isValid(); }
};

class SelfModel
{
public:
    SelfModel(EventStore *journal, Identity *identity, Intentions *intentions, Predictor *predictor);

    /// Records a self-assessment caused by an existing contribution.
    SelfReport assess(const QUuid &causeId);

    SelfReport measure() const;
    QString narrate(const SelfReport &report) const;

    QString lastError() const { return m_lastError; }

private:
    QStringList testedSubjects() const;

    EventStore *m_events;
    Identity *m_identity;
    Intentions *m_intentions;
    Predictor *m_predictor;
    QString m_lastError;
};

} // namespace cybou
