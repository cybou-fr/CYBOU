// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/context/AssociativeProjection.h"

#include <algorithm>

namespace cybou {

QString conceptKindToString(ConceptKind kind)
{
    switch (kind) {
    case ConceptKind::Subject:    return QStringLiteral("subject");
    case ConceptKind::Value:      return QStringLiteral("value");
    case ConceptKind::Episode:    return QStringLiteral("episode");
    case ConceptKind::Preference: return QStringLiteral("preference");
    }
    return QStringLiteral("subject");
}

QString relationTypeToString(RelationType type)
{
    switch (type) {
    case RelationType::IsA:           return QStringLiteral("is-a");
    case RelationType::HasValue:      return QStringLiteral("has-value");
    case RelationType::UsedWith:      return QStringLiteral("used-with");
    case RelationType::PreferredBy:   return QStringLiteral("preferred-by");
    case RelationType::PartOfEpisode: return QStringLiteral("part-of-episode");
    case RelationType::CoOccursWith:  return QStringLiteral("co-occurs-with");
    }
    return QStringLiteral("co-occurs-with");
}

QString associationOriginToString(AssociationOrigin origin)
{
    switch (origin) {
    case AssociationOrigin::Observed:       return QStringLiteral("observed");
    case AssociationOrigin::UserDeclared:   return QStringLiteral("user-declared");
    case AssociationOrigin::Derived:        return QStringLiteral("derived");
    case AssociationOrigin::ModelSuggested: return QStringLiteral("model-suggested");
    case AssociationOrigin::Statistical:    return QStringLiteral("statistical");
    }
    return QStringLiteral("statistical");
}

bool AssociativeProjection::addConcept(const ConceptNode &node)
{
    if (!node.isValid()) {
        return false;
    }
    if (!m_concepts.contains(node.id)) {
        m_order.append(node.id);
    }
    m_concepts.insert(node.id, node);
    return true;
}

bool AssociativeProjection::addAssociation(const Association &association)
{
    // Both ends must be known concepts. An edge to something that does not exist would produce a
    // retrieval nobody could explain, and explanation is the property this graph is for.
    if (!association.isValid() || !m_concepts.contains(association.from)
        || !m_concepts.contains(association.to)) {
        return false;
    }
    m_outgoing[association.from].append(association);
    return true;
}

QList<ConceptNode> AssociativeProjection::concepts() const
{
    QList<ConceptNode> all;
    all.reserve(m_order.size());
    for (const QString &id : m_order) {
        all.append(m_concepts.value(id));
    }
    return all;
}

QList<Association> AssociativeProjection::associationsFrom(const QString &conceptId) const
{
    return m_outgoing.value(conceptId);
}

ContextBundle AssociativeProjection::activate(
    const QList<QString> &seeds, const ActivationBudget &budget, const QUuid &requestId) const
{
    ContextBundle bundle;
    bundle.requestId = requestId;

    QHash<QString, double> relevance;
    QHash<QString, QString> reason;
    QList<QString> discovered;

    // Breadth-first from the seeds, so depth means what it says and a wide shallow graph cannot be
    // mistaken for a deep one. Every budget dimension is a hard stop rather than a hint: the point
    // of bounding activation is that the word "lemon" cannot cost an unbounded amount of work.
    QList<QString> frontier;
    for (const QString &seed : seeds) {
        if (!m_concepts.contains(seed) || relevance.contains(seed)) {
            continue;
        }
        relevance.insert(seed, 1.0);
        reason.insert(seed, QStringLiteral("seed"));
        discovered.append(seed);
        frontier.append(seed);
    }

    int edgesWalked = 0;
    bool truncated = false;

    for (int depth = 0; depth < budget.maxDepth && !frontier.isEmpty(); ++depth) {
        QList<QString> next;
        for (const QString &current : frontier) {
            // Sorted before walking, so the order edges are considered in does not depend on hash
            // iteration order. Determinism is A1, and a bundle that varied between runs of the same
            // question would be unauditable.
            QList<Association> edges = m_outgoing.value(current);
            std::sort(edges.begin(), edges.end(), [](const Association &a, const Association &b) {
                if (a.strength != b.strength) {
                    return a.strength > b.strength;
                }
                if (a.to != b.to) {
                    return a.to < b.to;
                }
                return static_cast<int>(a.type) < static_cast<int>(b.type);
            });

            for (const Association &edge : edges) {
                if (edgesWalked >= budget.maxEdges || discovered.size() >= budget.maxNodes) {
                    truncated = true;
                    break;
                }
                ++edgesWalked;

                const double reached = relevance.value(current) * edge.strength;
                const auto existing = relevance.constFind(edge.to);
                if (existing != relevance.constEnd() && *existing >= reached) {
                    continue;
                }

                if (existing == relevance.constEnd()) {
                    discovered.append(edge.to);
                    next.append(edge.to);
                }
                relevance.insert(edge.to, reached);
                reason.insert(
                    edge.to,
                    QStringLiteral("%1 -> %2 -> %3 (%4, strength %5)")
                        .arg(
                            current,
                            relationTypeToString(edge.type),
                            edge.to,
                            associationOriginToString(edge.origin))
                        .arg(edge.strength, 0, 'f', 2));
            }
            if (truncated) {
                break;
            }
        }
        if (truncated) {
            break;
        }
        frontier = next;
    }

    for (const QString &id : discovered) {
        const ConceptNode node = m_concepts.value(id);
        ContextItem item;
        item.conceptId = id;
        item.relevance = relevance.value(id);
        item.evidence = node.evidence;
        item.privacy = node.privacy;
        item.activationReason = reason.value(id);
        bundle.items.append(item);
    }

    std::sort(
        bundle.items.begin(), bundle.items.end(), [](const ContextItem &a, const ContextItem &b) {
            if (a.relevance != b.relevance) {
                return a.relevance > b.relevance;
            }
            return a.conceptId < b.conceptId;
        });

    // Complete means the graph was exhausted within budget, not that something was returned. A
    // retrieval that hit a limit says so, and a caller that wanted everything relevant knows it did
    // not get it.
    bundle.complete = !truncated;
    return bundle;
}

} // namespace cybou
