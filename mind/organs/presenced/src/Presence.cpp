// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/presence/Presence.h"

#include "cybou/events/EventStore.h"
#include "cybou/ipc/EventClient.h"
#include "cybou/runtime/StatePaths.h"
#include "cybou/storage/Journal.h"

#include <QCborMap>
#include <QCborValue>
#include <QDir>
#include <QFileInfo>
#include <QHash>
#include <QMutex>
#include <QMutexLocker>

#include <memory>
#include <utility>

namespace cybou {

enum class RuntimeTransport {
    LocalJournal,
    Event1,
};

class PresenceRuntime
{
public:
    PresenceRuntime(QString path, RuntimeTransport transportValue)
        : dataDir(std::move(path))
        , transport(transportValue)
    {
    }

    QString dataDir;
    RuntimeTransport transport;
    QString lastError;
    bool awake{false};

    std::unique_ptr<EventStore> events;
    std::unique_ptr<Identity> identity;
    std::unique_ptr<Intentions> intentions;
    std::unique_ptr<Predictor> predictor;
    std::unique_ptr<SelfModel> self;
    std::unique_ptr<Workspace> workspace;

    QMutex wakeMutex;
};

namespace {

QString cleanPath(const QString &path)
{
    return QDir::cleanPath(QFileInfo(path).absoluteFilePath());
}

QString registryKey(
    const QString &dataDir,
    RuntimeTransport transport)
{
    const QString prefix =
        transport == RuntimeTransport::Event1
            ? QStringLiteral("event1:")
            : QStringLiteral("local:");
    return prefix + cleanPath(dataDir);
}

QMutex &registryMutex()
{
    static QMutex mutex;
    return mutex;
}

QHash<QString, std::weak_ptr<PresenceRuntime>> &runtimeRegistry()
{
    static QHash<QString, std::weak_ptr<PresenceRuntime>> registry;
    return registry;
}

std::shared_ptr<PresenceRuntime> acquireRuntime(
    const QString &dataDir,
    RuntimeTransport transport)
{
    const QString key = registryKey(dataDir, transport);
    QMutexLocker locker(&registryMutex());

    auto &registry = runtimeRegistry();
    if (const auto existing = registry.value(key).lock()) {
        return existing;
    }

    auto runtime =
        std::make_shared<PresenceRuntime>(cleanPath(dataDir), transport);
    registry.insert(key, runtime);
    return runtime;
}

bool usesCanonicalState(const QString &dataDir)
{
    return cleanPath(dataDir) == cleanPath(StatePaths::persistentRoot());
}

} // namespace

Presence::Presence(const QString &dataDir, QObject *parent)
    : QObject(parent)
    , m_runtime(acquireRuntime(dataDir, RuntimeTransport::LocalJournal))
{
}

Presence::Presence(QObject *parent)
    : QObject(parent)
    , m_runtime(acquireRuntime(
          StatePaths::persistentRoot(),
          RuntimeTransport::Event1))
{
}

Presence::~Presence() = default;

bool Presence::isAwake() const
{
    return m_runtime && m_runtime->awake;
}

void Presence::subscribeToRuntime()
{
    if (m_subscribed || !m_runtime || !m_runtime->awake || !m_runtime->events) {
        return;
    }

    connect(
        m_runtime->events.get(),
        &EventStore::accepted,
        this,
        [this](const CognitiveEnvelope &, quint64) {
            Q_EMIT changed();
        });

    m_subscribed = true;
}

bool Presence::wake()
{
    if (!m_runtime) {
        m_lastError = QStringLiteral("no Presence runtime");
        return false;
    }

    {
        QMutexLocker locker(&m_runtime->wakeMutex);

        if (!m_runtime->awake) {
            m_runtime->lastError.clear();

            std::unique_ptr<EventStore> events;

            if (m_runtime->transport == RuntimeTransport::Event1) {
                // M1 migration must happen before the first Event1 call, because that call can
                // D-Bus-activate eventd and make the canonical Journal live.
                if (usesCanonicalState(m_runtime->dataDir)) {
                    QString migrationError;
                    if (!StatePaths::migrateLegacy(&migrationError)) {
                        m_runtime->lastError =
                            QStringLiteral("cannot migrate legacy Mind state: %1")
                                .arg(migrationError);
                        m_lastError = m_runtime->lastError;
                        return false;
                    }
                }

                auto client = std::make_unique<EventClient>();
                if (!client->isOpen()) {
                    m_runtime->lastError = client->lastError();
                    m_lastError = m_runtime->lastError;
                    return false;
                }
                events = std::move(client);
            } else {
                if (!QDir().mkpath(m_runtime->dataDir)) {
                    m_runtime->lastError =
                        QStringLiteral("cannot create %1").arg(m_runtime->dataDir);
                    m_lastError = m_runtime->lastError;
                    return false;
                }

                auto journal = std::make_unique<Journal>(
                    QDir(m_runtime->dataDir).filePath(QStringLiteral("journal.db")));
                if (!journal->isOpen()) {
                    m_runtime->lastError = journal->lastError();
                    m_lastError = m_runtime->lastError;
                    return false;
                }
                events = std::move(journal);
            }

            auto identity = std::make_unique<Identity>(
                QDir(m_runtime->dataDir).filePath(QStringLiteral("identity.json")),
                events.get());
            if (!identity->beginSession()) {
                m_runtime->lastError = identity->lastError();
                m_lastError = m_runtime->lastError;
                return false;
            }

            auto intentions = std::make_unique<Intentions>(events.get());
            auto predictor = std::make_unique<Predictor>(events.get());
            auto self = std::make_unique<SelfModel>(
                events.get(),
                identity.get(),
                intentions.get(),
                predictor.get());
            auto workspace = std::make_unique<Workspace>(events.get());
            workspace->rehydrate();

            m_runtime->events = std::move(events);
            m_runtime->identity = std::move(identity);
            m_runtime->intentions = std::move(intentions);
            m_runtime->predictor = std::move(predictor);
            m_runtime->self = std::move(self);
            m_runtime->workspace = std::move(workspace);
            m_runtime->awake = true;
        }
    }

    m_lastError.clear();

    const bool wasSubscribed = m_subscribed;
    subscribeToRuntime();
    if (!wasSubscribed && m_subscribed) {
        Q_EMIT changed();
    }

    return true;
}

QString Presence::narration() const
{
    return isAwake() && m_runtime->self
               ? m_runtime->self->narrate(m_runtime->self->measure())
               : QString();
}

QStringList Presence::obligations() const
{
    QStringList result;
    if (!isAwake() || !m_runtime->intentions) {
        return result;
    }

    const auto open = m_runtime->intentions->open();
    result.reserve(open.size());
    for (const Intention &intention : open) {
        result.append(intention.description);
    }
    return result;
}

QString Presence::attention() const
{
    if (!isAwake() || !m_runtime->workspace) {
        return {};
    }

    const Coalition focus = m_runtime->workspace->focus();
    if (!focus.isValid()) {
        return {};
    }

    const CognitiveEnvelope &latest = focus.members.last();
    const QStringList voices = focus.organs();
    if (voices.size() > 1) {
        return QObject::tr(
                   "%1, with %n organ(s) involved",
                   nullptr,
                   voices.size())
            .arg(kindToString(latest.kind));
    }
    return QObject::tr("%1, from %2")
        .arg(kindToString(latest.kind), latest.originOrgan);
}

int Presence::contributions() const
{
    return isAwake() && m_runtime->events
               ? static_cast<int>(m_runtime->events->count())
               : 0;
}

QList<Moment> Presence::recent(int limit) const
{
    QList<Moment> result;
    if (!isAwake() || !m_runtime->events || limit <= 0) {
        return result;
    }

    const auto envelopes = m_runtime->events->recent(limit);
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
        entry[QStringLiteral("thread")] =
            moment.thread.toString(QUuid::WithoutBraces);
        result.append(entry);
    }
    return result;
}

bool Presence::appendUserObservation(
    const QString &event,
    const QCborMap &details,
    QUuid *messageId)
{
    if (!isAwake() || !m_runtime->events) {
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

    if (m_runtime->events->append(observation) == 0) {
        m_lastError = m_runtime->events->lastError();
        return false;
    }

    if (messageId) {
        *messageId = observation.messageId;
    }
    return true;
}

QUuid Presence::promise(const QString &description)
{
    m_lastError.clear();
    if (!isAwake() || !m_runtime->intentions || description.trimmed().isEmpty()) {
        return {};
    }

    QCborMap details;
    details[QStringLiteral("description")] = description.trimmed();

    QUuid requestId;
    if (!appendUserObservation(
            QStringLiteral("user-requested-intention"),
            details,
            &requestId)) {
        return {};
    }

    const QUuid intentionId = m_runtime->intentions->form(
        description,
        QStringLiteral("asked by the user"),
        requestId);
    if (intentionId.isNull()) {
        m_lastError = m_runtime->intentions->lastError();
    }
    return intentionId;
}

bool Presence::reflect()
{
    m_lastError.clear();
    if (!isAwake() || !m_runtime->self) {
        return false;
    }

    QUuid requestId;
    if (!appendUserObservation(
            QStringLiteral("self-inspection-requested"),
            QCborMap(),
            &requestId)) {
        return false;
    }

    const SelfReport report = m_runtime->self->assess(requestId);
    if (!report.isValid()) {
        m_lastError = m_runtime->self->lastError();
        return false;
    }
    return true;
}

bool Presence::fulfillIndex(int index)
{
    m_lastError.clear();
    if (!isAwake() || !m_runtime->intentions) {
        return false;
    }

    const auto open = m_runtime->intentions->open();
    if (index < 0 || index >= open.size()) {
        return false;
    }

    const bool ok =
        m_runtime->intentions->close(
            open.at(index).id,
            Resolution::Fulfilled);
    if (!ok) {
        m_lastError = m_runtime->intentions->lastError();
    }
    return ok;
}

bool Presence::abandonIndex(int index)
{
    m_lastError.clear();
    if (!isAwake() || !m_runtime->intentions) {
        return false;
    }

    const auto open = m_runtime->intentions->open();
    if (index < 0 || index >= open.size()) {
        return false;
    }

    const bool ok =
        m_runtime->intentions->close(
            open.at(index).id,
            Resolution::Abandoned);
    if (!ok) {
        m_lastError = m_runtime->intentions->lastError();
    }
    return ok;
}

QVariantList Presence::detailedObligations() const
{
    QVariantList list;
    if (!isAwake() || !m_runtime->intentions) {
        return list;
    }

    for (const Intention &intention : m_runtime->intentions->open()) {
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
    m_lastError.clear();
    if (!isAwake() || !m_runtime->predictor) {
        return false;
    }

    const bool ok = m_runtime->predictor->observe(subject, value);
    if (!ok) {
        m_lastError = m_runtime->predictor->lastError();
    }
    return ok;
}

QVariantMap Presence::stats() const
{
    QVariantMap map;
    if (!isAwake() || !m_runtime->self) {
        return map;
    }

    const SelfReport report = m_runtime->self->measure();
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
    if (!isAwake() || !m_runtime->identity) {
        return map;
    }

    const IdentityState state = m_runtime->identity->state();
    map[QStringLiteral("uuid")] =
        state.identityId.toString(QUuid::WithoutBraces);
    map[QStringLiteral("origin")] = state.origin.toString();
    map[QStringLiteral("sessionCount")] =
        static_cast<qint64>(state.sessionCount);
    map[QStringLiteral("architectureVersion")] =
        state.architectureVersion;
    map[QStringLiteral("wasBorn")] =
        m_runtime->identity->wasBorn();
    return map;
}

QVariantList Presence::calibrations() const
{
    QVariantList list;
    if (!isAwake() || !m_runtime->predictor) {
        return list;
    }

    for (const Calibration &calibration :
         m_runtime->predictor->allCalibrations()) {
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
    m_lastError.clear();
    QVariantMap map;
    if (!isAwake() || !m_runtime->predictor) {
        return map;
    }

    const Forecast forecast = m_runtime->predictor->predict(subject);
    if (forecast.id.isNull()) {
        m_lastError = m_runtime->predictor->lastError();
        return map;
    }

    map[QStringLiteral("subject")] = forecast.subject;
    map[QStringLiteral("estimate")] = forecast.estimate;
    map[QStringLiteral("margin")] = forecast.margin;
    map[QStringLiteral("confidence")] = forecast.confidence;
    map[QStringLiteral("samples")] = forecast.samples;
    return map;
}

QVariantList Presence::coalitions() const
{
    QVariantList list;
    if (!isAwake() || !m_runtime->workspace) {
        return list;
    }

    for (const Coalition &coalition :
         m_runtime->workspace->coalitions()) {
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
    if (!isAwake() || !m_runtime->workspace) {
        return map;
    }

    const MomentState state = m_runtime->workspace->momentState();
    map[QStringLiteral("focus")] =
        state.focus.toString(QUuid::WithoutBraces);
    map[QStringLiteral("salience")] = state.salience;
    map[QStringLiteral("organs")] = state.organs;
    return map;
}

} // namespace cybou
