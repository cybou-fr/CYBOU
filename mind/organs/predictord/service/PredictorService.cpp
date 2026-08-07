// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "PredictorService.h"

#include "cybou/fabric/FabricCodec.h"

namespace cybou {

namespace {

QVariantMap forecastMap(const Forecast &forecast)
{
    if (forecast.id.isNull()) {
        return {};
    }

    QVariantMap map;
    map[QStringLiteral("id")] =
        forecast.id.toString(QUuid::WithoutBraces);
    map[QStringLiteral("subject")] =
        forecast.subject;
    map[QStringLiteral("estimate")] =
        forecast.estimate;
    map[QStringLiteral("margin")] =
        forecast.margin;
    map[QStringLiteral("confidence")] =
        forecast.confidence;
    map[QStringLiteral("samples")] =
        forecast.samples;
    return map;
}

QVariantMap calibrationMap(const Calibration &calibration)
{
    QVariantMap map;
    map[QStringLiteral("subject")] =
        calibration.subject;
    map[QStringLiteral("settled")] =
        calibration.settled;
    map[QStringLiteral("meanError")] =
        calibration.meanError;
    map[QStringLiteral("bias")] =
        calibration.bias;
    return map;
}

} // namespace

PredictorService::PredictorService(
    EventStore *events,
    QObject *parent)
    : QObject(parent)
    , m_events(events)
    , m_predictor(events)
{
}

bool PredictorService::Ready() const
{
    return m_events && m_events->isOpen();
}

QString PredictorService::Health() const
{
    return Ready()
        ? QStringLiteral("healthy")
        : QStringLiteral("unavailable");
}

QString PredictorService::LastError() const
{
    if (!m_predictor.lastError().isEmpty()) {
        return m_predictor.lastError();
    }
    return m_events ? m_events->lastError() : QString();
}

bool PredictorService::Observe(
    const QString &subject,
    double value)
{
    return m_predictor.observe(subject, value);
}

QByteArray PredictorService::Predict(
    const QString &subject,
    const QString &correlationId)
{
    const Forecast forecast = m_predictor.predict(
        subject,
        QUuid::fromString(correlationId));

    return FabricCodec::encodeMap(
        forecastMap(forecast));
}

bool PredictorService::Settle(
    const QString &forecastId,
    double actual)
{
    return m_predictor.settle(
        QUuid::fromString(forecastId),
        actual);
}

QByteArray PredictorService::Calibrations() const
{
    QVariantList result;
    for (const Calibration &calibration :
         m_predictor.allCalibrations()) {
        result.append(calibrationMap(calibration));
    }
    return FabricCodec::encodeList(result);
}

} // namespace cybou
