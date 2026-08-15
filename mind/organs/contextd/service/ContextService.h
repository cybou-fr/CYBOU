// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/context/AssociativeProjection.h"
#include "cybou/context/ContextDelivery.h"
#include "cybou/events/EventStore.h"

#include <QDBusContext>
#include <QObject>
#include <QString>

namespace cybou {

/// Owns the associative projection over accepted contributions.
///
/// ADR-0029 fixes what this may be. It owns association, activation and context bundles — and owns
/// neither the Journal, nor truth, nor identity, nor attention, nor prompts, nor any language model.
/// It never writes to Event1.
///
/// The graph is a cache of the Journal, and the checkpoint is a cache of the graph. Deleting either
/// costs the speed of recall and no memory at all. Where either disagrees with the Journal, the
/// Journal is right.
class ContextService
    : public QObject
    , protected QDBusContext
{
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.cybou.Mind.Context1")

public:
    ContextService(EventStore *events, QString checkpointPath, QObject *parent = nullptr);

    bool isReady() const { return m_ready; }
    QString startupError() const { return m_startupError; }

    /// Take in everything accepted since the cursor, building concepts and associations.
    ///
    /// Fails closed, for the reason every projection here does: a graph built from part of the
    /// history is not a smaller graph but a differently-shaped one, and nothing downstream could
    /// tell.
    bool catchUp();

    void admitAccepted(const CognitiveEnvelope &envelope, quint64 sequence);

    /// The projection itself, for callers inside this process. The D-Bus surface is below.
    const AssociativeProjection &projection() const { return m_projection; }

public Q_SLOTS:
    bool Ready() const;
    QString Health() const;
    QString LastError() const;

    /// Activate from seeds and return a bundle, or a D-Bus error if the projection cannot answer.
    ///
    /// Erroring rather than returning an empty bundle: an empty result means nothing is related,
    /// which is a fact, and a projection that could not be assembled has no facts to offer.
    QByteArray Activate(const QStringList &seeds, int maxNodes, int maxDepth);

    /// Activate, apply a consumer's policy, and return the full disposition of every item.
    ///
    /// ADR-0030. The reply carries a decision per activated item rather than the delivered subset,
    /// so a caller cannot render "what was sent" without also holding what was withheld. That is
    /// B6, and it survives the wire only because the wire carries the same one list.
    ///
    /// This produces the plan and reports whether the delivery is owed a durable record. It does
    /// not write one: contextd never writes to Event1, and a projection that could record a fact
    /// about the person's data would be a second writer whatever the ADRs said. The caller that
    /// actually performs the delivery owns that contribution.
    QByteArray Deliver(
        const QStringList &seeds,
        const QString &destinationId,
        int trust,
        bool retains,
        bool externalBoundary,
        const QStringList &selected,
        const QStringList &excluded);

    qulonglong Cursor() const;

Q_SIGNALS:
    void Changed();

private:
    bool refuseWhenUnready(const QString &method);
    bool load();
    void persist();

    /// Fold one contribution into the graph.
    ///
    /// Only observations produce concepts today. That is deliberately narrow: an association
    /// derived from something Mind never actually observed would be a relation with no evidence
    /// behind it, which is the one thing this graph may not contain.
    void admitToGraph(const CognitiveEnvelope &envelope);

    EventStore *m_events{nullptr};
    QString m_checkpointPath;
    AssociativeProjection m_projection;
    quint64 m_cursor{0};
    quint64 m_erasureEpoch{0};
    bool m_ready{false};
    QString m_startupError;
    mutable QString m_lastError;
};

} // namespace cybou
