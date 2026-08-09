// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "HealthPolicy.h"

#include <algorithm>

namespace cybou {
namespace {

CapabilityState stateFor(ComponentHealth health)
{
    switch (health) {
    case ComponentHealth::Healthy: return CapabilityState::Available;
    case ComponentHealth::Starting:
    case ComponentHealth::Recovering: return CapabilityState::Recovering;
    case ComponentHealth::Degraded: return CapabilityState::Limited;
    case ComponentHealth::Unavailable: return CapabilityState::Unavailable;
    case ComponentHealth::Conflicted: return CapabilityState::Unknown;
    }
    return CapabilityState::Unknown;
}

DeficitCause causeFor(ComponentHealth health, const QString &detail)
{
    if (detail.startsWith(QStringLiteral("timed-out"))) return DeficitCause::TimedOut;
    if (detail.startsWith(QStringLiteral("rejected"))) return DeficitCause::Rejected;
    if (detail.startsWith(QStringLiteral("unknown-outcome"))) return DeficitCause::UnknownOutcome;
    switch (health) {
    case ComponentHealth::Degraded: return DeficitCause::DependencyDegraded;
    case ComponentHealth::Conflicted: return DeficitCause::ConflictingState;
    case ComponentHealth::Starting:
    case ComponentHealth::Recovering: return DeficitCause::UnknownOutcome;
    case ComponentHealth::Healthy:
    case ComponentHealth::Unavailable: return DeficitCause::DependencyUnavailable;
    }
    return DeficitCause::DependencyUnavailable;
}

int severity(CapabilityState state)
{
    switch (state) {
    case CapabilityState::Available: return 0;
    case CapabilityState::Recovering: return 1;
    case CapabilityState::Limited: return 2;
    case CapabilityState::Stale: return 3;
    case CapabilityState::Unknown: return 4;
    case CapabilityState::Unavailable: return 5;
    }
    return 5;
}

} // namespace

QList<CapabilityDefinition> HealthPolicy::definitions()
{
    return {
        {QStringLiteral("accepted-biography"), {QStringLiteral("eventd")}, true,
         QStringLiteral("accepted cognitive history is unavailable")},
        {QStringLiteral("identity-continuity"),
         {QStringLiteral("eventd"), QStringLiteral("identityd")}, true,
         QStringLiteral("identity continuity cannot be verified")},
        {QStringLiteral("commitment-access"),
         {QStringLiteral("eventd"), QStringLiteral("intentiond")}, true,
         QStringLiteral("accepted commitments are unavailable")},
        {QStringLiteral("prediction"), {QStringLiteral("predictord")}, false,
         QStringLiteral("new predictions are unavailable")},
        {QStringLiteral("self-assessment"), {QStringLiteral("selfd")}, false,
         QStringLiteral("self assessment is unavailable")},
        {QStringLiteral("attention-workspace"), {QStringLiteral("workspaced")}, false,
         QStringLiteral("bounded attention is unavailable")},
        {QStringLiteral("consolidation"),
         {QStringLiteral("lifecycled"), QStringLiteral("predictord"),
          QStringLiteral("workspaced")}, false,
         QStringLiteral("consolidation is limited by an unavailable owner")},
        {QStringLiteral("presence-presentation"), {QStringLiteral("presenced")}, false,
         QStringLiteral("Mind presentation is unavailable")},
    };
}

QStringList HealthPolicy::componentIds()
{
    return {
        QStringLiteral("eventd"),
        QStringLiteral("lifecycled"),
        QStringLiteral("identityd"),
        QStringLiteral("intentiond"),
        QStringLiteral("predictord"),
        QStringLiteral("selfd"),
        QStringLiteral("workspaced"),
        QStringLiteral("presenced"),
    };
}

CapabilitySnapshot HealthPolicy::evaluate(
    const QMap<QString, ComponentHealthRecord> &observations,
    const QDateTime &observedAt)
{
    CapabilitySnapshot snapshot;
    snapshot.snapshotId = QUuid::createUuid();
    snapshot.observedAt = observedAt.toUTC();

    for (const QString &componentId : componentIds()) {
        ComponentHealthRecord record = observations.value(componentId);
        if (record.componentId.isEmpty()) {
            record.componentId = componentId;
            record.state = ComponentHealth::Unavailable;
            record.observedAt = snapshot.observedAt;
            record.detail = QStringLiteral("component was not observed");
        }
        snapshot.components.append(record);
    }

    bool requiredDeficit = false;
    int aggregateSeverity = 0;
    for (const CapabilityDefinition &definition : definitions()) {
        for (const QString &dependency : definition.dependencies) {
            const ComponentHealth health = observations.contains(dependency)
                ? observations.value(dependency).state
                : ComponentHealth::Unavailable;
            const CapabilityState state = stateFor(health);
            if (state == CapabilityState::Available) continue;

            CapabilityDeficit deficit;
            deficit.capabilityId = definition.capabilityId;
            deficit.dependencyId = dependency;
            deficit.state = state;
            const ComponentHealthRecord record = observations.value(dependency);
            deficit.cause = causeFor(health, record.detail);
            deficit.detectedAt = snapshot.observedAt;
            deficit.lastVerifiedAt = record.lastVerifiedAt;
            deficit.impact = definition.unavailableImpact;
            deficit.recoveryPolicy = state == CapabilityState::Recovering
                ? RecoveryPolicy::Reconcile
                : RecoveryPolicy::Observe;
            deficit.errorReference = record.detail;
            snapshot.deficits.append(deficit);
            requiredDeficit = requiredDeficit || definition.required;
            aggregateSeverity = std::max(aggregateSeverity, severity(state));
        }
    }

    if (snapshot.deficits.isEmpty()) {
        snapshot.aggregateState = CapabilityState::Available;
    } else if (requiredDeficit && aggregateSeverity >= severity(CapabilityState::Unknown)) {
        snapshot.aggregateState = aggregateSeverity == severity(CapabilityState::Unavailable)
            ? CapabilityState::Unavailable
            : CapabilityState::Unknown;
    } else if (aggregateSeverity == severity(CapabilityState::Recovering)) {
        snapshot.aggregateState = CapabilityState::Recovering;
    } else {
        snapshot.aggregateState = CapabilityState::Limited;
    }
    return snapshot;
}

} // namespace cybou
