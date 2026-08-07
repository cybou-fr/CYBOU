// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "IntentionService.h"

#include "cybou/fabric/FabricCodec.h"

namespace cybou {

IntentionService::IntentionService(
    EventStore *events,
    QObject *parent)
    : QObject(parent)
    , m_events(events)
    , m_intentions(events)
{
}

bool IntentionService::Ready() const
{
    return m_events && m_events->isOpen();
}

QString IntentionService::Health() const
{
    return Ready()
        ? QStringLiteral("healthy")
        : QStringLiteral("unavailable");
}

QString IntentionService::LastError() const
{
    if (!m_intentions.lastError().isEmpty()) {
        return m_intentions.lastError();
    }
    return m_events ? m_events->lastError() : QString();
}

QByteArray IntentionService::Open() const
{
    QVariantList result;
    for (const Intention &intention : m_intentions.open()) {
        QVariantMap map;
        map[QStringLiteral("correlationId")] =
            intention.id.toString(QUuid::WithoutBraces);
        map[QStringLiteral("description")] =
            intention.description;
        map[QStringLiteral("trigger")] =
            intention.trigger;
        map[QStringLiteral("formed")] =
            intention.formed;
        result.append(map);
    }

    return FabricCodec::encodeList(result);
}

QString IntentionService::Form(
    const QString &description,
    const QString &trigger,
    const QString &causeId)
{
    const QUuid id = m_intentions.form(
        description,
        trigger,
        QUuid::fromString(causeId));

    return id.isNull()
        ? QString()
        : id.toString(QUuid::WithoutBraces);
}

bool IntentionService::Close(
    const QString &intentionId,
    int resolution,
    const QString &note)
{
    if (resolution < static_cast<int>(Resolution::Fulfilled)
        || resolution > static_cast<int>(Resolution::Obsolete)) {
        return false;
    }

    return m_intentions.close(
        QUuid::fromString(intentionId),
        static_cast<Resolution>(resolution),
        note);
}

} // namespace cybou
