// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/presence/Presence.h"

#include <QCborMap>
#include <QCborValue>
#include <QDir>
#include <QStandardPaths>

namespace cybou {

Presence::Presence(const QString &dataDir, QObject *parent)
    : QObject(parent)
    , m_dataDir(dataDir)
{
}

Presence::Presence(QObject *parent)
    : Presence(QDir(QStandardPaths::writableLocation(QStandardPaths::AppDataLocation))
                   .filePath(QStringLiteral("cybou")),
               parent)
{
}

Presence::~Presence() = default;

bool Presence::wake()
{
    if (m_awake) {
        return true;
    }

    if (!QDir().mkpath(m_dataDir)) {
        m_lastError = QStringLiteral("cannot create %1").arg(m_dataDir);
        return false;
    }

    auto journal = std::make_unique<Journal>(QDir(m_dataDir).filePath(QStringLiteral("journal.db")));
    if (!journal->isOpen()) {
        m_lastError = journal->lastError();
        return false;
    }

    auto identity = std::make_unique<Identity>(
        QDir(m_dataDir).filePath(QStringLiteral("identity.json")), journal.get());
    if (!identity->beginSession()) {
        m_lastError = identity->lastError();
        return false;
    }

    m_journal = std::move(journal);
    m_identity = std::move(identity);
    m_intentions = std::make_unique<Intentions>(m_journal.get());
    m_predictor = std::make_unique<Predictor>(m_journal.get());
    m_self = std::make_unique<SelfModel>(
        m_journal.get(), m_identity.get(), m_intentions.get(), m_predictor.get());
    m_workspace = std::make_unique<Workspace>(m_journal.get());
    m_workspace->rehydrate();

    connect(m_workspace.get(), &Workspace::focusChanged, this, &Presence::changed);

    m_awake = true;
    Q_EMIT changed();
    return true;
}

QString Presence::narration() const
{
    return m_awake ? m_self->narrate(m_self->measure()) : QString();
}

QStringList Presence::obligations() const
{
    QStringList result;
    if (!m_awake) {
        return result;
    }

    const auto open = m_intentions->open();
    result.reserve(open.size());
    for (const Intention &intention : open) {
        result.append(intention.description);
    }
    return result;
}

QString Presence::attention() const
{
    if (!m_awake) {
        return {};
    }

    const Coalition focus = m_workspace->focus();
    if (!focus.isValid()) {
        return {};
    }

    const CognitiveEnvelope &latest = focus.members.last();
    const QStringList voices = focus.organs();
    if (voices.size() > 1) {
        return QObject::tr("%1, with %n organ(s) involved", nullptr, voices.size())
            .arg(kindToString(latest.kind));
    }
    return QObject::tr("%1, from %2").arg(kindToString(latest.kind), latest.originOrgan);
}

int Presence::contributions() const
{
    return m_awake ? static_cast<int>(m_journal->count()) : 0;
}

QList<Moment> Presence::recent(int limit) const
{
    QList<Moment> result;
    if (!m_awake || limit <= 0) {
        return result;
    }

    const auto envelopes = m_journal->recent(limit);
    result.reserve(envelopes.size());
    for (const auto &envelope : envelopes) {
        Moment moment;
        moment.when = envelope.wallTime;
        moment.organ = envelope.originOrgan;
        moment.kind = kindToString(envelope.kind);
        moment.thread = envelope.correlationId;
        result.append(moment);
    }
    return result;
}

QVariantList Presence::activity(int limit) const
{
    QVariantList result;
    for (const Moment &moment : recent(limit)) {
        QVariantMap entry;
        entry[QStringLiteral("when")] = moment.when.toLocalTime();
        entry[QStringLiteral("organ")] = moment.organ;
        entry[QStringLiteral("kind")] = moment.kind;
        entry[QStringLiteral("thread")] = moment.thread.toString(QUuid::WithoutBraces);
        result.append(entry);
    }
    return result;
}

bool Presence::appendUserObservation(
    const QString &event, const QCborMap &details, QUuid *messageId)
{
    if (!m_awake || !m_journal) {
        return false;
    }

    CognitiveEnvelope observation;
    observation.messageId = QUuid::createUuid();
    observation.correlationId = observation.messageId;
    observation.originOrgan = QStringLiteral("presenced");
    observation.kind = ContributionKind::Observation;
    observation.wallTime = QDateTime::currentDateTimeUtc();
    observation.confidence = 1.0;
    observation.privacy = PrivacyClass::Node;

    QCborMap payload = details;
    payload[QStringLiteral("event")] = event;
    observation.payloadCbor = payload.toCborValue().toCbor();

    if (m_journal->append(observation) == 0) {
        m_lastError = m_journal->lastError();
        return false;
    }

    if (messageId) {
        *messageId = observation.messageId;
    }
    return true;
}

QUuid Presence::promise(const QString &description)
{
    if (!m_awake || description.trimmed().isEmpty()) {
        return {};
    }

    QCborMap details;
    details[QStringLiteral("description")] = description.trimmed();

    QUuid requestId;
    if (!appendUserObservation(
            QStringLiteral("user-requested-intention"), details, &requestId)) {
        return {};
    }

    const QUuid intentionId = m_intentions->form(
        description,
        QStringLiteral("asked by the user"),
        requestId);
    if (intentionId.isNull()) {
        m_lastError = m_intentions->lastError();
        return {};
    }

    Q_EMIT changed();
    return intentionId;
}

bool Presence::reflect()
{
    if (!m_awake) {
        return false;
    }

    QUuid requestId;
    if (!appendUserObservation(
            QStringLiteral("self-inspection-requested"), QCborMap(), &requestId)) {
        return false;
    }

    const SelfReport report = m_self->assess(requestId);
    if (!report.isValid()) {
        m_lastError = m_self->lastError();
        return false;
    }

    Q_EMIT changed();
    return true;
}

bool Presence::fulfillIndex(int index)
{
    if (!m_awake || !m_intentions) {
        return false;
    }

    const auto open = m_intentions->open();
    if (index < 0 || index >= open.size()) {
        return false;
    }

    const bool ok = m_intentions->close(open.at(index).id, Resolution::Fulfilled);
    if (ok) {
        Q_EMIT changed();
    } else {
        m_lastError = m_intentions->lastError();
    }
    return ok;
}

bool Presence::abandonIndex(int index)
{
    if (!m_awake || !m_intentions) {
        return false;
    }

    const auto open = m_intentions->open();
    if (index < 0 || index >= open.size()) {
        return false;
    }

    const bool ok = m_intentions->close(open.at(index).id, Resolution::Abandoned);
    if (ok) {
        Q_EMIT changed();
    } else {
        m_lastError = m_intentions->lastError();
    }
    return ok;
}

QVariantList Presence::detailedObligations() const
{
    QVariantList list;
    if (!m_awake || !m_intentions) {
        return list;
    }

    for (const Intention &intention : m_intentions->open()) {
        QVariantMap map;
        map[QStringLiteral("correlationId")] =
            intention.id.toString(QUuid::WithoutBraces);
        map[QStringLiteral("description")] = intention.description;
        map[QStringLiteral("trigger")] = intention.trigger;
        map[QStringLiteral("formed")] = intention.formed.toLocalTime();
        list.append(map);
    }
    return list;
}

bool Presence::observe(const QString &subject, double value)
{
    if (!m_awake || !m_predictor) {
        return false;
    }

    const bool ok = m_predictor->observe(subject, value);
    if (ok) {
        Q_EMIT changed();
    } else {
        m_lastError = m_predictor->lastError();
    }
    return ok;
}

QVariantMap Presence::stats() const
{
    QVariantMap map;
    if (!m_awake || !m_self) {
        return map;
    }

    const SelfReport report = m_self->measure();
    map[QStringLiteral("ageInDays")] = report.ageInDays;
    map[QStringLiteral("sessions")] = report.sessions;
    map[QStringLiteral("openIntentions")] = report.openIntentions;
    map[QStringLiteral("oldestObligationDays")] = report.oldestObligationDays;
    map[QStringLiteral("settledPredictions")] = report.settledPredictions;
    map[QStringLiteral("contributions")] = report.contributions;
    map[QStringLiteral("journalIntact")] = report.journalIntact;
    map[QStringLiteral("firstBrokenAt")] = report.firstBrokenAt;
    return map;
}

QVariantMap Presence::identityState() const
{
    QVariantMap map;
    if (!m_awake || !m_identity) {
        return map;
    }

    const IdentityState state = m_identity->state();
    map[QStringLiteral("uuid")] = state.identityId.toString(QUuid::WithoutBraces);
    map[QStringLiteral("origin")] = state.origin.toString();
    map[QStringLiteral("sessionCount")] = static_cast<qint64>(state.sessionCount);
    map[QStringLiteral("architectureVersion")] = state.architectureVersion;
    map[QStringLiteral("wasBorn")] = m_identity->wasBorn();
    return map;
}

QVariantList Presence::calibrations() const
{
    QVariantList list;
    if (!m_awake || !m_predictor) {
        return list;
    }

    for (const Calibration &calibration : m_predictor->allCalibrations()) {
        QVariantMap map;
        map[QStringLiteral("subject")] = calibration.subject;
        map[QStringLiteral("settled")] = calibration.settled;
        map[QStringLiteral("meanError")] = calibration.meanError;
        map[QStringLiteral("bias")] = calibration.bias;
        list.append(map);
    }
    return list;
}

QVariantMap Presence::predict(const QString &subject)
{
    QVariantMap map;
    if (!m_awake || !m_predictor) {
        return map;
    }

    const Forecast forecast = m_predictor->predict(subject);
    if (forecast.id.isNull()) {
        m_lastError = m_predictor->lastError();
        return map;
    }

    map[QStringLiteral("subject")] = forecast.subject;
    map[QStringLiteral("estimate")] = forecast.estimate;
    map[QStringLiteral("margin")] = forecast.margin;
    map[QStringLiteral("confidence")] = forecast.confidence;
    map[QStringLiteral("samples")] = forecast.samples;
    Q_EMIT changed();
    return map;
}

QVariantList Presence::coalitions() const
{
    QVariantList list;
    if (!m_awake || !m_workspace) {
        return list;
    }

    for (const Coalition &coalition : m_workspace->coalitions()) {
        QVariantMap map;
        map[QStringLiteral("correlationId")] =
            coalition.correlationId.toString(QUuid::WithoutBraces);
        map[QStringLiteral("salience")] = coalition.salience;
        map[QStringLiteral("organs")] = coalition.organs();
        map[QStringLiteral("threads")] = coalition.threadCount();
        list.append(map);
    }
    return list;
}

QVariantMap Presence::moment() const
{
    QVariantMap map;
    if (!m_awake || !m_workspace) {
        return map;
    }

    const MomentState state = m_workspace->momentState();
    map[QStringLiteral("focus")] = state.focus.toString(QUuid::WithoutBraces);
    map[QStringLiteral("salience")] = state.salience;
    map[QStringLiteral("organs")] = state.organs;
    return map;
}

} // namespace cybou
