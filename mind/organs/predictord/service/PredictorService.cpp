// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "PredictorService.h"

#include "cybou/fabric/FabricCodec.h"

#include <QCborMap>
#include <QDateTime>

namespace cybou {

namespace {

const QUuid kConsolidationNamespace(
    QStringLiteral("8fcbaf7c-b31a-5c7d-b15e-a09b7b816ca7"));

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

QByteArray PredictorService::Consolidate(const QString &runId,const QString &operationKey,qulonglong mark) const
{
    if (!Ready() || QUuid(runId).isNull() || operationKey.trimmed().isEmpty()
        || mark == 0 || mark > m_events->count()) return {};
    const auto input = m_events->atSequence(mark);
    if (!input) return {};

    const QUuid contributionId = QUuid::createUuidV5(
        kConsolidationNamespace,
        QStringLiteral("predictor:%1").arg(operationKey).toUtf8());
    if (!m_events->contains(contributionId)) {
        CognitiveEnvelope contribution;
        contribution.messageId = contributionId;
        contribution.correlationId = QUuid(runId);
        contribution.causationId = input->messageId;
        contribution.originOrgan = QStringLiteral("predictord");
        contribution.originNode = QStringLiteral("local");
        contribution.kind = ContributionKind::Learning;
        contribution.wallTime = QDateTime::currentDateTimeUtc();
        contribution.privacy = input->privacy;
        contribution.capabilityScope = QStringLiteral("lifecycle.consolidation");
        QCborMap payload;
        payload[QStringLiteral("operationKey")] = operationKey;
        payload[QStringLiteral("inputHighWaterMark")] = static_cast<qint64>(mark);
        payload[QStringLiteral("calibrationCount")] = m_predictor.allCalibrations().size();
        contribution.payloadCbor = payload.toCborValue().toCbor();
        if (m_events->append(contribution) == 0) return {};
    }
    QVariantMap receipt;
    receipt[QStringLiteral("accepted")]=true;
    receipt[QStringLiteral("owner")]=QStringLiteral("predictor");
    receipt[QStringLiteral("operationKey")]=operationKey;
    receipt[QStringLiteral("inputHighWaterMark")]=mark;
    receipt[QStringLiteral("contributionId")]=contributionId.toString(QUuid::WithoutBraces);
    receipt[QStringLiteral("calibrationCount")]=m_predictor.allCalibrations().size();
    return FabricCodec::encodeMap(receipt);
}

} // namespace cybou
