// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/fabric/OrganClients.h"
#include "cybou/fabric/RpcResilience.h"

#include <QDBusContext>
#include <QObject>
#include <QUuid>
#include <QVariant>

#include <functional>
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

/// One in-flight compound mutation.
///
/// Unlike the projection, a mutation's steps are causally ordered: the capability gate decides
/// whether the command is legal, the Event1 preflight decides whether it can be made durable, and
/// the durable Observation is the cause the domain mutation is linked to. Running them concurrently
/// would be wrong. Running them asynchronously is not - each step continues from the previous one's
/// reply instead of blocking the thread waiting for it.
struct CommandRequest;

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
    /// Shared prefix of every user-initiated mutation: capability gate, Event1 durability preflight,
    /// lifecycle activity notification, and the durable Observation that records what the user
    /// asked for. `committed` receives the accepted Observation's message id, which the caller links
    /// its domain mutation to as the cause. Any failure replies with `failureValue` and `committed`
    /// is not run.
    void beginUserCommand(
        const std::shared_ptr<CommandRequest> &request,
        const QStringList &requiredCapabilities,
        const QString &activityCause,
        const QString &observationEvent,
        const QVariantMap &observationDetails,
        const QVariant &failureValue,
        std::function<void(const QUuid &causeId)> committed) const;

    /// Send the delayed reply exactly once.
    void replyOnce(
        const std::shared_ptr<CommandRequest> &request,
        const QVariant &value) const;

    std::shared_ptr<CommandRequest> beginRequest() const;

    void gatedRead(
        const std::shared_ptr<CommandRequest> &request,
        const QStringList &requiredCapabilities,
        AsyncRpcClient &client,
        const QString &method,
        const QVariantList &arguments,
        std::function<QVariant(const RpcResult &)> project,
        const QVariant &failureValue) const;

    void closeIntentionAtIndex(
        const std::shared_ptr<CommandRequest> &request,
        int index,
        int resolution,
        const QString &commandId,
        const QString &activityCause,
        const QString &observationEvent) const;

    QVariantMap healthMap(const CapabilitySnapshot &snapshot) const;

    /// Issue every capability-gated owner read at once and reply when the last one lands or the
    /// shared budget expires, whichever comes first.
    void gatherSnapshot(const std::shared_ptr<SnapshotRequest> &request) const;
    void finishSnapshot(const std::shared_ptr<SnapshotRequest> &request) const;
    QVariantMap assembleSnapshot(const SnapshotRequest &request) const;

    mutable QString m_lastError;
    mutable ProjectionOutcome m_lastProjection{ProjectionOutcome::NotAttempted};

    // Transport for the whole Presence surface. Every projection, read and mutation goes through
    // these, so no call on this object blocks the thread.
    //
    // The policy is single-attempt and non-latching. Safety for mutations comes from the operation
    // semantics rather than the policy: a NonIdempotentMutation is never retried whatever the
    // policy says, and a timeout on one surfaces as unknown-outcome rather than failure, which is
    // the contract the shell relies on for InterruptLifecycle.
    mutable AsyncRpcClient m_healthRpc;
    mutable AsyncRpcClient m_selfRpc;
    mutable AsyncRpcClient m_lifecycleRpc;
    mutable AsyncRpcClient m_intentionRpc;
    mutable AsyncRpcClient m_workspaceRpc;
    mutable AsyncRpcClient m_identityRpc;
    mutable AsyncRpcClient m_predictorRpc;
    mutable AsyncRpcClient m_eventRpc;
    mutable AsyncRpcClient m_epistemicRpc;

    // Retained only for their Changed subscriptions, which is how presenced learns to re-emit its
    // own Changed signal. They no longer carry any call.
    HealthClient m_health;
    WorkspaceClient m_workspace;
    LifecycleClient m_lifecycle;
};

} // namespace cybou
