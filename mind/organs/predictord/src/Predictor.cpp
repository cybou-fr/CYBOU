// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/predictor/Predictor.h"

#include <QCborMap>
#include <QCborValue>
#include <QSet>

#include <algorithm>
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

Predictor::Predictor(Journal *journal)
    : m_journal(journal)
{
}

QList<Predictor::PredictionSample> Predictor::history(const QString &subject) const
{
    QList<PredictionSample> values;
    if (!m_journal) {
        return values;
    }

    const auto all = m_journal->recent(0);
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

    if (!m_journal || subject.trimmed().isEmpty() || !std::isfinite(value)) {
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

    if (m_journal->append(e) == 0) {
        m_lastError = m_journal->lastError();
        return false;
    }
    return true;
}

Forecast Predictor::predict(const QString &subject, const QUuid &correlationId)
{
    m_lastError.clear();

    Forecast forecast;
    forecast.subject = subject.trimmed();

    if (!m_journal || forecast.subject.isEmpty()) {
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

    if (m_journal->append(e) == 0) {
        m_lastError = m_journal->lastError();
        return Forecast{{}, forecast.subject, 0.0, 0.0, 0.0, forecast.samples};
    }

    forecast.id = e.messageId;
    return forecast;
}

bool Predictor::settle(const QUuid &forecastId, double actual)
{
    m_lastError.clear();

    if (!m_journal || forecastId.isNull() || !std::isfinite(actual)) {
        m_lastError = QStringLiteral("settling needs a journal, a forecast, and an actual value");
        return false;
    }

    const auto forecast = m_journal->contribution(forecastId);
    if (!forecast || forecast->kind != ContributionKind::Prediction
        || forecast->originOrgan != QLatin1String(kOrgan)) {
        m_lastError = QStringLiteral("no such forecast in the journal");
        return false;
    }
    if (m_journal->hasOutcomeFor(forecastId, QString::fromLatin1(kOrgan))) {
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

    if (m_journal->append(e) == 0) {
        m_lastError = m_journal->lastError();
        return false;
    }
    return true;
}

Calibration Predictor::calibration(const QString &subject) const
{
    Calibration calibration;
    calibration.subject = subject;
    if (!m_journal) {
        return calibration;
    }

    double absolute = 0.0;
    double signedSum = 0.0;

    for (const auto &e : m_journal->recent(0)) {
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
    if (!m_journal) {
        return result;
    }

    QSet<QString> subjects;
    for (const auto &e : m_journal->recent(0)) {
        if (e.kind == ContributionKind::Outcome && e.originOrgan == QLatin1String(kOrgan)) {
            const QString subject = subjectOf(payloadOf(e));
            if (!subject.isEmpty()) {
                subjects.insert(subject);
            }
        }
    }

    for (const QString &subject : subjects) {
        result.append(calibration(subject));
    }
    return result;
}

} // namespace cybou
