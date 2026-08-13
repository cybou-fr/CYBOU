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

Predictor::Predictor(EventStore *journal)
    : m_events(journal)
{
}

bool Predictor::catchUp() const
{
    if (!m_events) {
        return false;
    }

    // One page at a time, so catching up after a long absence never holds the whole biography in
    // memory at once. The common case is a page that comes back empty.
    constexpr int kPageSize = 1000;
    for (;;) {
        const ContributionPage page = m_events->after(m_cursor, kPageSize);
        if (!page.ok) {
            m_lastError = QStringLiteral("could not read contributions after %1").arg(m_cursor);
            return false;
        }
        if (page.envelopes.isEmpty()) {
            return true;
        }

        for (const CognitiveEnvelope &e : page.envelopes) {
            if (e.originOrgan != QLatin1String(kOrgan)) {
                continue;
            }
            const QCborMap payload = payloadOf(e);
            const QString subject = subjectOf(payload);
            if (subject.isEmpty()) {
                continue;
            }

            // A settled Outcome carries both: the value that was actually seen, which is a sample
            // like any other, and the error against what was forecast, which is not.
            const bool measured = e.kind == ContributionKind::Outcome
                                  || e.kind == ContributionKind::Observation;
            if (measured && payload.contains(QStringLiteral("actual"))) {
                m_bySubject[subject].samples.append(PredictionSample{
                    e.messageId,
                    payload[QStringLiteral("actual")].toDouble(),
                    e.privacy,
                });
            }

            if (e.kind == ContributionKind::Outcome
                && payload.contains(QStringLiteral("error"))) {
                const double error = payload[QStringLiteral("error")].toDouble();
                SubjectState &state = m_bySubject[subject];
                state.absoluteError += std::fabs(error);
                state.signedError += error;
                ++state.settled;
            }
        }

        if (page.lastSequence <= m_cursor) {
            m_lastError = QStringLiteral("the journal did not advance past %1").arg(m_cursor);
            return false;
        }
        m_cursor = page.lastSequence;

        if (!page.hasMore) {
            return true;
        }
    }
}

QList<Predictor::PredictionSample> Predictor::history(const QString &subject) const
{
    if (!catchUp()) {
        return {};
    }
    // Accumulated oldest first, which is the order `after` yields and the order predictions are
    // built in. The `recent(0)` version ended with a reverse because that call yields newest first.
    return m_bySubject.value(subject).samples;
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
    if (!m_events || !catchUp()) {
        return calibration;
    }

    const SubjectState state = m_bySubject.value(subject);
    calibration.settled = state.settled;
    if (calibration.settled > 0) {
        calibration.meanError = state.absoluteError / calibration.settled;
        calibration.bias = state.signedError / calibration.settled;
    }
    return calibration;
}

QList<Calibration> Predictor::allCalibrations() const
{
    QList<Calibration> result;
    if (!m_events || !catchUp()) {
        return result;
    }

    // Every subject at once, from state that was accumulated as the contributions arrived.
    //
    // This used to replay the biography to collect the subjects and then replay it again for each
    // of them, so the cost was the length of a life multiplied by the number of subjects. Reducing
    // that to a single pass fixed the multiplication; keeping the pass incremental removes the
    // length of the life as well. selfd reaches this through Reflect under a five second budget.
    for (auto it = m_bySubject.cbegin(); it != m_bySubject.cend(); ++it) {
        if (it.value().settled == 0) {
            continue;
        }
        Calibration calibration;
        calibration.subject = it.key();
        calibration.settled = it.value().settled;
        calibration.meanError = it.value().absoluteError / calibration.settled;
        calibration.bias = it.value().signedError / calibration.settled;
        result.append(calibration);
    }

    // Subject order was incidental to a QMap before and is incidental to a hash now, but a stable
    // answer is worth more than a fast one here: callers compare successive readings.
    std::sort(result.begin(), result.end(), [](const Calibration &a, const Calibration &b) {
        return a.subject < b.subject;
    });
    return result;
}

} // namespace cybou
