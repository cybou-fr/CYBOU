// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/fabric/ServiceHost.h"

#include <QDBusConnection>
#include <QDBusError>

namespace cybou {

namespace {

void setError(QString *error, const QString &message)
{
    if (error) {
        *error = message;
    }
}

QString text(const char *value)
{
    return QString::fromLatin1(value);
}

} // namespace

bool ServiceHost::publish(
    QObject *object,
    const BusEndpoint &endpoint,
    QString *error)
{
    if (error) {
        error->clear();
    }

    QDBusConnection bus = QDBusConnection::sessionBus();
    if (!bus.isConnected()) {
        setError(
            error,
            QStringLiteral("the user D-Bus session is unavailable"));
        return false;
    }

    if (!bus.registerObject(
            text(endpoint.objectPath),
            object,
            QDBusConnection::ExportAllSlots
                | QDBusConnection::ExportAllSignals)) {
        setError(
            error,
            QStringLiteral("cannot register %1 object: %2")
                .arg(
                    text(endpoint.interfaceName),
                    bus.lastError().message()));
        return false;
    }

    if (!bus.registerService(text(endpoint.service))) {
        setError(
            error,
            QStringLiteral("cannot own %1: %2")
                .arg(
                    text(endpoint.service),
                    bus.lastError().message()));
        bus.unregisterObject(text(endpoint.objectPath));
        return false;
    }

    return true;
}

} // namespace cybou
