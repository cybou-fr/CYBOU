// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "ContextService.h"

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
