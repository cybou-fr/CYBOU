// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/events/EventStore.h"

namespace cybou {

struct Forecast {
    QUuid id;
    QString subject;
    double estimate{0.0};
    double margin{0.0};
    double confidence{0.0};
    int samples{0};
};

struct Calibration {
    QString subject;
    int settled{0};
    double meanError{0.0};
    double bias{0.0};
};

class Predictor
{
public:
    explicit Predictor(EventStore *journal);

    bool observe(const QString &subject, double value);
    Forecast predict(const QString &subject, const QUuid &correlationId = QUuid());
    bool settle(const QUuid &forecastId, double actual);

    Calibration calibration(const QString &subject) const;
    QList<Calibration> allCalibrations() const;

    QString lastError() const { return m_lastError; }

private:
    struct PredictionSample {
        QUuid contributionId;
        double value{0.0};
        PrivacyClass privacy{PrivacyClass::Local};
    };

    QList<PredictionSample> history(const QString &subject) const;

    EventStore *m_events;
    QString m_lastError;
};

} // namespace cybou
