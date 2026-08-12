// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/predictor/Predictor.h"

#include <QCborMap>
#include <QCborValue>
#include <QSet>

#include <algorithm>
#include <QMap>

#include <cmath>

namespace cybou {

namespace {

constexpr auto kOrgan = "predictord";

QCborMap payloadOf(const CognitiveEnvelope &e)
{
    return QCborValue::fromCbor(e.payloadCbor).toMap();
}

QString subjectOf(const QCborMap &payload)
{
    return payload[QStringLiteral("subject")].toString();
}

} // namespace

Predictor::Predictor(EventStore *journal)
    : m_events(journal)
{
}

QList<Predictor::PredictionSample> Predictor::history(const QString &subject) const
{
    QList<PredictionSample> values;
    if (!m_events) {
        return values;
    }

    const auto all = m_events->recent(0);
    for (const auto &e : all) {
        const bool measured = e.kind == ContributionKind::Outcome
                              || e.kind == ContributionKind::Observation;
        if (!measured || e.originOrgan != QLatin1String(kOrgan)) {
            continue;
        }

        const QCborMap payload = payloadOf(e);
        if (subjectOf(payload) != subject || !payload.contains(QStringLiteral("actual"))) {
            continue;
        }

        values.append(PredictionSample{
            e.messageId,
            payload[QStringLiteral("actual")].toDouble(),
            e.privacy,
        });
    }

    std::reverse(values.begin(), values.end());
    return values;
}

bool Predictor::observe(const QString &subject, double value)
{
    m_lastError.clear();

    if (!m_events || subject.trimmed().isEmpty() || !std::isfinite(value)) {
        m_lastError = QStringLiteral("an observation needs a journal, a subject, and a value");
        return false;
    }

    CognitiveEnvelope e;
    e.messageId = QUuid::createUuid();
    e.correlationId = e.messageId;
    e.originOrgan = QString::fromLatin1(kOrgan);
    e.kind = ContributionKind::Observation;
    e.wallTime = QDateTime::currentDateTimeUtc();
    e.confidence = 1.0;
    e.privacy = PrivacyClass::Node;

    QCborMap payload;
    payload[QStringLiteral("subject")] = subject.trimmed();
    payload[QStringLiteral("actual")] = value;
    e.payloadCbor = payload.toCborValue().toCbor();

    if (m_events->append(e) == 0) {
        m_lastError = m_events->lastError();
        return false;
    }
    return true;
}

Forecast Predictor::predict(const QString &subject, const QUuid &correlationId)
{
    m_lastError.clear();

    Forecast forecast;
    forecast.subject = subject.trimmed();

    if (!m_events || forecast.subject.isEmpty()) {
        m_lastError = QStringLiteral("a forecast needs a journal and a subject");
        return forecast;
    }

    const QList<PredictionSample> past = history(forecast.subject);
    forecast.samples = static_cast<int>(past.size());

    if (past.isEmpty()) {
        m_lastError = QStringLiteral("no history for '%1' yet").arg(forecast.subject);
        return forecast;
    }

    double sum = 0.0;
    for (const PredictionSample &sample : past) {
        sum += sample.value;
    }
    forecast.estimate = sum / past.size();

    double spread = 0.0;
    for (const PredictionSample &sample : past) {
        spread += std::fabs(sample.value - forecast.estimate);
    }
    forecast.margin = spread / past.size();
    forecast.confidence = past.size() / static_cast<double>(past.size() + 3);

    CognitiveEnvelope e;
    e.messageId = QUuid::createUuid();
    e.correlationId = correlationId.isNull() ? e.messageId : correlationId;
    e.originOrgan = QString::fromLatin1(kOrgan);
    e.kind = ContributionKind::Prediction;
    e.wallTime = QDateTime::currentDateTimeUtc();
    e.confidence = forecast.confidence;
    e.privacy = PrivacyClass::Public;

    for (const PredictionSample &sample : past) {
        e.evidence.append(sample.contributionId);
        e.privacy = mostRestrictive(e.privacy, sample.privacy);
    }

    QCborMap payload;
    payload[QStringLiteral("subject")] = forecast.subject;
    payload[QStringLiteral("estimate")] = forecast.estimate;
    payload[QStringLiteral("margin")] = forecast.margin;
    payload[QStringLiteral("samples")] = forecast.samples;
    e.payloadCbor = payload.toCborValue().toCbor();

    if (m_events->append(e) == 0) {
        m_lastError = m_events->lastError();
        return Forecast{{}, forecast.subject, 0.0, 0.0, 0.0, forecast.samples};
    }

    forecast.id = e.messageId;
    return forecast;
}

bool Predictor::settle(const QUuid &forecastId, double actual)
{
    m_lastError.clear();

    if (!m_events || forecastId.isNull() || !std::isfinite(actual)) {
        m_lastError = QStringLiteral("settling needs a journal, a forecast, and an actual value");
        return false;
    }

    const auto forecast = m_events->contribution(forecastId);
    if (!forecast || forecast->kind != ContributionKind::Prediction
        || forecast->originOrgan != QLatin1String(kOrgan)) {
        m_lastError = QStringLiteral("no such forecast in the journal");
        return false;
    }
    if (m_events->hasOutcomeFor(forecastId, QString::fromLatin1(kOrgan))) {
        m_lastError = QStringLiteral("the forecast is already settled");
        return false;
    }

    const QCborMap claimed = payloadOf(*forecast);
    const double estimate = claimed[QStringLiteral("estimate")].toDouble();

    CognitiveEnvelope e;
    e.messageId = QUuid::createUuid();
    e.correlationId = forecast->correlationId;
    e.causationId = forecastId;
    e.originOrgan = QString::fromLatin1(kOrgan);
    e.originNode = forecast->originNode;
    e.kind = ContributionKind::Outcome;
    e.wallTime = QDateTime::currentDateTimeUtc();
    e.confidence = 1.0;
    e.privacy = forecast->privacy;

    QCborMap payload;
    payload[QStringLiteral("subject")] = subjectOf(claimed);
    payload[QStringLiteral("actual")] = actual;
    payload[QStringLiteral("estimate")] = estimate;
    payload[QStringLiteral("error")] = actual - estimate;
    e.payloadCbor = payload.toCborValue().toCbor();

    if (m_events->append(e) == 0) {
        m_lastError = m_events->lastError();
        return false;
    }
    return true;
}

Calibration Predictor::calibration(const QString &subject) const
{
    Calibration calibration;
    calibration.subject = subject;
    if (!m_events) {
        return calibration;
    }

    double absolute = 0.0;
    double signedSum = 0.0;

    for (const auto &e : m_events->recent(0)) {
        if (e.kind != ContributionKind::Outcome || e.originOrgan != QLatin1String(kOrgan)) {
            continue;
        }
        const QCborMap payload = payloadOf(e);
        if (subjectOf(payload) != subject || !payload.contains(QStringLiteral("error"))) {
            continue;
        }

        const double error = payload[QStringLiteral("error")].toDouble();
        absolute += std::fabs(error);
        signedSum += error;
        ++calibration.settled;
    }

    if (calibration.settled > 0) {
        calibration.meanError = absolute / calibration.settled;
        calibration.bias = signedSum / calibration.settled;
    }
    return calibration;
}

QList<Calibration> Predictor::allCalibrations() const
{
    QList<Calibration> result;
    if (!m_events) {
        return result;
    }

    // One pass over the biography, accumulating every subject at once.
    //
    // This used to replay the history to collect the subjects and then call calibration() for each
    // of them, which replays it again - so the cost was the length of the biography multiplied by
    // the number of subjects. selfd reaches this through Reflect under a five second budget, and
    // the multiplication is what made that budget a function of two growing quantities rather than
    // one. The per-subject arithmetic never needed more than a single read.
    struct Accumulator {
        int settled{0};
        double absolute{0.0};
        double signedSum{0.0};
    };
    QMap<QString, Accumulator> bySubject;

    for (const auto &e : m_events->recent(0)) {
        if (e.kind != ContributionKind::Outcome || e.originOrgan != QLatin1String(kOrgan)) {
            continue;
        }
        const QCborMap payload = payloadOf(e);
        const QString subject = subjectOf(payload);
        if (subject.isEmpty() || !payload.contains(QStringLiteral("error"))) {
            continue;
        }

        const double error = payload[QStringLiteral("error")].toDouble();
        Accumulator &accumulator = bySubject[subject];
        accumulator.absolute += std::fabs(error);
        accumulator.signedSum += error;
        ++accumulator.settled;
    }

    for (auto it = bySubject.cbegin(); it != bySubject.cend(); ++it) {
        Calibration calibration;
        calibration.subject = it.key();
        calibration.settled = it.value().settled;
        if (calibration.settled > 0) {
            calibration.meanError = it.value().absolute / calibration.settled;
            calibration.bias = it.value().signedSum / calibration.settled;
        }
        result.append(calibration);
    }
    return result;
}

} // namespace cybou
