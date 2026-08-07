// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/fabric/RpcClient.h"

#include <QObject>
#include <QVariant>

namespace cybou {

class IdentityClient
{
public:
    IdentityClient();

    bool ready() const { return m_rpc.ready(); }
    QString health() const { return m_rpc.health(); }
    QVariantMap state() const;
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

    QVariantList open() const;
    QString form(
        const QString &description,
        const QString &trigger,
        const QString &causeId) const;
    bool close(
        const QString &intentionId,
        int resolution,
        const QString &note = QString()) const;

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

    bool observe(const QString &subject, double value) const;
    QVariantMap predict(
        const QString &subject,
        const QString &correlationId = QString()) const;
    bool settle(const QString &forecastId, double actual) const;
    QVariantList calibrations() const;

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

    QVariantMap measure() const;
    QVariantMap assess(const QString &causeId) const;
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

    QVariantList coalitions() const;
    QVariantMap moment() const;
    QString attention() const;

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
