// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include <QByteArray>
#include <QDateTime>
#include <QList>
#include <QString>
#include <QUuid>

namespace cybou {

inline constexpr quint16 kHealthSchemaVersion = 2;

enum class ComponentHealth : quint8 {
    Starting = 1,
    Healthy,
    Degraded,
    Unavailable,
    Recovering,
    Conflicted,
};

enum class CapabilityState : quint8 {
    Available = 1,
    Limited,
    Unavailable,
    Unknown,
    Stale,
    Recovering,
};

enum class DeficitCause : quint8 {
    DependencyUnavailable = 1,
    DependencyDegraded,
    TimedOut,
    Rejected,
    UnknownOutcome,
    StaleEvidence,
    ConflictingState,
};

enum class RecoveryPolicy : quint8 {
    None = 1,
    Observe,
    RetryIdempotent,
    Reconcile,
    OperatorRequired,
};

QString componentHealthToString(ComponentHealth health);
QString capabilityStateToString(CapabilityState state);
QString deficitCauseToString(DeficitCause cause);
QString recoveryPolicyToString(RecoveryPolicy policy);

bool canTransition(ComponentHealth from, ComponentHealth to) noexcept;

struct ComponentHealthRecord {
    QString componentId;
    ComponentHealth state{ComponentHealth::Starting};
    QDateTime observedAt;
    QDateTime lastVerifiedAt;
    QString detail;

    bool isValid() const;
};

struct CapabilityDeficit {
    QString capabilityId;
    QString dependencyId;
    CapabilityState state{CapabilityState::Unknown};
    DeficitCause cause{DeficitCause::DependencyUnavailable};
    QDateTime detectedAt;
    QDateTime lastVerifiedAt;
    QString impact;
    RecoveryPolicy recoveryPolicy{RecoveryPolicy::Observe};
    QUuid evidenceId;
    QString errorReference;

    bool isValid() const;
};

struct CapabilitySnapshot {
    quint16 schemaVersion{kHealthSchemaVersion};
    QUuid snapshotId;
    QDateTime observedAt;
    CapabilityState aggregateState{CapabilityState::Unknown};
    QList<ComponentHealthRecord> components;
    QList<CapabilityDeficit> deficits;

    bool isValid() const;
};

QByteArray encodeCapabilitySnapshot(const CapabilitySnapshot &snapshot);
CapabilitySnapshot decodeCapabilitySnapshot(
    const QByteArray &encoded,
    QString *error = nullptr);

} // namespace cybou
