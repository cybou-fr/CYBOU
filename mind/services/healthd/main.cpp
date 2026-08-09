// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "HealthService.h"
#include "cybou/fabric/OrganBus.h"
#include "cybou/fabric/ServiceHost.h"
#include "cybou/runtime/StatePaths.h"

#include <QCoreApplication>
#include <QDir>
#include <QTextStream>
#include <QTimer>

int main(int argc, char **argv)
{
    QCoreApplication app(argc, argv);
    QCoreApplication::setApplicationName(QStringLiteral("cybou-healthd"));

    const QString path = QDir(cybou::StatePaths::persistentRoot())
                             .filePath(QStringLiteral("health/snapshot.cbor"));
    cybou::HealthService service(path);
    if (!service.isReady()) {
        QTextStream(stderr) << service.startupError() << Qt::endl;
        return 2;
    }
    QString error;
    if (!cybou::ServiceHost::publish(&service, cybou::kHealthEndpoint, &error)) {
        QTextStream(stderr) << error << Qt::endl;
        return 3;
    }
    if (!qEnvironmentVariableIsSet("CYBOU_HEALTH_DISABLE_AUTO_REFRESH")) {
        QTimer::singleShot(0, &service, [&service]() { service.Refresh(); });
    }
    return app.exec();
}
