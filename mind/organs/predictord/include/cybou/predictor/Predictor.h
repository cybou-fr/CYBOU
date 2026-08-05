// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT
//
// Forecasts, and the part that matters: joining each forecast to what actually happened.
//
// ADR-0003 makes this the measurable test of whether Presence is alive or merely animated.
// A system that predicts and never checks is decoration; one that predicts, checks, and lets
// the result change its own confidence is not.
//
// The alpha predicts from a rolling mean of past outcomes. That is deliberately the simplest
// estimator that can be wrong in a measurable way - the model can be replaced later without
// touching the protocol, because an organ publishes a Prediction the same way whether it
// arrived by arithmetic or by a network.

#pragma once

#include "cybou/storage/Journal.h"

namespace cybou {

struct Forecast {
    QUuid id;
    /// What is being predicted - "nixos-rebuild", "plasma-startup". Predictions are only
    /// comparable within a subject.
    QString subject;
    double estimate{0.0};
    /// Half-width of the interval, so the surface can say "11 to 17 minutes" rather than a
    /// single number pretending to precision it does not have.
    double margin{0.0};
    /// 0..1, and low when there is little history. An estimate from two samples should not
    /// look as sure as one from fifty.
    double confidence{0.0};
    /// How many past outcomes it was drawn from. Shown so the number can be judged.
    int samples{0};
};

/// What the system knows about its own accuracy on one subject.
struct Calibration {
    QString subject;
    int settled{0};
    /// Mean absolute error, in the units of the subject.
    double meanError{0.0};
    /// Mean signed error. Positive means the system predicts high - a bias it can state.
    double bias{0.0};
};

class Predictor
{
public:
    explicit Predictor(Journal *journal);

    /// Records a measured value with no forecast attached. This is how experience first enters
    /// the system: before anything can be predicted, something has to have been lived through.
    bool observe(const QString &subject, double value);

    /// Forms a forecast from history and records it. Returns a Forecast with confidence 0 and
    /// no id when there is no history at all - the system says "I do not know" rather than
    /// inventing a number.
    Forecast predict(const QString &subject, const QUuid &correlationId = QUuid());

    /// Records what actually happened. The Outcome names the prediction as its cause, which
    /// is the join that makes error measurable.
    bool settle(const QUuid &forecastId, double actual);

    /// Measured accuracy over settled predictions for a subject.
    Calibration calibration(const QString &subject) const;

    /// All calibrations for all subjects with settled predictions.
    QList<Calibration> allCalibrations() const;

    QString lastError() const { return m_lastError; }

private:
    /// Past actual values for a subject, oldest first.
    QList<double> history(const QString &subject) const;

    Journal *m_journal;
    QString m_lastError;
};

} // namespace cybou
