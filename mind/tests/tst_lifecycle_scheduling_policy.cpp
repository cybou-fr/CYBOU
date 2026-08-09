// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "LifecycleSchedulingPolicy.h"

#include <QTest>

using namespace cybou;

namespace {

CapabilitySnapshot healthyCapabilities(const QDateTime &now)
{
    CapabilitySnapshot snapshot;
    snapshot.snapshotId = QUuid::createUuid();
    snapshot.observedAt = now;
    snapshot.aggregateState = CapabilityState::Available;
    return snapshot;
}

HomeostasisSnapshot backlogSnapshot(
    const QDateTime &now,
    double value,
    MeasurementStatus status = MeasurementStatus::Current)
{
    HomeostasisSnapshot snapshot;
    snapshot.snapshotId = QUuid::createUuid();
    snapshot.observedAt = now;
    HomeostaticMeasurement backlog;
    backlog.metricId = QStringLiteral("event.backlog.count");
    backlog.sourceId = QStringLiteral("eventd");
    backlog.kind = MeasurementKind::Counter;
    backlog.status = status;
    backlog.observedAt = now;
    if (status == MeasurementStatus::Current) {
        backlog.hasValue = true;
        backlog.value = value;
        backlog.unit = QStringLiteral("{event}");
        backlog.validUntil = now.addSecs(30);
    } else {
        backlog.reason = QStringLiteral("Event1 has no consumer-offset contract");
    }
    snapshot.measurements.append(backlog);
    return snapshot;
}

} // namespace

class TestLifecycleSchedulingPolicy : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    void invalidEvidenceBlocks()
    {
        const SchedulingEvaluation result = LifecycleSchedulingPolicy::evaluate(
            LifecycleMode::Idle, false, {}, {});
        QCOMPARE(result.decision, SchedulingDecision::Block);
        QVERIFY(result.reason.contains(QStringLiteral("capability snapshot")));
    }

    void nonIdleLifecycleDefersWithoutChangingCapabilityHealth()
    {
        const QDateTime now = QDateTime::currentDateTimeUtc();
        const SchedulingEvaluation result = LifecycleSchedulingPolicy::evaluate(
            LifecycleMode::Awake, false, healthyCapabilities(now), backlogSnapshot(now, 50),
            false, now);
        QCOMPARE(result.decision, SchedulingDecision::Defer);
        QCOMPARE(result.reason, QStringLiteral("lifecycle is not idle"));
    }

    void staleCapabilityEvidenceBlocks()
    {
        const QDateTime now = QDateTime::currentDateTimeUtc();
        const SchedulingEvaluation result = LifecycleSchedulingPolicy::evaluate(
            LifecycleMode::Idle, false, healthyCapabilities(now.addSecs(-61)),
            backlogSnapshot(now, 50), false, now);
        QCOMPARE(result.decision, SchedulingDecision::Block);
        QVERIFY(result.reason.contains(QStringLiteral("60 second")));
    }

    void unsupportedTriggerDefersAndExplainsRemainingWorkers()
    {
        const QDateTime now = QDateTime::currentDateTimeUtc();
        const SchedulingEvaluation result = LifecycleSchedulingPolicy::evaluate(
            LifecycleMode::Idle, false, healthyCapabilities(now),
            backlogSnapshot(now, 0, MeasurementStatus::Unsupported), false, now);
        QCOMPARE(result.decision, SchedulingDecision::Defer);
        QVERIFY(result.reason.contains(QStringLiteral("no consumer-offset contract")));
        QCOMPARE(result.eligibleWorkers, QStringList({QStringLiteral("predictor"),
                                                      QStringLiteral("workspace")}));
    }

    void thresholdsApplyHysteresisBeforeAuthorization()
    {
        const QDateTime now = QDateTime::currentDateTimeUtc();
        const CapabilitySnapshot capabilities = healthyCapabilities(now);
        const SchedulingEvaluation entered = LifecycleSchedulingPolicy::evaluate(
            LifecycleMode::Idle, false, capabilities, backlogSnapshot(now, 32), false, now);
        QVERIFY(entered.pressureLatched);
        QCOMPARE(entered.decision, SchedulingDecision::Defer);
        QCOMPARE(entered.reason, QStringLiteral("homeostasis schema is observation-only"));

        const SchedulingEvaluation held = LifecycleSchedulingPolicy::evaluate(
            LifecycleMode::Idle, false, capabilities, backlogSnapshot(now, 9), true, now);
        QVERIFY(held.pressureLatched);

        const SchedulingEvaluation exited = LifecycleSchedulingPolicy::evaluate(
            LifecycleMode::Idle, false, capabilities, backlogSnapshot(now, 8), true, now);
        QVERIFY(!exited.pressureLatched);
        QCOMPARE(exited.reason, QStringLiteral("event backlog is below scheduling hysteresis"));
    }

    void requiredBiographyDeficitBlocksButOptionalWorkerDeficitDoesNot()
    {
        const QDateTime now = QDateTime::currentDateTimeUtc();
        CapabilitySnapshot capabilities = healthyCapabilities(now);
        capabilities.aggregateState = CapabilityState::Unavailable;
        capabilities.components.append({QStringLiteral("eventd"), ComponentHealth::Unavailable,
                                        now, {}, QStringLiteral("owner absent")});
        capabilities.deficits.append({QStringLiteral("accepted-biography"),
                                      QStringLiteral("eventd"), CapabilityState::Unavailable,
                                      DeficitCause::DependencyUnavailable, now, {},
                                      QStringLiteral("accepted history unavailable"),
                                      RecoveryPolicy::Observe, {}, QStringLiteral("owner absent")});
        QVERIFY(capabilities.isValid());
        const SchedulingEvaluation blocked = LifecycleSchedulingPolicy::evaluate(
            LifecycleMode::Idle, false, capabilities, backlogSnapshot(now, 50), false, now);
        QCOMPARE(blocked.decision, SchedulingDecision::Block);
        QVERIFY(blocked.reason.contains(QStringLiteral("accepted-biography")));

        CapabilitySnapshot optional = healthyCapabilities(now);
        optional.aggregateState = CapabilityState::Limited;
        optional.components.append({QStringLiteral("predictord"), ComponentHealth::Unavailable,
                                    now, {}, QStringLiteral("owner absent")});
        optional.deficits.append({QStringLiteral("prediction"),
                                  QStringLiteral("predictord"), CapabilityState::Unavailable,
                                  DeficitCause::DependencyUnavailable, now, {},
                                  QStringLiteral("new predictions unavailable"),
                                  RecoveryPolicy::Observe, {}, QStringLiteral("owner absent")});
        QVERIFY(optional.isValid());
        const SchedulingEvaluation partial = LifecycleSchedulingPolicy::evaluate(
            LifecycleMode::Idle, false, optional,
            backlogSnapshot(now, 0, MeasurementStatus::Unsupported), false, now);
        QCOMPARE(partial.decision, SchedulingDecision::Defer);
        QCOMPARE(partial.eligibleWorkers, QStringList({QStringLiteral("workspace")}));
        QVERIFY(partial.missingWorkers.contains(QStringLiteral("predictor")));
    }
};

QTEST_MAIN(TestLifecycleSchedulingPolicy)
#include "tst_lifecycle_scheduling_policy.moc"
