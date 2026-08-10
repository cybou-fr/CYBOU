// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/fabric/RpcClient.h"

#include <QDBusPendingCall>

namespace cybou {

namespace {

constexpr int kCallTimeoutMs = 5000;

QString text(const char *value)
{
    return QString::fromLatin1(value);
}

} // namespace

RpcClient::RpcClient(const BusEndpoint &endpoint)
    : m_endpoint(endpoint)
    , m_bus(QDBusConnection::sessionBus())
{
    if (!m_bus.isConnected()) {
        m_lastError =
            QStringLiteral("the user D-Bus session is unavailable");
    }
}

QDBusMessage RpcClient::call(
    const QString &method,
    const QVariantList &arguments,
    int timeoutMs) const
{
    if (!m_bus.isConnected()) {
        m_lastError =
            QStringLiteral("the user D-Bus session is unavailable");
        return {};
    }

    QDBusMessage message = QDBusMessage::createMethodCall(
        text(m_endpoint.service),
        text(m_endpoint.objectPath),
        text(m_endpoint.interfaceName),
        method);
    message.setArguments(arguments);

    const int boundedTimeout = timeoutMs > 0 ? timeoutMs : kCallTimeoutMs;
    QDBusPendingCall pending = m_bus.asyncCall(message, boundedTimeout);
    pending.waitForFinished();
    const QDBusMessage reply = pending.reply();

    if (reply.type() == QDBusMessage::ErrorMessage) {
        m_lastError = QStringLiteral("%1: %2")
                          .arg(
                              reply.errorName(),
                              reply.errorMessage());
    } else {
        m_lastError.clear();
    }

    return reply;
}

QByteArray RpcClient::callBytes(
    const QString &method,
    const QVariantList &arguments,
    int timeoutMs) const
{
    const QDBusMessage reply = call(method, arguments, timeoutMs);
    if (reply.type() == QDBusMessage::ErrorMessage
        || reply.arguments().isEmpty()) {
        return {};
    }
    return reply.arguments().first().toByteArray();
}

bool RpcClient::callBool(
    const QString &method,
    const QVariantList &arguments,
    int timeoutMs) const
{
    const QDBusMessage reply = call(method, arguments, timeoutMs);
    if (reply.type() == QDBusMessage::ErrorMessage
        || reply.arguments().isEmpty()) {
        return false;
    }
    return reply.arguments().first().toBool();
}

QString RpcClient::callString(
    const QString &method,
    const QVariantList &arguments,
    int timeoutMs) const
{
    const QDBusMessage reply = call(method, arguments, timeoutMs);
    if (reply.type() == QDBusMessage::ErrorMessage
        || reply.arguments().isEmpty()) {
        return {};
    }
    return reply.arguments().first().toString();
}

bool RpcClient::ready() const
{
    return callBool(QStringLiteral("Ready"));
}

QString RpcClient::health() const
{
    return callString(QStringLiteral("Health"));
}

} // namespace cybou
