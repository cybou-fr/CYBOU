// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "PresenceService.h"

#include "cybou/fabric/FabricCodec.h"

#include <QCborMap>
#include <QCborValue>
#include <QDateTime>
#include <QThread>

#include <algorithm>

namespace cybou {

namespace {

const QStringList kProjectedCapabilities = {
    QStringLiteral("accepted-biography"), QStringLiteral("identity-continuity"),
    QStringLiteral("commitment-access"), QStringLiteral("prediction"),
    QStringLiteral("self-assessment"), QStringLiteral("attention-workspace"),
    QStringLiteral("consolidation"), QStringLiteral("presence-presentation"),
};

bool isAvailable(const CapabilitySnapshot &snapshot, const QString &capabilityId)
{
    if (!snapshot.isValid()) return false;
    return std::none_of(
        snapshot.deficits.cbegin(), snapshot.deficits.cend(),
        [&capabilityId](const CapabilityDeficit &deficit) {
            return deficit.capabilityId == capabilityId;
        });
}

QVariantMap capabilityProjection(const CapabilitySnapshot &snapshot)
{
    QVariantMap projection;
    projection[QStringLiteral("aggregateState")] = snapshot.isValid()
        ? capabilityStateToString(snapshot.aggregateState) : QStringLiteral("unknown");
    projection[QStringLiteral("observedAt")] = snapshot.observedAt;
    QVariantMap states;
    for (const QString &capabilityId : kProjectedCapabilities)
        states[capabilityId] = snapshot.isValid()
            ? QStringLiteral("available") : QStringLiteral("unknown");
    QVariantList deficits;
    for (const CapabilityDeficit &source : snapshot.deficits) {
        QVariantMap deficit;
        deficit[QStringLiteral("capabilityId")] = source.capabilityId;
        deficit[QStringLiteral("dependencyId")] = source.dependencyId;
        deficit[QStringLiteral("state")] = capabilityStateToString(source.state);
        deficit[QStringLiteral("cause")] = deficitCauseToString(source.cause);
        deficit[QStringLiteral("impact")] = source.impact;
        deficit[QStringLiteral("lastVerifiedAt")] = source.lastVerifiedAt;
        deficit[QStringLiteral("recoveryPolicy")] = recoveryPolicyToString(source.recoveryPolicy);
        deficit[QStringLiteral("errorReference")] = source.errorReference;
        deficits.append(deficit);
        const QString current = states.value(source.capabilityId).toString();
        const auto rank = [](const QString &state) {
            if (state == QStringLiteral("available")) return 0;
            if (state == QStringLiteral("recovering")) return 1;
            if (state == QStringLiteral("limited")) return 2;
            if (state == QStringLiteral("stale")) return 3;
            if (state == QStringLiteral("unknown")) return 4;
            return 5;
        };
        const QString candidate = capabilityStateToString(source.state);
        if (rank(candidate) > rank(current)) states[source.capabilityId] = candidate;
    }
    projection[QStringLiteral("states")] = states;
    projection[QStringLiteral("deficits")] = deficits;
    return projection;
}

QVariantMap lifecycleProjection(const QVariantMap &state)
{
    QVariantMap projection;
    const QString mode = state.value(QStringLiteral("mode")).toString();
    const QString status = state.value(QStringLiteral("status")).toString();
    const QStringList required = state.value(QStringLiteral("requiredCapabilities")).toStringList();
    const QStringList optional = state.value(QStringLiteral("optionalCapabilities")).toStringList();
    const QStringList completed = state.value(QStringLiteral("completedWork")).toStringList();
    const QStringList missing = state.value(QStringLiteral("missingWork")).toStringList();
    const int total = required.size() + optional.size();
    const int resolved = completed.size() + missing.size();

    QString progressClass = QStringLiteral("inactive");
    if (mode == QStringLiteral("recovering")) progressClass = QStringLiteral("recovering");
    else if (mode == QStringLiteral("degraded")) progressClass = QStringLiteral("degraded");
    else if (status == QStringLiteral("active")) progressClass = QStringLiteral("running");
    else if (status == QStringLiteral("completed")) progressClass = QStringLiteral("complete");
    else if (status == QStringLiteral("failed") || status == QStringLiteral("interrupted"))
        progressClass = QStringLiteral("failed");

    QVariantList deficits;
    const QVariantMap causes = state.value(QStringLiteral("missingCauses")).toMap();
    for (const QString &capability : missing) {
        QVariantMap deficit;
        deficit[QStringLiteral("capability")] = capability;
        deficit[QStringLiteral("cause")] = causes.value(capability).toString();
        deficits.append(deficit);
    }

    const QDateTime requestedAt = state.value(QStringLiteral("requestedAt")).toDateTime();
    const qint64 ageSeconds = requestedAt.isValid()
        ? qMax<qint64>(0, requestedAt.secsTo(QDateTime::currentDateTimeUtc()))
        : -1;
    QString freshnessClass = QStringLiteral("unknown");
    if (ageSeconds >= 0 && ageSeconds < 300) freshnessClass = QStringLiteral("current");
    else if (ageSeconds >= 0 && ageSeconds < 3600) freshnessClass = QStringLiteral("aging");
    else if (ageSeconds >= 0) freshnessClass = QStringLiteral("stale");

    projection[QStringLiteral("mode")] = mode;
    projection[QStringLiteral("status")] = status;
    projection[QStringLiteral("progressClass")] = progressClass;
    projection[QStringLiteral("progressPercent")] = total > 0 ? (resolved * 100) / total : 0;
    projection[QStringLiteral("resolvedWork")] = resolved;
    projection[QStringLiteral("totalWork")] = total;
    projection[QStringLiteral("deficits")] = deficits;
    projection[QStringLiteral("freshnessClass")] = freshnessClass;
    projection[QStringLiteral("ageSeconds")] = ageSeconds;
    projection[QStringLiteral("requestedAt")] = requestedAt;
    return projection;
}

} // namespace

PresenceService::PresenceService(QObject *parent)
    : QObject(parent)
{
    connect(
        &m_workspace,
        &WorkspaceClient::changed,
        this,
        [this]() {
            Q_EMIT Changed();
        });
    connect(&m_lifecycle, &LifecycleClient::changed, this, [this]() { Q_EMIT Changed(); });
    connect(&m_health, &HealthClient::changed, this, [this]() { Q_EMIT Changed(); });
}

bool PresenceService::Ready() const
{
    return true;
}

QString PresenceService::Health() const
{
    return QStringLiteral("healthy");
}

QString PresenceService::LastError() const
{
    if (!m_lastError.isEmpty()) {
        return m_lastError;
    }

    if (!m_events.lastError().isEmpty()) {
        return m_events.lastError();
    }
    if (!m_health.lastError().isEmpty()) return m_health.lastError();
    if (!m_identity.lastError().isEmpty()) {
        return m_identity.lastError();
    }
    if (!m_intentions.lastError().isEmpty()) {
        return m_intentions.lastError();
    }
    if (!m_predictor.lastError().isEmpty()) {
        return m_predictor.lastError();
    }
    if (!m_self.lastError().isEmpty()) {
        return m_self.lastError();
    }
    if (!m_workspace.lastError().isEmpty()) {
        return m_workspace.lastError();
    }
    return m_lifecycle.lastError();
}

QVariantMap PresenceService::healthMap() const
{
    QVariantMap map;
    map[QStringLiteral("eventd")] =
        m_events.isOpen()
            ? QStringLiteral("healthy")
            : QStringLiteral("unavailable");
    map[QStringLiteral("identityd")] =
        m_identity.health();
    map[QStringLiteral("intentiond")] =
        m_intentions.health();
    map[QStringLiteral("predictord")] =
        m_predictor.health();
    map[QStringLiteral("selfd")] =
        m_self.health();
    map[QStringLiteral("workspaced")] =
        m_workspace.health();
    map[QStringLiteral("lifecycled")] =
        m_lifecycle.health();
    map[QStringLiteral("presenced")] =
        Health();
    map[QStringLiteral("healthd")] = m_health.health();
    return map;
}

bool PresenceService::capabilityAvailable(const QString &capabilityId) const
{
    return isAvailable(m_health.snapshot(), capabilityId);
}

QVariantMap PresenceService::snapshotMap() const
{
    QVariantMap map;

    const CapabilitySnapshot health = m_health.snapshot();
    const QVariantMap capability = capabilityProjection(health);
    const QVariantMap self = isAvailable(health, QStringLiteral("self-assessment"))
        ? m_self.measure() : QVariantMap{};
    const QVariantMap lifecycle = m_lifecycle.ready() ? m_lifecycle.state() : QVariantMap{};
    const QVariantList intentions = isAvailable(health, QStringLiteral("commitment-access"))
        ? m_intentions.open() : QVariantList{};

    QStringList obligations;
    for (const QVariant &entry : intentions) {
        obligations.append(
            entry.toMap()
                .value(QStringLiteral("description"))
                .toString());
    }

    map[QStringLiteral("runtimeReachable")] = true;
    map[QStringLiteral("awake")] = true;
    map[QStringLiteral("aggregateCapabilityState")] =
        capability.value(QStringLiteral("aggregateState"));
    map[QStringLiteral("capabilityStates")] = capability.value(QStringLiteral("states"));
    map[QStringLiteral("capabilityDeficits")] = capability.value(QStringLiteral("deficits"));
    map[QStringLiteral("capabilityObservedAt")] = capability.value(QStringLiteral("observedAt"));
    map[QStringLiteral("lifecycleState")] = lifecycle;
    map[QStringLiteral("lifecycleMode")] = lifecycle.value(QStringLiteral("mode"));
    map[QStringLiteral("lifecycleStatus")] = lifecycle.value(QStringLiteral("status"));
    map[QStringLiteral("lifecycleProjection")] = lifecycleProjection(lifecycle);
    map[QStringLiteral("narration")] =
        self.value(QStringLiteral("narration")).toString();
    map[QStringLiteral("obligations")] = obligations;
    map[QStringLiteral("attention")] =
        isAvailable(health, QStringLiteral("attention-workspace"))
            ? m_workspace.attention() : QString();
    map[QStringLiteral("contributions")] =
        isAvailable(health, QStringLiteral("accepted-biography"))
            ? static_cast<qulonglong>(m_events.count()) : 0;
    map[QStringLiteral("stats")] = self;
    map[QStringLiteral("identityState")] =
        isAvailable(health, QStringLiteral("identity-continuity"))
            ? m_identity.state() : QVariantMap{};
    map[QStringLiteral("calibrations")] =
        isAvailable(health, QStringLiteral("prediction"))
            ? m_predictor.calibrations() : QVariantList{};
    map[QStringLiteral("coalitions")] =
        isAvailable(health, QStringLiteral("attention-workspace"))
            ? m_workspace.coalitions() : QVariantList{};
    map[QStringLiteral("moment")] =
        isAvailable(health, QStringLiteral("attention-workspace"))
            ? m_workspace.moment() : QVariantMap{};
    map[QStringLiteral("organHealth")] =
        healthMap();

    return map;
}

QByteArray PresenceService::Snapshot() const
{
    return FabricCodec::encodeMap(snapshotMap());
}

QByteArray PresenceService::Activity(int limit) const
{
    QVariantList result;

    if (limit <= 0 || !capabilityAvailable(QStringLiteral("accepted-biography"))) {
        return FabricCodec::encodeList(result);
    }

    for (const CognitiveEnvelope &envelope :
         m_events.recent(limit)) {
        QVariantMap map;
        map[QStringLiteral("when")] =
            envelope.wallTime.toLocalTime();
        map[QStringLiteral("organ")] =
            envelope.originOrgan;
        map[QStringLiteral("kind")] =
            kindToString(envelope.kind);
        map[QStringLiteral("thread")] =
            envelope.correlationId.toString(QUuid::WithoutBraces);
        result.append(map);
    }

    return FabricCodec::encodeList(result);
}

QByteArray PresenceService::DetailedObligations() const
{
    if (!capabilityAvailable(QStringLiteral("commitment-access")))
        return FabricCodec::encodeList(QVariantList{});
    return FabricCodec::encodeList(
        m_intentions.open());
}

bool PresenceService::appendUserObservation(
    const QString &event,
    const QVariantMap &details,
    QUuid *messageId)
{
    CognitiveEnvelope observation;
    observation.messageId = QUuid::createUuid();
    observation.correlationId = observation.messageId;
    observation.originOrgan = QStringLiteral("presenced");
    observation.kind = ContributionKind::Observation;
    observation.wallTime = QDateTime::currentDateTimeUtc();
    observation.confidence = 1.0;
    observation.privacy = PrivacyClass::Node;

    QCborMap payload;
    payload.insert(QStringLiteral("event"), event);
    for (auto it = details.cbegin(); it != details.cend(); ++it) {
        payload.insert(
            it.key(),
            QCborValue::fromVariant(it.value()));
    }
    observation.payloadCbor =
        payload.toCborValue().toCbor();

    if (m_events.append(observation) == 0) {
        m_lastError = m_events.lastError();
        return false;
    }

    if (messageId) {
        *messageId = observation.messageId;
    }

    return true;
}

QString PresenceService::Promise(
    const QString &description)
{
    m_lastError.clear();

    if (description.trimmed().isEmpty()
        || !capabilityAvailable(QStringLiteral("accepted-biography"))
        || !capabilityAvailable(QStringLiteral("commitment-access"))) {
        return {};
    }

    QVariantMap details;
    details[QStringLiteral("description")] =
        description.trimmed();

    QUuid requestId;
    if (!appendUserObservation(
            QStringLiteral("user-requested-intention"),
            details,
            &requestId)) {
        return {};
    }

    const QString intentionId = m_intentions.form(
        description,
        QStringLiteral("asked by the user"),
        requestId.toString(QUuid::WithoutBraces));

    if (intentionId.isEmpty()) {
        m_lastError = m_intentions.lastError();
    }

    return intentionId;
}

bool PresenceService::Reflect()
{
    m_lastError.clear();
    if (!capabilityAvailable(QStringLiteral("accepted-biography"))
        || !capabilityAvailable(QStringLiteral("self-assessment"))) return false;

    QUuid requestId;
    if (!appendUserObservation(
            QStringLiteral("self-inspection-requested"),
            {},
            &requestId)) {
        return false;
    }

    const QVariantMap report = m_self.assess(
        requestId.toString(QUuid::WithoutBraces));

    if (report.isEmpty()) {
        m_lastError = m_self.lastError();
        return false;
    }

    return true;
}

bool PresenceService::FulfillIndex(int index)
{
    m_lastError.clear();
    if (!capabilityAvailable(QStringLiteral("commitment-access"))) return false;

    const QVariantList open = m_intentions.open();
    if (index < 0 || index >= open.size()) {
        return false;
    }

    const QString id =
        open.at(index)
            .toMap()
            .value(QStringLiteral("correlationId"))
            .toString();

    const bool ok = m_intentions.close(
        id,
        0,
        QString());
    if (!ok) {
        m_lastError = m_intentions.lastError();
    }
    return ok;
}

bool PresenceService::AbandonIndex(int index)
{
    m_lastError.clear();
    if (!capabilityAvailable(QStringLiteral("commitment-access"))) return false;

    const QVariantList open = m_intentions.open();
    if (index < 0 || index >= open.size()) {
        return false;
    }

    const QString id =
        open.at(index)
            .toMap()
            .value(QStringLiteral("correlationId"))
            .toString();

    const bool ok = m_intentions.close(
        id,
        1,
        QString());
    if (!ok) {
        m_lastError = m_intentions.lastError();
    }
    return ok;
}

bool PresenceService::Observe(
    const QString &subject,
    double value)
{
    m_lastError.clear();
    if (!capabilityAvailable(QStringLiteral("prediction"))) return false;

    const bool ok = m_predictor.observe(subject, value);
    if (!ok) {
        m_lastError = m_predictor.lastError();
    }
    return ok;
}

QByteArray PresenceService::Predict(
    const QString &subject)
{
    m_lastError.clear();
    if (!capabilityAvailable(QStringLiteral("prediction")))
        return FabricCodec::encodeMap(QVariantMap{});

    const QVariantMap prediction =
        m_predictor.predict(subject);

    if (prediction.isEmpty()) {
        m_lastError = m_predictor.lastError();
    }

    return FabricCodec::encodeMap(prediction);
}

bool PresenceService::InterruptLifecycle(const QString &cause)
{
    m_lastError.clear();
    bool delayOk = false;
    const int delayMs = qEnvironmentVariableIntValue(
        "CYBOU_PRESENCE_INTERRUPT_DELAY_MS", &delayOk);
    if (delayOk && delayMs > 0) {
        QThread::msleep(static_cast<unsigned long>(delayMs));
        m_lastError = QStringLiteral("injected lifecycle interruption timeout");
        return false;
    }
    const QString reason = cause.trimmed().isEmpty()
        ? QStringLiteral("interrupted by user")
        : cause.trimmed();
    const QVariantMap state = m_lifecycle.state();
    if (state.value(QStringLiteral("status")).toString() != QStringLiteral("active")) {
        m_lastError = QStringLiteral("no active lifecycle run to interrupt");
        return false;
    }
    if (!m_lifecycle.finishRun(QStringLiteral("interrupted"), reason)) {
        m_lastError = m_lifecycle.lastError();
        return false;
    }
    return true;
}

} // namespace cybou
