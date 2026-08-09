// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "HealthPolicy.h"
#include "HealthService.h"
#include "cybou/protocol/Health.h"
#include "cybou/protocol/Homeostasis.h"

#include <QFile>
#include <QTemporaryDir>
#include <QTest>

#include <algorithm>

using namespace cybou;

namespace {

QMap<QString, ComponentHealthRecord> healthyObservations(const QDateTime &now)
{
    QMap<QString, ComponentHealthRecord> result;
    for (const QString &componentId : HealthPolicy::componentIds()) {
        result.insert(
            componentId,
            {componentId, ComponentHealth::Healthy, now, now, QStringLiteral("healthy")});
    }
    return result;
}

} // namespace

class TestHealthService : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    void mapsOnlyDependentOptionalCapabilities()
    {
        const QDateTime now = QDateTime::currentDateTimeUtc();
        auto observations = healthyObservations(now);
        observations[QStringLiteral("predictord")].state = ComponentHealth::Unavailable;
        observations[QStringLiteral("predictord")].lastVerifiedAt = now.addSecs(-5);
        observations[QStringLiteral("predictord")].detail = QStringLiteral("owner absent");

        const CapabilitySnapshot snapshot = HealthPolicy::evaluate(observations, now);
        QVERIFY(snapshot.isValid());
        QCOMPARE(snapshot.aggregateState, CapabilityState::Limited);
        QCOMPARE(snapshot.deficits.size(), 2);
        QCOMPARE(snapshot.deficits.at(0).capabilityId, QStringLiteral("prediction"));
        QCOMPARE(snapshot.deficits.at(1).capabilityId, QStringLiteral("consolidation"));
        for (const CapabilityDeficit &deficit : snapshot.deficits) {
            QCOMPARE(deficit.dependencyId, QStringLiteral("predictord"));
            QCOMPARE(deficit.cause, DeficitCause::DependencyUnavailable);
        }
    }

    void requiredCapabilityFailsClosed()
    {
        const QDateTime now = QDateTime::currentDateTimeUtc();
        auto observations = healthyObservations(now);
        observations.remove(QStringLiteral("identityd"));

        const CapabilitySnapshot snapshot = HealthPolicy::evaluate(observations, now);
        QVERIFY(snapshot.isValid());
        QCOMPARE(snapshot.aggregateState, CapabilityState::Unavailable);
        const auto identity = std::find_if(
            snapshot.deficits.cbegin(),
            snapshot.deficits.cend(),
            [](const CapabilityDeficit &deficit) {
                return deficit.capabilityId == QStringLiteral("identity-continuity");
            });
        QVERIFY(identity != snapshot.deficits.cend());
        QCOMPARE(identity->dependencyId, QStringLiteral("identityd"));
    }

    void persistsAndReloadsExactSnapshot()
    {
        QTemporaryDir root;
        QVERIFY(root.isValid());
        const QString path = root.filePath(QStringLiteral("health/snapshot.cbor"));
        QUuid snapshotId;
        {
            HealthService service(path);
            QVERIFY(service.isReady());
            QVERIFY(!service.HasSnapshot());
            QVERIFY(service.Refresh());
            QVERIFY(service.HasSnapshot());
            QVERIFY(service.HasMeasurements());
            QString error;
            const CapabilitySnapshot snapshot = decodeCapabilitySnapshot(service.Snapshot(), &error);
            QVERIFY2(error.isEmpty(), qPrintable(error));
            snapshotId = snapshot.snapshotId;
            QVERIFY(!snapshotId.isNull());
            const HomeostasisSnapshot homeostasis = decodeHomeostasisSnapshot(
                service.Measurements(), &error);
            QVERIFY2(error.isEmpty(), qPrintable(error));
            QVERIFY(homeostasis.isValid());
            QVERIFY(!homeostasis.schedulingAuthorized);
            const auto backlog = std::find_if(
                homeostasis.measurements.cbegin(), homeostasis.measurements.cend(),
                [](const HomeostaticMeasurement &measurement) {
                    return measurement.metricId == QStringLiteral("event.backlog.count");
                });
            QVERIFY(backlog != homeostasis.measurements.cend());
            QCOMPARE(backlog->status, MeasurementStatus::Unsupported);
            QVERIFY(!backlog->hasValue);
        }
        {
            HealthService recovered(path);
            QVERIFY(recovered.isReady());
            QVERIFY(recovered.HasSnapshot());
            QVERIFY(!recovered.HasMeasurements());
            QString error;
            const CapabilitySnapshot snapshot = decodeCapabilitySnapshot(recovered.Snapshot(), &error);
            QVERIFY2(error.isEmpty(), qPrintable(error));
            QCOMPARE(snapshot.snapshotId, snapshotId);
        }
    }

    void corruptStateFailsClosed()
    {
        QTemporaryDir root;
        const QString path = root.filePath(QStringLiteral("snapshot.cbor"));
        QFile file(path);
        QVERIFY(file.open(QIODevice::WriteOnly));
        file.write("corrupt");
        file.close();
        HealthService service(path);
        QVERIFY(!service.isReady());
        QVERIFY(!service.startupError().isEmpty());
    }
};

QTEST_MAIN(TestHealthService)
#include "tst_health_service.moc"
