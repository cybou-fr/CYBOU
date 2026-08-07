// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/self/SelfModel.h"
#include "cybou/workspace/Workspace.h"

#include <QCborMap>
#include <QObject>
#include <QVariant>

#include <memory>

namespace cybou {

struct Moment {
    QDateTime when;
    QString organ;
    QString kind;
    QUuid thread;
};

class Presence : public QObject
{
    Q_OBJECT

    Q_PROPERTY(bool awake READ isAwake NOTIFY changed)
    Q_PROPERTY(QString narration READ narration NOTIFY changed)
    Q_PROPERTY(QStringList obligations READ obligations NOTIFY changed)
    Q_PROPERTY(QString attention READ attention NOTIFY changed)
    Q_PROPERTY(int contributions READ contributions NOTIFY changed)
    Q_PROPERTY(QVariantMap stats READ stats NOTIFY changed)
    Q_PROPERTY(QVariantMap identityState READ identityState NOTIFY changed)
    Q_PROPERTY(QVariantList calibrations READ calibrations NOTIFY changed)
    Q_PROPERTY(QVariantList coalitions READ coalitions NOTIFY changed)
    Q_PROPERTY(QVariantMap moment READ moment NOTIFY changed)

public:
    explicit Presence(const QString &dataDir, QObject *parent = nullptr);
    explicit Presence(QObject *parent = nullptr);
    ~Presence() override;

    bool wake();
    bool isAwake() const { return m_awake; }

    QString narration() const;
    QStringList obligations() const;
    QString attention() const;
    int contributions() const;

    QList<Moment> recent(int limit = 12) const;
    Q_INVOKABLE QVariantList activity(int limit = 12) const;

    Q_INVOKABLE QUuid promise(const QString &description);
    Q_INVOKABLE bool reflect();
    Q_INVOKABLE bool fulfillIndex(int index);
    Q_INVOKABLE bool abandonIndex(int index);
    Q_INVOKABLE QVariantList detailedObligations() const;
    Q_INVOKABLE bool observe(const QString &subject, double value);

    Q_INVOKABLE QVariantMap stats() const;
    Q_INVOKABLE QVariantMap identityState() const;
    Q_INVOKABLE QVariantList calibrations() const;
    Q_INVOKABLE QVariantMap predict(const QString &subject);
    Q_INVOKABLE QVariantList coalitions() const;
    Q_INVOKABLE QVariantMap moment() const;

    QString lastError() const { return m_lastError; }

Q_SIGNALS:
    void changed();

private:
    bool appendUserObservation(const QString &event, const QCborMap &details, QUuid *messageId);

    QString m_dataDir;
    QString m_lastError;
    bool m_awake{false};

    std::unique_ptr<Journal> m_journal;
    std::unique_ptr<Identity> m_identity;
    std::unique_ptr<Intentions> m_intentions;
    std::unique_ptr<Predictor> m_predictor;
    std::unique_ptr<SelfModel> m_self;
    std::unique_ptr<Workspace> m_workspace;
};

} // namespace cybou

Q_DECLARE_METATYPE(cybou::Moment)
