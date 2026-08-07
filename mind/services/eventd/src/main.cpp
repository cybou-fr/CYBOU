// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/eventd/EventService.h"

#include "cybou/events/EventBus.h"
#include "cybou/runtime/StatePaths.h"

#include <QCoreApplication>
#include <QDBusConnection>
#include <QDir>
#include <QFileInfo>
#include <QTextStream>

namespace {

int fail(const QString &message, int code)
{
    QTextStream(stderr) << "cybou-eventd: " << message << Qt::endl;
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
        return fail(QStringLiteral("cannot create state root %1").arg(root), 2);
    }

    const QString journalPath =
        QDir(root).filePath(QStringLiteral("journal.db"));
    cybou::EventService service(journalPath);
    if (!service.isReady()) {
        return fail(service.startupError(), 3);
    }

    QDBusConnection bus = QDBusConnection::sessionBus();
    if (!bus.isConnected()) {
        return fail(QStringLiteral("user D-Bus session is unavailable"), 4);
    }

    if (!bus.registerObject(
            QString::fromLatin1(cybou::kEventObjectPath),
            &service,
            QDBusConnection::ExportAllSlots
                | QDBusConnection::ExportAllSignals)) {
        return fail(
            QStringLiteral("cannot register Event1 object: %1")
                .arg(bus.lastError().message()),
            5);
    }

    if (!bus.registerService(
            QString::fromLatin1(cybou::kEventServiceName))) {
        return fail(
            QStringLiteral("cannot own %1: %2")
                .arg(
                    QString::fromLatin1(cybou::kEventServiceName),
                    bus.lastError().message()),
            6);
    }

    return app.exec();
}
