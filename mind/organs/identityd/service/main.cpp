// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "IdentityService.h"

#include "cybou/fabric/OrganBus.h"
#include "cybou/fabric/ServiceHost.h"
#include "cybou/ipc/EventClient.h"
#include "cybou/runtime/StatePaths.h"

#include <QCoreApplication>
#include <QDir>
#include <QTextStream>

namespace {

int fail(const QString &message, int code)
{
    QTextStream(stderr)
        << "cybou-identityd: " << message << Qt::endl;
    return code;
}

} // namespace

int main(int argc, char **argv)
{
    QCoreApplication app(argc, argv);
    QCoreApplication::setApplicationName(QStringLiteral("cybou-identityd"));
    QCoreApplication::setOrganizationName(QStringLiteral("Cybou"));

    cybou::EventClient events;
    if (!events.isOpen()) {
        return fail(events.lastError(), 2);
    }

    const QString runtimeRoot =
        cybou::StatePaths::runtimeRoot();
    if (runtimeRoot.isEmpty() || !QDir().mkpath(runtimeRoot)) {
        return fail(
            QStringLiteral("no safe XDG runtime directory"),
            3);
    }

    const QString statePath =
        QDir(cybou::StatePaths::persistentRoot())
            .filePath(QStringLiteral("identity.json"));
    const QString markerPath =
        QDir(runtimeRoot)
            .filePath(QStringLiteral("identity-session"));

    cybou::IdentityService service(
        &events,
        statePath,
        markerPath);
    if (!service.isReady()) {
        return fail(service.startupError(), 4);
    }

    QString error;
    if (!cybou::ServiceHost::publish(
            &service,
            cybou::kIdentityEndpoint,
            &error)) {
        return fail(error, 5);
    }

    return app.exec();
}
