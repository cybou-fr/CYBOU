// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "HealthService.h"

#include "HealthPolicy.h"
#include "cybou/fabric/OrganBus.h"
#include "cybou/fabric/RpcClient.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
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
    for (const auto &[componentId, endpoint] : endpoints()) {
        RpcClient client(endpoint);
        const bool ready = client.ready();
        QString health = client.health();
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
    m_snapshot = candidate;
    m_hasSnapshot = true;
    Q_EMIT Changed();
    return true;
}

} // namespace cybou
