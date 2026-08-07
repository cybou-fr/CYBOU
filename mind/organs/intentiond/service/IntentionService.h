// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/intentions/Intentions.h"

#include <QObject>

namespace cybou {

class IntentionService : public QObject
{
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.cybou.Mind.Intention1")

public:
    explicit IntentionService(
        EventStore *events,
        QObject *parent = nullptr);

public Q_SLOTS:
    bool Ready() const;
    QString Health() const;
    QString LastError() const;

    QByteArray Open() const;

    QString Form(
        const QString &description,
        const QString &trigger,
        const QString &causeId);

    bool Close(
        const QString &intentionId,
        int resolution,
        const QString &note);

private:
    EventStore *m_events;
    Intentions m_intentions;
};

} // namespace cybou
