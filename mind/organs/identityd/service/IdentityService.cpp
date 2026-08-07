// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "IdentityService.h"

#include "cybou/fabric/FabricCodec.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QSaveFile>

namespace cybou {

IdentityService::IdentityService(
    EventStore *events,
    const QString &statePath,
    const QString &sessionMarkerPath,
    QObject *parent)
    : QObject(parent)
    , m_events(events)
    , m_identity(std::make_unique<Identity>(statePath, events))
{
    m_ready = initialize(sessionMarkerPath);
}

bool IdentityService::initialize(
    const QString &sessionMarkerPath)
{
    if (!m_events || !m_events->isOpen()) {
        m_startupError =
            m_events
                ? m_events->lastError()
                : QStringLiteral("identityd has no EventStore");
        return false;
    }

    QFile marker(sessionMarkerPath);
    if (marker.exists()) {
        if (!m_identity->resumeSession()) {
            m_startupError = m_identity->lastError();
            return false;
        }

        if (!marker.open(QIODevice::ReadOnly)) {
            m_startupError =
                QStringLiteral("cannot read identity session marker");
            return false;
        }

        const QUuid marked =
            QUuid::fromString(
                QString::fromUtf8(marker.readAll()).trimmed());
        if (marked.isNull()
            || marked != m_identity->state().identityId) {
            m_startupError =
                QStringLiteral(
                    "identity runtime marker does not match persistent identity");
            return false;
        }

        return true;
    }

    if (!m_identity->beginSession()) {
        m_startupError = m_identity->lastError();
        return false;
    }

    return persistSessionMarker(sessionMarkerPath);
}

bool IdentityService::persistSessionMarker(
    const QString &path)
{
    QDir().mkpath(QFileInfo(path).absolutePath());

    QSaveFile marker(path);
    if (!marker.open(QIODevice::WriteOnly)) {
        m_startupError =
            QStringLiteral("cannot create identity session marker");
        return false;
    }

    marker.write(
        m_identity->state()
            .identityId
            .toString(QUuid::WithoutBraces)
            .toUtf8());
    marker.write("\n");

    if (!marker.commit()) {
        m_startupError =
            QStringLiteral("cannot commit identity session marker");
        return false;
    }

    return true;
}

QString IdentityService::Health() const
{
    return m_ready
        ? QStringLiteral("healthy")
        : QStringLiteral("unavailable");
}

QString IdentityService::LastError() const
{
    if (!m_startupError.isEmpty()) {
        return m_startupError;
    }
    return m_identity
        ? m_identity->lastError()
        : QString();
}

QByteArray IdentityService::State() const
{
    if (!m_ready || !m_identity) {
        return FabricCodec::encodeMap({});
    }

    const IdentityState state = m_identity->state();

    QVariantMap map;
    map[QStringLiteral("uuid")] =
        state.identityId.toString(QUuid::WithoutBraces);
    map[QStringLiteral("origin")] = state.origin;
    map[QStringLiteral("sessionCount")] =
        static_cast<qulonglong>(state.sessionCount);
    map[QStringLiteral("architectureVersion")] =
        state.architectureVersion;
    map[QStringLiteral("wasBorn")] = m_identity->wasBorn();

    return FabricCodec::encodeMap(map);
}

} // namespace cybou
