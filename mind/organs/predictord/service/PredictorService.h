// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/predictor/Predictor.h"

#include <QObject>

namespace cybou {

class PredictorService : public QObject
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

    QByteArray Calibrations() const;
    QByteArray Consolidate(const QString &runId, const QString &operationKey,
                           qulonglong inputHighWaterMark) const;

private:
    EventStore *m_events;
    Predictor m_predictor;
};

} // namespace cybou
