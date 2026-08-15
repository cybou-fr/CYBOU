// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/context/AssociativeProjection.h"

#include <QList>
#include <QSet>
#include <QString>
#include <QUuid>

namespace cybou {

/// What happened to one activated item on its way out — or on its way to not going out.
///
/// ADR-0030's B6 is the reason this is an enum rather than a filtered list. An item dropped for
/// policy reasons and an item that was never relevant look identical the moment the interface is
/// allowed to express "absent". Every activated item carries one of these, so absence is not a
/// representable state.
enum class Disposition : quint8 {
    Delivered = 0,      ///< left the machine
    HeldBackByPolicy,   ///< permitted set excluded it; the person still sees that it exists
    ExcludedByPerson,   ///< the person removed it
    NotSelected,        ///< available and permitted, but not chosen for this request
};

QString dispositionToString(Disposition disposition);

/// Where a request is going. `remote` is the only property that changes what policy permits.
struct Destination {
    QString id;
    bool remote{false};

    bool isValid() const { return !id.isEmpty(); }
};

/// What a destination is permitted to consider.
///
/// Policy narrows; it never widens and never edits. It answers one question per item and records
/// the answer, which is what makes it a reviewable object rather than a filter buried in a request
/// builder.
struct DeliveryPolicy {
    /// The most restrictive class a remote destination may see. PrivacyClass runs from most to
    /// least restrictive, so a floor of `Local` permits everything and `Public` permits only what
    /// was already public. A local destination is not filtered at all.
    PrivacyClass maxPrivacyForRemote{PrivacyClass::Household};

    bool permits(const ContextItem &item, const Destination &destination) const;
};

/// One item's outcome, with the reason it had that outcome.
struct DeliveryDecision {
    QString conceptId;
    Disposition disposition{Disposition::NotSelected};
    QString reason;
    QList<QUuid> evidence;
};

/// The four sets of ADR-0030, materialised as one list rather than four.
///
/// Four separate lists would let a caller render one and call it the truth. One list of decisions
/// cannot be rendered without rendering the held-back items too.
class DeliveryPlan
{
public:
    DeliveryPlan() = default;

    /// Build the plan. `bundle` is const on purpose: B2 requires that policy produce a different
    /// delivered set without altering what Mind considered relevant.
    static DeliveryPlan build(
        const ContextBundle &bundle,
        const DeliveryPolicy &policy,
        const Destination &destination,
        const QSet<QString> &selected,
        const QSet<QString> &excludedByPerson);

    QList<DeliveryDecision> decisions() const { return m_decisions; }
    QList<DeliveryDecision> withDisposition(Disposition disposition) const;

    /// Items that may be considered at all: everything policy did not hold back.
    QList<QString> availableIds() const;
    QList<QString> deliveredIds() const;

    QUuid requestId() const { return m_requestId; }
    Destination destination() const { return m_destination; }

    /// Whether the retrieval this plan was built from finished. A plan over a truncated bundle is
    /// itself partial, and says so rather than looking like a complete answer with fewer items.
    bool complete() const { return m_complete; }

    int size() const { return m_decisions.size(); }

private:
    QList<DeliveryDecision> m_decisions;
    QUuid m_requestId;
    Destination m_destination;
    bool m_complete{false};
};

/// The durable fact that something left the machine.
///
/// Item ids and evidence, never content. A record that copied what it recorded would make the
/// Journal a second store of the material the delivery was already the risk of.
struct DeliveryRecord {
    QUuid requestId;
    QString destinationId;
    bool remote{false};
    QList<QString> deliveredConceptIds;
    QList<QUuid> evidence;
    int heldBackCount{0};

    bool isValid() const { return !destinationId.isEmpty() && !requestId.isNull(); }
};

DeliveryRecord recordFor(const DeliveryPlan &plan);

} // namespace cybou
