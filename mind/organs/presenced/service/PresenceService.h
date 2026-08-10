// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/fabric/OrganClients.h"
#include "cybou/ipc/EventClient.h"

#include <QObject>
#include <QUuid>

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

class PresenceService : public QObject
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
    QVariantMap snapshotMap() const;

    mutable QString m_lastError;
    mutable ProjectionOutcome m_lastProjection{ProjectionOutcome::NotAttempted};

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
