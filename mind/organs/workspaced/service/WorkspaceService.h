// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/workspace/Workspace.h"

#include <QObject>

namespace cybou {

class WorkspaceService : public QObject
{
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.cybou.Mind.Workspace1")

public:
    explicit WorkspaceService(
        EventStore *events,
        QObject *parent = nullptr);

public Q_SLOTS:
    bool Ready() const;
    QString Health() const;
    QString LastError() const;

    QByteArray Coalitions() const;
    QByteArray Moment() const;
    QString Attention() const;
    QByteArray Consolidate(const QString &runId, const QString &operationKey,
                           qulonglong inputHighWaterMark) const;

Q_SIGNALS:
    void Changed();

private:
    EventStore *m_events;
    Workspace m_workspace;
};

} // namespace cybou
