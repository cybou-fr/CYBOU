// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "PresenceService.h"

#include "cybou/fabric/OrganBus.h"
#include "cybou/fabric/ServiceHost.h"

#include <QCoreApplication>
#include <QTextStream>

namespace {

int fail(const QString &message, int code)
{
    QTextStream(stderr)
        << "cybou-presenced: " << message << Qt::endl;
    return code;
}

} // namespace

int main(int argc, char **argv)
{
    QCoreApplication app(argc, argv);
    QCoreApplication::setApplicationName(QStringLiteral("cybou-presenced"));
    QCoreApplication::setOrganizationName(QStringLiteral("Cybou"));

    cybou::PresenceService service;
    if (!service.Ready()) {
        return fail(service.LastError(), 2);
    }

    QString error;
    if (!cybou::ServiceHost::publish(
            &service,
            cybou::kPresenceEndpoint,
            &error)) {
        return fail(error, 3);
    }

    return app.exec();
}
