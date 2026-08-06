// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/presence/Presence.h"

#include <QTest>

namespace cybou {

class TestPresenceExtended : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    void initTestCase()
    {
        // Create a temporary directory for testing
        m_tempDir = QDir::tempPath() + "/cybou-test-" + QUuid::createUuid().toString();
        QVERIFY(QDir().mkpath(m_tempDir));
    }

    void cleanupTestCase()
    {
        // Clean up temporary directory
        QDir(m_tempDir).removeRecursively();
    }

    void testIdentityState()
    {
        Presence presence(m_tempDir);
        QVERIFY(presence.wake());

        QVariantMap state = presence.identityState();
        QVERIFY(!state.isEmpty());
        QVERIFY(state.contains("uuid"));
        QVERIFY(state.contains("origin"));
        QVERIFY(state.contains("sessionCount"));
        QVERIFY(state.contains("archVersion"));
        QVERIFY(state.contains("wasBorn"));
    }

    void testCalibrations()
    {
        Presence presence(m_tempDir);
        QVERIFY(presence.wake());

        // Initially empty
        QVariantList calibrations = presence.calibrations();
        QVERIFY(calibrations.isEmpty());

        // Add some observations and predictions
        QVERIFY(presence.observe("test-subject", 10.0));
        QVERIFY(presence.observe("test-subject", 12.0));
        QVERIFY(presence.observe("test-subject", 8.0));

        // Still empty until we have outcomes
        calibrations = presence.calibrations();
        QVERIFY(calibrations.isEmpty());
    }

    void testPredict()
    {
        Presence presence(m_tempDir);
        QVERIFY(presence.wake());

        // Add observations
        QVERIFY(presence.observe("test-subject", 10.0));
        QVERIFY(presence.observe("test-subject", 12.0));

        // Predict
        QVariantMap prediction = presence.predict("test-subject");
        QVERIFY(!prediction.isEmpty());
        QVERIFY(prediction.contains("subject"));
        QVERIFY(prediction.contains("value"));
        QVERIFY(prediction.contains("confidence"));
        QVERIFY(prediction["subject"].toString() == "test-subject");
    }

    void testCoalitions()
    {
        Presence presence(m_tempDir);
        QVERIFY(presence.wake());

        // Initially empty
        QVariantList coalitions = presence.coalitions();
        QVERIFY(coalitions.isEmpty());
    }

    void testMoment()
    {
        Presence presence(m_tempDir);
        QVERIFY(presence.wake());

        // Initially empty focus
        QVariantMap moment = presence.moment();
        QVERIFY(moment.isEmpty() || !moment["focus"].toString().isEmpty());
    }

private:
    QString m_tempDir;
};

} // namespace cybou

QTEST_MAIN(cybou::TestPresenceExtended)
#include "tst_presence_extended.moc"
