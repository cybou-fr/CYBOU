// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/presence/Presence.h"
#include "cybou/storage/Journal.h"

#include <QDir>
#include <QSignalSpy>
#include <QTemporaryDir>
#include <QTest>

using namespace cybou;

class TestPresence : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    void asleepItShowsNothing()
    {
        QTemporaryDir dir;
        Presence presence(dir.filePath(QStringLiteral("state")));

        QVERIFY(!presence.isAwake());
        QVERIFY(presence.narration().isEmpty());
        QVERIFY(presence.obligations().isEmpty());
        QCOMPARE(presence.contributions(), 0);
        QVERIFY(presence.promise(QStringLiteral("something")).isNull());
        QVERIFY(!presence.reflect());
    }

    void wakingIsRemembered()
    {
        QTemporaryDir dir;
        Presence presence(dir.filePath(QStringLiteral("state")));
        QSignalSpy spy(&presence, &Presence::changed);

        QVERIFY2(presence.wake(), qPrintable(presence.lastError()));
        QVERIFY(presence.isAwake());
        QCOMPARE(spy.count(), 1);
        QVERIFY(presence.contributions() > 0);
    }

    void aPromiseCreatesObservationThenIntention()
    {
        QTemporaryDir dir;
        Presence presence(dir.filePath(QStringLiteral("state")));
        QVERIFY(presence.wake());

        const int before = presence.contributions();
        const QUuid intentionId = presence.promise(QStringLiteral("verify sound after reboot"));
        QVERIFY(!intentionId.isNull());
        QCOMPARE(presence.contributions(), before + 2);

        const QVariantList activity = presence.activity(2);
        QCOMPARE(activity.size(), 2);
        QCOMPARE(activity.at(0).toMap().value(QStringLiteral("kind")).toString(),
                 kindToString(ContributionKind::Intention));
        QCOMPARE(activity.at(1).toMap().value(QStringLiteral("kind")).toString(),
                 kindToString(ContributionKind::Observation));
        QCOMPARE(activity.at(1).toMap().value(QStringLiteral("organ")).toString(),
                 QStringLiteral("presenced"));
    }

    void promiseSurvivesTheSession()
    {
        QTemporaryDir dir;
        const QString state = dir.filePath(QStringLiteral("state"));

        {
            Presence presence(state);
            QVERIFY(presence.wake());
            QVERIFY(!presence.promise(
                QStringLiteral("verify sound after reboot")).isNull());
        }

        Presence again(state);
        QVERIFY(again.wake());
        QCOMPARE(again.obligations(),
                 QStringList{QStringLiteral("verify sound after reboot")});
    }

    void reflectingCreatesObservationThenAssessment()
    {
        QTemporaryDir dir;
        Presence presence(dir.filePath(QStringLiteral("state")));
        QVERIFY(presence.wake());

        const int before = presence.contributions();
        QVERIFY(presence.reflect());
        QCOMPARE(presence.contributions(), before + 2);

        const QVariantList activity = presence.activity(2);
        QCOMPARE(activity.at(0).toMap().value(QStringLiteral("kind")).toString(),
                 kindToString(ContributionKind::SelfAssessment));
        QCOMPARE(activity.at(1).toMap().value(QStringLiteral("kind")).toString(),
                 kindToString(ContributionKind::Observation));
    }

    void visibleOperationsProduceNoSelfReferences()
    {
        QTemporaryDir dir;
        const QString state = dir.filePath(QStringLiteral("state"));
        Presence presence(state);
        QVERIFY(presence.wake());
        QVERIFY(!presence.promise(QStringLiteral("task")).isNull());
        QVERIFY(presence.reflect());
        QVERIFY(presence.observe(QStringLiteral("build"), 10.0));
        QVERIFY(!presence.predict(QStringLiteral("build")).isEmpty());

        Journal journal(QDir(state).filePath(QStringLiteral("journal.db")),
                        QStringLiteral("inspection"));
        for (const CognitiveEnvelope &envelope : journal.recent(0)) {
            QVERIFY(envelope.causationId != envelope.messageId);
            QVERIFY(!envelope.evidence.contains(envelope.messageId));
        }
    }

    void fulfillingAndAbandoningObligationsUpdatesStats()
    {
        QTemporaryDir dir;
        Presence presence(dir.filePath(QStringLiteral("state")));
        QVERIFY(presence.wake());

        presence.promise(QStringLiteral("task 1"));
        presence.promise(QStringLiteral("task 2"));
        presence.promise(QStringLiteral("task 3"));
        QCOMPARE(presence.obligations().size(), 3);

        QVERIFY(presence.fulfillIndex(0));
        QVERIFY(presence.abandonIndex(1));
        QCOMPARE(presence.obligations().size(), 1);

        const QVariantMap stats = presence.stats();
        QCOMPARE(stats.value(QStringLiteral("openIntentions")).toInt(), 1);
        QVERIFY(stats.value(QStringLiteral("journalIntact")).toBool());
    }
};

QTEST_MAIN(TestPresence)
#include "tst_presence.moc"
