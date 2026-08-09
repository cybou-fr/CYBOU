// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "HealthService.h"
#include "cybou/fabric/OrganBus.h"
#include "cybou/fabric/ServiceHost.h"
#include "cybou/runtime/StatePaths.h"

#include <QCoreApplication>
#include <QDBusConnection>
#include <QDBusServiceWatcher>
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
        QTimer refreshTimer;
        refreshTimer.setInterval(30000);
        QObject::connect(&refreshTimer, &QTimer::timeout, &service, [&service]() {
            service.Refresh();
        });
        refreshTimer.start();

        QTimer ownerChangeDebounce;
        ownerChangeDebounce.setSingleShot(true);
        ownerChangeDebounce.setInterval(100);
        QObject::connect(&ownerChangeDebounce, &QTimer::timeout, &service, [&service]() {
            service.Refresh();
        });
        QDBusServiceWatcher watcher(
            {}, QDBusConnection::sessionBus(), QDBusServiceWatcher::WatchForOwnerChange);
        for (const cybou::BusEndpoint &endpoint : {
                 cybou::kEventEndpoint, cybou::kLifecycleEndpoint, cybou::kIdentityEndpoint,
                 cybou::kIntentionEndpoint, cybou::kPredictorEndpoint, cybou::kSelfEndpoint,
                 cybou::kWorkspaceEndpoint, cybou::kPresenceEndpoint}) {
            watcher.addWatchedService(QString::fromLatin1(endpoint.service));
        }
        QObject::connect(
            &watcher, &QDBusServiceWatcher::serviceOwnerChanged,
            &ownerChangeDebounce, [&ownerChangeDebounce]() { ownerChangeDebounce.start(); });
        QTimer::singleShot(0, &service, [&service]() { service.Refresh(); });
        return app.exec();
    }
    return app.exec();
}
