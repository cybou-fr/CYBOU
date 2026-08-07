// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/presence/Presence.h"

#include <QSignalSpy>
#include <QTemporaryDir>
#include <QTest>

using namespace cybou;

class TestPresenceProxy : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    void noBackendFailsClosedAndNotifiesUi()
    {
        QTemporaryDir state;
        QVERIFY(state.isValid());

        const QByteArray oldBus =
            qgetenv("DBUS_SESSION_BUS_ADDRESS");
        const QByteArray oldState =
            qgetenv("XDG_STATE_HOME");

        qputenv(
            "DBUS_SESSION_BUS_ADDRESS",
            QByteArrayLiteral("unix:path=/nonexistent/cybou-ui-test-bus"));
        qputenv(
            "XDG_STATE_HOME",
            state.path().toUtf8());

        Presence presence;
        QSignalSpy changed(
            &presence,
            &Presence::changed);

        QVERIFY(!presence.isAwake());
        QVERIFY(!presence.wake());
        QCOMPARE(changed.count(), 1);

        QVERIFY(!presence.lastError().isEmpty());
        QCOMPARE(
            presence.property("lastError").toString(),
            presence.lastError());

        QVERIFY(!presence.isAwake());
        QVERIFY(presence.obligations().isEmpty());
        QCOMPARE(presence.contributions(), 0);

        if (oldBus.isNull()) {
            qunsetenv("DBUS_SESSION_BUS_ADDRESS");
        } else {
            qputenv("DBUS_SESSION_BUS_ADDRESS", oldBus);
        }

        if (oldState.isNull()) {
            qunsetenv("XDG_STATE_HOME");
        } else {
            qputenv("XDG_STATE_HOME", oldState);
        }
    }
};

QTEST_MAIN(TestPresenceProxy)
#include "tst_presence.moc"
