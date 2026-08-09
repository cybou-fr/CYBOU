// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/protocol/Homeostasis.h"

#include <QCborArray>
#include <QCborMap>
#include <QCborValue>
#include <QTest>

#include <limits>

using namespace cybou;

namespace {

HomeostasisSnapshot validSnapshot()
{
    const QDateTime now = QDateTime::currentDateTimeUtc();
    HomeostasisSnapshot snapshot;
    snapshot.snapshotId = QUuid::createUuid();
    snapshot.observedAt = now;
    snapshot.measurements = {
        {QStringLiteral("event.accepted.count"), QStringLiteral("eventd"),
         MeasurementKind::Counter, MeasurementStatus::Current, 12.0, true,
         QStringLiteral("{event}"), now, now.addSecs(30), {}},
        {QStringLiteral("event.backlog.count"), QStringLiteral("eventd"),
         MeasurementKind::Counter, MeasurementStatus::Unsupported, 0.0, false,
         {}, now, {}, QStringLiteral("consumer offsets are not exposed")},
    };
    return snapshot;
}

} // namespace

class TestHomeostasisProtocol : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    void roundTripsTypedMeasurements()
    {
        const HomeostasisSnapshot source = validSnapshot();
        QVERIFY(source.isValid());
        QString error;
        const HomeostasisSnapshot decoded = decodeHomeostasisSnapshot(
            encodeHomeostasisSnapshot(source), &error);
        QVERIFY2(error.isEmpty(), qPrintable(error));
        QCOMPARE(decoded.snapshotId, source.snapshotId);
        QVERIFY(decoded.authorizedPolicyIds.isEmpty());
        QCOMPARE(decoded.measurements.size(), 2);
        QCOMPARE(decoded.measurements.at(0).value, 12.0);
        QVERIFY(!decoded.measurements.at(1).hasValue);
        QCOMPARE(decoded.measurements.at(1).status, MeasurementStatus::Unsupported);
    }

    void validatesPolicyScopedAuthorizationAndFreshness()
    {
        auto snapshot = validSnapshot();
        snapshot.authorizedPolicyIds = {QStringLiteral("event-backlog-v1")};
        QVERIFY(snapshot.isValid());
        QVERIFY(snapshot.authorizes(QStringLiteral("event-backlog-v1")));
        QVERIFY(!snapshot.authorizes(QStringLiteral("another-policy")));
        snapshot.authorizedPolicyIds.append(QStringLiteral("event-backlog-v1"));
        QVERIFY(!snapshot.isValid());
        snapshot.authorizedPolicyIds = {QStringLiteral("Invalid Policy")};
        QVERIFY(!snapshot.isValid());
        snapshot.authorizedPolicyIds.clear();
        snapshot.measurements[0].validUntil = snapshot.observedAt.addMSecs(-1);
        QVERIFY(!snapshot.isValid());
        snapshot.measurements[0].status = MeasurementStatus::Stale;
        snapshot.measurements[0].observedAt = snapshot.observedAt.addSecs(-1);
        QVERIFY(snapshot.isValid());
    }

    void rejectsDuplicatesAndNonFiniteValues()
    {
        auto snapshot = validSnapshot();
        snapshot.measurements.append(snapshot.measurements.first());
        QVERIFY(!snapshot.isValid());
        snapshot.measurements.removeLast();
        snapshot.measurements[0].value = std::numeric_limits<double>::quiet_NaN();
        QVERIFY(!snapshot.isValid());
    }

    void decoderRejectsUnknownSchemaEnumsAndWrongTypes()
    {
        const QByteArray encoded = encodeHomeostasisSnapshot(validSnapshot());
        QCborMap root = QCborValue::fromCbor(encoded).toMap();
        root.insert(QStringLiteral("schemaVersion"), 3);
        QString error;
        decodeHomeostasisSnapshot(root.toCborValue().toCbor(), &error);
        QVERIFY(!error.isEmpty());

        root = QCborValue::fromCbor(encoded).toMap();
        QCborArray measurements = root.value(QStringLiteral("measurements")).toArray();
        QCborMap item = measurements.first().toMap();
        item.insert(QStringLiteral("status"), 99);
        measurements[0] = item;
        root.insert(QStringLiteral("measurements"), measurements);
        error.clear();
        decodeHomeostasisSnapshot(root.toCborValue().toCbor(), &error);
        QVERIFY(!error.isEmpty());

        root = QCborValue::fromCbor(encoded).toMap();
        root.insert(QStringLiteral("authorizedPolicyIds"), QStringLiteral("event-backlog-v1"));
        error.clear();
        decodeHomeostasisSnapshot(root.toCborValue().toCbor(), &error);
        QVERIFY(!error.isEmpty());
    }

    void migratesObservationOnlySchemaV1()
    {
        QCborMap legacy = QCborValue::fromCbor(
            encodeHomeostasisSnapshot(validSnapshot())).toMap();
        legacy.insert(QStringLiteral("schemaVersion"), 1);
        legacy.remove(QStringLiteral("authorizedPolicyIds"));
        legacy.insert(QStringLiteral("schedulingAuthorized"), false);
        QString error;
        const HomeostasisSnapshot migrated = decodeHomeostasisSnapshot(
            legacy.toCborValue().toCbor(), &error);
        QVERIFY2(error.isEmpty(), qPrintable(error));
        QCOMPARE(migrated.schemaVersion, kHomeostasisSchemaVersion);
        QVERIFY(migrated.authorizedPolicyIds.isEmpty());

        legacy.insert(QStringLiteral("schedulingAuthorized"), true);
        decodeHomeostasisSnapshot(legacy.toCborValue().toCbor(), &error);
        QVERIFY(!error.isEmpty());
    }
};

QTEST_MAIN(TestHomeostasisProtocol)
#include "tst_homeostasis_protocol.moc"
