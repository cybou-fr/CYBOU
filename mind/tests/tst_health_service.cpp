// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "HealthPolicy.h"
#include "HealthService.h"
#include "cybou/protocol/CapabilityRegistry.h"
#include "cybou/protocol/Health.h"
#include "cybou/protocol/Homeostasis.h"

#include <QFile>
#include <QDir>
#include <QFileInfo>
#include <QCborMap>
#include <QCborValue>
#include <QSet>
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
    // The registry is the single declaration of what Mind can do and what each ability rests on.
    // These checks are what make it a declaration rather than a fourth copy: if it disagrees with
    // itself, or with the policy derived from it, that is caught here rather than as a capability
    // silently never appearing in a projection.
    void registryIsInternallyConsistent()
    {
        const QStringList components = CapabilityRegistry::componentIds();
        const QStringList capabilities = CapabilityRegistry::capabilityIds();

        QVERIFY(!components.isEmpty());
        QVERIFY(!capabilities.isEmpty());
        QCOMPARE(capabilities.size(), QSet<QString>(capabilities.begin(), capabilities.end()).size());

        for (const CapabilityDeclaration &capability : CapabilityRegistry::capabilities()) {
            QVERIFY2(
                !capability.components.isEmpty(),
                qPrintable(QStringLiteral("%1 rests on nothing").arg(capability.capabilityId)));
            QVERIFY2(
                !capability.unavailableImpact.isEmpty(),
                qPrintable(QStringLiteral("%1 does not say what its loss costs")
                               .arg(capability.capabilityId)));
            for (const QString &component : capability.components) {
                QVERIFY2(
                    components.contains(component),
                    qPrintable(QStringLiteral("%1 depends on unknown component %2")
                                   .arg(capability.capabilityId, component)));
            }
        }

        // A command requiring a capability that does not exist can never become available, and
        // nothing else would report that.
        for (const CommandDeclaration &command : CapabilityRegistry::commands()) {
            QVERIFY2(
                !command.requiredCapabilities.isEmpty(),
                qPrintable(QStringLiteral("%1 declares no requirement").arg(command.commandId)));
            for (const QString &capability : command.requiredCapabilities) {
                QVERIFY2(
                    capabilities.contains(capability),
                    qPrintable(QStringLiteral("command %1 requires unknown capability %2")
                                   .arg(command.commandId, capability)));
            }
        }
    }

    // The whole dependency graph, one component at a time, rather than the representative sample
    // the checkpoint flagged as insufficient. Expectations come from the registry, so adding a
    // capability extends this matrix without anyone editing it.
    void everyComponentLossProducesTheDeclaredDeficits()
    {
        const QDateTime now = QDateTime::currentDateTimeUtc();

        for (const QString &lost : CapabilityRegistry::componentIds()) {
            auto observations = healthyObservations(now);
            observations[lost] = {lost, ComponentHealth::Unavailable, now, {}, QStringLiteral("down")};

            const CapabilitySnapshot snapshot = HealthPolicy::evaluate(observations, now);
            QVERIFY2(snapshot.isValid(), qPrintable(QStringLiteral("losing %1").arg(lost)));

            QSet<QString> expected;
            for (const CapabilityDeclaration &capability : CapabilityRegistry::capabilities()) {
                if (capability.components.contains(lost)) {
                    expected.insert(capability.capabilityId);
                }
            }

            QSet<QString> observed;
            for (const CapabilityDeficit &deficit : snapshot.deficits) {
                observed.insert(deficit.capabilityId);
            }

            QVERIFY2(
                observed == expected,
                qPrintable(
                    QStringLiteral("losing %1 produced deficits {%2}, registry declares {%3}")
                        .arg(lost,
                             QStringList(observed.begin(), observed.end()).join(QStringLiteral(", ")),
                             QStringList(expected.begin(), expected.end()).join(QStringLiteral(", ")))));

            // A capability that does not depend on the lost component must remain untouched.
            for (const CapabilityDeclaration &capability : CapabilityRegistry::capabilities()) {
                if (capability.components.contains(lost)) {
                    continue;
                }
                QVERIFY2(
                    !observed.contains(capability.capabilityId),
                    qPrintable(QStringLiteral("losing %1 wrongly degraded %2")
                                   .arg(lost, capability.capabilityId)));
            }
        }
    }

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

    void mapsBoundedProbeTimeoutToTypedCause()
    {
        const QDateTime now = QDateTime::currentDateTimeUtc();
        auto observations = healthyObservations(now);
        observations[QStringLiteral("predictord")].state = ComponentHealth::Unavailable;
        observations[QStringLiteral("predictord")].detail =
            QStringLiteral("timed-out: org.freedesktop.DBus.Error.NoReply");
        const CapabilitySnapshot snapshot = HealthPolicy::evaluate(observations, now);
        const auto prediction = std::find_if(
            snapshot.deficits.cbegin(), snapshot.deficits.cend(),
            [](const CapabilityDeficit &deficit) {
                return deficit.capabilityId == QStringLiteral("prediction");
            });
        QVERIFY(prediction != snapshot.deficits.cend());
        QCOMPARE(prediction->cause, DeficitCause::TimedOut);
    }

    void reportsEveryUnavailableDependency()
    {
        const QDateTime now = QDateTime::currentDateTimeUtc();
        auto observations = healthyObservations(now);
        observations[QStringLiteral("predictord")].state = ComponentHealth::Unavailable;
        observations[QStringLiteral("workspaced")].state = ComponentHealth::Unavailable;
        const CapabilitySnapshot snapshot = HealthPolicy::evaluate(observations, now);
        QStringList consolidationDependencies;
        for (const CapabilityDeficit &deficit : snapshot.deficits) {
            if (deficit.capabilityId == QStringLiteral("consolidation"))
                consolidationDependencies.append(deficit.dependencyId);
        }
        QCOMPARE(
            QSet<QString>(consolidationDependencies.begin(), consolidationDependencies.end()),
            QSet<QString>({QStringLiteral("predictord"), QStringLiteral("workspaced")}));
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
            QVERIFY(!homeostasis.authorizes(QStringLiteral("event-backlog-v1")));
            const auto backlog = std::find_if(
                homeostasis.measurements.cbegin(), homeostasis.measurements.cend(),
                [](const HomeostaticMeasurement &measurement) {
                    return measurement.metricId == QStringLiteral("event.backlog.count");
                });
            QVERIFY(backlog != homeostasis.measurements.cend());
            QCOMPARE(backlog->status, MeasurementStatus::Unknown);
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

    void loadsPersistedSchemaV1AsV2()
    {
        QTemporaryDir root;
        const QString path = root.filePath(QStringLiteral("health/snapshot.cbor"));
        const QDateTime now = QDateTime::currentDateTimeUtc();
        auto observations = healthyObservations(now);
        observations[QStringLiteral("predictord")].state = ComponentHealth::Unavailable;
        const CapabilitySnapshot source = HealthPolicy::evaluate(observations, now);
        QCborMap legacy = QCborValue::fromCbor(encodeCapabilitySnapshot(source)).toMap();
        legacy.insert(QStringLiteral("schemaVersion"), 1);
        QVERIFY(QDir().mkpath(QFileInfo(path).absolutePath()));
        QFile file(path);
        QVERIFY(file.open(QIODevice::WriteOnly | QIODevice::Truncate));
        QCOMPARE(file.write(legacy.toCborValue().toCbor()) > 0, true);
        file.close();

        HealthService recovered(path);
        QVERIFY(recovered.isReady());
        QString error;
        const CapabilitySnapshot migrated = decodeCapabilitySnapshot(
            recovered.Snapshot(), &error);
        QVERIFY2(error.isEmpty(), qPrintable(error));
        QCOMPARE(migrated.schemaVersion, kHealthSchemaVersion);
    }
};

QTEST_MAIN(TestHealthService)
#include "tst_health_service.moc"
