// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/identity/Identity.h"

#include <QObject>

#include <memory>

namespace cybou {

class IdentityService : public QObject
{
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.cybou.Mind.Identity1")

public:
    IdentityService(
        EventStore *events,
        const QString &statePath,
        const QString &sessionMarkerPath,
        QObject *parent = nullptr);

    bool isReady() const { return m_ready; }
    QString startupError() const { return m_startupError; }

public Q_SLOTS:
    bool Ready() const { return m_ready; }
    QString Health() const;
    QString LastError() const;
    QByteArray State() const;

private:
    bool initialize(const QString &sessionMarkerPath);
    bool persistSessionMarker(const QString &path);

    EventStore *m_events;
    std::unique_ptr<Identity> m_identity;
    bool m_ready{false};
    QString m_startupError;
};

} // namespace cybou
