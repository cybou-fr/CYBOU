// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "ContextService.h"

#include "cybou/ipc/CallerIdentity.h"

#include <QCryptographicHash>
#include <QDBusMessage>

#include "cybou/fabric/FabricCodec.h"
#include "cybou/protocol/Observation.h"

#include <QCborMap>
#include <QCborValue>
#include <QDBusError>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QSaveFile>

namespace cybou {

namespace {

inline constexpr quint16 kCheckpointSchemaVersion = 1;
inline constexpr int kReplayPageSize = 1000;

QVariantMap encodeItem(const ContextItem &item)
{
    QVariantMap map;
    map.insert(QStringLiteral("concept"), item.conceptId);
    map.insert(QStringLiteral("relevance"), item.relevance);
    map.insert(QStringLiteral("privacy"), privacyToString(item.privacy));
    // Always present, never optional. A retrieval nobody can explain is the failure this layer is
    // built to avoid, so the explanation crosses the wire with the item rather than being available
    // on request.
    map.insert(QStringLiteral("why"), item.activationReason);

    QVariantList evidence;
    for (const QUuid &id : item.evidence) {
        evidence.append(id.toString(QUuid::WithoutBraces));
    }
    map.insert(QStringLiteral("evidence"), evidence);
    return map;
}

} // namespace

ContextService::ContextService(EventStore *events, QString checkpointPath, QObject *parent)
    : QObject(parent)
    , m_events(events)
    , m_checkpointPath(std::move(checkpointPath))
{
    if (!m_events || !m_events->isOpen()) {
        m_startupError = QStringLiteral("contextd has no journal to derive from");
        return;
    }

    load();

    if (!catchUp()) {
        m_startupError = m_lastError;
        return;
    }
    m_ready = true;
}

bool ContextService::Ready() const
{
    return m_ready;
}

QString ContextService::Health() const
{
    return m_ready ? QStringLiteral("healthy") : QStringLiteral("unavailable");
}

QString ContextService::LastError() const
{
    return m_lastError;
}

qulonglong ContextService::Cursor() const
{
    return static_cast<qulonglong>(m_cursor);
}

ConsumerTrust ContextService::callerCeiling() const
{
    if (!calledFromDBus()) {
        return ConsumerTrust::Full;
    }
    return ConsumerRegistry::ceilingFor(callerBinaryName(connection(), message().service()));
}

bool ContextService::refuseWhenUnready(const QString &method)
{
    if (m_ready) {
        return true;
    }
    if (calledFromDBus()) {
        sendErrorReply(
            QDBusError::Failed,
            QStringLiteral("%1 cannot answer: %2")
                .arg(method, m_startupError.isEmpty() ? m_lastError : m_startupError));
    }
    return false;
}

QByteArray ContextService::Activate(const QStringList &seeds, int maxNodes, int maxDepth)
{
    if (!refuseWhenUnready(QStringLiteral("Activate"))) {
        return {};
    }

    ActivationBudget budget;
    // A caller may ask for less than the default but never for more: the budget exists so that one
    // word cannot cost an unbounded amount of work, and a limit a caller can raise is not a limit.
    budget.maxNodes = maxNodes > 0 ? std::min(maxNodes, budget.maxNodes) : budget.maxNodes;
    budget.maxDepth = maxDepth > 0 ? std::min(maxDepth, budget.maxDepth) : budget.maxDepth;

    const ContextBundle bundle = m_projection.activate(QList<QString>(seeds.cbegin(), seeds.cend()), budget);

    QVariantList items;
    for (const ContextItem &item : bundle.items) {
        items.append(encodeItem(item));
    }

    QVariantMap map;
    map.insert(QStringLiteral("items"), items);
    // Carried across the wire, because a caller must be able to tell "nothing else is related" from
    // "I stopped looking". Those are different answers and only one of them is knowledge.
    map.insert(QStringLiteral("complete"), bundle.complete);
    return FabricCodec::encode(map);
}

QByteArray ContextService::Prepare(const QStringList &seeds, int maxNodes, int maxDepth)
{
    if (!refuseWhenUnready(QStringLiteral("Prepare"))) {
        return {};
    }

    ActivationBudget budget;
    budget.maxNodes = maxNodes > 0 ? std::min(maxNodes, budget.maxNodes) : budget.maxNodes;
    budget.maxDepth = maxDepth > 0 ? std::min(maxDepth, budget.maxDepth) : budget.maxDepth;

    // Identity before activation, so the bundle carries it. Minting it afterwards would leave every
    // bundle with a null id, which is what a delivery record cannot be built from.
    const QUuid requestId = QUuid::createUuid();
    const ContextBundle bundle = m_projection.activate(
        QList<QString>(seeds.cbegin(), seeds.cend()), budget, requestId);

    PreparedRequest prepared;
    prepared.bundle = bundle;
    prepared.cursor = m_cursor;
    prepared.erasureEpoch = m_erasureEpoch;

    m_prepared.insert(requestId, prepared);
    m_preparedOrder.append(requestId);
    while (m_preparedOrder.size() > kMaxPreparedRequests) {
        m_prepared.remove(m_preparedOrder.takeFirst());
    }

    QVariantList items;
    for (const ContextItem &item : bundle.items) {
        items.append(encodeItem(item));
    }

    QVariantMap map;
    // Reported from the bundle, never from the local variable. Sending a parallel copy would let
    // the bundle keep a null id while the wire looked correct, which is precisely the gap that
    // made every runtime DeliveryRecord invalid while the library tests passed.
    map.insert(QStringLiteral("requestId"), bundle.requestId.toString(QUuid::WithoutBraces));
    map.insert(QStringLiteral("items"), items);
    map.insert(QStringLiteral("complete"), bundle.complete);
    return FabricCodec::encode(map);
}

QByteArray ContextService::Deliver(
    const QString &requestId,
    const QString &destinationId,
    int trust,
    bool retains,
    bool externalBoundary,
    const QStringList &selected,
    const QStringList &excluded)
{
    if (!refuseWhenUnready(QStringLiteral("Deliver"))) {
        return {};
    }

    const auto refuse = [this](QDBusError::ErrorType type, const QString &why) -> QByteArray {
        if (calledFromDBus()) {
            sendErrorReply(type, why);
        }
        m_lastError = why;
        return {};
    };

    if (destinationId.isEmpty()) {
        return refuse(QDBusError::InvalidArgs, QStringLiteral("Deliver needs a named destination"));
    }

    // An unrecognised trust level is refused, never defaulted. Reading an unknown value as the most
    // trusted would hand a caller full context by sending a number nobody implemented; reading it
    // as the least would hide the caller's bug behind a silently narrowed answer.
    if (trust < static_cast<int>(ConsumerTrust::Untrusted)
        || trust > static_cast<int>(ConsumerTrust::Full)) {
        return refuse(QDBusError::InvalidArgs,
                      QStringLiteral("Deliver does not know trust level %1").arg(trust));
    }

    // What the caller asked for is a request, not a fact about itself. The ceiling comes from its
    // verified process, and asking above it is refused rather than quietly reduced: a consumer
    // handed a narrowed answer to a request it believed was granted would reason as though nothing
    // had been withheld.
    const ConsumerTrust ceiling = callerCeiling();
    if (!ConsumerRegistry::permitsRequest(ceiling, static_cast<ConsumerTrust>(trust))) {
        return refuse(
            QDBusError::AccessDenied,
            QStringLiteral("this caller may request at most %1 trust, not %2")
                .arg(consumerTrustToString(ceiling))
                .arg(consumerTrustToString(static_cast<ConsumerTrust>(trust))));
    }

    const QUuid id = QUuid::fromString(requestId);
    if (id.isNull() || !m_prepared.contains(id)) {
        return refuse(QDBusError::InvalidArgs,
                      QStringLiteral("Deliver has no prepared request %1").arg(requestId));
    }

    const PreparedRequest prepared = m_prepared.value(id);

    // The projection has moved: what was inspected is no longer what this organ holds. Refused
    // rather than re-activated, because delivering a freshly computed bundle under an id the person
    // approved for a different one is the substitution this whole flow exists to prevent.
    if (prepared.cursor != m_cursor || prepared.erasureEpoch != m_erasureEpoch) {
        return refuse(
            QDBusError::Failed,
            QStringLiteral("request %1 is stale: prepared at cursor %2 epoch %3, now %4 and %5")
                .arg(requestId)
                .arg(prepared.cursor)
                .arg(prepared.erasureEpoch)
                .arg(m_cursor)
                .arg(m_erasureEpoch));
    }

    Destination destination;
    destination.id = destinationId;
    destination.trust = static_cast<ConsumerTrust>(trust);
    destination.retains = retains;
    destination.externalBoundary = externalBoundary;

    const DeliveryPlan plan = DeliveryPlan::build(
        prepared.bundle,
        DeliveryPolicy(),
        destination,
        QSet<QString>(selected.cbegin(), selected.cend()),
        QSet<QString>(excluded.cbegin(), excluded.cend()));

    // Keep the plan for the person. Inspection must report what was actually acted on rather than
    // recompute a plan that could differ from the one this delivery used.
    PreparedRequest recorded = prepared;
    recorded.plan = plan.decisions();
    recorded.destination = destination;
    recorded.delivered = true;
    m_prepared.insert(id, recorded);

    const int deliveredCount = plan.withDisposition(Disposition::Delivered).size();

    QVariantMap map;
    map.insert(QStringLiteral("requestId"), id.toString(QUuid::WithoutBraces));

    if (requiresRecord(destination)) {
        // The payload stays here until the disclosure is on record. Handing it over first and
        // asking for the record afterwards makes the record optional in practice: the disclosure
        // has happened either way, and only a well-behaved consumer produces the evidence.
        recorded.disclosureDigest = deliveryDigest(plan.decisions());
        m_prepared.insert(id, recorded);

        map.insert(QStringLiteral("disclosurePending"), true);
        map.insert(QStringLiteral("disclosureDigest"),
                   QString::fromLatin1(recorded.disclosureDigest.toHex()));
        map.insert(QStringLiteral("deliveredCount"), deliveredCount);
    } else {
        map.insert(QStringLiteral("disclosurePending"), false);
        map.insert(QStringLiteral("delivered"), deliveredPayload(recorded));
    }

    // A count, never identities. The consumer is owed the knowledge that its answer was narrowed --
    // partial is not empty, and a consumer that believed it had everything would reason as though
    // nothing had been withheld -- but it is not owed what was withheld or why.
    map.insert(QStringLiteral("withheldCount"), plan.size() - deliveredCount);
    map.insert(QStringLiteral("complete"), plan.complete());
    map.insert(QStringLiteral("destination"), destination.id);
    map.insert(QStringLiteral("trust"), consumerTrustToString(destination.trust));
    map.insert(QStringLiteral("sourceCursor"), static_cast<qulonglong>(prepared.cursor));
    // Reported, not written. The caller that performs the delivery owns that contribution, because
    // this organ owns no writes at all.
    map.insert(QStringLiteral("recordRequired"), requiresRecord(destination));
    return FabricCodec::encode(map);
}

QVariantList ContextService::deliveredPayload(const PreparedRequest &prepared) const
{
    // Evidence travels with what was delivered: whoever holds the disclosure has to be able to
    // name its provenance, and cannot reconstruct that from concept ids alone.
    QVariantList delivered;
    for (const DeliveryDecision &decision : prepared.plan) {
        if (decision.disposition != Disposition::Delivered) {
            continue;
        }

        QVariantList evidence;
        for (const QUuid &source : decision.evidence) {
            evidence.append(source.toString(QUuid::WithoutBraces));
        }

        QVariantMap entry;
        entry.insert(QStringLiteral("concept"), decision.conceptId);
        entry.insert(QStringLiteral("reason"), decision.reason);
        entry.insert(QStringLiteral("evidence"), evidence);
        delivered.append(entry);
    }
    return delivered;
}

QByteArray ContextService::Release(const QString &requestId, const QString &disclosureId)
{
    if (!refuseWhenUnready(QStringLiteral("Release"))) {
        return {};
    }

    const auto refuse = [this](QDBusError::ErrorType type, const QString &why) -> QByteArray {
        m_lastError = why;
        if (calledFromDBus()) {
            sendErrorReply(type, why);
        }
        return {};
    };

    const QUuid id = QUuid::fromString(requestId);
    if (id.isNull() || !m_prepared.contains(id)) {
        return refuse(QDBusError::InvalidArgs,
                      QStringLiteral("Release has no prepared request %1").arg(requestId));
    }

    const PreparedRequest prepared = m_prepared.value(id);
    if (!prepared.delivered) {
        return refuse(QDBusError::Failed,
                      QStringLiteral("request %1 has no delivery to release").arg(requestId));
    }
    if (prepared.disclosureDigest.isEmpty()) {
        return refuse(QDBusError::Failed,
                      QStringLiteral("request %1 owes no disclosure record").arg(requestId));
    }

    if (!m_events) {
        return refuse(QDBusError::Failed, QStringLiteral("no journal to verify a disclosure"));
    }

    const auto record = m_events->contribution(QUuid::fromString(disclosureId));
    if (!record.has_value()) {
        return refuse(QDBusError::InvalidArgs,
                      QStringLiteral("no contribution %1").arg(disclosureId));
    }
    if (record->kind != ContributionKind::ContextDisclosed) {
        return refuse(QDBusError::InvalidArgs,
                      QStringLiteral("contribution %1 is not a disclosure").arg(disclosureId));
    }

    // The record must commit to this request, this consumer, and exactly what is about to be
    // released. A disclosure naming a different digest records a different event than the one it
    // would be used to authorise.
    const QCborMap payload = QCborValue::fromCbor(record->payloadCbor).toMap();
    const QString recordedRequest
        = payload.value(QStringLiteral("requestId")).toString();
    const QString recordedDestination
        = payload.value(QStringLiteral("destination")).toString();
    const QByteArray recordedDigest = QByteArray::fromHex(
        payload.value(QStringLiteral("digest")).toString().toLatin1());

    if (recordedRequest != id.toString(QUuid::WithoutBraces)
        || recordedDestination != prepared.destination.id
        || recordedDigest != prepared.disclosureDigest) {
        return refuse(
            QDBusError::AccessDenied,
            QStringLiteral("disclosure %1 does not match this delivery").arg(disclosureId));
    }

    QVariantMap map;
    map.insert(QStringLiteral("requestId"), id.toString(QUuid::WithoutBraces));
    map.insert(QStringLiteral("delivered"), deliveredPayload(prepared));
    map.insert(QStringLiteral("disclosure"), disclosureId);
    return FabricCodec::encode(map);
}

QByteArray ContextService::Inspect(const QString &requestId)
{
    if (!refuseWhenUnready(QStringLiteral("Inspect"))) {
        return {};
    }

    const QUuid id = QUuid::fromString(requestId);
    if (id.isNull() || !m_prepared.contains(id)) {
        m_lastError = QStringLiteral("Inspect has no prepared request %1").arg(requestId);
        if (calledFromDBus()) {
            sendErrorReply(QDBusError::InvalidArgs, m_lastError);
        }
        return {};
    }

    // Inspection is the one surface that names held-back items. Granting it to anything less than
    // a fully trusted caller would hand back through Inspect exactly what Deliver refuses to say.
    if (ConsumerRegistry::permitsRequest(callerCeiling(), ConsumerTrust::Full) == false) {
        m_lastError = QStringLiteral("this caller may not inspect a delivery plan");
        if (calledFromDBus()) {
            sendErrorReply(QDBusError::AccessDenied, m_lastError);
        }
        return {};
    }

    const PreparedRequest prepared = m_prepared.value(id);
    if (!prepared.delivered) {
        m_lastError = QStringLiteral("request %1 has no delivery to inspect").arg(requestId);
        if (calledFromDBus()) {
            sendErrorReply(QDBusError::Failed, m_lastError);
        }
        return {};
    }

    // Every disposition, including what was held back and why. This is the one surface where the
    // gap between available and delivered is visible, which is the whole of B6.
    QVariantList decisions;
    for (const DeliveryDecision &decision : prepared.plan) {
        QVariantList evidence;
        for (const QUuid &source : decision.evidence) {
            evidence.append(source.toString(QUuid::WithoutBraces));
        }

        QVariantMap entry;
        entry.insert(QStringLiteral("concept"), decision.conceptId);
        entry.insert(QStringLiteral("disposition"), dispositionToString(decision.disposition));
        entry.insert(QStringLiteral("reason"), decision.reason);
        entry.insert(QStringLiteral("evidence"), evidence);
        decisions.append(entry);
    }

    QVariantMap map;
    map.insert(QStringLiteral("requestId"), id.toString(QUuid::WithoutBraces));
    map.insert(QStringLiteral("decisions"), decisions);
    map.insert(QStringLiteral("destination"), prepared.destination.id);
    map.insert(QStringLiteral("trust"), consumerTrustToString(prepared.destination.trust));
    map.insert(QStringLiteral("sourceCursor"), static_cast<qulonglong>(prepared.cursor));
    return FabricCodec::encode(map);
}

void ContextService::admitToGraph(const CognitiveEnvelope &envelope)
{
    const auto observation = decodeObservation(envelope.payloadCbor);
    if (!observation.has_value()) {
        return;
    }

    const QString subject = observation->subject;
    const QString value = observation->value.toVariant().toString();
    if (subject.isEmpty() || value.isEmpty()) {
        return;
    }

    // Privacy and retention are inherited from the contribution the concept was derived from, not
    // chosen here. A concept that was more permissive than its evidence would be a way to launder
    // a private observation into a retrievable one.
    ConceptNode subjectNode;
    subjectNode.id = subject;
    subjectNode.kind = ConceptKind::Subject;
    subjectNode.evidence = {envelope.messageId};
    subjectNode.privacy = envelope.privacy;
    subjectNode.sensitivity = envelope.schemaVersion == kClassifiedEnvelopeSchemaVersion
        ? envelope.sensitivity
        : kUnclassifiedSensitivity;
    subjectNode.retentionClass = envelope.retentionClass;
    subjectNode.retainUntil = envelope.retainUntil;
    m_projection.addConcept(subjectNode);

    ConceptNode valueNode = subjectNode;
    valueNode.id = value;
    valueNode.kind = ConceptKind::Value;
    m_projection.addConcept(valueNode);

    Association observed;
    observed.from = subject;
    observed.to = value;
    observed.type = RelationType::HasValue;
    // Observed rather than inferred, and recorded as such: this edge exists because perception
    // reported it, which is a different kind of thing from a model suggesting it.
    observed.origin = AssociationOrigin::Observed;
    observed.strength = 0.9;
    observed.evidence = {envelope.messageId};
    m_projection.addAssociation(observed);
}

bool ContextService::catchUp()
{
    if (!m_events || !m_events->isOpen()) {
        m_lastError = QStringLiteral("the journal is unavailable");
        return false;
    }

    const quint64 startedAt = m_cursor;
    m_erasureEpoch = m_events->erasureEpoch();

    for (;;) {
        const ContributionPage page = m_events->after(m_cursor, kReplayPageSize);
        if (!page.ok) {
            m_lastError = QStringLiteral("could not read contributions after %1").arg(m_cursor);
            return false;
        }
        if (page.envelopes.isEmpty()) {
            break;
        }

        for (const CognitiveEnvelope &envelope : page.envelopes) {
            admitToGraph(envelope);
        }
        if (page.lastSequence <= m_cursor) {
            m_lastError = QStringLiteral("the journal did not advance past %1").arg(m_cursor);
            return false;
        }
        m_cursor = page.lastSequence;

        if (!page.hasMore) {
            break;
        }
    }

    m_lastError.clear();
    if (m_cursor != startedAt) {
        persist();
        Q_EMIT Changed();
    }
    return true;
}

void ContextService::admitAccepted(const CognitiveEnvelope &envelope, quint64 sequence)
{
    if (sequence <= m_cursor) {
        return;
    }

    // Same rule as epistemicd: a gap that cannot be closed means this announcement is not admitted,
    // because moving the cursor past unread history is the one failure nothing downstream could
    // discover.
    if (sequence > m_cursor + 1 && !catchUp()) {
        return;
    }
    if (sequence <= m_cursor) {
        return;
    }
    if (sequence != m_cursor + 1) {
        m_lastError =
            QStringLiteral("a gap below %1 is still unread; not admitting it").arg(sequence);
        return;
    }

    admitToGraph(envelope);
    m_cursor = sequence;
    persist();
    Q_EMIT Changed();
}

bool ContextService::load()
{
    QFile file(m_checkpointPath);
    if (!file.exists() || !file.open(QIODevice::ReadOnly)) {
        return false;
    }

    const QCborValue root = QCborValue::fromCbor(file.readAll());
    if (!root.isMap()
        || root.toMap().value(QStringLiteral("schemaVersion")).toInteger(-1)
            != kCheckpointSchemaVersion) {
        return false;
    }

    const QCborMap map = root.toMap();

    // A checkpoint built before an erasure may hold a concept whose evidence has since been
    // redacted, and working out which one would need exactly the payload that is gone. So it is
    // discarded whole and the graph rebuilt - the epoch makes that decidable without inspecting
    // anything.
    bool epochParsed = false;
    const quint64 storedEpoch =
        map.value(QStringLiteral("erasureEpoch")).toString().toULongLong(&epochParsed);
    if (!epochParsed || !m_events || storedEpoch != m_events->erasureEpoch()) {
        return false;
    }

    bool cursorParsed = false;
    const quint64 cursor =
        map.value(QStringLiteral("cursor")).toString().toULongLong(&cursorParsed);
    if (!cursorParsed) {
        return false;
    }

    m_cursor = cursor;
    m_erasureEpoch = storedEpoch;
    return true;
}

void ContextService::persist()
{
    QDir().mkpath(QFileInfo(m_checkpointPath).absolutePath());

    QCborMap root;
    root.insert(QStringLiteral("schemaVersion"), kCheckpointSchemaVersion);
    root.insert(QStringLiteral("cursor"), QString::number(m_cursor));
    root.insert(QStringLiteral("erasureEpoch"), QString::number(m_erasureEpoch));

    QSaveFile file(m_checkpointPath);
    if (!file.open(QIODevice::WriteOnly)) {
        return;
    }
    if (file.write(root.toCborValue().toCbor()) < 0) {
        return;
    }
    file.commit();
}

} // namespace cybou
