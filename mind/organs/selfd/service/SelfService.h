// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/fabric/OrganClients.h"
#include "cybou/ipc/EventClient.h"
#include "cybou/self/SelfModel.h"

#include <QObject>

namespace cybou {

class SelfService : public QObject
{
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.cybou.Mind.Self1")

public:
    explicit SelfService(QObject *parent = nullptr);

public Q_SLOTS:
    bool Ready() const;
    QString Health() const;
    QString LastError() const;

    QByteArray Measure() const;
    QByteArray Assess(const QString &causeId);
    QString Narration() const;

private:
    SelfReport measureReport() const;
    QVariantMap reportMap(const SelfReport &report) const;

    mutable QString m_lastError;

    EventClient m_events;
    IdentityClient m_identity;
    IntentionClient m_intentions;
    PredictorClient m_predictor;
};

} // namespace cybou
