// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include <QDateTime>
#include <QObject>
#include <QVariant>
#include <QStringList>
#include <QUuid>

#include <memory>

namespace cybou {

class PresenceClient;
class AsyncRpcClient;

struct Moment {
    QDateTime when;
    QString organ;
    QString kind;
    QUuid thread;
};

/// QML proxy only. No mutable cognition is constructed in plasmashell.
class Presence : public QObject
{
    Q_OBJECT

    Q_PROPERTY(bool awake READ isAwake NOTIFY changed)
    Q_PROPERTY(bool runtimeReachable READ runtimeReachable NOTIFY changed)
    Q_PROPERTY(QString aggregateCapabilityState READ aggregateCapabilityState NOTIFY changed)
    Q_PROPERTY(QVariantMap capabilityStates READ capabilityStates NOTIFY changed)
    Q_PROPERTY(QVariantMap capabilityDetails READ capabilityDetails NOTIFY changed)
    Q_PROPERTY(QVariantList capabilityDeficits READ capabilityDeficits NOTIFY changed)
    Q_PROPERTY(QDateTime capabilityObservedAt READ capabilityObservedAt NOTIFY changed)
    Q_PROPERTY(QVariantMap commandAvailability READ commandAvailability NOTIFY changed)
    Q_PROPERTY(QString lastError READ lastError NOTIFY changed)
    Q_PROPERTY(QString narration READ narration NOTIFY changed)
    Q_PROPERTY(QStringList obligations READ obligations NOTIFY changed)
    Q_PROPERTY(QString attention READ attention NOTIFY changed)
    Q_PROPERTY(int contributions READ contributions NOTIFY changed)
    Q_PROPERTY(QVariantMap stats READ stats NOTIFY changed)
    Q_PROPERTY(QVariantMap identityState READ identityState NOTIFY changed)
    Q_PROPERTY(QVariantList calibrations READ calibrations NOTIFY changed)
    Q_PROPERTY(QVariantList coalitions READ coalitions NOTIFY changed)
    Q_PROPERTY(QVariantMap moment READ moment NOTIFY changed)
    Q_PROPERTY(QVariantMap organHealth READ organHealth NOTIFY changed)
    Q_PROPERTY(QString lifecycleMode READ lifecycleMode NOTIFY changed)
    Q_PROPERTY(QString lifecycleStatus READ lifecycleStatus NOTIFY changed)
    Q_PROPERTY(QVariantMap lifecycleState READ lifecycleState NOTIFY changed)
    Q_PROPERTY(QVariantMap lifecycleProjection READ lifecycleProjection NOTIFY changed)
    Q_PROPERTY(QVariantMap lifecycleScheduling READ lifecycleScheduling NOTIFY changed)
    Q_PROPERTY(bool lifecycleCommandPending READ lifecycleCommandPending NOTIFY changed)

public:
    explicit Presence(QObject *parent = nullptr);
    ~Presence() override;

    Q_INVOKABLE bool wake();
    bool isAwake() const { return m_awake; }
    bool runtimeReachable() const;
    QString aggregateCapabilityState() const;
    QVariantMap capabilityStates() const;
    QVariantMap capabilityDetails() const;
    QVariantList capabilityDeficits() const;
    QDateTime capabilityObservedAt() const;
    QVariantMap commandAvailability() const;
    Q_INVOKABLE bool hasCapability(const QString &capabilityId) const;
    Q_INVOKABLE bool canCommand(const QString &commandId) const;

    QString narration() const;
    QStringList obligations() const;
    QString attention() const;
    int contributions() const;

    QList<Moment> recent(int limit = 12) const;
    Q_INVOKABLE QVariantList activity(int limit = 12) const;

    Q_INVOKABLE QUuid promise(const QString &description);
    Q_INVOKABLE bool reflect();
    Q_INVOKABLE bool fulfillIndex(int index);
    Q_INVOKABLE bool abandonIndex(int index);
    Q_INVOKABLE QVariantList detailedObligations() const;
    Q_INVOKABLE bool observe(const QString &subject, double value);

    Q_INVOKABLE QVariantMap stats() const;
    Q_INVOKABLE QVariantMap identityState() const;
    Q_INVOKABLE QVariantList calibrations() const;
    Q_INVOKABLE QVariantMap predict(const QString &subject);
    Q_INVOKABLE QVariantList coalitions() const;
    Q_INVOKABLE QVariantMap moment() const;
    Q_INVOKABLE QVariantMap organHealth() const;
    QString lifecycleMode() const;
    QString lifecycleStatus() const;
    QVariantMap lifecycleState() const;
    QVariantMap lifecycleProjection() const;
    QVariantMap lifecycleScheduling() const;
    bool lifecycleCommandPending() const { return m_lifecycleCommandPending; }
    Q_INVOKABLE void interruptLifecycle(const QString &cause = QString());

    QString lastError() const { return m_lastError; }

Q_SIGNALS:
    void changed();

private:
    bool refresh();
    void remoteChanged();

    std::unique_ptr<PresenceClient> m_client;
    std::unique_ptr<AsyncRpcClient> m_resilientClient;
    QVariantMap m_snapshot;
    QString m_lastError;
    bool m_awake{false};
    bool m_lifecycleCommandPending{false};
};

} // namespace cybou

Q_DECLARE_METATYPE(cybou::Moment)
