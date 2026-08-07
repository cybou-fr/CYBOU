// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT
//
// The global workspace: the bounded place where accepted contributions group into coalitions
// and compete for attention. Biography remains owned by the Journal.

#pragma once

#include "cybou/storage/Journal.h"

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
    explicit Workspace(Journal *journal, int capacity = 32, QObject *parent = nullptr);

    /// Submit through the current Journal. Admission happens from Journal::accepted after COMMIT,
    /// so this method and direct organ Journal writes have identical Workspace semantics.
    bool publish(const CognitiveEnvelope &envelope);

    /// Admit a contribution that is already durable. Idempotent by messageId.
    ///
    /// This is public deliberately: M3 can feed the same method from eventd without teaching
    /// Workspace how durable storage or IPC works.
    void accept(const CognitiveEnvelope &envelope);

    QList<Coalition> coalitions(const QDateTime &now = QDateTime()) const;
    Coalition focus(const QDateTime &now = QDateTime()) const;

    QList<CognitiveEnvelope> moment() const { return m_moment; }
    int capacity() const { return m_capacity; }

    MomentState momentState() const;

    /// Startup/recovery reconstruction only. Normal live operation follows accepted events and
    /// does not reread the full recent Journal after every organ action.
    void rehydrate();

Q_SIGNALS:
    void contributed(const CognitiveEnvelope &envelope);
    void focusChanged(const Coalition &focus);

private:
    double salienceOf(const Coalition &coalition, const QDateTime &now) const;
    void reevaluateFocus();

    Journal *m_journal;
    int m_capacity;
    QList<CognitiveEnvelope> m_moment;
    QUuid m_lastFocus;
};

double attentionWeight(ContributionKind kind);

} // namespace cybou

Q_DECLARE_METATYPE(cybou::Coalition)
