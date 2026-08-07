// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "WorkspaceService.h"

#include "cybou/fabric/OrganBus.h"
#include "cybou/fabric/ServiceHost.h"
#include "cybou/ipc/EventClient.h"

#include <QCoreApplication>
#include <QTextStream>

namespace {

int fail(const QString &message, int code)
{
    QTextStream(stderr)
        << "cybou-workspaced: " << message << Qt::endl;
    return code;
}

} // namespace

int main(int argc, char **argv)
{
    QCoreApplication app(argc, argv);
    QCoreApplication::setApplicationName(QStringLiteral("cybou-workspaced"));
    QCoreApplication::setOrganizationName(QStringLiteral("Cybou"));

    cybou::EventClient events;
    if (!events.isOpen()) {
        return fail(events.lastError(), 2);
    }

    cybou::WorkspaceService service(&events);

    QString error;
    if (!cybou::ServiceHost::publish(
            &service,
            cybou::kWorkspaceEndpoint,
            &error)) {
        return fail(error, 3);
    }

    return app.exec();
}
