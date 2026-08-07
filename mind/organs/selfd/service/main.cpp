// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "SelfService.h"

#include "cybou/fabric/OrganBus.h"
#include "cybou/fabric/ServiceHost.h"

#include <QCoreApplication>
#include <QTextStream>

namespace {

int fail(const QString &message, int code)
{
    QTextStream(stderr)
        << "cybou-selfd: " << message << Qt::endl;
    return code;
}

} // namespace

int main(int argc, char **argv)
{
    QCoreApplication app(argc, argv);
    QCoreApplication::setApplicationName(QStringLiteral("cybou-selfd"));
    QCoreApplication::setOrganizationName(QStringLiteral("Cybou"));

    cybou::SelfService service;
    if (!service.Ready()) {
        return fail(service.LastError(), 2);
    }

    QString error;
    if (!cybou::ServiceHost::publish(
            &service,
            cybou::kSelfEndpoint,
            &error)) {
        return fail(error, 3);
    }

    return app.exec();
}
