// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "SelfService.h"

#include "cybou/fabric/FabricCodec.h"

#include <QCborArray>
#include <QCborMap>

namespace cybou {

namespace {

QDateTime variantDateTime(const QVariant &value)
{
    if (value.canConvert<QDateTime>()) {
        const QDateTime dateTime = value.toDateTime();
        if (dateTime.isValid()) {
            return dateTime;
        }
    }
    return QDateTime::fromString(
        value.toString(),
        Qt::ISODateWithMs);
}

Calibration calibrationFromMap(const QVariantMap &map)
{
    Calibration calibration;
    calibration.subject =
        map.value(QStringLiteral("subject")).toString();
    calibration.settled =
        map.value(QStringLiteral("settled")).toInt();
    calibration.meanError =
        map.value(QStringLiteral("meanError")).toDouble();
    calibration.bias =
        map.value(QStringLiteral("bias")).toDouble();
    return calibration;
}

} // namespace

SelfService::SelfService(QObject *parent)
    : QObject(parent)
{
}

bool SelfService::Ready() const
{
    return m_events.isOpen()
        && m_identity.ready()
        && m_intentions.ready()
        && m_predictor.ready();
}

QString SelfService::Health() const
{
    return Ready()
        ? QStringLiteral("healthy")
        : QStringLiteral("degraded");
}

QString SelfService::LastError() const
{
    if (!m_lastError.isEmpty()) {
        return m_lastError;
    }

    if (!m_events.lastError().isEmpty()) {
        return m_events.lastError();
    }
    if (!m_identity.lastError().isEmpty()) {
        return m_identity.lastError();
    }
    if (!m_intentions.lastError().isEmpty()) {
        return m_intentions.lastError();
    }
    return m_predictor.lastError();
}

SelfReport SelfService::measureReport() const
{
    m_lastError.clear();

    SelfReport report;
    if (!Ready()) {
        m_lastError =
            QStringLiteral("selfd dependency is unavailable");
        return report;
    }

    const QVariantMap identity = m_identity.state();
    const QDateTime origin =
        variantDateTime(identity.value(QStringLiteral("origin")));

    if (!origin.isValid()) {
        m_lastError =
            QStringLiteral("identityd returned no valid origin");
        return report;
    }

    report.taken = QDateTime::currentDateTimeUtc();
    report.ageInDays = origin.daysTo(report.taken);
    report.sessions =
        identity.value(QStringLiteral("sessionCount")).toULongLong();
    report.architectureVersion =
        identity.value(QStringLiteral("architectureVersion")).toString();

    bool obligationsRead = false;
    const QVariantList intentions = m_intentions.open(-1, &obligationsRead);
    report.obligationsKnown = obligationsRead;
    report.openIntentions = intentions.size();
    if (!intentions.isEmpty()) {
        const QDateTime formed =
            variantDateTime(
                intentions.first()
                    .toMap()
                    .value(QStringLiteral("formed")));
        if (formed.isValid()) {
            report.oldestObligationDays =
                formed.daysTo(report.taken);
        }
    }

    for (const QVariant &entry : m_predictor.calibrations()) {
        const Calibration calibration =
            calibrationFromMap(entry.toMap());
        if (calibration.settled > 0) {
            report.calibrations.append(calibration);
            report.settledPredictions += calibration.settled;
        }
    }

    report.contributions = m_events.count();

    // Incremental where a checkpoint exists. Full verification is reachable from this ordinary
    // path, and at ~10.9 us per contribution it exhausts the five second Presence budget near 460k
    // contributions - so Reflect would stop being possible on a long-lived biography.
    const VerificationResult verification = m_events.verifyIncremental();
    report.verification = verification.status;
    report.verifiedFrom = verification.verifiedFrom;
    report.journalIntact = verification.intact();
    report.firstBrokenAt = verification.brokenAt;
    return report;
}

QVariantMap SelfService::reportMap(
    const SelfReport &report) const
{
    if (!report.isValid()) {
        return {};
    }

    QVariantList calibrations;
    for (const Calibration &calibration : report.calibrations) {
        QVariantMap map;
        map[QStringLiteral("subject")] = calibration.subject;
        map[QStringLiteral("settled")] = calibration.settled;
        map[QStringLiteral("meanError")] = calibration.meanError;
        map[QStringLiteral("bias")] = calibration.bias;
        calibrations.append(map);
    }

    QVariantMap map;
    map[QStringLiteral("taken")] = report.taken;
    map[QStringLiteral("ageInDays")] = report.ageInDays;
    map[QStringLiteral("sessions")] =
        static_cast<qulonglong>(report.sessions);
    map[QStringLiteral("architectureVersion")] =
        report.architectureVersion;
    map[QStringLiteral("openIntentions")] =
        report.openIntentions;
    // Carried across the wire so a reader can tell a Mind with no obligations from one that could
    // not find out. Without it the number is unfalsifiable.
    map[QStringLiteral("obligationsKnown")] =
        report.obligationsKnown;
    map[QStringLiteral("oldestObligationDays")] =
        report.oldestObligationDays;
    map[QStringLiteral("calibrations")] = calibrations;
    map[QStringLiteral("settledPredictions")] =
        report.settledPredictions;
    map[QStringLiteral("contributions")] =
        static_cast<qulonglong>(report.contributions);
    map[QStringLiteral("journalIntact")] =
        report.journalIntact;
    map[QStringLiteral("firstBrokenAt")] =
        static_cast<qulonglong>(report.firstBrokenAt);
    map[QStringLiteral("verification")] =
        verificationStatusToString(report.verification);
    map[QStringLiteral("verifiedFrom")] =
        static_cast<qulonglong>(report.verifiedFrom);
    map[QStringLiteral("narration")] =
        narrateSelfReport(report);
    return map;
}

QByteArray SelfService::Measure() const
{
    return FabricCodec::encodeMap(
        reportMap(measureReport()));
}

QByteArray SelfService::Assess(const QString &causeId)
{
    m_lastError.clear();

    SelfReport report = measureReport();
    if (!report.isValid()) {
        return FabricCodec::encodeMap({});
    }

    const QUuid causeUuid = QUuid::fromString(causeId);
    const auto cause = m_events.contribution(causeUuid);
    if (!cause) {
        m_lastError =
            QStringLiteral("the self-assessment cause does not exist");
        return FabricCodec::encodeMap({});
    }

    CognitiveEnvelope envelope;
    envelope.messageId = QUuid::createUuid();
    envelope.correlationId = cause->correlationId;
    envelope.causationId = causeUuid;
    envelope.originOrgan = QStringLiteral("selfd");
    envelope.originNode = cause->originNode;
    envelope.kind = ContributionKind::SelfAssessment;
    envelope.wallTime = report.taken;
    envelope.confidence = 1.0;
    envelope.privacy = cause->privacy;

    QCborMap payload;
    payload[QStringLiteral("ageInDays")] =
        static_cast<qint64>(report.ageInDays);
    payload[QStringLiteral("sessions")] =
        static_cast<qint64>(report.sessions);
    payload[QStringLiteral("architectureVersion")] =
        report.architectureVersion;
    payload[QStringLiteral("openIntentions")] =
        report.openIntentions;
    payload[QStringLiteral("oldestObligationDays")] =
        static_cast<qint64>(report.oldestObligationDays);
    payload[QStringLiteral("settledPredictions")] =
        report.settledPredictions;
    payload[QStringLiteral("contributions")] =
        static_cast<qint64>(report.contributions);
    payload[QStringLiteral("journalIntact")] =
        report.journalIntact;

    QCborArray accuracy;
    for (const Calibration &calibration : report.calibrations) {
        QCborMap entry;
        entry[QStringLiteral("subject")] = calibration.subject;
        entry[QStringLiteral("settled")] = calibration.settled;
        entry[QStringLiteral("meanError")] = calibration.meanError;
        entry[QStringLiteral("bias")] = calibration.bias;
        accuracy.append(entry);
    }
    payload[QStringLiteral("accuracy")] = accuracy;
    envelope.payloadCbor = payload.toCborValue().toCbor();

    if (m_events.append(envelope) == 0) {
        m_lastError = m_events.lastError();
        return FabricCodec::encodeMap({});
    }

    return FabricCodec::encodeMap(reportMap(report));
}

QString SelfService::Narration() const
{
    return narrateSelfReport(measureReport());
}

} // namespace cybou
