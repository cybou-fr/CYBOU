// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/intentions/Intentions.h"

#include <QDBusContext>
#include <QObject>

namespace cybou {

class IntentionService
    : public QObject
    , protected QDBusContext
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

    /// The open set, or a D-Bus error if it could not be assembled.
    ///
    /// Erroring rather than answering an empty list is the point: every caller already treats a
    /// failed call as a section it could not measure and leaves a typed default, whereas an empty
    /// success is indistinguishable from "Mind owes nothing" - a comforting lie about the one thing
    /// this organ exists to be sure about.
    QByteArray Open();

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
