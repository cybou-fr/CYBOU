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

public:
    explicit Presence(QObject *parent = nullptr);
    ~Presence() override;

    Q_INVOKABLE bool wake();
    bool isAwake() const { return m_awake; }

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

    QString lastError() const { return m_lastError; }

Q_SIGNALS:
    void changed();

private:
    bool refresh();
    void remoteChanged();

    std::unique_ptr<PresenceClient> m_client;
    QVariantMap m_snapshot;
    QString m_lastError;
    bool m_awake{false};
};

} // namespace cybou

Q_DECLARE_METATYPE(cybou::Moment)
