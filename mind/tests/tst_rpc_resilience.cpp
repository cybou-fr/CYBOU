// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/fabric/RpcResilience.h"

#include <QDBusMessage>
#include <QEventLoop>
#include <QTest>
#include <QTimer>

using namespace cybou;

class TestRpcResilience : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    void retryRequiresExplicitSafeSemantics()
    {
        RpcRetryPolicy policy;
        QCOMPARE(policy.maximumAttempts, 3);
        QVERIFY(shouldRetry(
            RpcOutcome::TimedOut,
            RpcOperationSemantics::ReadOnly,
            1,
            policy));
        QVERIFY(shouldRetry(
            RpcOutcome::Unavailable,
            RpcOperationSemantics::IdempotentMutation,
            2,
            policy));
        QVERIFY(!shouldRetry(
            RpcOutcome::UnknownOutcome,
            RpcOperationSemantics::IdempotentMutation,
            1,
            policy));
        QVERIFY(!shouldRetry(
            RpcOutcome::TimedOut,
            RpcOperationSemantics::NonIdempotentMutation,
            1,
            policy));
        QVERIFY(!shouldRetry(
            RpcOutcome::TimedOut,
            RpcOperationSemantics::ReadOnly,
            3,
            policy));
    }

    void backoffIsDeterministicAndBounded()
    {
        RpcRetryPolicy policy;
        policy.baseDelayMs = 100;
        policy.maximumDelayMs = 500;
        policy.jitterPercent = 20;
        const int first = retryDelayMs(1, 42, policy);
        QCOMPARE(first, retryDelayMs(1, 42, policy));
        QVERIFY(first >= 80 && first <= 120);
        const int saturated = retryDelayMs(10, 42, policy);
        QVERIFY(saturated >= 400 && saturated <= 500);
    }

    void classifiesTimeoutWithoutInventingMutationFailure()
    {
        const QDBusMessage timeout = QDBusMessage::createError(
            QStringLiteral("org.freedesktop.DBus.Error.NoReply"),
            QStringLiteral("deadline exceeded"));
        QCOMPARE(
            classifyDBusReply(timeout, RpcOperationSemantics::ReadOnly),
            RpcOutcome::TimedOut);
        QCOMPARE(
            classifyDBusReply(timeout, RpcOperationSemantics::NonIdempotentMutation),
            RpcOutcome::UnknownOutcome);

        const QDBusMessage unavailable = QDBusMessage::createError(
            QStringLiteral("org.freedesktop.DBus.Error.ServiceUnknown"),
            QStringLiteral("owner absent"));
        QCOMPARE(
            classifyDBusReply(unavailable, RpcOperationSemantics::ReadOnly),
            RpcOutcome::Unavailable);

        QDBusMessage call = QDBusMessage::createMethodCall(
            QStringLiteral("org.example.Test"),
            QStringLiteral("/org/example/Test"),
            QStringLiteral("org.example.Test"),
            QStringLiteral("Mutate"));
        QDBusMessage rejected = call.createReply(QVariantList{false});
        QCOMPARE(
            classifyDBusReply(
                rejected,
                RpcOperationSemantics::IdempotentMutation,
                true),
            RpcOutcome::Rejected);
    }

    void circuitBreakerUsesOneHalfOpenProbe()
    {
        RpcRetryPolicy policy;
        policy.circuitFailureThreshold = 2;
        policy.circuitOpenMs = 1000;
        CircuitBreaker breaker(policy);

        QVERIFY(breaker.allow(0));
        breaker.record(RpcOutcome::TimedOut, 0);
        QCOMPARE(breaker.state(0), CircuitState::Closed);
        breaker.record(RpcOutcome::Unavailable, 10);
        QCOMPARE(breaker.state(10), CircuitState::Open);
        QVERIFY(!breaker.allow(500));
        QCOMPARE(breaker.state(1010), CircuitState::HalfOpen);
        QVERIFY(breaker.allow(1010));
        QVERIFY(!breaker.allow(1011));
        breaker.record(RpcOutcome::Succeeded, 1012);
        QCOMPARE(breaker.state(1012), CircuitState::Closed);
        QVERIFY(breaker.allow(1012));
    }

    void rejectedCallDoesNotOpenInfrastructureCircuit()
    {
        RpcRetryPolicy policy;
        policy.circuitFailureThreshold = 1;
        CircuitBreaker breaker(policy);
        breaker.record(RpcOutcome::Rejected, 0);
        QCOMPARE(breaker.state(0), CircuitState::Closed);
        QCOMPARE(breaker.consecutiveFailures(), 0);
    }

    void asyncClientRetriesUnavailableAndOpensCircuit()
    {
        const BusEndpoint missing{
            "org.cybou.Mind.Missing1",
            "/org/cybou/Mind/Missing1",
            "org.cybou.Mind.Missing1",
            "missing.service",
        };
        RpcRetryPolicy policy;
        policy.maximumAttempts = 2;
        policy.baseDelayMs = 1;
        policy.maximumDelayMs = 1;
        policy.jitterPercent = 0;
        policy.circuitFailureThreshold = 2;
        policy.circuitOpenMs = 10000;
        AsyncRpcClient client(missing, policy);

        RpcResult first;
        QEventLoop loop;
        QTimer guard;
        guard.setSingleShot(true);
        connect(&guard, &QTimer::timeout, &loop, &QEventLoop::quit);
        client.call(
            QStringLiteral("Ready"),
            {},
            RpcOperationSemantics::ReadOnly,
            [&first, &loop](const RpcResult &result) {
                first = result;
                loop.quit();
            },
            100);
        guard.start(2000);
        loop.exec();
        QVERIFY(guard.isActive());
        QCOMPARE(first.outcome, RpcOutcome::Unavailable);
        QCOMPARE(first.attempts, 2);
        QCOMPARE(client.circuitState(), CircuitState::Open);

        RpcResult second;
        client.call(
            QStringLiteral("Ready"),
            {},
            RpcOperationSemantics::ReadOnly,
            [&second](const RpcResult &result) { second = result; },
            100);
        QCOMPARE(second.outcome, RpcOutcome::CircuitOpen);
        QCOMPARE(second.attempts, 0);
    }
};

QTEST_MAIN(TestRpcResilience)
#include "tst_rpc_resilience.moc"
