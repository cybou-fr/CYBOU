// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/eventd/EventService.h"

#include "cybou/events/EventBus.h"
#include "cybou/fabric/ServiceHost.h"
#include "cybou/runtime/StatePaths.h"

#include <QCoreApplication>
#include <QDir>
#include <QTextStream>

namespace {

int fail(const QString &message, int code)
{
    QTextStream(stderr)
        << "cybou-eventd: " << message << Qt::endl;
    return code;
}

} // namespace

int main(int argc, char **argv)
{
    QCoreApplication app(argc, argv);
    QCoreApplication::setApplicationName(QStringLiteral("cybou-eventd"));
    QCoreApplication::setOrganizationName(QStringLiteral("Cybou"));

    const QString root = cybou::StatePaths::persistentRoot();
    if (!QDir().mkpath(root)) {
        return fail(
            QStringLiteral("cannot create state root %1").arg(root),
            2);
    }

    const QString journalPath =
        QDir(root).filePath(QStringLiteral("journal.db"));
    cybou::EventService service(journalPath);
    if (!service.isReady()) {
        return fail(service.startupError(), 3);
    }

    const cybou::BusEndpoint endpoint{
        cybou::kEventServiceName,
        cybou::kEventObjectPath,
        cybou::kEventInterfaceName,
        "cybou-eventd.service",
    };

    QString error;
    if (!cybou::ServiceHost::publish(
            &service,
            endpoint,
            &error)) {
        return fail(error, 4);
    }

    return app.exec();
}
