// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "ContextService.h"

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
    QTextStream(stderr) << "cybou-contextd: " << message << Qt::endl;
    return code;
}

} // namespace

int main(int argc, char **argv)
{
    QCoreApplication app(argc, argv);
    QCoreApplication::setApplicationName(QStringLiteral("cybou-contextd"));
    QCoreApplication::setOrganizationName(QStringLiteral("Cybou"));

    cybou::EventClient events;
    if (!events.isOpen()) {
        return fail(events.lastError(), 2);
    }

    const QString checkpoint =
        QDir(cybou::StatePaths::persistentRoot())
            .filePath(QStringLiteral("context/graph.cbor"));

    cybou::ContextService service(&events, checkpoint);
    if (!service.isReady()) {
        return fail(service.startupError(), 3);
    }

    // Live acceptances rather than polling, on the same terms as every other projection: Event1
    // publishes only after commit, so nothing here can see a proposal a power loss could take back.
    QObject::connect(
        &events,
        &cybou::EventStore::accepted,
        &service,
        [&service](const cybou::CognitiveEnvelope &envelope, quint64 sequence) {
            service.admitAccepted(envelope, sequence);
        });

    QString error;
    if (!cybou::ServiceHost::publish(&service, cybou::kContextEndpoint, &error)) {
        return fail(error, 4);
    }

    return app.exec();
}
