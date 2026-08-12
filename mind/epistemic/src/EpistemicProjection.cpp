// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/epistemic/EpistemicProjection.h"

#include <QCborArray>
#include <QCborMap>

#include <algorithm>

namespace cybou {

QString epistemicStatusToString(EpistemicStatus status)
{
    switch (status) {
    case EpistemicStatus::Unknown: return QStringLiteral("unknown");
    case EpistemicStatus::Observed: return QStringLiteral("observed");
    case EpistemicStatus::Stale: return QStringLiteral("stale");
    case EpistemicStatus::Disputed: return QStringLiteral("disputed");
    case EpistemicStatus::Superseded: return QStringLiteral("superseded");
    }
    return QStringLiteral("unknown");
}

bool EpistemicProjection::admit(const CognitiveEnvelope &envelope)
{
    const auto observation = decodeObservation(envelope.payloadCbor);
    if (!observation.has_value()) {
        return false;
    }

    EpistemicClaim claim;
    claim.sourceId = observation->sourceId;
    claim.subject = observation->subject;
    claim.value = observation->value;
    claim.acquiredAt = observation->acquiredAt;
    claim.freshUntil = observation->freshnessUntil;

    if (!m_bySubject.contains(claim.subject)) {
        m_order.append(claim.subject);
        m_bySubject[claim.subject].subject = claim.subject;
    }
    History &history = m_bySubject[claim.subject];

    const auto existing = history.latestBySource.constFind(claim.sourceId);
    if (existing != history.latestBySource.constEnd()) {
        // Ordering is by acquisition, not by arrival. Contributions can reach a projection out of
        // order after a replay or a restart, and an older reading must not be able to unseat a
        // newer one merely by being admitted second.
        if (existing->acquiredAt > claim.acquiredAt) {
            EpistemicClaim late = claim;
            late.status = EpistemicStatus::Superseded;
            history.superseded.append(late);
            ++m_admitted;
            return true;
        }

        // A source restating what it already said is re-affirmation, not replacement: the same
        // value observed again is one fact confirmed, and filing the earlier reading as superseded
        // would make an unchanging world look like a changing one.
        if (existing->value != claim.value) {
            EpistemicClaim previous = *existing;
            previous.status = EpistemicStatus::Superseded;
            history.superseded.append(previous);
        }
    }

    history.latestBySource.insert(claim.sourceId, claim);
    ++m_admitted;
    return true;
}

SubjectKnowledge EpistemicProjection::knowledgeOf(
    const QString &subject,
    const QDateTime &now) const
{
    SubjectKnowledge knowledge;
    knowledge.subject = subject;

    const auto found = m_bySubject.constFind(subject);
    if (found == m_bySubject.constEnd() || found->latestBySource.isEmpty()) {
        // Never observed. Distinct from stale, and the distinction is load-bearing: one says nobody
        // looked, the other says somebody looked and the answer has aged.
        knowledge.status = EpistemicStatus::Unknown;
        return knowledge;
    }

    QList<EpistemicClaim> fresh;
    QList<EpistemicClaim> lapsed;
    for (const EpistemicClaim &claim : found->latestBySource) {
        EpistemicClaim resolved = claim;
        if (claim.acquiredAt <= now && now < claim.freshUntil) {
            resolved.status = EpistemicStatus::Observed;
            fresh.append(resolved);
        } else {
            resolved.status = EpistemicStatus::Stale;
            lapsed.append(resolved);
        }
    }

    const auto byAcquisition = [](const EpistemicClaim &a, const EpistemicClaim &b) {
        return a.acquiredAt < b.acquiredAt;
    };
    std::sort(fresh.begin(), fresh.end(), byAcquisition);
    std::sort(lapsed.begin(), lapsed.end(), byAcquisition);

    knowledge.superseded = found->superseded;
    std::sort(knowledge.superseded.begin(), knowledge.superseded.end(), byAcquisition);

    if (fresh.isEmpty()) {
        // Everything known about this subject has aged out. The values are kept: discarding them
        // would lose evidence that was actually gathered, and "was X, last checked then" is a more
        // useful answer than silence.
        knowledge.status = EpistemicStatus::Stale;
        knowledge.current = lapsed;
        return knowledge;
    }

    // Disagreement counts only among claims that currently speak. A lapsed reading differing from a
    // fresh one is not a contradiction — it is the past.
    bool disagrees = false;
    for (const EpistemicClaim &claim : fresh) {
        if (claim.value != fresh.first().value) {
            disagrees = true;
            break;
        }
    }

    if (disagrees) {
        // Deliberately unresolved. Picking a winner by recency or by source would be inventing
        // knowledge; surfacing the disagreement is the honest answer and the one a reconciliation
        // policy can later act on.
        knowledge.status = EpistemicStatus::Disputed;
        for (EpistemicClaim &claim : fresh) {
            claim.status = EpistemicStatus::Disputed;
        }
        knowledge.current = fresh;
        return knowledge;
    }

    knowledge.status = EpistemicStatus::Observed;
    knowledge.current = fresh;
    return knowledge;
}

QList<SubjectKnowledge> EpistemicProjection::knowledgeAt(const QDateTime &now) const
{
    QList<SubjectKnowledge> all;
    all.reserve(m_order.size());
    for (const QString &subject : m_order) {
        all.append(knowledgeOf(subject, now));
    }
    return all;
}

namespace {

QCborMap encodeClaim(const EpistemicClaim &claim)
{
    QCborMap map;
    map.insert(QStringLiteral("sourceId"), claim.sourceId);
    map.insert(QStringLiteral("subject"), claim.subject);
    map.insert(QStringLiteral("value"), claim.value);
    map.insert(
        QStringLiteral("acquiredAt"), claim.acquiredAt.toUTC().toString(Qt::ISODateWithMs));
    map.insert(
        QStringLiteral("freshUntil"), claim.freshUntil.toUTC().toString(Qt::ISODateWithMs));
    return map;
}

// Status is deliberately not stored. It is derived from the instant a caller asks about, so
// persisting it would freeze an answer that was only ever true at one moment and hand it back as
// though it were still current.
bool decodeClaim(const QCborValue &encoded, EpistemicClaim *claim)
{
    if (!encoded.isMap()) {
        return false;
    }
    const QCborMap map = encoded.toMap();
    claim->sourceId = map.value(QStringLiteral("sourceId")).toString();
    claim->subject = map.value(QStringLiteral("subject")).toString();
    claim->value = map.value(QStringLiteral("value"));
    claim->acquiredAt = QDateTime::fromString(
        map.value(QStringLiteral("acquiredAt")).toString(), Qt::ISODateWithMs);
    claim->freshUntil = QDateTime::fromString(
        map.value(QStringLiteral("freshUntil")).toString(), Qt::ISODateWithMs);

    return !claim->sourceId.isEmpty() && !claim->subject.isEmpty()
        && !claim->value.isNull() && !claim->value.isUndefined()
        && claim->acquiredAt.isValid() && claim->freshUntil.isValid();
}

} // namespace

QByteArray EpistemicProjection::snapshot() const
{
    QCborArray subjects;
    // Written in first-seen order so a restored projection reports subjects in the same order it
    // would have after a replay. Order is part of the answer, not decoration.
    for (const QString &subject : m_order) {
        const History &history = m_bySubject.value(subject);

        QCborArray current;
        for (const EpistemicClaim &claim : history.latestBySource) {
            current.append(encodeClaim(claim));
        }
        QCborArray superseded;
        for (const EpistemicClaim &claim : history.superseded) {
            superseded.append(encodeClaim(claim));
        }

        QCborMap entry;
        entry.insert(QStringLiteral("subject"), subject);
        entry.insert(QStringLiteral("current"), current);
        entry.insert(QStringLiteral("superseded"), superseded);
        subjects.append(entry);
    }

    QCborMap root;
    root.insert(QStringLiteral("schemaVersion"), kCurrentProjectionSchemaVersion);
    root.insert(QStringLiteral("admitted"), m_admitted);
    root.insert(QStringLiteral("subjects"), subjects);
    return root.toCborValue().toCbor();
}

bool EpistemicProjection::restore(const QByteArray &encoded, QString *error)
{
    const auto fail = [error](const QString &reason) {
        if (error) {
            *error = reason;
        }
        return false;
    };
    if (error) {
        error->clear();
    }

    const QCborValue root = QCborValue::fromCbor(encoded);
    if (!root.isMap()) {
        return fail(QStringLiteral("projection checkpoint is not a map"));
    }
    const QCborMap map = root.toMap();
    if (map.value(QStringLiteral("schemaVersion")).toInteger(-1)
        != kCurrentProjectionSchemaVersion) {
        // Refused rather than best-effort parsed, in both directions. A checkpoint is a cache, and
        // rebuilding from the Journal is always available and always correct, so guessing at an
        // unrecognised one buys nothing and risks a projection that is quietly wrong.
        return fail(QStringLiteral("projection checkpoint schema is not supported"));
    }
    if (!map.value(QStringLiteral("subjects")).isArray()) {
        return fail(QStringLiteral("projection checkpoint has no subjects"));
    }

    QList<QString> order;
    QHash<QString, History> bySubject;

    for (const QCborValue &entry : map.value(QStringLiteral("subjects")).toArray()) {
        if (!entry.isMap()) {
            return fail(QStringLiteral("projection checkpoint has a malformed subject"));
        }
        const QCborMap subjectMap = entry.toMap();
        const QString subject = subjectMap.value(QStringLiteral("subject")).toString();
        if (subject.isEmpty()) {
            return fail(QStringLiteral("projection checkpoint has an unnamed subject"));
        }

        History history;
        history.subject = subject;
        for (const QCborValue &encodedClaim : subjectMap.value(QStringLiteral("current")).toArray()) {
            EpistemicClaim claim;
            if (!decodeClaim(encodedClaim, &claim)) {
                return fail(QStringLiteral("projection checkpoint has a malformed claim"));
            }
            history.latestBySource.insert(claim.sourceId, claim);
        }
        for (const QCborValue &encodedClaim :
             subjectMap.value(QStringLiteral("superseded")).toArray()) {
            EpistemicClaim claim;
            if (!decodeClaim(encodedClaim, &claim)) {
                return fail(QStringLiteral("projection checkpoint has a malformed claim"));
            }
            claim.status = EpistemicStatus::Superseded;
            history.superseded.append(claim);
        }

        order.append(subject);
        bySubject.insert(subject, history);
    }

    // Applied only once every subject parsed. A partially restored projection would be a third
    // thing - neither the Journal's answer nor a clean rebuild - and nothing would say which.
    m_order = order;
    m_bySubject = bySubject;
    m_admitted = map.value(QStringLiteral("admitted")).toInteger(0);
    return true;
}

} // namespace cybou
