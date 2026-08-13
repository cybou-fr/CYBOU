// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/epistemic/EpistemicProjection.h"
#include "cybou/events/EventStore.h"

#include <QObject>
#include <QString>

namespace cybou {

/// Owns the epistemic projection over accepted observations.
///
/// ADR-0027 fixes what this may be. It owns the derived projection, freshness, contradiction and
/// reconciliation state — and owns neither the Journal, nor any perception source, nor system-wide
/// retention. It never writes to Event1: it reads what perception proposed and says what is known.
///
/// The projection is a cache of the Journal, and the checkpoint below is a cache of the projection.
/// Losing either costs a replay. Where any of them disagrees with the Journal, the Journal is right.
class EpistemicService : public QObject
{
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.cybou.Mind.Epistemic1")

public:
    EpistemicService(EventStore *events, QString checkpointPath, QObject *parent = nullptr);

    bool isReady() const { return m_ready; }
    QString startupError() const { return m_startupError; }

    /// Catch up with everything accepted since the cursor. Public so startup and the tests drive
    /// the same path rather than the tests exercising a private shortcut.
    bool catchUp();

    /// Admit one contribution announced live, advancing the cursor to its sequence.
    void admitAccepted(const CognitiveEnvelope &envelope, quint64 sequence);

public Q_SLOTS:
    bool Ready() const;
    QString Health() const;
    QString LastError() const;

    /// Everything known, as a CBOR list. Status is evaluated at the moment of the call, because it
    /// is an answer about now rather than a stored property.
    QByteArray Knowledge() const;

    /// What is known about one subject, including when nothing is: an unfamiliar subject answers
    /// `unknown` rather than failing, because not knowing is a normal state.
    QByteArray KnowledgeOf(const QString &subject) const;

    /// Highest Event1 sequence this projection has taken in.
    qulonglong Cursor() const;

Q_SIGNALS:
    void Changed();

private:
    bool load();
    /// Write the cursor and the projection as one value.
    ///
    /// They must not be separable. A checkpoint ahead of its cursor only re-admits contributions,
    /// which is harmless because admission is idempotent; a cursor ahead of its checkpoint leaves a
    /// gap in what was admitted, and nothing downstream would ever discover it.
    void persist();

    EventStore *m_events{nullptr};
    QString m_checkpointPath;
    EpistemicProjection m_projection;
    quint64 m_cursor{0};
    bool m_ready{false};
    QString m_startupError;
    mutable QString m_lastError;
};

} // namespace cybou
