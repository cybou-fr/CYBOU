// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "PerceptionService.h"

#include "cybou/fabric/OrganBus.h"
#include "cybou/fabric/ServiceHost.h"
#include "cybou/ipc/EventClient.h"

#include <QCoreApplication>
#include <QTextStream>
#include <QTimer>

namespace {

int fail(const QString &message, int code)
{
    QTextStream(stderr) << "cybou-perceptiond: " << message << Qt::endl;
    return code;
}

// ADR-0027 bounds acquisition at one reading per source per ten seconds. The source changes only
// when the system is rebuilt, so polling faster would spend the shared ingestion budget to learn
// nothing. Overridable because the tests cannot wait ten seconds to observe one reading.
int acquisitionIntervalMs()
{
    bool ok = false;
    const int configured =
        qEnvironmentVariableIntValue("CYBOU_PERCEPTION_INTERVAL_MS", &ok);
    return ok ? qBound(50, configured, 3600000) : 10000;
}

} // namespace

int main(int argc, char **argv)
{
    QCoreApplication app(argc, argv);
    QCoreApplication::setApplicationName(QStringLiteral("cybou-perceptiond"));
    QCoreApplication::setOrganizationName(QStringLiteral("Cybou"));

    cybou::EventClient events;
    if (!events.isOpen()) {
        return fail(events.lastError(), 2);
    }

    const QString systemLink = qEnvironmentVariableIsSet("CYBOU_PERCEPTION_SYSTEM_LINK")
        ? qEnvironmentVariable("CYBOU_PERCEPTION_SYSTEM_LINK")
        : QStringLiteral("/run/current-system");

    cybou::PerceptionService service(&events, cybou::SystemGenerationSource(systemLink));

    QString error;
    if (!cybou::ServiceHost::publish(&service, cybou::kPerceptionEndpoint, &error)) {
        return fail(error, 3);
    }

    // Read once at startup rather than waiting a full interval: a session that has just begun
    // should not have to wait to know what system it is running.
    QTimer::singleShot(0, &service, [&service]() { service.acquireOnce(); });

    QTimer poll;
    poll.setInterval(acquisitionIntervalMs());
    QObject::connect(&poll, &QTimer::timeout, &service, [&service]() { service.acquireOnce(); });
    poll.start();

    return app.exec();
}
