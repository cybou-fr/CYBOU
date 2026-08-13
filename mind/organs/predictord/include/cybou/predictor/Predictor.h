// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/events/EventStore.h"

#include <QHash>

#include <optional>

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

    /// How well this organ has predicted a subject, or nothing if that could not be assembled.
    ///
    /// A zeroed Calibration and a failed read used to be the same value, so a Journal that could
    /// not be read reported every subject as never settled and perfectly unbiased - flattering,
    /// and indistinguishable from the truth.
    std::optional<Calibration> calibration(const QString &subject) const;
    std::optional<QList<Calibration>> allCalibrations() const;

    QString lastError() const { return m_lastError; }

private:
    struct PredictionSample {
        QUuid contributionId;
        double value{0.0};
        PrivacyClass privacy{PrivacyClass::Local};
    };

    /// Everything this organ has derived about one subject, kept oldest first.
    struct SubjectState {
        QList<PredictionSample> samples;
        int settled{0};
        double absoluteError{0.0};
        double signedError{0.0};
    };

    QList<PredictionSample> history(const QString &subject) const;

    /// Take in everything accepted since the cursor.
    ///
    /// Every read used to scan the whole biography, so answering cost the length of a life rather
    /// than the length of what had changed since the last question - and selfd asks on the ordinary
    /// self-assessment path, under a budget. This makes a read cost the new contributions only.
    ///
    /// Failing here is failing closed. A projection built from part of the history is not a smaller
    /// answer, it is a wrong one: an unread Outcome makes a subject look better calibrated than it
    /// is, and nothing downstream could tell.
    bool catchUp() const;

    EventStore *m_events;
    mutable QString m_lastError;

    // Derived state, rebuilt from the Journal and never authoritative over it. Mutable because
    // answering a question may require reading what has been accepted since the last one, which is
    // not a change to what this organ believes - only to how much of the Journal it has read.
    mutable QHash<QString, SubjectState> m_bySubject;
    mutable quint64 m_cursor{0};
};

} // namespace cybou
