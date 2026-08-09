// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "LifecycleSchedulingPolicy.h"

#include <utility>

namespace cybou {
namespace {

QString decisionName(SchedulingDecision decision)
{
    switch (decision) {
    case SchedulingDecision::Run: return QStringLiteral("run");
    case SchedulingDecision::Defer: return QStringLiteral("defer");
    case SchedulingDecision::Block: return QStringLiteral("block");
    }
    return QStringLiteral("block");
}

const CapabilityDeficit *deficitFor(
    const CapabilitySnapshot &snapshot,
    const QString &capabilityId)
{
    for (const CapabilityDeficit &deficit : snapshot.deficits)
        if (deficit.capabilityId == capabilityId) return &deficit;
    return nullptr;
}

const HomeostaticMeasurement *measurementFor(
    const HomeostasisSnapshot &snapshot,
    const QString &metricId)
{
    for (const HomeostaticMeasurement &measurement : snapshot.measurements)
        if (measurement.metricId == metricId) return &measurement;
    return nullptr;
}

} // namespace

QVariantMap SchedulingEvaluation::toMap() const
{
    return {
        {QStringLiteral("decision"), decisionName(decision)},
        {QStringLiteral("policyId"), policyId},
        {QStringLiteral("reason"), reason},
        {QStringLiteral("observedAt"), observedAt},
        {QStringLiteral("capabilitySnapshotId"),
         capabilitySnapshotId.toString(QUuid::WithoutBraces)},
        {QStringLiteral("homeostasisSnapshotId"),
         homeostasisSnapshotId.toString(QUuid::WithoutBraces)},
        {QStringLiteral("pressureLatched"), pressureLatched},
        {QStringLiteral("eligibleWorkers"), eligibleWorkers},
        {QStringLiteral("missingWorkers"), missingWorkers},
    };
}

SchedulingEvaluation LifecycleSchedulingPolicy::evaluate(
    LifecycleMode mode,
    bool hasActiveRun,
    const CapabilitySnapshot &capabilities,
    const HomeostasisSnapshot &homeostasis,
    bool pressureLatched,
    const QDateTime &now)
{
    SchedulingEvaluation result;
    result.observedAt = now.toUTC();
    result.capabilitySnapshotId = capabilities.snapshotId;
    result.homeostasisSnapshotId = homeostasis.snapshotId;
    result.pressureLatched = pressureLatched;

    if (!capabilities.isValid()) {
        result.reason = QStringLiteral("capability snapshot is unavailable or invalid");
        return result;
    }
    const qint64 capabilityAge = capabilities.observedAt.secsTo(now);
    if (capabilityAge < 0 || capabilityAge > kMaximumCapabilityAgeSeconds) {
        result.reason = QStringLiteral("capability snapshot is outside the 60 second policy window");
        return result;
    }
    if (!homeostasis.isValid()) {
        result.reason = QStringLiteral("homeostasis snapshot is unavailable or invalid");
        return result;
    }
    if (mode != LifecycleMode::Idle || hasActiveRun) {
        result.decision = SchedulingDecision::Defer;
        result.reason = QStringLiteral("lifecycle is not idle");
        return result;
    }

    if (const CapabilityDeficit *biography = deficitFor(
            capabilities, QStringLiteral("accepted-biography"))) {
        result.reason = QStringLiteral("required capability accepted-biography is %1: %2")
                            .arg(capabilityStateToString(biography->state), biography->impact);
        return result;
    }

    for (const auto &[worker, capability] : {
             std::pair{QStringLiteral("predictor"), QStringLiteral("prediction")},
             std::pair{QStringLiteral("workspace"), QStringLiteral("attention-workspace")}}) {
        if (const CapabilityDeficit *deficit = deficitFor(capabilities, capability)) {
            result.missingWorkers[worker] = QStringLiteral("%1: %2")
                                                .arg(capabilityStateToString(deficit->state),
                                                     deficit->impact);
        } else {
            result.eligibleWorkers.append(worker);
        }
    }
    if (result.eligibleWorkers.isEmpty()) {
        result.reason = QStringLiteral("no consolidation worker capability is available");
        return result;
    }

    const HomeostaticMeasurement *backlog = measurementFor(
        homeostasis, QStringLiteral("event.backlog.count"));
    if (!backlog || backlog->status != MeasurementStatus::Current || !backlog->hasValue
        || backlog->validUntil < now) {
        result.decision = SchedulingDecision::Defer;
        result.reason = backlog && !backlog->reason.isEmpty()
            ? QStringLiteral("event backlog cannot schedule work: %1").arg(backlog->reason)
            : QStringLiteral("event backlog is not a current supported measurement");
        return result;
    }

    result.pressureLatched = pressureLatched
        ? backlog->value > kExitBacklog
        : backlog->value >= kEnterBacklog;
    if (!result.pressureLatched) {
        result.decision = SchedulingDecision::Defer;
        result.reason = QStringLiteral("event backlog is below scheduling hysteresis");
        return result;
    }
    if (!homeostasis.authorizes(result.policyId)) {
        result.decision = SchedulingDecision::Defer;
        result.reason = QStringLiteral("homeostasis snapshot does not authorize this policy");
        return result;
    }

    result.decision = SchedulingDecision::Run;
    result.reason = QStringLiteral("current event backlog exceeds scheduling hysteresis");
    return result;
}

} // namespace cybou
