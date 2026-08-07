// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/fabric/OrganClients.h"
#include "cybou/ipc/EventClient.h"

#include <QObject>
#include <QUuid>

namespace cybou {

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

Q_SIGNALS:
    void Changed();

private:
    bool appendUserObservation(
        const QString &event,
        const QVariantMap &details,
        QUuid *messageId);

    QVariantMap healthMap() const;
    QVariantMap snapshotMap() const;

    mutable QString m_lastError;

    EventClient m_events;
    IdentityClient m_identity;
    IntentionClient m_intentions;
    PredictorClient m_predictor;
    SelfClient m_self;
    WorkspaceClient m_workspace;
};

} // namespace cybou
