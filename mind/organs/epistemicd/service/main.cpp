// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "EpistemicService.h"

#include "cybou/events/EnvelopeCodec.h"
#include "cybou/fabric/OrganBus.h"
#include "cybou/fabric/ServiceHost.h"
#include "cybou/ipc/EventClient.h"
#include "cybou/runtime/StatePaths.h"

#include <QCoreApplication>
#include <QDBusConnection>
#include <QDir>
#include <QTextStream>

namespace {

int fail(const QString &message, int code)
{
    QTextStream(stderr) << "cybou-epistemicd: " << message << Qt::endl;
    return code;
}

} // namespace

int main(int argc, char **argv)
{
    QCoreApplication app(argc, argv);
    QCoreApplication::setApplicationName(QStringLiteral("cybou-epistemicd"));
    QCoreApplication::setOrganizationName(QStringLiteral("Cybou"));

    cybou::EventClient events;
    if (!events.isOpen()) {
        return fail(events.lastError(), 2);
    }

    const QString checkpoint =
        QDir(cybou::StatePaths::persistentRoot())
            .filePath(QStringLiteral("epistemic/projection.cbor"));

    cybou::EpistemicService service(&events, checkpoint);
    if (!service.isReady()) {
        return fail(service.startupError(), 3);
    }

    // Live announcements rather than polling. Event1 publishes acceptance only after commit, so
    // anything arriving here is already durable - the projection never sees a proposal that a power
    // loss could take back.
    QObject::connect(
        &events,
        &cybou::EventStore::accepted,
        &service,
        [&service](const cybou::CognitiveEnvelope &envelope, quint64 sequence) {
            service.admitAccepted(envelope, sequence);
        });

    QString error;
    if (!cybou::ServiceHost::publish(&service, cybou::kEpistemicEndpoint, &error)) {
        return fail(error, 4);
    }

    return app.exec();
}
