// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/protocol/Health.h"
#include "cybou/protocol/Homeostasis.h"
#include "cybou/protocol/Lifecycle.h"

#include <QDateTime>
#include <QStringList>
#include <QVariantMap>

namespace cybou {

enum class SchedulingDecision { Run, Defer, Block };

struct SchedulingEvaluation {
    SchedulingDecision decision{SchedulingDecision::Block};
    QString policyId{QStringLiteral("event-backlog-v1")};
    QString reason;
    QDateTime observedAt;
    QUuid capabilitySnapshotId;
    QUuid homeostasisSnapshotId;
    bool pressureLatched{false};
    QStringList eligibleWorkers;
    QVariantMap missingWorkers;

    QVariantMap toMap() const;
};

class LifecycleSchedulingPolicy
{
public:
    static constexpr double kEnterBacklog = 32.0;
    static constexpr double kExitBacklog = 8.0;
    static constexpr qint64 kMaximumCapabilityAgeSeconds = 60;

    static SchedulingEvaluation evaluate(
        LifecycleMode mode,
        bool hasActiveRun,
        const CapabilitySnapshot &capabilities,
        const HomeostasisSnapshot &homeostasis,
        bool pressureLatched = false,
        const QDateTime &now = QDateTime::currentDateTimeUtc());
};

} // namespace cybou
