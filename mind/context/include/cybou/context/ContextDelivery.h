// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/context/AssociativeProjection.h"

#include <QList>
#include <optional>
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
    Delivered = 0,      ///< supplied to the consumer
    HeldBackByPolicy,   ///< permitted set excluded it; the person still sees that it exists
    ExcludedByPerson,   ///< the person removed it
    NotSelected,        ///< available and permitted, but not chosen for this request
};

QString dispositionToString(Disposition disposition);

/// How much of the person's context a consumer is permitted to see.
///
/// ADR-0030's B7. A single `remote` bit was the original abstraction and it was too weak: once
/// ADR-0021 moved cognition off remote models entirely, a policy that only filtered remote
/// consumers filtered nothing that mattered. A parser, a local model and an inspector run in the
/// same place and deserve different answers.
enum class ConsumerTrust : quint8 {
    Untrusted = 0, ///< a plugin or third-party consumer; sees only what was already public
    Bounded,       ///< an ordinary faculty; sees up to the household class
    Full,          ///< a first-party surface acting for the person
};

QString consumerTrustToString(ConsumerTrust trust);

/// A named consumer, described by what it may see and what it does with what it gets.
struct Destination {
    QString id;
    ConsumerTrust trust{ConsumerTrust::Bounded};

    /// Whether what it receives outlives the request -- stored, indexed, or learned from.
    ///
    /// This, not distance, is what makes a delivery consequential. A local model that adapts on
    /// delivered context has written it into parameters ADR-0033 admits cannot be surgically
    /// unlearned, and the delivery record is the only evidence of how that influence travelled.
    bool retains{false};

    /// Whether delivery crosses a network or trust boundary. Irreversible on its own account.
    bool externalBoundary{false};

    bool isValid() const { return !id.isEmpty(); }
};

/// What a destination is permitted to consider.
///
/// Policy narrows; it never widens and never edits. It answers one question per item and records
/// the answer, which is what makes it a reviewable object rather than a filter buried in a request
/// builder.
struct DeliveryPolicy {
    /// The most restrictive class each trust level may see. PrivacyClass runs from most to least
    /// restrictive, so a floor of `Local` permits everything and `Public` only what was already
    /// public.
    ///
    /// Every level has a floor, including the most trusted one. There is no unfiltered case: a
    /// consumer gains context by being permitted, never by being nearby.
    PrivacyClass floorForUntrusted{PrivacyClass::Public};
    PrivacyClass floorForBounded{PrivacyClass::Household};
    PrivacyClass floorForFull{PrivacyClass::Local};

    PrivacyClass floorFor(ConsumerTrust trust) const;
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

/// The durable fact that context was supplied to a named consumer.
///
/// Not "what left the machine": ADR-0030 was amended once cognition moved off remote models, and
/// a local consumer that retains what it receives is the case that most needs a trail. Distance
/// stopped being the axis.
///
/// Item ids and evidence, never content. A record that copied what it recorded would make the
/// Journal a second store of the material the delivery was already the risk of.
struct DeliveryRecord {
    QUuid requestId;
    QString destinationId;
    bool externalBoundary{false};
    bool retained{false};
    QList<QString> deliveredConceptIds;
    QList<QUuid> evidence;
    int heldBackCount{0};

    bool isValid() const { return !destinationId.isEmpty() && !requestId.isNull(); }
};

/// A commitment to exactly what a plan would release, and to nothing else.
///
/// Covers the delivered items only. A digest over the whole plan would tie a permanent record to
/// material the consumer never received -- and concept spaces are small enough to brute-force, so
/// that record would become standing evidence about what was withheld, written into the one place
/// that is never erased.
///
/// It lives here rather than inside the service because it is a property of a plan, and because a
/// commitment nobody can test in isolation is a commitment nobody has checked.
QByteArray deliveryDigest(const QList<DeliveryDecision> &plan);

/// Whether this delivery must leave a durable trace.
///
/// True when the consumer retains or adapts on what it receives, or when delivery crosses an
/// external boundary. Recording every render of an inspector that forgets immediately would grow
/// the Journal with use and prove nothing; recording nothing about a consumer that learned from
/// the context would leave ADR-0033's erasure invalidation with no trail to follow.
bool requiresRecord(const Destination &destination);

/// The record, when one is required. Nothing otherwise, rather than an empty record that a caller
/// could mistake for a delivery that happened to carry nothing.
std::optional<DeliveryRecord> recordFor(const DeliveryPlan &plan);

} // namespace cybou
