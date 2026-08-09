// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/protocol/Health.h"

#include <QCborArray>
#include <QCborMap>
#include <QCborValue>
#include <QTest>

using namespace cybou;

namespace {

CapabilitySnapshot degradedSnapshot()
{
    const QDateTime now = QDateTime::fromString(
        QStringLiteral("2026-08-09T12:00:00.000Z"),
        Qt::ISODateWithMs);
    CapabilitySnapshot snapshot;
    snapshot.snapshotId = QUuid(QStringLiteral("{57cc23ac-5a7b-44ca-b035-c26016f55f5a}"));
    snapshot.observedAt = now;
    snapshot.aggregateState = CapabilityState::Limited;
    snapshot.components = {
        {QStringLiteral("identityd"), ComponentHealth::Healthy, now, now, QString()},
        {QStringLiteral("predictord"), ComponentHealth::Unavailable, now, now.addSecs(-15),
         QStringLiteral("D-Bus owner absent")},
    };
    CapabilityDeficit deficit;
    deficit.capabilityId = QStringLiteral("prediction");
    deficit.dependencyId = QStringLiteral("predictord");
    deficit.state = CapabilityState::Unavailable;
    deficit.cause = DeficitCause::DependencyUnavailable;
    deficit.detectedAt = now;
    deficit.lastVerifiedAt = now.addSecs(-15);
    deficit.impact = QStringLiteral("new predictions are unavailable");
    deficit.recoveryPolicy = RecoveryPolicy::Observe;
    deficit.errorReference = QStringLiteral("dbus-name-absent");
    snapshot.deficits.append(deficit);
    return snapshot;
}

} // namespace

class TestHealthProtocol : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    void roundTripsVersionedSnapshot()
    {
        const CapabilitySnapshot source = degradedSnapshot();
        QVERIFY(source.isValid());

        QString error;
        const CapabilitySnapshot decoded = decodeCapabilitySnapshot(
            encodeCapabilitySnapshot(source),
            &error);
        QVERIFY2(error.isEmpty(), qPrintable(error));
        QCOMPARE(decoded.schemaVersion, kHealthSchemaVersion);
        QCOMPARE(decoded.snapshotId, source.snapshotId);
        QCOMPARE(decoded.aggregateState, CapabilityState::Limited);
        QCOMPARE(decoded.components.size(), 2);
        QCOMPARE(decoded.deficits.size(), 1);
        QCOMPARE(decoded.deficits.first().cause, DeficitCause::DependencyUnavailable);
        QCOMPARE(decoded.deficits.first().recoveryPolicy, RecoveryPolicy::Observe);
        QCOMPARE(decoded.deficits.first().errorReference, QStringLiteral("dbus-name-absent"));
    }

    void rejectsUnknownSchemaAndEnums()
    {
        const QByteArray encoded = encodeCapabilitySnapshot(degradedSnapshot());
        QCborMap root = QCborValue::fromCbor(encoded).toMap();
        root.insert(QStringLiteral("schemaVersion"), 99);
        QString error;
        QVERIFY(decodeCapabilitySnapshot(root.toCborValue().toCbor(), &error).snapshotId.isNull());
        QVERIFY(!error.isEmpty());

        root = QCborValue::fromCbor(encoded).toMap();
        root.insert(QStringLiteral("aggregateState"), 99);
        QVERIFY(decodeCapabilitySnapshot(root.toCborValue().toCbor(), &error).snapshotId.isNull());
        QVERIFY(!error.isEmpty());

        root = QCborValue::fromCbor(encoded).toMap();
        root.insert(QStringLiteral("schemaVersion"), 65537);
        QVERIFY(decodeCapabilitySnapshot(root.toCborValue().toCbor(), &error).snapshotId.isNull());
        QVERIFY(!error.isEmpty());

        root = QCborValue::fromCbor(encoded).toMap();
        QCborArray components = root.value(QStringLiteral("components")).toArray();
        QCborMap component = components.first().toMap();
        component.insert(QStringLiteral("state"), 257);
        components[0] = component;
        root.insert(QStringLiteral("components"), components);
        QVERIFY(decodeCapabilitySnapshot(root.toCborValue().toCbor(), &error).snapshotId.isNull());
        QVERIFY(!error.isEmpty());
    }

    void rejectsMalformedAndInconsistentState()
    {
        QString error;
        QVERIFY(decodeCapabilitySnapshot(QByteArrayLiteral("not-cbor"), &error).snapshotId.isNull());
        QVERIFY(!error.isEmpty());

        CapabilitySnapshot snapshot = degradedSnapshot();
        snapshot.components.append(snapshot.components.first());
        QVERIFY(!snapshot.isValid());

        snapshot = degradedSnapshot();
        snapshot.deficits.first().dependencyId = QStringLiteral("missing-owner");
        QVERIFY(!snapshot.isValid());

        snapshot = degradedSnapshot();
        snapshot.aggregateState = CapabilityState::Available;
        QVERIFY(!snapshot.isValid());
    }

    void enforcesComponentTransitions()
    {
        QVERIFY(canTransition(ComponentHealth::Healthy, ComponentHealth::Degraded));
        QVERIFY(canTransition(ComponentHealth::Unavailable, ComponentHealth::Recovering));
        QVERIFY(canTransition(ComponentHealth::Recovering, ComponentHealth::Healthy));
        QVERIFY(!canTransition(ComponentHealth::Unavailable, ComponentHealth::Healthy));
        QVERIFY(!canTransition(ComponentHealth::Healthy, ComponentHealth::Healthy));
        QVERIFY(!canTransition(
            static_cast<ComponentHealth>(99),
            ComponentHealth::Healthy));
    }
};

QTEST_MAIN(TestHealthProtocol)
#include "tst_health_protocol.moc"
