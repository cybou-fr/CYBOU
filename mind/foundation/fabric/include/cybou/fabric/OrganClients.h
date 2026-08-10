// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/fabric/RpcClient.h"
#include "cybou/protocol/Health.h"
#include "cybou/protocol/Homeostasis.h"

#include <QObject>
#include <QVariant>

namespace cybou {

class HealthClient : public QObject
{
    Q_OBJECT

public:
    explicit HealthClient(QObject *parent = nullptr);

    bool ready() const { return m_rpc.ready(); }
    QString health() const { return m_rpc.health(); }
    CapabilitySnapshot snapshot(int timeoutMs = -1) const;
    HomeostasisSnapshot measurements() const;
    QString lastError() const;

Q_SIGNALS:
    void changed();

private Q_SLOTS:
    void onChanged();

private:
    mutable QString m_codecError;
    RpcClient m_rpc;
};

class IdentityClient
{
public:
    IdentityClient();

    bool ready() const { return m_rpc.ready(); }
    QString health() const { return m_rpc.health(); }
    QVariantMap state(int timeoutMs = -1) const;
    QString lastError() const;

private:
    mutable QString m_codecError;
    RpcClient m_rpc;
};

class IntentionClient
{
public:
    IntentionClient();

    bool ready() const { return m_rpc.ready(); }
    QString health() const { return m_rpc.health(); }

    QVariantList open(int timeoutMs = -1) const;
    QString form(
        const QString &description,
        const QString &trigger,
        const QString &causeId,
        int timeoutMs = -1) const;
    bool close(
        const QString &intentionId,
        int resolution,
        const QString &note = QString(),
        int timeoutMs = -1) const;

    QString lastError() const;

private:
    mutable QString m_codecError;
    RpcClient m_rpc;
};

class PredictorClient
{
public:
    PredictorClient();

    bool ready() const { return m_rpc.ready(); }
    QString health() const { return m_rpc.health(); }

    bool observe(const QString &subject, double value, int timeoutMs = -1) const;
    QVariantMap predict(
        const QString &subject,
        const QString &correlationId = QString(),
        int timeoutMs = -1) const;
    bool settle(const QString &forecastId, double actual) const;
    QVariantList calibrations(int timeoutMs = -1) const;

    QString lastError() const;

private:
    mutable QString m_codecError;
    RpcClient m_rpc;
};

class SelfClient
{
public:
    SelfClient();

    bool ready() const { return m_rpc.ready(); }
    QString health() const { return m_rpc.health(); }

    QVariantMap measure(int timeoutMs = -1) const;
    QVariantMap assess(const QString &causeId, int timeoutMs = -1) const;
    QString narration() const;

    QString lastError() const;

private:
    mutable QString m_codecError;
    RpcClient m_rpc;
};

class WorkspaceClient : public QObject
{
    Q_OBJECT

public:
    explicit WorkspaceClient(QObject *parent = nullptr);

    bool ready() const { return m_rpc.ready(); }
    QString health() const { return m_rpc.health(); }

    QVariantList coalitions(int timeoutMs = -1) const;
    QVariantMap moment(int timeoutMs = -1) const;
    QString attention(int timeoutMs = -1) const;

    QString lastError() const;

Q_SIGNALS:
    void changed();

private Q_SLOTS:
    void onChanged();

private:
    mutable QString m_codecError;
    RpcClient m_rpc;
};

class LifecycleClient : public QObject
{
    Q_OBJECT

public:
    explicit LifecycleClient(QObject *parent = nullptr);

    bool ready() const { return m_rpc.ready(); }
    QString health() const { return m_rpc.health(); }
    QVariantMap state(int timeoutMs = -1) const;
    QVariantMap schedulingEvaluation(int timeoutMs = -1) const;
    bool notifyUserActivity(const QString &cause, int timeoutMs = -1) const;
    bool finishRun(
        const QString &status,
        const QString &cause,
        int timeoutMs = -1) const;
    QString lastError() const;

Q_SIGNALS:
    void changed();

private Q_SLOTS:
    void onChanged();

private:
    mutable QString m_codecError;
    RpcClient m_rpc;
};

class PresenceClient : public QObject
{
    Q_OBJECT

public:
    explicit PresenceClient(QObject *parent = nullptr);

    bool ready() const { return m_rpc.ready(); }
    QString health() const { return m_rpc.health(); }

    QVariantMap snapshot() const;
    QVariantList activity(int limit) const;
    QVariantList detailedObligations() const;

    QString promise(const QString &description) const;
    bool reflect() const;
    bool fulfillIndex(int index) const;
    bool abandonIndex(int index) const;
    bool observe(const QString &subject, double value) const;
    QVariantMap predict(const QString &subject) const;

    QString lastError() const;

Q_SIGNALS:
    void changed();

private Q_SLOTS:
    void onChanged();

private:
    mutable QString m_codecError;
    RpcClient m_rpc;
};

} // namespace cybou
