// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/predictor/Predictor.h"

#include <QDBusContext>
#include <QObject>

namespace cybou {

class PredictorService
    : public QObject
    , protected QDBusContext
{
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.cybou.Mind.Predictor1")

public:
    explicit PredictorService(
        EventStore *events,
        QObject *parent = nullptr);

public Q_SLOTS:
    bool Ready() const;
    QString Health() const;
    QString LastError() const;

    bool Observe(const QString &subject, double value);

    QByteArray Predict(
        const QString &subject,
        const QString &correlationId);

    bool Settle(
        const QString &forecastId,
        double actual);

    /// Every subject's calibration, or a D-Bus error if it could not be assembled.
    ///
    /// An empty success would read as "nothing has been settled yet", which is exactly the answer a
    /// self-assessment would find reassuring and exactly the one it must not invent.
    QByteArray Calibrations();
    QByteArray Consolidate(const QString &runId, const QString &operationKey,
                           qulonglong inputHighWaterMark) const;

private:
    EventStore *m_events;
    Predictor m_predictor;
};

} // namespace cybou
