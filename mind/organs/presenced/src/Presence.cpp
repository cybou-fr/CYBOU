// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/presence/Presence.h"

#include "cybou/fabric/OrganClients.h"
#include "cybou/fabric/RpcResilience.h"
#include "cybou/runtime/StatePaths.h"

namespace cybou {

namespace {

QStringList toStringList(const QVariant &value)
{
    if (value.canConvert<QStringList>()) {
        const QStringList direct = value.toStringList();
        if (!direct.isEmpty() || value.toList().isEmpty()) {
            return direct;
        }
    }

    QStringList result;
    for (const QVariant &entry : value.toList()) {
        result.append(entry.toString());
    }
    return result;
}

} // namespace

Presence::Presence(QObject *parent)
    : QObject(parent)
    , m_client(std::make_unique<PresenceClient>())
    , m_resilientClient(std::make_unique<AsyncRpcClient>(kPresenceEndpoint, RpcRetryPolicy{}, this))
{
    connect(
        m_client.get(),
        &PresenceClient::changed,
        this,
        [this]() {
            remoteChanged();
        });
}

Presence::~Presence() = default;

bool Presence::wake()
{
    m_lastError.clear();

    QString migrationError;
    if (!StatePaths::migrateLegacy(&migrationError)) {
        m_lastError =
            QStringLiteral("cannot migrate legacy Mind state: %1")
                .arg(migrationError);
        m_awake = false;
        Q_EMIT changed();
        return false;
    }

    if (!m_client->ready()) {
        m_lastError = m_client->lastError();
        m_awake = false;
        Q_EMIT changed();
        return false;
    }

    if (!refresh()) {
        m_awake = false;
        Q_EMIT changed();
        return false;
    }

    m_awake = true;
    Q_EMIT changed();
    return true;
}

bool Presence::refresh()
{
    const QVariantMap snapshot = m_client->snapshot();
    if (snapshot.isEmpty()) {
        m_lastError = m_client->lastError();
        return false;
    }

    m_snapshot = snapshot;
    m_lastError.clear();
    return true;
}

void Presence::remoteChanged()
{
    if (refresh()) {
        m_awake =
            m_snapshot.value(QStringLiteral("awake")).toBool();
        Q_EMIT changed();
    }
}

bool Presence::runtimeReachable() const
{
    return m_snapshot.value(QStringLiteral("runtimeReachable")).toBool();
}

QString Presence::aggregateCapabilityState() const
{
    return m_snapshot.value(QStringLiteral("aggregateCapabilityState")).toString();
}

QVariantMap Presence::capabilityStates() const
{
    return m_snapshot.value(QStringLiteral("capabilityStates")).toMap();
}

QVariantList Presence::capabilityDeficits() const
{
    return m_snapshot.value(QStringLiteral("capabilityDeficits")).toList();
}

QDateTime Presence::capabilityObservedAt() const
{
    return m_snapshot.value(QStringLiteral("capabilityObservedAt")).toDateTime();
}

bool Presence::hasCapability(const QString &capabilityId) const
{
    return capabilityStates().value(capabilityId).toString() == QStringLiteral("available");
}

QString Presence::narration() const
{
    return m_snapshot
        .value(QStringLiteral("narration"))
        .toString();
}

QStringList Presence::obligations() const
{
    return toStringList(
        m_snapshot.value(QStringLiteral("obligations")));
}

QString Presence::attention() const
{
    return m_snapshot
        .value(QStringLiteral("attention"))
        .toString();
}

int Presence::contributions() const
{
    return m_snapshot
        .value(QStringLiteral("contributions"))
        .toInt();
}

QList<Moment> Presence::recent(int limit) const
{
    QList<Moment> result;
    for (const QVariant &entry : activity(limit)) {
        const QVariantMap map = entry.toMap();

        Moment moment;
        moment.when =
            map.value(QStringLiteral("when")).toDateTime();
        moment.organ =
            map.value(QStringLiteral("organ")).toString();
        moment.kind =
            map.value(QStringLiteral("kind")).toString();
        moment.thread =
            QUuid::fromString(
                map.value(QStringLiteral("thread")).toString());
        result.append(moment);
    }
    return result;
}

QVariantList Presence::activity(int limit) const
{
    if (!hasCapability(QStringLiteral("accepted-biography"))) {
        return {};
    }
    return m_client->activity(limit);
}

QUuid Presence::promise(const QString &description)
{
    if (!hasCapability(QStringLiteral("accepted-biography"))
        || !hasCapability(QStringLiteral("commitment-access"))) {
        return {};
    }

    const QString id = m_client->promise(description);
    if (id.isEmpty()) {
        m_lastError = m_client->lastError();
        return {};
    }

    refresh();
    return QUuid::fromString(id);
}

bool Presence::reflect()
{
    if (!hasCapability(QStringLiteral("accepted-biography"))
        || !hasCapability(QStringLiteral("self-assessment"))) {
        return false;
    }

    const bool ok = m_client->reflect();
    if (!ok) {
        m_lastError = m_client->lastError();
    } else {
        refresh();
    }
    return ok;
}

bool Presence::fulfillIndex(int index)
{
    if (!hasCapability(QStringLiteral("commitment-access"))) {
        return false;
    }

    const bool ok = m_client->fulfillIndex(index);
    if (!ok) {
        m_lastError = m_client->lastError();
    } else {
        refresh();
    }
    return ok;
}

bool Presence::abandonIndex(int index)
{
    if (!hasCapability(QStringLiteral("commitment-access"))) {
        return false;
    }

    const bool ok = m_client->abandonIndex(index);
    if (!ok) {
        m_lastError = m_client->lastError();
    } else {
        refresh();
    }
    return ok;
}

QVariantList Presence::detailedObligations() const
{
    if (!hasCapability(QStringLiteral("commitment-access"))) {
        return {};
    }
    return m_client->detailedObligations();
}

bool Presence::observe(
    const QString &subject,
    double value)
{
    if (!hasCapability(QStringLiteral("prediction"))) {
        return false;
    }

    const bool ok = m_client->observe(subject, value);
    if (!ok) {
        m_lastError = m_client->lastError();
    } else {
        refresh();
    }
    return ok;
}

QVariantMap Presence::stats() const
{
    return m_snapshot
        .value(QStringLiteral("stats"))
        .toMap();
}

QVariantMap Presence::identityState() const
{
    return m_snapshot
        .value(QStringLiteral("identityState"))
        .toMap();
}

QVariantList Presence::calibrations() const
{
    return m_snapshot
        .value(QStringLiteral("calibrations"))
        .toList();
}

QVariantMap Presence::predict(const QString &subject)
{
    if (!hasCapability(QStringLiteral("prediction"))) {
        return {};
    }

    const QVariantMap prediction =
        m_client->predict(subject);
    if (prediction.isEmpty()) {
        m_lastError = m_client->lastError();
    } else {
        refresh();
    }
    return prediction;
}

QVariantList Presence::coalitions() const
{
    return m_snapshot
        .value(QStringLiteral("coalitions"))
        .toList();
}

QVariantMap Presence::moment() const
{
    return m_snapshot
        .value(QStringLiteral("moment"))
        .toMap();
}

QVariantMap Presence::organHealth() const
{
    return m_snapshot
        .value(QStringLiteral("organHealth"))
        .toMap();
}

QString Presence::lifecycleMode() const
{
    return m_snapshot.value(QStringLiteral("lifecycleMode")).toString();
}

QString Presence::lifecycleStatus() const
{
    return m_snapshot.value(QStringLiteral("lifecycleStatus")).toString();
}

QVariantMap Presence::lifecycleState() const
{
    return m_snapshot.value(QStringLiteral("lifecycleState")).toMap();
}

QVariantMap Presence::lifecycleProjection() const
{
    return m_snapshot.value(QStringLiteral("lifecycleProjection")).toMap();
}

QVariantMap Presence::lifecycleScheduling() const
{
    return m_snapshot.value(QStringLiteral("lifecycleScheduling")).toMap();
}

void Presence::interruptLifecycle(const QString &cause)
{
    if (!runtimeReachable() || m_lifecycleCommandPending) {
        return;
    }

    m_lifecycleCommandPending = true;
    m_lastError.clear();
    Q_EMIT changed();

    m_resilientClient->call(
        QStringLiteral("InterruptLifecycle"),
        {cause},
        RpcOperationSemantics::NonIdempotentMutation,
        [this](const RpcResult &result) {
            m_lifecycleCommandPending = false;
            if (!result.succeeded()) {
                m_lastError = QStringLiteral("%1: %2")
                                  .arg(
                                      rpcOutcomeToString(result.outcome),
                                      result.errorMessage.isEmpty()
                                          ? QStringLiteral("lifecycle interruption was not accepted")
                                          : result.errorMessage);
            } else {
                refresh();
            }
            Q_EMIT changed();
        },
        5000,
        true);
}

} // namespace cybou
