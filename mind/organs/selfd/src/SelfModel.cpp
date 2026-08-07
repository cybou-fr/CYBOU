// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/self/SelfModel.h"

#include <QCborArray>
#include <QCborMap>
#include <QCborValue>

#include <cmath>
#include <utility>

namespace cybou {

SelfModel::SelfModel(
    Journal *journal, Identity *identity, Intentions *intentions, Predictor *predictor)
    : m_journal(journal)
    , m_identity(identity)
    , m_intentions(intentions)
    , m_predictor(predictor)
{
}

QStringList SelfModel::testedSubjects() const
{
    QStringList subjects;
    if (!m_journal) {
        return subjects;
    }

    const auto all = m_journal->recent(0);
    for (auto it = all.crbegin(); it != all.crend(); ++it) {
        if (it->kind != ContributionKind::Outcome
            || it->originOrgan != QLatin1String("predictord")) {
            continue;
        }

        const QCborMap payload = QCborValue::fromCbor(it->payloadCbor).toMap();
        if (!payload.contains(QStringLiteral("error"))) {
            continue;
        }

        const QString subject = payload[QStringLiteral("subject")].toString();
        if (!subject.isEmpty() && !subjects.contains(subject)) {
            subjects.append(subject);
        }
    }
    return subjects;
}

SelfReport SelfModel::measure() const
{
    SelfReport report;
    if (!m_journal || !m_identity || !m_intentions || !m_predictor) {
        return report;
    }

    report.taken = QDateTime::currentDateTimeUtc();

    const IdentityState identity = m_identity->state();
    report.ageInDays = identity.ageInDays();
    report.sessions = identity.sessionCount;
    report.architectureVersion = identity.architectureVersion;

    const QList<Intention> openIntentions = m_intentions->open();
    report.openIntentions = static_cast<int>(openIntentions.size());
    if (!openIntentions.isEmpty()) {
        report.oldestObligationDays = openIntentions.first().formed.daysTo(report.taken);
    }

    for (const QString &subject : testedSubjects()) {
        const Calibration calibration = m_predictor->calibration(subject);
        if (calibration.settled > 0) {
            report.calibrations.append(calibration);
            report.settledPredictions += calibration.settled;
        }
    }

    report.contributions = m_journal->count();
    report.firstBrokenAt = m_journal->verify();
    report.journalIntact = report.firstBrokenAt == 0;
    return report;
}

SelfReport SelfModel::assess(const QUuid &causeId)
{
    m_lastError.clear();

    SelfReport report = measure();
    if (!report.isValid()) {
        m_lastError = QStringLiteral("the self-model is missing an organ it depends on");
        return report;
    }

    const auto cause = m_journal->contribution(causeId);
    if (!cause) {
        m_lastError = QStringLiteral("the self-assessment cause does not exist");
        return SelfReport{};
    }

    CognitiveEnvelope e;
    e.messageId = QUuid::createUuid();
    e.correlationId = cause->correlationId;
    e.causationId = causeId;
    e.originOrgan = QStringLiteral("selfd");
    e.originNode = cause->originNode;
    e.kind = ContributionKind::SelfAssessment;
    e.wallTime = report.taken;
    e.confidence = 1.0;
    e.privacy = cause->privacy;

    QCborMap payload;
    payload[QStringLiteral("ageInDays")] = static_cast<qint64>(report.ageInDays);
    payload[QStringLiteral("sessions")] = static_cast<qint64>(report.sessions);
    payload[QStringLiteral("architectureVersion")] = report.architectureVersion;
    payload[QStringLiteral("openIntentions")] = report.openIntentions;
    payload[QStringLiteral("oldestObligationDays")] =
        static_cast<qint64>(report.oldestObligationDays);
    payload[QStringLiteral("settledPredictions")] = report.settledPredictions;
    payload[QStringLiteral("contributions")] = static_cast<qint64>(report.contributions);
    payload[QStringLiteral("journalIntact")] = report.journalIntact;

    QCborArray accuracy;
    for (const Calibration &calibration : std::as_const(report.calibrations)) {
        QCborMap entry;
        entry[QStringLiteral("subject")] = calibration.subject;
        entry[QStringLiteral("settled")] = calibration.settled;
        entry[QStringLiteral("meanError")] = calibration.meanError;
        entry[QStringLiteral("bias")] = calibration.bias;
        accuracy.append(entry);
    }
    payload[QStringLiteral("accuracy")] = accuracy;
    e.payloadCbor = payload.toCborValue().toCbor();

    if (m_journal->append(e) == 0) {
        m_lastError = m_journal->lastError();
        return SelfReport{};
    }
    return report;
}

QString SelfModel::narrate(const SelfReport &report) const
{
    if (!report.isValid()) {
        return QStringLiteral("I cannot see myself clearly enough to say.");
    }

    QStringList lines;

    if (report.ageInDays <= 0) {
        lines << QObject::tr("This is my first day.") + QLatin1Char(' ')
                     + QObject::tr("This is session %1.").arg(report.sessions);
    } else {
        lines << QObject::tr("I am %n day(s) old.", nullptr, static_cast<int>(report.ageInDays))
                     + QLatin1Char(' ')
              + QObject::tr("This is session %1.").arg(report.sessions);
    }

    if (report.openIntentions == 0) {
        lines << QObject::tr("I owe you nothing right now.");
    } else if (report.oldestObligationDays >= 1) {
        lines << QObject::tr(
                     "I owe you %n thing(s); the oldest has been waiting %1 day(s).",
                     nullptr,
                     report.openIntentions)
                     .arg(report.oldestObligationDays);
    } else {
        lines << QObject::tr("I owe you %n thing(s).", nullptr, report.openIntentions);
    }

    if (report.settledPredictions == 0) {
        lines << QObject::tr("I have not yet been tested against anything I predicted.");
    } else {
        double worstBias = 0.0;
        QString worstSubject;
        for (const Calibration &calibration : std::as_const(report.calibrations)) {
            if (std::fabs(calibration.bias) > std::fabs(worstBias)) {
                worstBias = calibration.bias;
                worstSubject = calibration.subject;
            }
        }

        lines << QObject::tr(
                     "I have checked myself against reality %n time(s).",
                     nullptr,
                     report.settledPredictions);
        if (!worstSubject.isEmpty() && std::fabs(worstBias) > 0.0) {
            lines << (worstBias > 0.0
                          ? QObject::tr("On %1 I tend to be optimistic.").arg(worstSubject)
                          : QObject::tr("On %1 I tend to overestimate.").arg(worstSubject));
        }
    }

    if (!report.journalIntact) {
        lines << QObject::tr("My memory is damaged from record %1 onward.")
                     .arg(report.firstBrokenAt);
    }

    return lines.join(QLatin1Char('\n'));
}

} // namespace cybou
