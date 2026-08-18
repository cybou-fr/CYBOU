// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/context/ContextDelivery.h"

#include <QCryptographicHash>

namespace cybou {

QString dispositionToString(Disposition disposition)
{
    switch (disposition) {
    case Disposition::Delivered:
        return QStringLiteral("delivered");
    case Disposition::HeldBackByPolicy:
        return QStringLiteral("held-back-by-policy");
    case Disposition::ExcludedByPerson:
        return QStringLiteral("excluded-by-person");
    case Disposition::NotSelected:
        return QStringLiteral("not-selected");
    }
    return QStringLiteral("unknown");
}

QString consumerTrustToString(ConsumerTrust trust)
{
    switch (trust) {
    case ConsumerTrust::Untrusted:
        return QStringLiteral("untrusted");
    case ConsumerTrust::Bounded:
        return QStringLiteral("bounded");
    case ConsumerTrust::Full:
        return QStringLiteral("full");
    }
    return QStringLiteral("unknown");
}

SensitivityClass DeliveryPolicy::ceilingFor(ConsumerTrust trust) const
{
    switch (trust) {
    case ConsumerTrust::Untrusted:
        return ceilingForUntrusted;
    case ConsumerTrust::Bounded:
        return ceilingForBounded;
    case ConsumerTrust::Full:
        return ceilingForFull;
    }
    return SensitivityClass::Ordinary;
}

bool DeliveryPolicy::permits(const ContextItem &item, const Destination &destination) const
{
    // No early return for a local consumer. ADR-0030's B7: every destination is filtered by its own
    // trust, and being on the same machine is not a permission.
    //
    // Disclosure is decided on sensitivity. No ceiling reaches Secret or Credential, so those
    // never travel to any consumer at all, which makes ADR-0033 A9 a property of the delivery path
    // rather than a rule a training pipeline has to remember.
    if (static_cast<int>(item.sensitivity) > static_cast<int>(ceilingFor(destination.trust))) {
        return false;
    }

    // Scope still governs distance: a contribution this consumer may be shown can still be one
    // that must not leave the device.
    if (destination.externalBoundary
        && static_cast<int>(item.privacy) < static_cast<int>(scopeForExternal)) {
        return false;
    }
    return true;
}


DeliveryPlan DeliveryPlan::build(
    const ContextBundle &bundle,
    const DeliveryPolicy &policy,
    const Destination &destination,
    const QSet<QString> &selected,
    const QSet<QString> &excludedByPerson)
{
    DeliveryPlan plan;
    plan.m_requestId = bundle.requestId;
    plan.m_destination = destination;
    plan.m_complete = bundle.complete;

    // Every activated item produces exactly one decision. The loop has no `continue` that skips an
    // item, because that is precisely the shape B6 forbids.
    for (const ContextItem &item : bundle.items) {
        DeliveryDecision decision;
        decision.conceptId = item.conceptId;
        decision.evidence = item.evidence;

        if (excludedByPerson.contains(item.conceptId)) {
            decision.disposition = Disposition::ExcludedByPerson;
            decision.reason = QStringLiteral("excluded by the person");
        } else if (!policy.permits(item, destination)) {
            decision.disposition = Disposition::HeldBackByPolicy;
            decision.reason = QStringLiteral("%1 trust of %2 does not permit %3 material")
                                  .arg(consumerTrustToString(destination.trust))
                                  .arg(destination.id)
                                  .arg(sensitivityToString(item.sensitivity));
        } else if (!selected.contains(item.conceptId)) {
            decision.disposition = Disposition::NotSelected;
            decision.reason = QStringLiteral("available, not selected for this request");
        } else {
            decision.disposition = Disposition::Delivered;
            decision.reason = item.activationReason;
        }

        plan.m_decisions.append(decision);
    }

    return plan;
}

QList<DeliveryDecision> DeliveryPlan::withDisposition(Disposition disposition) const
{
    QList<DeliveryDecision> out;
    for (const DeliveryDecision &decision : m_decisions) {
        if (decision.disposition == disposition) {
            out.append(decision);
        }
    }
    return out;
}

QList<QString> DeliveryPlan::availableIds() const
{
    QList<QString> out;
    for (const DeliveryDecision &decision : m_decisions) {
        if (decision.disposition != Disposition::HeldBackByPolicy) {
            out.append(decision.conceptId);
        }
    }
    return out;
}

QList<QString> DeliveryPlan::deliveredIds() const
{
    QList<QString> out;
    for (const DeliveryDecision &decision : m_decisions) {
        if (decision.disposition == Disposition::Delivered) {
            out.append(decision.conceptId);
        }
    }
    return out;
}

QByteArray deliveryDigest(const QList<DeliveryDecision> &plan)
{
    QCryptographicHash hash(QCryptographicHash::Sha256);
    for (const DeliveryDecision &decision : plan) {
        if (decision.disposition != Disposition::Delivered) {
            continue;
        }
        hash.addData(decision.conceptId.toUtf8());
        for (const QUuid &source : decision.evidence) {
            hash.addData(source.toRfc4122());
        }
    }
    return hash.result();
}

bool requiresRecord(const Destination &destination)
{
    return destination.retains || destination.externalBoundary;
}

std::optional<DeliveryRecord> recordFor(const DeliveryPlan &plan)
{
    const Destination destination = plan.destination();
    if (!requiresRecord(destination)) {
        return std::nullopt;
    }

    DeliveryRecord record;
    record.requestId = plan.requestId();
    record.destinationId = destination.id;
    record.externalBoundary = destination.externalBoundary;
    record.retained = destination.retains;

    for (const DeliveryDecision &decision : plan.decisions()) {
        if (decision.disposition == Disposition::Delivered) {
            record.deliveredConceptIds.append(decision.conceptId);
            record.evidence.append(decision.evidence);
        } else if (decision.disposition == Disposition::HeldBackByPolicy) {
            ++record.heldBackCount;
        }
    }

    return record;
}

} // namespace cybou
