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
        QVERIFY(!decoded.schedulingAuthorized);
        QCOMPARE(decoded.measurements.size(), 2);
        QCOMPARE(decoded.measurements.at(0).value, 12.0);
        QVERIFY(!decoded.measurements.at(1).hasValue);
        QCOMPARE(decoded.measurements.at(1).status, MeasurementStatus::Unsupported);
    }

    void rejectsSchedulingAndFreshnessViolations()
    {
        auto snapshot = validSnapshot();
        snapshot.schedulingAuthorized = true;
        QVERIFY(!snapshot.isValid());
        snapshot.schedulingAuthorized = false;
        snapshot.measurements[0].validUntil = snapshot.observedAt.addMSecs(-1);
        QVERIFY(!snapshot.isValid());
        snapshot.measurements[0].status = MeasurementStatus::Stale;
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
        root.insert(QStringLiteral("schemaVersion"), 2);
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
        root.insert(QStringLiteral("schedulingAuthorized"), QStringLiteral("false"));
        error.clear();
        decodeHomeostasisSnapshot(root.toCborValue().toCbor(), &error);
        QVERIFY(!error.isEmpty());
    }
};

QTEST_MAIN(TestHomeostasisProtocol)
#include "tst_homeostasis_protocol.moc"
