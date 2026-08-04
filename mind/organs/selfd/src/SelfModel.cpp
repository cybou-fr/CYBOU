// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT

#include "cybou/self/SelfModel.h"

#include <QCborArray>
#include <QCborMap>
#include <QCborValue>

#include <cmath>

namespace cybou {

SelfModel::SelfModel(Journal *journal, Identity *identity, Intentions *intentions,
                     Predictor *predictor)
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
    // recent() is newest first; walk backwards so first-seen order is chronological.
    for (auto it = all.crbegin(); it != all.crend(); ++it) {
        if (it->kind != ContributionKind::Outcome
            || it->originOrgan != QLatin1String("predictord")) {
            continue;
        }
        const QCborMap p = QCborValue::fromCbor(it->payloadCbor).toMap();
        if (!p.contains(QStringLiteral("error"))) {
            continue;
        }
        const QString subject = p[QStringLiteral("subject")].toString();
        if (!subject.isEmpty() && !subjects.contains(subject)) {
            subjects.append(subject);
        }
    }
    return subjects;
}

SelfReport SelfModel::measure() const
{
    SelfReport r;
    if (!m_journal || !m_identity || !m_intentions || !m_predictor) {
        return r; // invalid: taken is unset
    }

    r.taken = QDateTime::currentDateTimeUtc();

    const IdentityState id = m_identity->state();
    r.ageInDays = id.ageInDays();
    r.sessions = id.sessionCount;
    r.architectureVersion = id.architectureVersion;

    const QList<Intention> open = m_intentions->open();
    r.openIntentions = static_cast<int>(open.size());
    if (!open.isEmpty()) {
        // open() is oldest first, so the first entry is the longest-standing obligation.
        r.oldestObligationDays = open.first().formed.daysTo(r.taken);
    }

    for (const QString &subject : testedSubjects()) {
        const Calibration c = m_predictor->calibration(subject);
        if (c.settled > 0) {
            r.calibrations.append(c);
            r.settledPredictions += c.settled;
        }
    }

    r.contributions = m_journal->count();
    r.firstBrokenAt = m_journal->verify();
    r.journalIntact = r.firstBrokenAt == 0;

    return r;
}

SelfReport SelfModel::assess()
{
    SelfReport r = measure();
    if (!r.isValid()) {
        m_lastError = QStringLiteral("the self-model is missing an organ it depends on");
        return r;
    }

    CognitiveEnvelope e;
    e.messageId = QUuid::createUuid();
    e.correlationId = e.messageId;
    e.causationId = e.messageId; // looking at oneself is its own occasion
    e.originOrgan = QStringLiteral("selfd");
    e.kind = ContributionKind::SelfAssessment;
    e.wallTime = r.taken;
    e.confidence = 1.0;
    e.privacy = PrivacyClass::Node;

    QCborMap payload;
    payload[QStringLiteral("ageInDays")] = static_cast<qint64>(r.ageInDays);
    payload[QStringLiteral("sessions")] = static_cast<qint64>(r.sessions);
    payload[QStringLiteral("architectureVersion")] = r.architectureVersion;
    payload[QStringLiteral("openIntentions")] = r.openIntentions;
    payload[QStringLiteral("oldestObligationDays")] = static_cast<qint64>(r.oldestObligationDays);
    payload[QStringLiteral("settledPredictions")] = r.settledPredictions;
    payload[QStringLiteral("contributions")] = static_cast<qint64>(r.contributions);
    payload[QStringLiteral("journalIntact")] = r.journalIntact;

    QCborArray accuracy;
    for (const Calibration &c : std::as_const(r.calibrations)) {
        QCborMap entry;
        entry[QStringLiteral("subject")] = c.subject;
        entry[QStringLiteral("settled")] = c.settled;
        entry[QStringLiteral("meanError")] = c.meanError;
        entry[QStringLiteral("bias")] = c.bias;
        accuracy.append(entry);
    }
    payload[QStringLiteral("accuracy")] = accuracy;
    e.payloadCbor = payload.toCborValue().toCbor();

    if (m_journal->append(e) == 0) {
        m_lastError = m_journal->lastError();
        return SelfReport{};
    }
    return r;
}

QString SelfModel::narrate(const SelfReport &report) const
{
    if (!report.isValid()) {
        return QStringLiteral("I cannot see myself clearly enough to say.");
    }

    QStringList lines;

    // Continuity. Day zero is worth naming rather than rounding to "0 days old".
    if (report.ageInDays <= 0) {
        lines << QObject::tr("This is my first day.") + QLatin1Char(' ')
                     + QObject::tr("This is session %1.").arg(report.sessions);
    } else {
        lines << QObject::tr("I am %n day(s) old.", nullptr, static_cast<int>(report.ageInDays))
                     + QLatin1Char(' ')
              + QObject::tr("This is session %1.").arg(report.sessions);
    }

    // Obligation.
    if (report.openIntentions == 0) {
        lines << QObject::tr("I owe you nothing right now.");
    } else if (report.oldestObligationDays >= 1) {
        lines << QObject::tr("I owe you %n thing(s); the oldest has been waiting %1 day(s).",
                             nullptr, report.openIntentions)
                     .arg(report.oldestObligationDays);
    } else {
        lines << QObject::tr("I owe you %n thing(s).", nullptr, report.openIntentions);
    }

    // Accuracy. Silence until it has actually been tested - an untested system claiming
    // reliability is exactly the fake affordance ADR-0003 forbids.
    if (report.settledPredictions == 0) {
        lines << QObject::tr("I have not yet been tested against anything I predicted.");
    } else {
        double worstBias = 0.0;
        QString worstSubject;
        for (const Calibration &c : std::as_const(report.calibrations)) {
            if (std::fabs(c.bias) > std::fabs(worstBias)) {
                worstBias = c.bias;
                worstSubject = c.subject;
            }
        }
        lines << QObject::tr("I have checked myself against reality %n time(s).", nullptr,
                             report.settledPredictions);
        if (!worstSubject.isEmpty() && std::fabs(worstBias) > 0.0) {
            lines << (worstBias > 0.0
                          ? QObject::tr("On %1 I tend to be optimistic.").arg(worstSubject)
                          : QObject::tr("On %1 I tend to overestimate.").arg(worstSubject));
        }
    }

    // Integrity. Only mentioned when it is broken: a system announcing that it is not corrupt
    // is noise, but a system hiding that it is would be a lie.
    if (!report.journalIntact) {
        lines << QObject::tr("My memory is damaged from record %1 onward.").arg(report.firstBrokenAt);
    }

    return lines.join(QLatin1Char('\n'));
}

} // namespace cybou
