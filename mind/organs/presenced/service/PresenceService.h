// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/fabric/OrganClients.h"
#include "cybou/fabric/RpcResilience.h"
#include "cybou/ipc/EventClient.h"

#include <QDBusContext>
#include <QObject>
#include <QUuid>

#include <memory>

namespace cybou {

/// Outcome of the most recent compound projection.
///
/// presenced owns no domain state, so "the process is running" and "the process can present" are
/// different questions. Health1 graphs the `presence-presentation` capability from the Health()
/// answer below, which means a constant answer would make that capability incapable of ever
/// entering a deficit. This records what the last aggregation actually achieved so the answer can
/// reflect it.
enum class ProjectionOutcome {
    /// No projection has been attempted yet. Nothing has been observed to be wrong, so this is not
    /// a deficit: reporting one at startup would make every session begin degraded.
    NotAttempted,
    /// Every required owner answered inside the shared budget.
    Complete,
    /// Health1 did not answer, so capability states are unknown and every gated section of the
    /// projection was skipped rather than measured.
    CapabilitiesUnavailable,
    /// The shared deadline expired partway through collection. The projection is structurally
    /// valid but some sections carry typed defaults instead of observations.
    BudgetExhausted,
};

/// One in-flight compound Snapshot.
///
/// Snapshot is a delayed-reply D-Bus method: it returns immediately and the reply is sent when the
/// gather finishes, so presenced keeps serving its event loop instead of blocking inside a
/// sequential chain of downstream calls. Each concurrent caller owns its own instance, so
/// overlapping requests never share partial state.
struct SnapshotRequest;

class PresenceService
    : public QObject
    , protected QDBusContext
{
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.cybou.Mind.Presence1")

public:
    explicit PresenceService(QObject *parent = nullptr);

public Q_SLOTS:
    bool Ready() const;
    QString Health() const;
    QString LastError() const;

    QByteArray Snapshot() const;
    QByteArray Activity(int limit) const;
    QByteArray DetailedObligations() const;

    QString Promise(const QString &description);
    bool Reflect();
    bool FulfillIndex(int index);
    bool AbandonIndex(int index);
    bool Observe(const QString &subject, double value);
    QByteArray Predict(const QString &subject);
    bool InterruptLifecycle(const QString &cause);

Q_SIGNALS:
    void Changed();

private:
    bool appendUserObservation(
        const QString &event,
        const QVariantMap &details,
        QUuid *messageId,
        int timeoutMs = -1);

    QVariantMap healthMap(const CapabilitySnapshot &snapshot) const;

    /// Issue every capability-gated owner read at once and reply when the last one lands or the
    /// shared budget expires, whichever comes first.
    void gatherSnapshot(const std::shared_ptr<SnapshotRequest> &request) const;
    void finishSnapshot(const std::shared_ptr<SnapshotRequest> &request) const;
    QVariantMap assembleSnapshot(const SnapshotRequest &request) const;

    mutable QString m_lastError;
    mutable ProjectionOutcome m_lastProjection{ProjectionOutcome::NotAttempted};

    // Async transport for the read-only projection. These carry the typed outcomes, bounded retry
    // and circuit behavior that the synchronous clients below do not, and they are what makes the
    // gather concurrent rather than sequential. The synchronous clients remain in use for the
    // compound mutations, which are ordered by construction and are migrated separately.
    mutable AsyncRpcClient m_healthRpc;
    mutable AsyncRpcClient m_selfRpc;
    mutable AsyncRpcClient m_lifecycleRpc;
    mutable AsyncRpcClient m_intentionRpc;
    mutable AsyncRpcClient m_workspaceRpc;
    mutable AsyncRpcClient m_identityRpc;
    mutable AsyncRpcClient m_predictorRpc;
    mutable AsyncRpcClient m_eventRpc;

    EventClient m_events;
    HealthClient m_health;
    IdentityClient m_identity;
    IntentionClient m_intentions;
    PredictorClient m_predictor;
    SelfClient m_self;
    WorkspaceClient m_workspace;
    LifecycleClient m_lifecycle;
};

} // namespace cybou
