// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/fabric/FabricCodec.h"

#include <QDateTime>
#include <QTest>

using namespace cybou;

class TestFabricCodec : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    void mapRoundTrip()
    {
        QVariantMap source;
        source[QStringLiteral("name")] =
            QStringLiteral("identityd");
        source[QStringLiteral("healthy")] = true;
        source[QStringLiteral("count")] =
            static_cast<qulonglong>(42);
        source[QStringLiteral("when")] =
            QDateTime::currentDateTimeUtc();

        QString error;
        const QVariantMap decoded =
            FabricCodec::decodeMap(
                FabricCodec::encodeMap(source),
                &error);

        QVERIFY2(error.isEmpty(), qPrintable(error));
        QCOMPARE(
            decoded.value(QStringLiteral("name")).toString(),
            QStringLiteral("identityd"));
        QCOMPARE(
            decoded.value(QStringLiteral("healthy")).toBool(),
            true);
        QCOMPARE(
            decoded.value(QStringLiteral("count")).toULongLong(),
            42u);
    }

    void rejectsUnversionedPayload()
    {
        QString error;
        QVERIFY(
            FabricCodec::decodeMap(
                QByteArrayLiteral("not-cbor"),
                &error)
                .isEmpty());
        QVERIFY(!error.isEmpty());
    }
};

QTEST_MAIN(TestFabricCodec)
#include "tst_fabric_codec.moc"
