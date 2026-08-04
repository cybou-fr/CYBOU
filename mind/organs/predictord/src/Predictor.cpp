// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT

#include "cybou/predictor/Predictor.h"

#include <QCborMap>
#include <QCborValue>

#include <algorithm>
#include <cmath>

namespace cybou {

namespace {

constexpr auto kOrgan = "predictord";

QCborMap payloadOf(const CognitiveEnvelope &e)
{
    return QCborValue::fromCbor(e.payloadCbor).toMap();
}

QString subjectOf(const QCborMap &p)
{
    return p[QStringLiteral("subject")].toString();
}

} // namespace

Predictor::Predictor(Journal *journal)
    : m_journal(journal)
{
}

QList<double> Predictor::history(const QString &subject) const
{
    QList<double> values;
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
        const QCborMap p = payloadOf(e);
        if (subjectOf(p) != subject || !p.contains(QStringLiteral("actual"))) {
            continue;
        }
        // Both count as lived experience: one was foreseen, one merely happened. For learning
        // what a build usually costs, that distinction does not matter.
        values.append(p[QStringLiteral("actual")].toDouble());
    }

    // recent() is newest first; history reads naturally oldest first.
    std::reverse(values.begin(), values.end());
    return values;
}

bool Predictor::observe(const QString &subject, double value)
{
    if (!m_journal || subject.isEmpty()) {
        m_lastError = QStringLiteral("an observation needs a journal and a subject");
        return false;
    }

    CognitiveEnvelope e;
    e.messageId = QUuid::createUuid();
    e.correlationId = e.messageId;
    e.originOrgan = QString::fromLatin1(kOrgan);
    // An Observation needs no cause - it is the one kind of contribution that may stand alone,
    // because it is the point where the world enters the journal.
    e.kind = ContributionKind::Observation;
    e.wallTime = QDateTime::currentDateTimeUtc();
    e.confidence = 1.0;
    e.privacy = PrivacyClass::Node;

    QCborMap payload;
    payload[QStringLiteral("subject")] = subject;
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
    Forecast f;
    f.subject = subject;

    if (!m_journal || subject.isEmpty()) {
        m_lastError = QStringLiteral("a forecast needs a journal and a subject");
        return f;
    }

    const QList<double> past = history(subject);
    f.samples = static_cast<int>(past.size());

    if (past.isEmpty()) {
        // Nothing to go on. The honest answer is silence, not a number with no basis - and
        // nothing is written to the journal, because a guess is not an observation.
        m_lastError = QStringLiteral("no history for '%1' yet").arg(subject);
        return f;
    }

    double sum = 0.0;
    for (double v : past) {
        sum += v;
    }
    f.estimate = sum / past.size();

    // Mean absolute deviation rather than standard deviation: it degrades gracefully at two
    // or three samples, which is where this organ will spend its early life.
    double spread = 0.0;
    for (double v : past) {
        spread += std::fabs(v - f.estimate);
    }
    f.margin = spread / past.size();

    // Confidence grows with evidence and never reaches certainty. Three samples give 0.5,
    // which is roughly the point where a number is worth showing at all.
    f.confidence = past.size() / static_cast<double>(past.size() + 3);

    CognitiveEnvelope e;
    e.messageId = QUuid::createUuid();
    e.correlationId = correlationId.isNull() ? e.messageId : correlationId;
    e.causationId = e.messageId; // a forecast is a root: it is formed, not derived
    e.originOrgan = QString::fromLatin1(kOrgan);
    e.kind = ContributionKind::Prediction;
    e.wallTime = QDateTime::currentDateTimeUtc();
    e.confidence = f.confidence;
    e.privacy = PrivacyClass::Node;

    QCborMap payload;
    payload[QStringLiteral("subject")] = subject;
    payload[QStringLiteral("estimate")] = f.estimate;
    payload[QStringLiteral("margin")] = f.margin;
    payload[QStringLiteral("samples")] = f.samples;
    e.payloadCbor = payload.toCborValue().toCbor();

    if (m_journal->append(e) == 0) {
        m_lastError = m_journal->lastError();
        return Forecast{{}, subject, 0.0, 0.0, 0.0, f.samples};
    }

    f.id = e.messageId;
    return f;
}

bool Predictor::settle(const QUuid &forecastId, double actual)
{
    if (!m_journal || forecastId.isNull()) {
        m_lastError = QStringLiteral("settling needs a journal and a forecast");
        return false;
    }

    // Find what was actually claimed. Re-reading it rather than trusting a caller-supplied
    // estimate is what keeps the error measurement honest.
    CognitiveEnvelope forecast;
    bool found = false;
    for (const auto &e : m_journal->recent(0)) {
        if (e.messageId == forecastId && e.kind == ContributionKind::Prediction) {
            forecast = e;
            found = true;
            break;
        }
    }
    if (!found) {
        m_lastError = QStringLiteral("no such forecast in the journal");
        return false;
    }

    const QCborMap claimed = payloadOf(forecast);
    const double estimate = claimed[QStringLiteral("estimate")].toDouble();

    CognitiveEnvelope e;
    e.messageId = QUuid::createUuid();
    e.correlationId = forecast.correlationId; // stays inside the forecast's episode
    e.causationId = forecastId;               // the join that makes error measurable
    e.originOrgan = QString::fromLatin1(kOrgan);
    e.kind = ContributionKind::Outcome;
    e.wallTime = QDateTime::currentDateTimeUtc();
    e.confidence = 1.0;
    e.privacy = PrivacyClass::Node;
    e.evidence = {forecastId};

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
    Calibration c;
    c.subject = subject;
    if (!m_journal) {
        return c;
    }

    double absolute = 0.0;
    double signedSum = 0.0;

    for (const auto &e : m_journal->recent(0)) {
        if (e.kind != ContributionKind::Outcome || e.originOrgan != QLatin1String(kOrgan)) {
            continue;
        }
        const QCborMap p = payloadOf(e);
        if (subjectOf(p) != subject || !p.contains(QStringLiteral("error"))) {
            continue;
        }
        // Sign convention: error = actual - estimate, so a positive mean bias means reality
        // ran longer than predicted, i.e. the system is optimistic.
        const double err = p[QStringLiteral("error")].toDouble();
        absolute += std::fabs(err);
        signedSum += err;
        ++c.settled;
    }

    if (c.settled > 0) {
        c.meanError = absolute / c.settled;
        c.bias = signedSum / c.settled;
    }
    return c;
}

} // namespace cybou
