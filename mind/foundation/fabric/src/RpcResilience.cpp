// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/fabric/RpcResilience.h"

#include <QDateTime>
#include <QDBusConnection>
#include <QDBusPendingCallWatcher>
#include <QTimer>

#include <algorithm>
#include <memory>

namespace cybou {
namespace {

QString text(const char *value)
{
    return QString::fromLatin1(value);
}

bool isInfrastructureFailure(RpcOutcome outcome)
{
    return outcome == RpcOutcome::Unavailable || outcome == RpcOutcome::TimedOut
        || outcome == RpcOutcome::UnknownOutcome;
}

} // namespace

struct AsyncRpcClient::Operation {
    QString method;
    QVariantList arguments;
    RpcOperationSemantics semantics{RpcOperationSemantics::ReadOnly};
    Completion completion;
    int timeoutMs{5000};
    bool expectBooleanAcceptance{false};
    int attempts{0};
    quint32 seed{0};
};

bool RpcRetryPolicy::isValid() const
{
    return maximumAttempts > 0 && baseDelayMs >= 0 && maximumDelayMs >= baseDelayMs
        && jitterPercent >= 0 && jitterPercent <= 100 && circuitFailureThreshold > 0
        && circuitOpenMs >= 0;
}

QString rpcOutcomeToString(RpcOutcome outcome)
{
    switch (outcome) {
    case RpcOutcome::Succeeded: return QStringLiteral("succeeded");
    case RpcOutcome::Unavailable: return QStringLiteral("unavailable");
    case RpcOutcome::TimedOut: return QStringLiteral("timed-out");
    case RpcOutcome::Rejected: return QStringLiteral("rejected");
    case RpcOutcome::UnknownOutcome: return QStringLiteral("unknown-outcome");
    case RpcOutcome::CircuitOpen: return QStringLiteral("circuit-open");
    }
    return QStringLiteral("unknown");
}

bool isRetrySafe(RpcOperationSemantics semantics) noexcept
{
    return semantics == RpcOperationSemantics::ReadOnly
        || semantics == RpcOperationSemantics::IdempotentMutation;
}

bool shouldRetry(
    RpcOutcome outcome,
    RpcOperationSemantics semantics,
    int completedAttempts,
    const RpcRetryPolicy &policy) noexcept
{
    return policy.isValid() && isRetrySafe(semantics)
        && completedAttempts < policy.maximumAttempts
        && (outcome == RpcOutcome::Unavailable || outcome == RpcOutcome::TimedOut);
}

int retryDelayMs(
    int completedAttempts,
    quint32 deterministicSeed,
    const RpcRetryPolicy &policy) noexcept
{
    if (!policy.isValid() || completedAttempts <= 0) {
        return 0;
    }
    qint64 delay = policy.baseDelayMs;
    for (int attempt = 1; attempt < completedAttempts && delay < policy.maximumDelayMs; ++attempt) {
        delay = std::min<qint64>(policy.maximumDelayMs, delay * 2);
    }
    const qint64 spread = delay * policy.jitterPercent / 100;
    if (spread == 0) {
        return static_cast<int>(delay);
    }
    const quint32 mixed = deterministicSeed ^ (0x9e3779b9U * static_cast<quint32>(completedAttempts));
    const qint64 offset = static_cast<qint64>(mixed % static_cast<quint32>(spread * 2 + 1)) - spread;
    return static_cast<int>(std::clamp<qint64>(delay + offset, 0, policy.maximumDelayMs));
}

RpcOutcome classifyDBusReply(
    const QDBusMessage &reply,
    RpcOperationSemantics semantics,
    bool expectBooleanAcceptance)
{
    if (reply.type() == QDBusMessage::ErrorMessage) {
        const QString name = reply.errorName();
        if (name.contains(QStringLiteral("NoReply"), Qt::CaseInsensitive)
            || name.contains(QStringLiteral("Timeout"), Qt::CaseInsensitive)) {
            return semantics == RpcOperationSemantics::NonIdempotentMutation
                ? RpcOutcome::UnknownOutcome
                : RpcOutcome::TimedOut;
        }
        if (name.contains(QStringLiteral("ServiceUnknown"), Qt::CaseInsensitive)
            || name.contains(QStringLiteral("NameHasNoOwner"), Qt::CaseInsensitive)
            || name.contains(QStringLiteral("Disconnected"), Qt::CaseInsensitive)) {
            return RpcOutcome::Unavailable;
        }
        return RpcOutcome::Rejected;
    }
    if (expectBooleanAcceptance
        && (reply.arguments().isEmpty() || !reply.arguments().first().toBool())) {
        return RpcOutcome::Rejected;
    }
    return RpcOutcome::Succeeded;
}

CircuitBreaker::CircuitBreaker(const RpcRetryPolicy &policy)
    : m_policy(policy)
{
}

CircuitState CircuitBreaker::state(qint64 nowMs) const noexcept
{
    if (m_openedAtMs < 0) {
        return CircuitState::Closed;
    }
    return nowMs - m_openedAtMs >= m_policy.circuitOpenMs
        ? CircuitState::HalfOpen
        : CircuitState::Open;
}

bool CircuitBreaker::allow(qint64 nowMs) noexcept
{
    const CircuitState current = state(nowMs);
    if (current == CircuitState::Closed) {
        return true;
    }
    if (current == CircuitState::Open || m_halfOpenProbeUsed) {
        return false;
    }
    m_halfOpenProbeUsed = true;
    return true;
}

void CircuitBreaker::record(RpcOutcome outcome, qint64 nowMs) noexcept
{
    if (outcome == RpcOutcome::Succeeded || outcome == RpcOutcome::Rejected) {
        m_failures = 0;
        m_openedAtMs = -1;
        m_halfOpenProbeUsed = false;
        return;
    }
    if (!isInfrastructureFailure(outcome)) {
        return;
    }
    if (state(nowMs) == CircuitState::HalfOpen) {
        m_openedAtMs = nowMs;
        m_halfOpenProbeUsed = false;
        return;
    }
    ++m_failures;
    if (m_failures >= m_policy.circuitFailureThreshold) {
        m_openedAtMs = nowMs;
        m_halfOpenProbeUsed = false;
    }
}

AsyncRpcClient::AsyncRpcClient(
    const BusEndpoint &endpoint,
    RpcRetryPolicy policy,
    QObject *parent)
    : QObject(parent)
    , m_endpoint(endpoint)
    , m_policy(policy)
    , m_breaker(policy)
{
    Q_ASSERT(m_policy.isValid());
}

void AsyncRpcClient::call(
    const QString &method,
    const QVariantList &arguments,
    RpcOperationSemantics semantics,
    Completion completion,
    int timeoutMs,
    bool expectBooleanAcceptance)
{
    auto operation = std::make_shared<Operation>();
    operation->method = method;
    operation->arguments = arguments;
    operation->semantics = semantics;
    operation->completion = std::move(completion);
    operation->timeoutMs = std::max(1, timeoutMs);
    operation->expectBooleanAcceptance = expectBooleanAcceptance;
    operation->seed = qHash(method);
    dispatch(operation);
}

CircuitState AsyncRpcClient::circuitState() const
{
    return m_breaker.state(QDateTime::currentMSecsSinceEpoch());
}

void AsyncRpcClient::dispatch(const std::shared_ptr<Operation> &operation)
{
    const qint64 now = QDateTime::currentMSecsSinceEpoch();
    if (!m_breaker.allow(now)) {
        RpcResult result;
        result.outcome = RpcOutcome::CircuitOpen;
        result.errorMessage = QStringLiteral("RPC circuit is open");
        result.attempts = operation->attempts;
        complete(operation, result);
        return;
    }

    ++operation->attempts;
    QDBusConnection bus = QDBusConnection::sessionBus();
    if (!bus.isConnected()) {
        RpcResult result;
        result.outcome = RpcOutcome::Unavailable;
        result.errorMessage = QStringLiteral("the user D-Bus session is unavailable");
        result.attempts = operation->attempts;
        m_breaker.record(result.outcome, now);
        if (shouldRetry(result.outcome, operation->semantics, operation->attempts, m_policy)) {
            QTimer::singleShot(
                retryDelayMs(operation->attempts, operation->seed, m_policy),
                this,
                [this, operation]() { dispatch(operation); });
        } else {
            complete(operation, result);
        }
        return;
    }

    QDBusMessage message = QDBusMessage::createMethodCall(
        text(m_endpoint.service),
        text(m_endpoint.objectPath),
        text(m_endpoint.interfaceName),
        operation->method);
    message.setArguments(operation->arguments);
    auto *watcher = new QDBusPendingCallWatcher(
        bus.asyncCall(message, operation->timeoutMs),
        this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this, [this, operation, watcher]() {
        const QDBusMessage reply = watcher->reply();
        watcher->deleteLater();
        RpcResult result;
        result.reply = reply;
        result.outcome = classifyDBusReply(
            reply,
            operation->semantics,
            operation->expectBooleanAcceptance);
        result.errorName = reply.errorName();
        result.errorMessage = reply.errorMessage();
        result.attempts = operation->attempts;
        const qint64 now = QDateTime::currentMSecsSinceEpoch();
        m_breaker.record(result.outcome, now);
        if (shouldRetry(result.outcome, operation->semantics, operation->attempts, m_policy)) {
            QTimer::singleShot(
                retryDelayMs(operation->attempts, operation->seed, m_policy),
                this,
                [this, operation]() { dispatch(operation); });
        } else {
            complete(operation, result);
        }
    });
}

void AsyncRpcClient::complete(
    const std::shared_ptr<Operation> &operation,
    RpcResult result)
{
    if (operation->completion) {
        operation->completion(result);
    }
}

} // namespace cybou
