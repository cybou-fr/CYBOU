// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/presence/Presence.h"

#include <QTemporaryDir>
#include <QTest>

using namespace cybou;

class TestPresenceProxy : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    void noBackendFailsClosed()
    {
        QTemporaryDir state;
        QVERIFY(state.isValid());

        const QByteArray oldBus =
            qgetenv("DBUS_SESSION_BUS_ADDRESS");
        const QByteArray oldState =
            qgetenv("XDG_STATE_HOME");

        qputenv(
            "DBUS_SESSION_BUS_ADDRESS",
            QByteArrayLiteral("unix:path=/nonexistent/cybou-m4-test-bus"));
        qputenv(
            "XDG_STATE_HOME",
            state.path().toUtf8());

        Presence presence;

        QVERIFY(!presence.isAwake());
        QVERIFY(!presence.wake());
        QVERIFY(!presence.lastError().isEmpty());
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
