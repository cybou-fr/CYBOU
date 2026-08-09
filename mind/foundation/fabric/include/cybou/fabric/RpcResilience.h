// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/fabric/OrganBus.h"

#include <QDBusMessage>
#include <QObject>
#include <QVariant>

#include <functional>
#include <memory>

namespace cybou {

enum class RpcOperationSemantics : quint8 {
    ReadOnly = 1,
    IdempotentMutation,
    NonIdempotentMutation,
};

enum class RpcOutcome : quint8 {
    Succeeded = 1,
    Unavailable,
    TimedOut,
    Rejected,
    UnknownOutcome,
    CircuitOpen,
};

enum class CircuitState : quint8 {
    Closed = 1,
    Open,
    HalfOpen,
};

struct RpcRetryPolicy {
    int maximumAttempts{3};
    int baseDelayMs{100};
    int maximumDelayMs{2000};
    int jitterPercent{20};
    int circuitFailureThreshold{3};
    int circuitOpenMs{5000};

    bool isValid() const;
};

struct RpcResult {
    RpcOutcome outcome{RpcOutcome::Unavailable};
    QDBusMessage reply;
    QString errorName;
    QString errorMessage;
    int attempts{0};

    bool succeeded() const { return outcome == RpcOutcome::Succeeded; }
};

QString rpcOutcomeToString(RpcOutcome outcome);
bool isRetrySafe(RpcOperationSemantics semantics) noexcept;
bool shouldRetry(
    RpcOutcome outcome,
    RpcOperationSemantics semantics,
    int completedAttempts,
    const RpcRetryPolicy &policy) noexcept;
int retryDelayMs(
    int completedAttempts,
    quint32 deterministicSeed,
    const RpcRetryPolicy &policy) noexcept;
RpcOutcome classifyDBusReply(
    const QDBusMessage &reply,
    RpcOperationSemantics semantics,
    bool expectBooleanAcceptance = false);

class CircuitBreaker
{
public:
    explicit CircuitBreaker(const RpcRetryPolicy &policy = {});

    CircuitState state(qint64 nowMs) const noexcept;
    bool allow(qint64 nowMs) noexcept;
    void record(RpcOutcome outcome, qint64 nowMs) noexcept;
    int consecutiveFailures() const { return m_failures; }

private:
    RpcRetryPolicy m_policy;
    int m_failures{0};
    qint64 m_openedAtMs{-1};
    bool m_halfOpenProbeUsed{false};
};

class AsyncRpcClient : public QObject
{
public:
    using Completion = std::function<void(const RpcResult &)>;

    explicit AsyncRpcClient(
        const BusEndpoint &endpoint,
        RpcRetryPolicy policy = {},
        QObject *parent = nullptr);

    void call(
        const QString &method,
        const QVariantList &arguments,
        RpcOperationSemantics semantics,
        Completion completion,
        int timeoutMs = 5000,
        bool expectBooleanAcceptance = false);

    CircuitState circuitState() const;

private:
    struct Operation;
    void dispatch(const std::shared_ptr<Operation> &operation);
    void complete(const std::shared_ptr<Operation> &operation, RpcResult result);

    BusEndpoint m_endpoint;
    RpcRetryPolicy m_policy;
    CircuitBreaker m_breaker;
};

} // namespace cybou
