// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/events/EventStore.h"

#include <QObject>

namespace cybou {

struct Coalition {
    QUuid correlationId;
    QList<CognitiveEnvelope> members;
    double salience{0.0};
    QDateTime latest;

    bool isValid() const { return !correlationId.isNull() && !members.isEmpty(); }
    QStringList organs() const;
    int threadCount() const { return members.size(); }
};

struct MomentState {
    QUuid focus;
    double salience{0.0};
    QStringList organs;

    bool isValid() const { return !focus.isNull(); }
};

class Workspace : public QObject
{
    Q_OBJECT

public:
    explicit Workspace(
        EventStore *events,
        int capacity = 32,
        QObject *parent = nullptr);

    bool publish(const CognitiveEnvelope &envelope);
    void accept(const CognitiveEnvelope &envelope);

    QList<Coalition> coalitions(const QDateTime &now = QDateTime()) const;
    Coalition focus(const QDateTime &now = QDateTime()) const;

    QList<CognitiveEnvelope> moment() const { return m_moment; }
    int capacity() const { return m_capacity; }

    MomentState momentState() const;
    void rehydrate();

Q_SIGNALS:
    void contributed(const CognitiveEnvelope &envelope);
    void focusChanged(const Coalition &focus);

private:
    double salienceOf(const Coalition &coalition, const QDateTime &now) const;
    void reevaluateFocus();

    EventStore *m_events;
    int m_capacity;
    QList<CognitiveEnvelope> m_moment;
    QUuid m_lastFocus;
};

double attentionWeight(ContributionKind kind);

} // namespace cybou

Q_DECLARE_METATYPE(cybou::Coalition)
