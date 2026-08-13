// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/fabric/OrganClients.h"

#include "cybou/fabric/FabricCodec.h"
#include "cybou/fabric/OrganBus.h"

#include <QDBusConnection>

namespace cybou {

namespace {

QString bestError(
    const QString &codec,
    const RpcClient &rpc)
{
    return !codec.isEmpty()
        ? codec
        : rpc.lastError();
}

QVariantMap decodeMap(
    const QByteArray &encoded,
    QString *codecError)
{
    if (codecError) {
        codecError->clear();
    }
    return FabricCodec::decodeMap(encoded, codecError);
}

QVariantList decodeList(
    const QByteArray &encoded,
    QString *codecError)
{
    if (codecError) {
        codecError->clear();
    }
    return FabricCodec::decodeList(encoded, codecError);
}

bool connectChanged(
    const BusEndpoint &endpoint,
    QObject *receiver,
    const char *slot)
{
    return QDBusConnection::sessionBus().connect(
        QString::fromLatin1(endpoint.service),
        QString::fromLatin1(endpoint.objectPath),
        QString::fromLatin1(endpoint.interfaceName),
        QStringLiteral("Changed"),
        receiver,
        slot);
}

} // namespace

HealthClient::HealthClient(QObject *parent)
    : QObject(parent)
    , m_rpc(kHealthEndpoint)
{
    if (!connectChanged(kHealthEndpoint, this, SLOT(onChanged())))
        m_codecError = QStringLiteral("cannot subscribe to Health1 Changed");
}

CapabilitySnapshot HealthClient::snapshot(int timeoutMs) const
{
    m_codecError.clear();
    const QByteArray encoded = m_rpc.callBytes(QStringLiteral("Snapshot"), {}, timeoutMs);
    if (encoded.isEmpty()) return {};
    return decodeCapabilitySnapshot(encoded, &m_codecError);
}

HomeostasisSnapshot HealthClient::measurements() const
{
    m_codecError.clear();
    const QByteArray encoded = m_rpc.callBytes(QStringLiteral("Measurements"));
    if (encoded.isEmpty()) return {};
    return decodeHomeostasisSnapshot(encoded, &m_codecError);
}

QString HealthClient::lastError() const
{
    return bestError(m_codecError, m_rpc);
}

void HealthClient::onChanged()
{
    Q_EMIT changed();
}

IdentityClient::IdentityClient()
    : m_rpc(kIdentityEndpoint)
{
}

QVariantMap IdentityClient::state(int timeoutMs) const
{
    return decodeMap(
        m_rpc.callBytes(QStringLiteral("State"), {}, timeoutMs),
        &m_codecError);
}

QString IdentityClient::lastError() const
{
    return bestError(m_codecError, m_rpc);
}

IntentionClient::IntentionClient()
    : m_rpc(kIntentionEndpoint)
{
}

QVariantList IntentionClient::open(int timeoutMs, bool *ok) const
{
    const QByteArray encoded = m_rpc.callBytes(QStringLiteral("Open"), {}, timeoutMs);
    if (ok) {
        // An encoded fabric list always carries its version envelope, so it is never empty. Empty
        // bytes therefore mean the call failed - which intentiond now reports explicitly rather
        // than answering with an empty set.
        *ok = !encoded.isEmpty();
    }
    return decodeList(encoded, &m_codecError);
}

QString IntentionClient::form(
    const QString &description,
    const QString &trigger,
    const QString &causeId,
    int timeoutMs) const
{
    m_codecError.clear();
    return m_rpc.callString(
        QStringLiteral("Form"),
        {description, trigger, causeId},
        timeoutMs);
}

bool IntentionClient::close(
    const QString &intentionId,
    int resolution,
    const QString &note,
    int timeoutMs) const
{
    m_codecError.clear();
    return m_rpc.callBool(
        QStringLiteral("Close"),
        {intentionId, resolution, note},
        timeoutMs);
}

QString IntentionClient::lastError() const
{
    return bestError(m_codecError, m_rpc);
}

PredictorClient::PredictorClient()
    : m_rpc(kPredictorEndpoint)
{
}

bool PredictorClient::observe(
    const QString &subject,
    double value,
    int timeoutMs) const
{
    m_codecError.clear();
    return m_rpc.callBool(
        QStringLiteral("Observe"),
        {subject, value},
        timeoutMs);
}

QVariantMap PredictorClient::predict(
    const QString &subject,
    const QString &correlationId,
    int timeoutMs) const
{
    return decodeMap(
        m_rpc.callBytes(
            QStringLiteral("Predict"),
            {subject, correlationId},
            timeoutMs),
        &m_codecError);
}

bool PredictorClient::settle(
    const QString &forecastId,
    double actual) const
{
    m_codecError.clear();
    return m_rpc.callBool(
        QStringLiteral("Settle"),
        {forecastId, actual});
}

QVariantList PredictorClient::calibrations(int timeoutMs) const
{
    return decodeList(
        m_rpc.callBytes(QStringLiteral("Calibrations"), {}, timeoutMs),
        &m_codecError);
}

QString PredictorClient::lastError() const
{
    return bestError(m_codecError, m_rpc);
}

SelfClient::SelfClient()
    : m_rpc(kSelfEndpoint)
{
}

QVariantMap SelfClient::measure(int timeoutMs) const
{
    return decodeMap(
        m_rpc.callBytes(QStringLiteral("Measure"), {}, timeoutMs),
        &m_codecError);
}

QVariantMap SelfClient::assess(
    const QString &causeId,
    int timeoutMs) const
{
    return decodeMap(
        m_rpc.callBytes(
            QStringLiteral("Assess"),
            {causeId},
            timeoutMs),
        &m_codecError);
}

QString SelfClient::narration() const
{
    m_codecError.clear();
    return m_rpc.callString(QStringLiteral("Narration"));
}

QString SelfClient::lastError() const
{
    return bestError(m_codecError, m_rpc);
}

WorkspaceClient::WorkspaceClient(QObject *parent)
    : QObject(parent)
    , m_rpc(kWorkspaceEndpoint)
{
    if (!connectChanged(
            kWorkspaceEndpoint,
            this,
            SLOT(onChanged()))) {
        m_codecError =
            QStringLiteral("cannot subscribe to Workspace1 Changed");
    }
}

QVariantList WorkspaceClient::coalitions(int timeoutMs) const
{
    return decodeList(
        m_rpc.callBytes(QStringLiteral("Coalitions"), {}, timeoutMs),
        &m_codecError);
}

QVariantMap WorkspaceClient::moment(int timeoutMs) const
{
    return decodeMap(
        m_rpc.callBytes(QStringLiteral("Moment"), {}, timeoutMs),
        &m_codecError);
}

QString WorkspaceClient::attention(int timeoutMs) const
{
    m_codecError.clear();
    return m_rpc.callString(QStringLiteral("Attention"), {}, timeoutMs);
}

QString WorkspaceClient::lastError() const
{
    return bestError(m_codecError, m_rpc);
}

void WorkspaceClient::onChanged()
{
    Q_EMIT changed();
}

LifecycleClient::LifecycleClient(QObject *parent)
    : QObject(parent)
    , m_rpc(kLifecycleEndpoint)
{
    if (!connectChanged(kLifecycleEndpoint, this, SLOT(onChanged()))) {
        m_codecError = QStringLiteral("cannot subscribe to Lifecycle1 Changed");
    }
}

QVariantMap LifecycleClient::state(int timeoutMs) const
{
    return decodeMap(
        m_rpc.callBytes(QStringLiteral("State"), {}, timeoutMs),
        &m_codecError);
}

QVariantMap LifecycleClient::schedulingEvaluation(int timeoutMs) const
{
    return decodeMap(
        m_rpc.callBytes(QStringLiteral("EvaluateScheduling"), {}, timeoutMs),
        &m_codecError);
}

bool LifecycleClient::notifyUserActivity(const QString &cause, int timeoutMs) const
{
    m_codecError.clear();
    return m_rpc.callBool(QStringLiteral("NotifyUserActivity"), {cause}, timeoutMs);
}

bool LifecycleClient::finishRun(
    const QString &status,
    const QString &cause,
    int timeoutMs) const
{
    m_codecError.clear();
    return m_rpc.callBool(
        QStringLiteral("FinishRun"),
        {status, cause},
        timeoutMs);
}

QString LifecycleClient::lastError() const
{
    return bestError(m_codecError, m_rpc);
}

void LifecycleClient::onChanged()
{
    Q_EMIT changed();
}

PresenceClient::PresenceClient(QObject *parent)
    : QObject(parent)
    , m_rpc(kPresenceEndpoint)
{
    if (!connectChanged(
            kPresenceEndpoint,
            this,
            SLOT(onChanged()))) {
        m_codecError =
            QStringLiteral("cannot subscribe to Presence1 Changed");
    }
}

QVariantMap PresenceClient::snapshot() const
{
    return decodeMap(
        m_rpc.callBytes(QStringLiteral("Snapshot")),
        &m_codecError);
}

QVariantList PresenceClient::activity(int limit) const
{
    return decodeList(
        m_rpc.callBytes(
            QStringLiteral("Activity"),
            {limit}),
        &m_codecError);
}

QVariantList PresenceClient::detailedObligations() const
{
    return decodeList(
        m_rpc.callBytes(QStringLiteral("DetailedObligations")),
        &m_codecError);
}

QString PresenceClient::promise(
    const QString &description) const
{
    m_codecError.clear();
    return m_rpc.callString(
        QStringLiteral("Promise"),
        {description});
}

bool PresenceClient::reflect() const
{
    m_codecError.clear();
    return m_rpc.callBool(QStringLiteral("Reflect"));
}

bool PresenceClient::fulfillIndex(int index) const
{
    m_codecError.clear();
    return m_rpc.callBool(
        QStringLiteral("FulfillIndex"),
        {index});
}

bool PresenceClient::abandonIndex(int index) const
{
    m_codecError.clear();
    return m_rpc.callBool(
        QStringLiteral("AbandonIndex"),
        {index});
}

bool PresenceClient::observe(
    const QString &subject,
    double value) const
{
    m_codecError.clear();
    return m_rpc.callBool(
        QStringLiteral("Observe"),
        {subject, value});
}

QVariantMap PresenceClient::predict(
    const QString &subject) const
{
    return decodeMap(
        m_rpc.callBytes(
            QStringLiteral("Predict"),
            {subject}),
        &m_codecError);
}

QString PresenceClient::lastError() const
{
    return bestError(m_codecError, m_rpc);
}

void PresenceClient::onChanged()
{
    Q_EMIT changed();
}

} // namespace cybou
