// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/fabric/OrganBus.h"

#include <QDBusConnection>
#include <QDBusMessage>
#include <QVariant>

namespace cybou {

class RpcClient
{
public:
    explicit RpcClient(const BusEndpoint &endpoint);

    bool ready() const;
    QString health() const;

    QDBusMessage call(
        const QString &method,
        const QVariantList &arguments = QVariantList(),
        int timeoutMs = -1) const;

    QByteArray callBytes(
        const QString &method,
        const QVariantList &arguments = QVariantList(),
        int timeoutMs = -1) const;

    bool callBool(
        const QString &method,
        const QVariantList &arguments = QVariantList(),
        int timeoutMs = -1) const;

    QString callString(
        const QString &method,
        const QVariantList &arguments = QVariantList(),
        int timeoutMs = -1) const;

    QString lastError() const { return m_lastError; }

private:
    BusEndpoint m_endpoint;
    QDBusConnection m_bus;
    mutable QString m_lastError;
};

} // namespace cybou
