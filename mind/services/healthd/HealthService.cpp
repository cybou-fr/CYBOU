// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "HealthService.h"

#include "HealthPolicy.h"
#include "cybou/fabric/FabricCodec.h"
#include "cybou/fabric/OrganBus.h"
#include "cybou/fabric/RpcResilience.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QElapsedTimer>
#include <QEventLoop>
#include <QSaveFile>
#include <QScopedValueRollback>
#include <QTimer>
#include <QCborMap>
#include <QCborValue>

#include <memory>
#include <vector>

namespace cybou {
namespace {

ComponentHealth stateFrom(const QString &health, bool ready)
{
    if (health == QStringLiteral("unavailable")) {
        return ComponentHealth::Unavailable;
    }
    if (health == QStringLiteral("healthy")) {
        return ready ? ComponentHealth::Healthy : ComponentHealth::Conflicted;
    }
    if (health == QStringLiteral("recovering")) {
        return ComponentHealth::Recovering;
    }
    if (health == QStringLiteral("conflicted")) {
        return ComponentHealth::Conflicted;
    }
    return health == QStringLiteral("degraded")
        ? ComponentHealth::Degraded
        : ComponentHealth::Unavailable;
}

QList<QPair<QString, BusEndpoint>> endpoints()
{
    return {
        {QStringLiteral("eventd"), kEventEndpoint},
        {QStringLiteral("lifecycled"), kLifecycleEndpoint},
        {QStringLiteral("identityd"), kIdentityEndpoint},
        {QStringLiteral("intentiond"), kIntentionEndpoint},
        {QStringLiteral("predictord"), kPredictorEndpoint},
        {QStringLiteral("selfd"), kSelfEndpoint},
        {QStringLiteral("workspaced"), kWorkspaceEndpoint},
        {QStringLiteral("presenced"), kPresenceEndpoint},
    };
}

HomeostaticMeasurement currentMeasurement(
    const QString &metricId,
    const QString &sourceId,
    MeasurementKind kind,
    double value,
    const QString &unit,
    const QDateTime &now)
{
    return {metricId, sourceId, kind, MeasurementStatus::Current, value, true,
            unit, now, now.addSecs(30), {}};
}

HomeostaticMeasurement unavailableMeasurement(
    const QString &metricId,
    const QString &sourceId,
    MeasurementKind kind,
    MeasurementStatus status,
    const QString &reason,
    const QDateTime &now)
{
    return {metricId, sourceId, kind, status, 0.0, false, {}, now, {}, reason};
}

} // namespace

HealthService::HealthService(const QString &path, QObject *parent)
    : QObject(parent)
    , m_path(path)
{
    m_ready = load();
}

QString HealthService::Health() const
{
    if (!m_ready) {
        return QStringLiteral("unavailable");
    }
    return m_hasSnapshot
        ? capabilityStateToString(m_snapshot.aggregateState)
        : QStringLiteral("starting");
}

QByteArray HealthService::Snapshot() const
{
    return m_hasSnapshot ? encodeCapabilitySnapshot(m_snapshot) : QByteArray();
}

QByteArray HealthService::Measurements() const
{
    return m_hasHomeostasis ? encodeHomeostasisSnapshot(m_homeostasis) : QByteArray();
}

bool HealthService::load()
{
    m_error.clear();
    if (!QFile::exists(m_path)) {
        return true;
    }
    QFile file(m_path);
    if (!file.open(QIODevice::ReadOnly)) {
        m_error = file.errorString();
        return false;
    }
    QString decodeError;
    const CapabilitySnapshot snapshot = decodeCapabilitySnapshot(file.readAll(), &decodeError);
    if (!decodeError.isEmpty()) {
        m_error = decodeError;
        return false;
    }
    m_snapshot = snapshot;
    m_hasSnapshot = true;
    return true;
}

bool HealthService::save(const CapabilitySnapshot &snapshot)
{
    QDir directory;
    if (!directory.mkpath(QFileInfo(m_path).absolutePath())) {
        m_error = QStringLiteral("cannot create health state directory");
        return false;
    }
    QSaveFile file(m_path);
    if (!file.open(QIODevice::WriteOnly)
        || file.write(encodeCapabilitySnapshot(snapshot)) < 0
        || !file.commit()) {
        m_error = file.errorString();
        return false;
    }
    return true;
}

bool HealthService::Refresh()
{
    if (!m_ready || m_refreshing) {
        return false;
    }
    QScopedValueRollback<bool> refreshing(m_refreshing, true);
    m_error.clear();
    const QDateTime now = QDateTime::currentDateTimeUtc();
    QMap<QString, ComponentHealthRecord> observations;
    QMap<QString, qint64> probeLatencyMs;
    QByteArray lifecycleState;
    QString lifecycleFailure;
    qulonglong acceptedCount = 0;
    bool hasAcceptedCount = false;
    QString countFailure;
    qulonglong eventBacklog = 0;
    bool hasEventBacklog = false;
    QString backlogFailure;
    RpcRetryPolicy probePolicy;
    probePolicy.maximumAttempts = 1;
    probePolicy.circuitFailureThreshold = 1;
    QEventLoop loop;
    QElapsedTimer elapsed;
    elapsed.start();
    int pending = endpoints().size() + 3;
    auto finish = [&pending, &loop]() {
        if (--pending == 0) loop.quit();
    };
    std::vector<std::unique_ptr<AsyncRpcClient>> clients;
    clients.reserve(pending + endpoints().size());
    for (const auto &[componentId, endpoint] : endpoints()) {
        const qint64 startedAt = elapsed.elapsed();
        auto client = std::make_unique<AsyncRpcClient>(endpoint, probePolicy);
        AsyncRpcClient *rpc = client.get();
        clients.push_back(std::move(client));
        rpc->call(QStringLiteral("Ready"), {}, RpcOperationSemantics::ReadOnly,
            [&, componentId, endpoint, startedAt, rpc](const RpcResult &readyResult) {
                const bool ready = readyResult.succeeded()
                    && !readyResult.reply.arguments().isEmpty()
                    && readyResult.reply.arguments().first().toBool();
                if (!ready || componentId == QStringLiteral("eventd")) {
                    const QString health = ready ? QStringLiteral("healthy") : QString();
                    ComponentHealthRecord record{componentId, stateFrom(health, ready), now, {},
                        ready ? health : rpcOutcomeToString(readyResult.outcome)
                            + QStringLiteral(": ") + readyResult.errorMessage};
                    observations.insert(componentId, record);
                    probeLatencyMs.insert(componentId, elapsed.elapsed() - startedAt);
                    finish();
                    return;
                }
                rpc->call(QStringLiteral("Health"), {}, RpcOperationSemantics::ReadOnly,
                    [&, componentId, startedAt](const RpcResult &healthResult) {
                        const QString health = healthResult.succeeded()
                            && !healthResult.reply.arguments().isEmpty()
                            ? healthResult.reply.arguments().first().toString() : QString();
                        ComponentHealthRecord record{componentId, stateFrom(health, true), now, {},
                            healthResult.succeeded() ? health
                                : rpcOutcomeToString(healthResult.outcome)
                                    + QStringLiteral(": ") + healthResult.errorMessage};
                        observations.insert(componentId, record);
                        probeLatencyMs.insert(componentId, elapsed.elapsed() - startedAt);
                        finish();
                    }, 750);
            }, 750);
    }

    auto eventClient = std::make_unique<AsyncRpcClient>(kEventEndpoint, probePolicy);
    eventClient->call(QStringLiteral("Count"), {}, RpcOperationSemantics::ReadOnly,
        [&](const RpcResult &result) {
            if (result.succeeded() && !result.reply.arguments().isEmpty()) {
                bool converted = false;
                acceptedCount = result.reply.arguments().first().toULongLong(&converted);
                hasAcceptedCount = converted;
            }
            if (!hasAcceptedCount) countFailure = rpcOutcomeToString(result.outcome)
                + QStringLiteral(": ") + result.errorMessage;
            finish();
        }, 750);
    clients.push_back(std::move(eventClient));

    auto backlogClient = std::make_unique<AsyncRpcClient>(kEventEndpoint, probePolicy);
    backlogClient->call(QStringLiteral("ConsumerBacklog"),
        {QStringLiteral("lifecycle.consolidation")}, RpcOperationSemantics::ReadOnly,
        [&](const RpcResult &result) {
            if (result.succeeded() && !result.reply.arguments().isEmpty()) {
                const QCborValue encoded = QCborValue::fromCbor(
                    result.reply.arguments().first().toByteArray());
                if (encoded.isMap()
                    && encoded.toMap().value(QStringLiteral("registered")).toBool()) {
                    bool converted = false;
                    eventBacklog = encoded.toMap().value(QStringLiteral("backlog"))
                                       .toString().toULongLong(&converted);
                    hasEventBacklog = converted;
                }
            }
            if (!hasEventBacklog) backlogFailure = result.succeeded()
                ? QStringLiteral("lifecycle consolidation consumer is not registered")
                : rpcOutcomeToString(result.outcome) + QStringLiteral(": ")
                    + result.errorMessage;
            finish();
        }, 750);
    clients.push_back(std::move(backlogClient));

    auto lifecycleClient = std::make_unique<AsyncRpcClient>(kLifecycleEndpoint, probePolicy);
    lifecycleClient->call(QStringLiteral("State"), {}, RpcOperationSemantics::ReadOnly,
        [&](const RpcResult &result) {
            if (result.succeeded() && !result.reply.arguments().isEmpty())
                lifecycleState = result.reply.arguments().first().toByteArray();
            else lifecycleFailure = rpcOutcomeToString(result.outcome)
                + QStringLiteral(": ") + result.errorMessage;
            finish();
        }, 750);
    clients.push_back(std::move(lifecycleClient));

    QTimer deadline;
    deadline.setSingleShot(true);
    connect(&deadline, &QTimer::timeout, &loop, &QEventLoop::quit);
    deadline.start(2000);
    loop.exec();

    for (const auto &[componentId, endpoint] : endpoints()) {
        Q_UNUSED(endpoint)
        ComponentHealthRecord record = observations.value(componentId);
        if (record.componentId.isEmpty()) {
            record = {componentId, ComponentHealth::Unavailable, now, {},
                      QStringLiteral("timed-out: refresh deadline exceeded")};
            probeLatencyMs.insert(componentId, elapsed.elapsed());
        }
        for (const ComponentHealthRecord &previous : m_snapshot.components) {
            if (previous.componentId == componentId) {
                record.lastVerifiedAt = previous.lastVerifiedAt;
                if (record.state == ComponentHealth::Healthy
                    && (previous.state == ComponentHealth::Unavailable
                        || previous.state == ComponentHealth::Conflicted))
                    record.state = ComponentHealth::Recovering;
                break;
            }
        }
        if (record.state == ComponentHealth::Healthy) record.lastVerifiedAt = now;
        observations.insert(componentId, record);
    }

    const CapabilitySnapshot candidate = HealthPolicy::evaluate(observations, now);
    if (!candidate.isValid()) {
        m_error = QStringLiteral("health policy produced an invalid snapshot");
        return false;
    }
    if (!save(candidate)) {
        return false;
    }

    HomeostasisSnapshot homeostasis;
    homeostasis.snapshotId = QUuid::createUuid();
    homeostasis.observedAt = now;
    homeostasis.measurements.append(currentMeasurement(
        QStringLiteral("health.capability-deficit.count"), QStringLiteral("healthd"),
        MeasurementKind::Counter, candidate.deficits.size(), QStringLiteral("{deficit}"), now));
    int failedProbes = 0;
    for (auto it = observations.cbegin(); it != observations.cend(); ++it) {
        homeostasis.measurements.append(currentMeasurement(
            QStringLiteral("rpc.probe.latency.%1.ms").arg(it.key()), QStringLiteral("healthd"),
            MeasurementKind::Duration, probeLatencyMs.value(it.key()), QStringLiteral("ms"), now));
        if (it->state == ComponentHealth::Unavailable) {
            ++failedProbes;
        }
    }
    homeostasis.measurements.append(currentMeasurement(
        QStringLiteral("rpc.probe-failure.count"), QStringLiteral("healthd"),
        MeasurementKind::Counter, failedProbes, QStringLiteral("{probe}"), now));

    if (hasAcceptedCount) {
        homeostasis.measurements.append(currentMeasurement(
            QStringLiteral("event.accepted.count"), QStringLiteral("eventd"),
            MeasurementKind::Counter, static_cast<double>(acceptedCount), QStringLiteral("{event}"), now));
    } else {
        homeostasis.measurements.append(unavailableMeasurement(
            QStringLiteral("event.accepted.count"), QStringLiteral("eventd"),
            MeasurementKind::Counter, MeasurementStatus::Unknown,
            countFailure.isEmpty() ? QStringLiteral("invalid Event1 Count reply") : countFailure, now));
    }

    QString lifecycleError;
    const QVariantMap lifecycle = FabricCodec::decodeMap(
        lifecycleState, &lifecycleError);
    if (lifecycleError.isEmpty() && lifecycle.contains(QStringLiteral("hasRun"))) {
        const QString status = lifecycle.value(QStringLiteral("status")).toString();
        const bool active = lifecycle.value(QStringLiteral("hasRun")).toBool()
            && (status == QStringLiteral("requested") || status == QStringLiteral("active"));
        homeostasis.measurements.append(currentMeasurement(
            QStringLiteral("lifecycle.active-run.count"), QStringLiteral("lifecycled"),
            MeasurementKind::Counter, active ? 1.0 : 0.0, QStringLiteral("{run}"), now));
    } else {
        homeostasis.measurements.append(unavailableMeasurement(
            QStringLiteral("lifecycle.active-run.count"), QStringLiteral("lifecycled"),
            MeasurementKind::Counter, MeasurementStatus::Unknown,
            lifecycleError.isEmpty() ? lifecycleFailure : lifecycleError, now));
    }

    if (hasEventBacklog) {
        homeostasis.measurements.append(currentMeasurement(
            QStringLiteral("event.backlog.count"), QStringLiteral("eventd"),
            MeasurementKind::Counter, static_cast<double>(eventBacklog),
            QStringLiteral("{event}"), now));
    } else {
        homeostasis.measurements.append(unavailableMeasurement(
            QStringLiteral("event.backlog.count"), QStringLiteral("eventd"),
            MeasurementKind::Counter, MeasurementStatus::Unknown,
            backlogFailure, now));
    }
    homeostasis.measurements.append(unavailableMeasurement(
        QStringLiteral("journal.storage.bytes"), QStringLiteral("eventd"),
        MeasurementKind::Bytes, MeasurementStatus::Unsupported,
        QStringLiteral("Event1 does not expose owner storage metrics"), now));
    homeostasis.measurements.append(unavailableMeasurement(
        QStringLiteral("prediction.calibration-pressure"), QStringLiteral("predictord"),
        MeasurementKind::Gauge, MeasurementStatus::Unsupported,
        QStringLiteral("calibration-pressure policy is not defined"), now));
    if (hasEventBacklog)
        homeostasis.authorizedPolicyIds.append(QStringLiteral("event-backlog-v1"));
    if (!homeostasis.isValid()) {
        m_error = QStringLiteral("health collector produced an invalid homeostasis snapshot");
        return false;
    }
    m_snapshot = candidate;
    m_homeostasis = homeostasis;
    m_hasSnapshot = true;
    m_hasHomeostasis = true;
    Q_EMIT Changed();
    return true;
}

} // namespace cybou
