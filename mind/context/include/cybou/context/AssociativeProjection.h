// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/protocol/CognitiveEnvelope.h"

#include <QDateTime>
#include <QHash>
#include <QList>
#include <QString>
#include <QUuid>

namespace cybou {

/// What kind of thing a node in the association graph is.
enum class ConceptKind : quint8 {
    Subject = 0, ///< something perception reports about
    Value,       ///< a value a subject has taken
    Episode,     ///< a stretch of the biography
    Preference,  ///< something the person has stated or shown
};

/// How two concepts are related.
enum class RelationType : quint8 {
    IsA = 0,
    HasValue,
    UsedWith,
    PreferredBy,
    PartOfEpisode,
    CoOccursWith,
};

/// Where an association came from, and therefore what it is worth.
///
/// ADR-0029 makes this a closed set because it is the field that keeps association from becoming
/// knowledge. `lemon → yellow` and `lemon → makes people kinder` may both exist; they must never be
/// indistinguishable, and this organ does not adjudicate between them.
enum class AssociationOrigin : quint8 {
    Observed = 0,     ///< seen in the biography
    UserDeclared,     ///< stated by the person
    Derived,          ///< inferred from other associations
    ModelSuggested,   ///< proposed by a language faculty
    Statistical,      ///< co-occurrence, nothing more
};

QString conceptKindToString(ConceptKind kind);
QString relationTypeToString(RelationType type);
QString associationOriginToString(AssociationOrigin origin);

struct ConceptNode {
    QString id;
    ConceptKind kind{ConceptKind::Subject};

    /// The contributions this concept was derived from. Without them a concept is an assertion the
    /// projection makes on its own authority.
    QList<QUuid> evidence;

    PrivacyClass privacy{PrivacyClass::Local};
    RetentionClass retentionClass{RetentionClass::Standard};
    QDateTime retainUntil;

    bool isValid() const { return !id.isEmpty(); }
};

struct Association {
    QString from;
    QString to;
    RelationType type{RelationType::CoOccursWith};

    double strength{0.0};
    AssociationOrigin origin{AssociationOrigin::Statistical};
    QList<QUuid> evidence;

    bool isValid() const
    {
        return !from.isEmpty() && !to.isEmpty() && strength > 0.0 && strength <= 1.0;
    }
};

/// One retrieved item, and why it was retrieved.
///
/// `activationReason` is not decoration. ADR-0029's A12 requires every item to answer "why was I
/// retrieved?" without a language model being asked to invent a plausible story — a memory that
/// needs a model to explain its own retrieval has given away the property the layer exists to keep.
struct ContextItem {
    QString conceptId;
    double relevance{0.0};
    QList<QUuid> evidence;
    PrivacyClass privacy{PrivacyClass::Local};
    QString activationReason;
};

struct ContextBundle {
    QUuid requestId;
    QList<ContextItem> items;

    /// Whether retrieval finished, as opposed to running out of budget or failing.
    ///
    /// A cut-short retrieval says so rather than returning a short list. Partial or unavailable is
    /// not empty truth, and this is that invariant applied to relevance.
    bool complete{false};
};

/// Budget for one activation. Every dimension is enforced rather than advisory.
struct ActivationBudget {
    int maxNodes{32};
    int maxEdges{64};
    int maxDepth{3};
};

/// The association graph, derived from the biography and never authoritative over it.
///
/// This owns what is *related*. It does not own what is true: for epistemic force a caller asks
/// epistemicd, and a claim never gains standing by being retrieved.
class AssociativeProjection
{
public:
    bool addConcept(const ConceptNode &node);
    bool addAssociation(const Association &association);

    /// Spread activation from seeds, bounded and deterministic.
    ///
    /// Deterministic for the same graph, seeds and budget: ADR-0029's A1. Ranking may improve; that
    /// property may not change, because a memory that answers differently each time it is asked the
    /// same question cannot be audited.
    ContextBundle activate(
        const QList<QString> &seeds,
        const ActivationBudget &budget,
        const QUuid &requestId = QUuid()) const;

    QList<ConceptNode> concepts() const;
    QList<Association> associationsFrom(const QString &conceptId) const;

    int conceptCount() const { return m_concepts.size(); }

private:
    QHash<QString, ConceptNode> m_concepts;
    QHash<QString, QList<Association>> m_outgoing;
    QList<QString> m_order;
};

} // namespace cybou
