// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "WorkspaceService.h"

#include "cybou/fabric/FabricCodec.h"

#include <QCborMap>
#include <QDateTime>

namespace cybou {

namespace {
const QUuid kConsolidationNamespace(
    QStringLiteral("8fcbaf7c-b31a-5c7d-b15e-a09b7b816ca7"));
}

WorkspaceService::WorkspaceService(
    EventStore *events,
    QObject *parent)
    : QObject(parent)
    , m_events(events)
    , m_workspace(events)
{
    m_workspace.rehydrate();

    connect(
        &m_workspace,
        &Workspace::contributed,
        this,
        [this](const CognitiveEnvelope &) {
            Q_EMIT Changed();
        });
}

bool WorkspaceService::Ready() const
{
    return m_events && m_events->isOpen();
}

QString WorkspaceService::Health() const
{
    return Ready()
        ? QStringLiteral("healthy")
        : QStringLiteral("unavailable");
}

QString WorkspaceService::LastError() const
{
    return m_events ? m_events->lastError() : QString();
}

QByteArray WorkspaceService::Coalitions() const
{
    QVariantList result;

    for (const Coalition &coalition :
         m_workspace.coalitions()) {
        QVariantMap map;
        map[QStringLiteral("correlationId")] =
            coalition.correlationId.toString(QUuid::WithoutBraces);
        map[QStringLiteral("salience")] =
            coalition.salience;
        map[QStringLiteral("organs")] =
            coalition.organs();
        map[QStringLiteral("threads")] =
            coalition.threadCount();
        map[QStringLiteral("latest")] =
            coalition.latest;
        result.append(map);
    }

    return FabricCodec::encodeList(result);
}

QByteArray WorkspaceService::Moment() const
{
    const MomentState state = m_workspace.momentState();

    QVariantMap map;
    map[QStringLiteral("focus")] =
        state.focus.toString(QUuid::WithoutBraces);
    map[QStringLiteral("salience")] =
        state.salience;
    map[QStringLiteral("organs")] =
        state.organs;

    return FabricCodec::encodeMap(map);
}

QString WorkspaceService::Attention() const
{
    const Coalition focus = m_workspace.focus();
    if (!focus.isValid()) {
        return {};
    }

    const CognitiveEnvelope &latest =
        focus.members.last();
    const QStringList voices = focus.organs();

    if (voices.size() > 1) {
        return QObject::tr(
                   "%1, with %n organ(s) involved",
                   nullptr,
                   voices.size())
            .arg(kindToString(latest.kind));
    }

    return QObject::tr("%1, from %2")
        .arg(
            kindToString(latest.kind),
            latest.originOrgan);
}

QByteArray WorkspaceService::Consolidate(const QString &runId,const QString &operationKey,qulonglong mark) const
{
    if (!Ready() || QUuid(runId).isNull() || operationKey.trimmed().isEmpty()
        || mark == 0 || mark > m_events->count()) return {};
    const auto input = m_events->atSequence(mark);
    if (!input) return {};

    const QUuid contributionId = QUuid::createUuidV5(
        kConsolidationNamespace,
        QStringLiteral("workspace:%1").arg(operationKey).toUtf8());
    if (!m_events->contains(contributionId)) {
        CognitiveEnvelope contribution;
        contribution.messageId = contributionId;
        contribution.correlationId = QUuid(runId);
        contribution.causationId = input->messageId;
        contribution.originOrgan = QStringLiteral("workspaced");
        contribution.originNode = QStringLiteral("local");
        contribution.kind = ContributionKind::Learning;
        contribution.wallTime = QDateTime::currentDateTimeUtc();
        contribution.privacy = input->privacy;
        contribution.capabilityScope = QStringLiteral("lifecycle.consolidation");
        QCborMap payload;
        payload[QStringLiteral("operationKey")] = operationKey;
        payload[QStringLiteral("inputHighWaterMark")] = static_cast<qint64>(mark);
        payload[QStringLiteral("coalitionCount")] = m_workspace.coalitions().size();
        contribution.payloadCbor = payload.toCborValue().toCbor();
        if (m_events->append(contribution) == 0) return {};
    }
    QVariantMap receipt;
    receipt[QStringLiteral("accepted")]=true;
    receipt[QStringLiteral("owner")]=QStringLiteral("workspace");
    receipt[QStringLiteral("operationKey")]=operationKey;
    receipt[QStringLiteral("inputHighWaterMark")]=mark;
    receipt[QStringLiteral("contributionId")]=contributionId.toString(QUuid::WithoutBraces);
    receipt[QStringLiteral("coalitionCount")]=m_workspace.coalitions().size();
    return FabricCodec::encodeMap(receipt);
}

} // namespace cybou
