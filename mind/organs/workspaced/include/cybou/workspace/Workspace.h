// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT
//
// The global workspace: the narrow place where contributions from different organs become
// visible to each other, group into coalitions, and compete for attention.
//
// Two properties matter more than the arithmetic here, and both are structural:
//
//   The working set is bounded. A moment that could grow forever is not a moment. Contributions
//   fall out of it as newer ones arrive - but they fall out of *attention*, not out of the
//   journal, which keeps everything. Forgetting here is not losing.
//
//   Nothing is central. The workspace does not decide anything or interpret payloads; it orders
//   what organs offered and lets the most salient coalition win. Replacing the salience
//   function changes what surfaces, never who is in charge - there is no one in charge.

#pragma once

#include "cybou/storage/Journal.h"

#include <QObject>

namespace cybou {

/// Contributions sharing a correlation id - one thread of concern, assembled from whichever
/// organs happened to speak to it.
struct Coalition {
    QUuid correlationId;
    QList<CognitiveEnvelope> members;
    /// Higher means more deserving of attention right now. Comparable only against other
    /// coalitions measured at the same instant.
    double salience{0.0};
    QDateTime latest;

    bool isValid() const { return !correlationId.isNull() && !members.isEmpty(); }
    /// The organs that contributed. A concern several organs agree on is not the same as one
    /// organ repeating itself, and the salience function treats them differently.
    QStringList organs() const;
    int threadCount() const { return members.size(); }
};

/// Current moment state for display in the Workspace tab.
struct MomentState {
    QUuid focus; // Current focus coalition id
    double salience{0.0};
    QStringList organs;

    bool isValid() const { return !focus.isNull(); }
};

class Workspace : public QObject
{
    Q_OBJECT

public:
    /// capacity is how many contributions stay in the current moment. The default is chosen to
    /// be small enough that attention means something.
    explicit Workspace(Journal *journal, int capacity = 32, QObject *parent = nullptr);

    /// Records the contribution and admits it to the current moment. Returns false if the
    /// journal refused it - nothing enters attention that was not first remembered.
    bool publish(const CognitiveEnvelope &envelope);

    /// Coalitions in the current moment, most salient first.
    QList<Coalition> coalitions(const QDateTime &now = QDateTime()) const;

    /// The single most salient coalition, or an invalid one when nothing is in the moment.
    Coalition focus(const QDateTime &now = QDateTime()) const;

    /// Contributions currently in the moment, newest first.
    QList<CognitiveEnvelope> moment() const { return m_moment; }
    int capacity() const { return m_capacity; }

    /// Current moment state for display in QML.
    MomentState momentState() const;

    /// Restores the moment from the journal after a restart, so the system does not wake up
    /// with nothing on its mind.
    void rehydrate();

Q_SIGNALS:
    /// Emitted after a contribution is admitted.
    void contributed(const CognitiveEnvelope &envelope);
    /// Emitted when the most salient coalition changes identity - what the system is attending
    /// to has shifted, which is the event a surface actually wants.
    void focusChanged(const Coalition &focus);

private:
    double salienceOf(const Coalition &coalition, const QDateTime &now) const;
    void reevaluateFocus();

    Journal *m_journal;
    int m_capacity;
    QList<CognitiveEnvelope> m_moment; // newest first
    QUuid m_lastFocus;
};

/// How much a kind of contribution pulls at attention. A need or an objection interrupts; an
/// observation waits its turn. Exposed so the weighting can be argued with rather than buried.
double attentionWeight(ContributionKind kind);

} // namespace cybou

// Both travel through signals, so QSignalSpy and any future queued connection need them
// registered.
Q_DECLARE_METATYPE(cybou::Coalition)
