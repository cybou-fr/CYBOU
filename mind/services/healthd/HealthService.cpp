// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "HealthService.h"

#include "HealthPolicy.h"
#include "cybou/fabric/FabricCodec.h"
#include "cybou/fabric/OrganBus.h"
#include "cybou/fabric/RpcClient.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QElapsedTimer>
#include <QSaveFile>

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
    if (!m_ready) {
        return false;
    }
    m_error.clear();
    const QDateTime now = QDateTime::currentDateTimeUtc();
    QMap<QString, ComponentHealthRecord> observations;
    QMap<QString, qint64> probeLatencyMs;
    for (const auto &[componentId, endpoint] : endpoints()) {
        RpcClient client(endpoint);
        QElapsedTimer timer;
        timer.start();
        const bool ready = client.ready();
        QString health = client.health();
        probeLatencyMs.insert(componentId, timer.elapsed());
        if (ready && health.isEmpty() && componentId == QStringLiteral("eventd")) {
            // Event1 predates the common Health() method; Ready() is its typed health boundary.
            health = QStringLiteral("healthy");
        }
        ComponentHealthRecord record;
        record.componentId = componentId;
        record.state = stateFrom(health, ready);
        record.observedAt = now;
        ComponentHealth previousState = ComponentHealth::Starting;
        bool hasPrevious = false;
        if (m_hasSnapshot) {
            for (const ComponentHealthRecord &previous : m_snapshot.components) {
                if (previous.componentId == componentId) {
                    previousState = previous.state;
                    hasPrevious = true;
                    record.lastVerifiedAt = previous.lastVerifiedAt;
                    break;
                }
            }
        }
        if (record.state == ComponentHealth::Healthy && hasPrevious
            && (previousState == ComponentHealth::Unavailable
                || previousState == ComponentHealth::Conflicted)) {
            record.state = ComponentHealth::Recovering;
        }
        if (record.state == ComponentHealth::Healthy) {
            record.lastVerifiedAt = now;
        }
        record.detail = ready ? health : client.lastError();
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

    RpcClient eventClient(kEventEndpoint);
    const QDBusMessage countReply = eventClient.call(QStringLiteral("Count"));
    if (countReply.type() == QDBusMessage::ReplyMessage && !countReply.arguments().isEmpty()) {
        bool converted = false;
        const qulonglong count = countReply.arguments().first().toULongLong(&converted);
        if (converted) {
            homeostasis.measurements.append(currentMeasurement(
                QStringLiteral("event.accepted.count"), QStringLiteral("eventd"),
                MeasurementKind::Counter, static_cast<double>(count), QStringLiteral("{event}"), now));
        }
    }
    if (homeostasis.measurements.size() == endpoints().size() + 2) {
        homeostasis.measurements.append(unavailableMeasurement(
            QStringLiteral("event.accepted.count"), QStringLiteral("eventd"),
            MeasurementKind::Counter, MeasurementStatus::Unknown,
            eventClient.lastError().isEmpty() ? QStringLiteral("invalid Event1 Count reply")
                                              : eventClient.lastError(), now));
    }

    RpcClient lifecycleClient(kLifecycleEndpoint);
    QString lifecycleError;
    const QVariantMap lifecycle = FabricCodec::decodeMap(
        lifecycleClient.callBytes(QStringLiteral("State")), &lifecycleError);
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
            lifecycleError.isEmpty() ? lifecycleClient.lastError() : lifecycleError, now));
    }

    homeostasis.measurements.append(unavailableMeasurement(
        QStringLiteral("event.backlog.count"), QStringLiteral("eventd"),
        MeasurementKind::Counter, MeasurementStatus::Unsupported,
        QStringLiteral("Event1 has no consumer-offset contract"), now));
    homeostasis.measurements.append(unavailableMeasurement(
        QStringLiteral("journal.storage.bytes"), QStringLiteral("eventd"),
        MeasurementKind::Bytes, MeasurementStatus::Unsupported,
        QStringLiteral("Event1 does not expose owner storage metrics"), now));
    homeostasis.measurements.append(unavailableMeasurement(
        QStringLiteral("prediction.calibration-pressure"), QStringLiteral("predictord"),
        MeasurementKind::Gauge, MeasurementStatus::Unsupported,
        QStringLiteral("calibration-pressure policy is not defined"), now));
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
