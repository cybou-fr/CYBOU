// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "PerceptionService.h"

#include "cybou/protocol/Observation.h"

#include <QCborMap>
#include <QCborValue>
#include <QUuid>

namespace cybou {

namespace {

// Distinct from the observation namespace: an availability transition is a different kind of fact
// and must not be able to collide with an observation's identity.
const QUuid kTransitionNamespace(QStringLiteral("2b7f4e60-91c8-5a3d-8f16-4c05e7d2ba98"));

constexpr char kTransitionPayloadType[] = "cybou.acquisition-state.v1";

} // namespace

PerceptionService::PerceptionService(
    EventStore *events,
    SystemGenerationSource source,
    QObject *parent)
    : QObject(parent)
    , m_events(events)
    , m_source(std::move(source))
{
}

bool PerceptionService::Ready() const
{
    return m_events != nullptr && m_events->isOpen();
}

// Being unable to read the source is not this adapter being unhealthy. The source's availability is
// what it reports; its own health is whether it can report at all, which means having a journal to
// report into. Conflating them would make an absent source look like a broken organ.
QString PerceptionService::Health() const
{
    return Ready() ? QStringLiteral("healthy") : QStringLiteral("unavailable");
}

QString PerceptionService::LastError() const
{
    return m_lastError;
}

QByteArray PerceptionService::State() const
{
    QCborMap state;
    state.insert(QStringLiteral("sourceId"), SystemGenerationSource::sourceId());
    state.insert(QStringLiteral("observed"), m_haveObserved);
    state.insert(
        QStringLiteral("status"),
        m_haveObserved ? acquisitionStatusToString(m_lastStatus) : QStringLiteral("never-attempted"));
    state.insert(
        QStringLiteral("acquiredAt"),
        m_lastAcquiredAt.isValid() ? m_lastAcquiredAt.toUTC().toString(Qt::ISODateWithMs)
                                   : QString());
    return state.toCborValue().toCbor();
}

void PerceptionService::acquireOnce()
{
    if (!Ready()) {
        m_lastError = QStringLiteral("no journal to contribute to");
        return;
    }

    const QDateTime now = QDateTime::currentDateTimeUtc();
    const AcquisitionResult result = m_source.acquire(now);

    const bool changed = !m_haveObserved || result.status != m_lastStatus;
    if (changed) {
        recordAvailabilityTransition(result.status, now);
    }
    m_haveObserved = true;
    m_lastStatus = result.status;

    if (!result.acquired()) {
        // Not an error of this organ, and not a contribution. The source's unavailability is
        // reported through State() and, on a change, through the transition above.
        m_lastError = result.detail;
        Q_EMIT Changed();
        return;
    }

    if (!shouldContribute(result.observation)) {
        // Read, unchanged, and the previous contribution still speaks for now. Nothing durable to
        // add; the source was still checked, which is what keeps the transition rule honest.
        m_lastError.clear();
        Q_EMIT Changed();
        return;
    }

    CognitiveEnvelope envelope;
    envelope.messageId = observationMessageId(
        result.observation.sourceId,
        result.observation.subject,
        result.observation.acquiredAt,
        result.observation.value);
    envelope.correlationId = envelope.messageId;
    envelope.originOrgan = QStringLiteral("perceptiond");
    envelope.originNode = QStringLiteral("local");
    envelope.kind = ContributionKind::Observation;
    envelope.wallTime = now;
    envelope.confidence = 1.0;
    // Non-sensitive by construction: ADR-0027 forbids ingesting anything sensitive until a retention
    // ADR exists, and this source was chosen so that constraint costs nothing.
    envelope.privacy = PrivacyClass::Local;
    envelope.payloadCbor = encodeObservation(result.observation);

    m_lastAcquiredAt = result.observation.acquiredAt;

    // A repeat of an unchanged reading resolves to the same messageId, so Event1 rejects it as a
    // duplicate and the poll costs nothing durable. That rejection is the expected path, not a
    // failure, which is why it is not recorded as an error.
    if (m_events->append(envelope) == 0) {
        const QString reason = m_events->lastError();
        m_lastError = reason.contains(QStringLiteral("already exists")) ? QString() : reason;
    } else {
        m_lastError.clear();
        m_haveContributed = true;
        m_lastContributedValue = result.observation.value;
        m_lastContributedFreshUntil = result.observation.freshnessUntil;
    }

    Q_EMIT Changed();
}

bool PerceptionService::shouldContribute(const ObservationV1 &observation) const
{
    if (!m_haveContributed) {
        return true;
    }
    // A different value is a different fact and is always worth recording, or nothing downstream
    // could supersede the earlier reading.
    if (observation.value != m_lastContributedValue) {
        return true;
    }
    // Unchanged: the previous contribution speaks until its horizon lapses. Re-affirm once it has,
    // so a fact that stays true does not drift into looking merely old.
    return !m_lastContributedFreshUntil.isValid()
        || observation.acquiredAt >= m_lastContributedFreshUntil;
}

void PerceptionService::recordAvailabilityTransition(
    AcquisitionStatus status,
    const QDateTime &at)
{
    CognitiveEnvelope envelope;
    // Deterministic over source, status and instant, so a restart that re-observes the same
    // transition does not record it twice.
    envelope.messageId = QUuid::createUuidV5(
        kTransitionNamespace,
        QStringLiteral("%1|%2|%3")
            .arg(
                SystemGenerationSource::sourceId(),
                acquisitionStatusToString(status),
                at.toUTC().toString(Qt::ISODateWithMs))
            .toUtf8());
    envelope.correlationId = envelope.messageId;
    envelope.originOrgan = QStringLiteral("perceptiond");
    envelope.originNode = QStringLiteral("local");
    envelope.kind = ContributionKind::Observation;
    envelope.wallTime = at;
    envelope.confidence = 1.0;
    envelope.privacy = PrivacyClass::Local;

    QCborMap payload;
    payload.insert(QStringLiteral("@type"), QString::fromLatin1(kTransitionPayloadType));
    payload.insert(QStringLiteral("sourceId"), SystemGenerationSource::sourceId());
    payload.insert(QStringLiteral("status"), acquisitionStatusToString(status));
    payload.insert(QStringLiteral("since"), at.toUTC().toString(Qt::ISODateWithMs));
    envelope.payloadCbor = payload.toCborValue().toCbor();

    m_events->append(envelope);
}

} // namespace cybou
