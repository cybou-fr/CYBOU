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

class PresenceRuntime;

/// Current presentation wrapper.
///
/// Default/QML construction uses Event1 and therefore cybou-eventd for the canonical Journal.
/// The explicit data-directory constructor is retained as a local test seam and uses a local
/// Journal only for isolated tests/tools.
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
    bool isAwake() const;

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
    void subscribeToRuntime();
    bool appendUserObservation(
        const QString &event,
        const QCborMap &details,
        QUuid *messageId);

    std::shared_ptr<PresenceRuntime> m_runtime;
    QString m_lastError;
    bool m_subscribed{false};
};

} // namespace cybou

Q_DECLARE_METATYPE(cybou::Moment)
