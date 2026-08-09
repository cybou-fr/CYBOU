// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/protocol/Health.h"

#include <QCborArray>
#include <QCborMap>
#include <QCborValue>
#include <QSet>

namespace cybou {
namespace {

void setError(QString *error, const QString &message)
{
    if (error) {
        *error = message;
    }
}

bool validComponentHealth(ComponentHealth health)
{
    switch (health) {
    case ComponentHealth::Starting:
    case ComponentHealth::Healthy:
    case ComponentHealth::Degraded:
    case ComponentHealth::Unavailable:
    case ComponentHealth::Recovering:
    case ComponentHealth::Conflicted:
        return true;
    }
    return false;
}

bool validCapabilityState(CapabilityState state)
{
    switch (state) {
    case CapabilityState::Available:
    case CapabilityState::Limited:
    case CapabilityState::Unavailable:
    case CapabilityState::Unknown:
    case CapabilityState::Stale:
    case CapabilityState::Recovering:
        return true;
    }
    return false;
}

bool validDeficitCause(DeficitCause cause)
{
    switch (cause) {
    case DeficitCause::DependencyUnavailable:
    case DeficitCause::DependencyDegraded:
    case DeficitCause::TimedOut:
    case DeficitCause::Rejected:
    case DeficitCause::UnknownOutcome:
    case DeficitCause::StaleEvidence:
    case DeficitCause::ConflictingState:
        return true;
    }
    return false;
}

bool validRecoveryPolicy(RecoveryPolicy policy)
{
    switch (policy) {
    case RecoveryPolicy::None:
    case RecoveryPolicy::Observe:
    case RecoveryPolicy::RetryIdempotent:
    case RecoveryPolicy::Reconcile:
    case RecoveryPolicy::OperatorRequired:
        return true;
    }
    return false;
}

QString timestamp(const QDateTime &value)
{
    return value.isValid() ? value.toUTC().toString(Qt::ISODateWithMs) : QString();
}

QDateTime parseTimestamp(const QCborValue &value)
{
    return QDateTime::fromString(value.toString(), Qt::ISODateWithMs);
}

bool requiredMapFields(
    const QCborMap &map,
    const QList<QString> &fields,
    QString *error,
    const QString &context)
{
    for (const QString &field : fields) {
        if (!map.contains(field)) {
            setError(error, context + QStringLiteral(" missing field: ") + field);
            return false;
        }
    }
    return true;
}

bool integerInRange(const QCborValue &value, qint64 minimum, qint64 maximum)
{
    return value.isInteger() && value.toInteger() >= minimum && value.toInteger() <= maximum;
}

} // namespace

QString componentHealthToString(ComponentHealth health)
{
    switch (health) {
    case ComponentHealth::Starting: return QStringLiteral("starting");
    case ComponentHealth::Healthy: return QStringLiteral("healthy");
    case ComponentHealth::Degraded: return QStringLiteral("degraded");
    case ComponentHealth::Unavailable: return QStringLiteral("unavailable");
    case ComponentHealth::Recovering: return QStringLiteral("recovering");
    case ComponentHealth::Conflicted: return QStringLiteral("conflicted");
    }
    return QStringLiteral("unknown");
}

QString capabilityStateToString(CapabilityState state)
{
    switch (state) {
    case CapabilityState::Available: return QStringLiteral("available");
    case CapabilityState::Limited: return QStringLiteral("limited");
    case CapabilityState::Unavailable: return QStringLiteral("unavailable");
    case CapabilityState::Unknown: return QStringLiteral("unknown");
    case CapabilityState::Stale: return QStringLiteral("stale");
    case CapabilityState::Recovering: return QStringLiteral("recovering");
    }
    return QStringLiteral("unknown");
}

QString deficitCauseToString(DeficitCause cause)
{
    switch (cause) {
    case DeficitCause::DependencyUnavailable: return QStringLiteral("dependency-unavailable");
    case DeficitCause::DependencyDegraded: return QStringLiteral("dependency-degraded");
    case DeficitCause::TimedOut: return QStringLiteral("timed-out");
    case DeficitCause::Rejected: return QStringLiteral("rejected");
    case DeficitCause::UnknownOutcome: return QStringLiteral("unknown-outcome");
    case DeficitCause::StaleEvidence: return QStringLiteral("stale-evidence");
    case DeficitCause::ConflictingState: return QStringLiteral("conflicting-state");
    }
    return QStringLiteral("unknown");
}

QString recoveryPolicyToString(RecoveryPolicy policy)
{
    switch (policy) {
    case RecoveryPolicy::None: return QStringLiteral("none");
    case RecoveryPolicy::Observe: return QStringLiteral("observe");
    case RecoveryPolicy::RetryIdempotent: return QStringLiteral("retry-idempotent");
    case RecoveryPolicy::Reconcile: return QStringLiteral("reconcile");
    case RecoveryPolicy::OperatorRequired: return QStringLiteral("operator-required");
    }
    return QStringLiteral("unknown");
}

bool canTransition(ComponentHealth from, ComponentHealth to) noexcept
{
    if (!validComponentHealth(from) || !validComponentHealth(to) || from == to) {
        return false;
    }
    switch (from) {
    case ComponentHealth::Starting:
        return to != ComponentHealth::Recovering;
    case ComponentHealth::Healthy:
        return to != ComponentHealth::Starting;
    case ComponentHealth::Degraded:
        return to != ComponentHealth::Starting;
    case ComponentHealth::Unavailable:
        return to == ComponentHealth::Recovering || to == ComponentHealth::Conflicted;
    case ComponentHealth::Recovering:
        return to == ComponentHealth::Healthy || to == ComponentHealth::Degraded
            || to == ComponentHealth::Unavailable || to == ComponentHealth::Conflicted;
    case ComponentHealth::Conflicted:
        return to == ComponentHealth::Recovering || to == ComponentHealth::Unavailable;
    }
    return false;
}

bool ComponentHealthRecord::isValid() const
{
    if (componentId.trimmed().isEmpty() || !validComponentHealth(state)
        || !observedAt.isValid()) {
        return false;
    }
    return !lastVerifiedAt.isValid() || lastVerifiedAt <= observedAt;
}

bool CapabilityDeficit::isValid() const
{
    if (capabilityId.trimmed().isEmpty() || dependencyId.trimmed().isEmpty()
        || !validCapabilityState(state) || state == CapabilityState::Available
        || !validDeficitCause(cause) || !detectedAt.isValid()
        || impact.trimmed().isEmpty() || !validRecoveryPolicy(recoveryPolicy)) {
        return false;
    }
    return !lastVerifiedAt.isValid() || lastVerifiedAt <= detectedAt;
}

bool CapabilitySnapshot::isValid() const
{
    if (schemaVersion != kHealthSchemaVersion || snapshotId.isNull()
        || !observedAt.isValid() || !validCapabilityState(aggregateState)) {
        return false;
    }

    QSet<QString> componentIds;
    for (const ComponentHealthRecord &component : components) {
        const QString id = component.componentId.trimmed();
        if (!component.isValid() || component.observedAt > observedAt
            || componentIds.contains(id)) {
            return false;
        }
        componentIds.insert(id);
    }

    QSet<QString> capabilityIds;
    for (const CapabilityDeficit &deficit : deficits) {
        const QString id = deficit.capabilityId.trimmed();
        if (!deficit.isValid() || deficit.detectedAt > observedAt
            || !componentIds.contains(deficit.dependencyId.trimmed())
            || capabilityIds.contains(id)) {
            return false;
        }
        capabilityIds.insert(id);
    }

    return deficits.isEmpty() == (aggregateState == CapabilityState::Available);
}

QByteArray encodeCapabilitySnapshot(const CapabilitySnapshot &snapshot)
{
    QCborMap root;
    root.insert(QStringLiteral("schemaVersion"), snapshot.schemaVersion);
    root.insert(QStringLiteral("snapshotId"), snapshot.snapshotId.toString(QUuid::WithoutBraces));
    root.insert(QStringLiteral("observedAt"), timestamp(snapshot.observedAt));
    root.insert(QStringLiteral("aggregateState"), static_cast<qint64>(snapshot.aggregateState));

    QCborArray components;
    for (const ComponentHealthRecord &component : snapshot.components) {
        QCborMap item;
        item.insert(QStringLiteral("componentId"), component.componentId);
        item.insert(QStringLiteral("state"), static_cast<qint64>(component.state));
        item.insert(QStringLiteral("observedAt"), timestamp(component.observedAt));
        item.insert(QStringLiteral("lastVerifiedAt"), timestamp(component.lastVerifiedAt));
        item.insert(QStringLiteral("detail"), component.detail);
        components.append(item);
    }
    root.insert(QStringLiteral("components"), components);

    QCborArray deficits;
    for (const CapabilityDeficit &deficit : snapshot.deficits) {
        QCborMap item;
        item.insert(QStringLiteral("capabilityId"), deficit.capabilityId);
        item.insert(QStringLiteral("dependencyId"), deficit.dependencyId);
        item.insert(QStringLiteral("state"), static_cast<qint64>(deficit.state));
        item.insert(QStringLiteral("cause"), static_cast<qint64>(deficit.cause));
        item.insert(QStringLiteral("detectedAt"), timestamp(deficit.detectedAt));
        item.insert(QStringLiteral("lastVerifiedAt"), timestamp(deficit.lastVerifiedAt));
        item.insert(QStringLiteral("impact"), deficit.impact);
        item.insert(QStringLiteral("recoveryPolicy"), static_cast<qint64>(deficit.recoveryPolicy));
        item.insert(QStringLiteral("evidenceId"), deficit.evidenceId.toString(QUuid::WithoutBraces));
        item.insert(QStringLiteral("errorReference"), deficit.errorReference);
        deficits.append(item);
    }
    root.insert(QStringLiteral("deficits"), deficits);
    return root.toCborValue().toCbor();
}

CapabilitySnapshot decodeCapabilitySnapshot(const QByteArray &encoded, QString *error)
{
    if (error) {
        error->clear();
    }
    const QCborValue value = QCborValue::fromCbor(encoded);
    if (!value.isMap()) {
        setError(error, QStringLiteral("health payload is not a CBOR map"));
        return {};
    }

    const QCborMap root = value.toMap();
    if (!requiredMapFields(
            root,
            {QStringLiteral("schemaVersion"), QStringLiteral("snapshotId"),
             QStringLiteral("observedAt"), QStringLiteral("aggregateState"),
             QStringLiteral("components"), QStringLiteral("deficits")},
            error,
            QStringLiteral("health snapshot"))) {
        return {};
    }

    if (!integerInRange(root.value(QStringLiteral("schemaVersion")), 1, 1)
        || !integerInRange(root.value(QStringLiteral("aggregateState")), 1, 6)) {
        setError(error, QStringLiteral("unsupported health schema or aggregate state"));
        return {};
    }

    CapabilitySnapshot snapshot;
    snapshot.schemaVersion = kHealthSchemaVersion;
    snapshot.snapshotId = QUuid(root.value(QStringLiteral("snapshotId")).toString());
    snapshot.observedAt = parseTimestamp(root.value(QStringLiteral("observedAt")));
    snapshot.aggregateState = static_cast<CapabilityState>(root.value(QStringLiteral("aggregateState")).toInteger());

    if (!root.value(QStringLiteral("components")).isArray()
        || !root.value(QStringLiteral("deficits")).isArray()) {
        setError(error, QStringLiteral("health components and deficits must be arrays"));
        return {};
    }

    for (const QCborValue &value : root.value(QStringLiteral("components")).toArray()) {
        if (!value.isMap()) {
            setError(error, QStringLiteral("health component is not a map"));
            return {};
        }
        const QCborMap item = value.toMap();
        if (!requiredMapFields(
                item,
                {QStringLiteral("componentId"), QStringLiteral("state"),
                 QStringLiteral("observedAt"), QStringLiteral("lastVerifiedAt"),
                 QStringLiteral("detail")},
                error,
                QStringLiteral("health component"))) {
            return {};
        }
        if (!integerInRange(item.value(QStringLiteral("state")), 1, 6)) {
            setError(error, QStringLiteral("unknown component health state"));
            return {};
        }
        ComponentHealthRecord component;
        component.componentId = item.value(QStringLiteral("componentId")).toString();
        component.state = static_cast<ComponentHealth>(item.value(QStringLiteral("state")).toInteger());
        component.observedAt = parseTimestamp(item.value(QStringLiteral("observedAt")));
        component.lastVerifiedAt = parseTimestamp(item.value(QStringLiteral("lastVerifiedAt")));
        component.detail = item.value(QStringLiteral("detail")).toString();
        snapshot.components.append(component);
    }

    for (const QCborValue &value : root.value(QStringLiteral("deficits")).toArray()) {
        if (!value.isMap()) {
            setError(error, QStringLiteral("capability deficit is not a map"));
            return {};
        }
        const QCborMap item = value.toMap();
        if (!requiredMapFields(
                item,
                {QStringLiteral("capabilityId"), QStringLiteral("dependencyId"),
                 QStringLiteral("state"), QStringLiteral("cause"),
                 QStringLiteral("detectedAt"), QStringLiteral("lastVerifiedAt"),
                 QStringLiteral("impact"), QStringLiteral("recoveryPolicy"),
                 QStringLiteral("evidenceId"), QStringLiteral("errorReference")},
                error,
                QStringLiteral("capability deficit"))) {
            return {};
        }
        if (!integerInRange(item.value(QStringLiteral("state")), 1, 6)
            || !integerInRange(item.value(QStringLiteral("cause")), 1, 7)
            || !integerInRange(item.value(QStringLiteral("recoveryPolicy")), 1, 5)) {
            setError(error, QStringLiteral("unknown capability deficit enum"));
            return {};
        }
        CapabilityDeficit deficit;
        deficit.capabilityId = item.value(QStringLiteral("capabilityId")).toString();
        deficit.dependencyId = item.value(QStringLiteral("dependencyId")).toString();
        deficit.state = static_cast<CapabilityState>(item.value(QStringLiteral("state")).toInteger());
        deficit.cause = static_cast<DeficitCause>(item.value(QStringLiteral("cause")).toInteger());
        deficit.detectedAt = parseTimestamp(item.value(QStringLiteral("detectedAt")));
        deficit.lastVerifiedAt = parseTimestamp(item.value(QStringLiteral("lastVerifiedAt")));
        deficit.impact = item.value(QStringLiteral("impact")).toString();
        deficit.recoveryPolicy = static_cast<RecoveryPolicy>(item.value(QStringLiteral("recoveryPolicy")).toInteger());
        deficit.evidenceId = QUuid(item.value(QStringLiteral("evidenceId")).toString());
        deficit.errorReference = item.value(QStringLiteral("errorReference")).toString();
        snapshot.deficits.append(deficit);
    }

    if (!snapshot.isValid()) {
        setError(error, QStringLiteral("invalid health capability snapshot"));
        return {};
    }
    return snapshot;
}

} // namespace cybou
