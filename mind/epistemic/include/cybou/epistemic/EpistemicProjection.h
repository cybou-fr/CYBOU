// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/protocol/CognitiveEnvelope.h"
#include "cybou/protocol/Observation.h"

#include <QDateTime>
#include <QSet>
#include <QUuid>
#include <QHash>
#include <QList>
#include <QString>

namespace cybou {

// 2 added the contribution id and provenance to every claim. 3 made a source's current state a list
// rather than a single claim, so a source contradicting itself is carried in the checkpoint instead
// of in side tables that were never written to it. Older checkpoints are refused by version: a
// restored projection that silently dropped a dispute would be weaker than the replay it stands in
// for, which is the one thing a checkpoint may never be.
inline constexpr quint16 kCurrentProjectionSchemaVersion = 3;

/// What is known about a subject right now.
///
/// The distinctions are the point. Never having looked is not the same as having looked and the
/// answer having aged, and neither is the same as two sources disagreeing — presenting any of them
/// as another is the failure ADR-0025 calls perception being treated as truth.
enum class EpistemicStatus {
    /// Never observed. Not a deficiency to be recovered from — simply no evidence.
    Unknown,
    /// Observed, and the observation's declared freshness horizon still covers now.
    Observed,
    /// Observed, but the horizon has lapsed. What was true then may still be true; nothing has
    /// checked. The value is kept, because forgetting it would lose evidence that was gathered.
    Stale,
    /// Two or more sources currently claim different values. Not resolved here: a projection that
    /// silently picked a winner would be inventing knowledge it does not have.
    Disputed,
    /// An earlier claim replaced by a later observation of the same subject from the same source.
    /// Only ever attached to history, never to what is currently claimed.
    Superseded,
};

QString epistemicStatusToString(EpistemicStatus status);

struct EpistemicClaim {
    /// The Event1 contribution this claim was derived from.
    ///
    /// Without it a claim is an assertion the projection makes on its own authority: a reader can
    /// see what is believed but cannot reach the evidence, and "perception is not truth" stops
    /// being checkable. With it, any answer can be traced back to the canonical record - who
    /// reported it, under what provenance, in what causal context - because the Journal still holds
    /// all of that and the projection does not have to duplicate it.
    QUuid contributionId;

    QString sourceId;

    /// How the source established this, carried through from the observation rather than
    /// reconstructed. `nixos.system` says which source spoke; this says what it actually did.
    QString provenance;

    QString subject;
    QCborValue value;
    QDateTime acquiredAt;
    QDateTime freshUntil;
    EpistemicStatus status{EpistemicStatus::Unknown};
};

/// What is currently believed about one subject, and what that replaced.
struct SubjectKnowledge {
    QString subject;
    EpistemicStatus status{EpistemicStatus::Unknown};
    /// The current claim, or claims when they disagree. Empty when nothing was ever observed.
    QList<EpistemicClaim> current;
    /// Earlier claims this subject has moved past, oldest first. Evidence, not noise: it is how a
    /// reader can see that something changed rather than merely that it is now different.
    QList<EpistemicClaim> superseded;
};

/// A reconstructible view over accepted observations.
///
/// It derives; it does not decide. Nothing here writes to Event1, resolves a contradiction, or
/// discards an observation. Losing this projection costs a replay and nothing else — the Journal
/// remains the only authority, and where the two disagree the Journal is right.
///
/// Status is evaluated against an instant supplied by the caller rather than read from the clock,
/// so the same admitted history always yields the same answer for the same moment. A projection
/// whose output depended on when it was asked could not be tested, and could not be compared
/// against itself after a rebuild.
class EpistemicProjection
{
public:
    /// Admit one accepted contribution. Anything that is not an `ObservationV1` is ignored rather
    /// than guessed at — `ContributionKind::Observation` carries several unrelated payload shapes,
    /// and the acquisition-state records the adapter writes are deliberately among them.
    ///
    /// Returns whether the contribution was an observation this projection took in.
    bool admit(const CognitiveEnvelope &envelope);

    /// Every subject observed so far, in the order first seen.
    QList<SubjectKnowledge> knowledgeAt(const QDateTime &now) const;

    /// What is known about one subject. `Unknown` when it was never observed, which is why this
    /// answers for any subject rather than failing for an unfamiliar one.
    SubjectKnowledge knowledgeOf(const QString &subject, const QDateTime &now) const;

    int observationCount() const { return m_admitted; }

    /// Serialise the derived state so a restart need not replay the whole biography.
    ///
    /// This is a checkpoint, not a second biography. ADR-0027 requires it - at ~8.9 us per
    /// contribution a full replay exhausts the Presence budget near 560k - and the same ADR fixes
    /// what it may be: an accelerator whose loss costs a replay and nothing else. If it ever
    /// disagrees with the Journal, the Journal is right.
    QByteArray snapshot() const;

    /// Restore a snapshot, failing closed.
    ///
    /// An unreadable or unrecognised checkpoint is discarded rather than partially applied: a
    /// projection half-built from a corrupt cache is worse than one rebuilt from the Journal, which
    /// is always possible and always correct.
    bool restore(const QByteArray &encoded, QString *error = nullptr);

private:
    struct History {
        QString subject;

        /// What each source currently says — one entry per source, but **one to many claims each**.
        ///
        /// A single claim per source cannot represent a source that said two different things about
        /// the same instant of acquisition, and an earlier version of this carried that case in two
        /// side tables instead. Those tables were not part of the checkpoint, so a dispute survived
        /// until the next restart and then quietly became agreement: checkpoint stopped equalling
        /// replay in precisely the case the projection exists to report.
        ///
        /// Co-current claims make the contradiction a property of the data rather than an annotation
        /// beside it. It is then persisted for free, and each claim is aged by its own freshness
        /// horizon rather than by whichever one happened to be listed first.
        QHash<QString, QList<EpistemicClaim>> currentBySource;
        QList<EpistemicClaim> superseded;
    };

    QList<QString> m_order;
    QHash<QString, History> m_bySubject;
    int m_admitted{0};
};

} // namespace cybou
