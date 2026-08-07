// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/presence/Presence.h"

#include <QTemporaryDir>
#include <QTest>

namespace cybou {

namespace {

QString statePath(QTemporaryDir &dir)
{
    return dir.filePath(QStringLiteral("state"));
}

} // namespace

class TestPresenceExtended : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    void identityStateMatchesPresentationContract()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());

        Presence presence(statePath(dir));
        QVERIFY2(presence.wake(), qPrintable(presence.lastError()));

        const QVariantMap state = presence.identityState();
        QVERIFY(!state.isEmpty());
        QVERIFY(state.contains(QStringLiteral("uuid")));
        QVERIFY(state.contains(QStringLiteral("origin")));
        QVERIFY(state.contains(QStringLiteral("sessionCount")));
        QVERIFY(state.contains(QStringLiteral("architectureVersion")));
        QVERIFY(state.contains(QStringLiteral("wasBorn")));
        QVERIFY(!state.value(QStringLiteral("uuid")).toString().isEmpty());
    }

    void calibrationsRemainEmptyUntilAForecastIsSettled()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());

        Presence presence(statePath(dir));
        QVERIFY2(presence.wake(), qPrintable(presence.lastError()));

        QVERIFY(presence.calibrations().isEmpty());
        QVERIFY(presence.observe(QStringLiteral("test-subject"), 10.0));
        QVERIFY(presence.observe(QStringLiteral("test-subject"), 12.0));
        QVERIFY(presence.observe(QStringLiteral("test-subject"), 8.0));
        QVERIFY(presence.calibrations().isEmpty());
    }

    void predictionMatchesPresentationContract()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());

        Presence presence(statePath(dir));
        QVERIFY2(presence.wake(), qPrintable(presence.lastError()));

        QVERIFY(presence.observe(QStringLiteral("test-subject"), 10.0));
        QVERIFY(presence.observe(QStringLiteral("test-subject"), 12.0));

        const QVariantMap prediction = presence.predict(QStringLiteral("test-subject"));
        QVERIFY(!prediction.isEmpty());
        QCOMPARE(prediction.value(QStringLiteral("subject")).toString(),
                 QStringLiteral("test-subject"));
        QVERIFY(prediction.contains(QStringLiteral("estimate")));
        QVERIFY(prediction.contains(QStringLiteral("margin")));
        QVERIFY(prediction.contains(QStringLiteral("confidence")));
        QVERIFY(prediction.contains(QStringLiteral("samples")));
        QCOMPARE(prediction.value(QStringLiteral("samples")).toInt(), 2);
    }

    void wakeProducesATraceableWorkspaceCoalition()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());

        Presence presence(statePath(dir));
        QVERIFY2(presence.wake(), qPrintable(presence.lastError()));

        const QVariantList coalitions = presence.coalitions();
        QVERIFY(!coalitions.isEmpty());

        const QVariantMap first = coalitions.first().toMap();
        QVERIFY(first.contains(QStringLiteral("correlationId")));
        QVERIFY(first.contains(QStringLiteral("salience")));
        QVERIFY(first.contains(QStringLiteral("organs")));
        QVERIFY(first.contains(QStringLiteral("threads")));
        QVERIFY(!first.value(QStringLiteral("correlationId")).toString().isEmpty());
    }

    void momentMatchesFocusedCoalition()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());

        Presence presence(statePath(dir));
        QVERIFY2(presence.wake(), qPrintable(presence.lastError()));

        const QVariantMap moment = presence.moment();
        QVERIFY(!moment.isEmpty());
        QVERIFY(!moment.value(QStringLiteral("focus")).toString().isEmpty());
        QVERIFY(moment.value(QStringLiteral("salience")).toDouble() >= 0.0);
        QVERIFY(!moment.value(QStringLiteral("organs")).toStringList().isEmpty());
    }
};

} // namespace cybou

QTEST_MAIN(cybou::TestPresenceExtended)
#include "tst_presence_extended.moc"
