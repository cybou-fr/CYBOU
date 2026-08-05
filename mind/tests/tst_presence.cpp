// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT
//
// The boundary where the mind meets the screen. What matters here is not that it renders, but
// that it cannot render anything untrue: asleep it shows nothing, and idle it says nothing
// rather than filling the space.

#include "cybou/presence/Presence.h"

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
        Presence p(dir.filePath(QStringLiteral("state")));

        QVERIFY(!p.isAwake());
        QVERIFY(p.narration().isEmpty());
        QVERIFY(p.obligations().isEmpty());
        QVERIFY(p.attention().isEmpty());
        QCOMPARE(p.contributions(), 0);
        QVERIFY(p.recent().isEmpty());

        // And it refuses to act, rather than half-working.
        QVERIFY(p.promise(QStringLiteral("something")).isNull());
        QVERIFY(!p.reflect());
    }

    void wakingIsRemembered()
    {
        QTemporaryDir dir;
        Presence p(dir.filePath(QStringLiteral("state")));
        QSignalSpy spy(&p, &Presence::changed);

        QVERIFY2(p.wake(), qPrintable(p.lastError()));
        QVERIFY(p.isAwake());
        QCOMPARE(spy.count(), 1);

        // Being born is an event: identityd wrote it down.
        QVERIFY(p.contributions() > 0);
        QVERIFY(!p.narration().isEmpty());
    }

    void attentionNamesSomethingThatActuallyHappened()
    {
        QTemporaryDir dir;
        Presence p(dir.filePath(QStringLiteral("state")));
        QVERIFY(p.wake());

        // Waking restores the moment from the journal, so there *is* something on its mind:
        // being born. That is a real event, and attending to it is honest. What must never
        // happen is attention naming something with no record behind it.
        const QString attention = p.attention();
        if (attention.isEmpty()) {
            return; // saying nothing is always allowed
        }

        bool traceable = false;
        for (const Moment &m : p.recent(50)) {
            if (attention.contains(m.organ) || attention.contains(m.kind)) {
                traceable = true;
                break;
            }
        }
        QVERIFY2(traceable, qPrintable(QStringLiteral("attention '%1' matches no journal entry")
                                           .arg(attention)));
    }

    void withoutAJournalItStaysAsleep()
    {
        // A path that cannot be created: waking must fail rather than render an empty shell.
        Presence p(QStringLiteral("/proc/cybou-cannot-exist/state"));
        QVERIFY(!p.wake());
        QVERIFY(!p.isAwake());
        QVERIFY(!p.lastError().isEmpty());
        QVERIFY(p.narration().isEmpty());
    }

    void aPromiseSurvivesTheSession()
    {
        QTemporaryDir dir;
        const QString state = dir.filePath(QStringLiteral("state"));

        {
            Presence p(state);
            QVERIFY(p.wake());
            QSignalSpy spy(&p, &Presence::changed);

            QVERIFY(!p.promise(QStringLiteral("verify sound after reboot")).isNull());
            QCOMPARE(spy.count(), 1);
            QCOMPARE(p.obligations(), QStringList{QStringLiteral("verify sound after reboot")});
        }

        // A reboot: new object, same data directory.
        Presence again(state);
        QVERIFY(again.wake());
        QCOMPARE(again.obligations(), QStringList{QStringLiteral("verify sound after reboot")});
        QVERIFY(again.narration().contains(QStringLiteral("owe")));
    }

    void reflectingIsItselfRecorded()
    {
        QTemporaryDir dir;
        Presence p(dir.filePath(QStringLiteral("state")));
        QVERIFY(p.wake());

        const int before = p.contributions();
        QVERIFY(p.reflect());
        QCOMPARE(p.contributions(), before + 1);

        const auto latest = p.recent(1);
        QCOMPARE(latest.size(), 1);
        QCOMPARE(latest.at(0).organ, QStringLiteral("selfd"));
        QCOMPARE(latest.at(0).kind, kindToString(ContributionKind::SelfAssessment));
    }

    void theActivityListIsNewestFirst()
    {
        QTemporaryDir dir;
        Presence p(dir.filePath(QStringLiteral("state")));
        QVERIFY(p.wake());

        p.promise(QStringLiteral("first"));
        p.reflect();

        const auto recent = p.recent(2);
        QCOMPARE(recent.size(), 2);
        QVERIFY(recent.at(0).when >= recent.at(1).when);
        QCOMPARE(recent.at(0).organ, QStringLiteral("selfd"));
        QCOMPARE(recent.at(1).organ, QStringLiteral("intentiond"));
    }

    void aSecondSessionCountsAsASecondSession()
    {
        QTemporaryDir dir;
        const QString state = dir.filePath(QStringLiteral("state"));

        {
            Presence p(state);
            QVERIFY(p.wake());
            QVERIFY(p.narration().contains(QStringLiteral("first day")));
        }

        Presence again(state);
        QVERIFY(again.wake());
        QVERIFY(again.narration().contains(QStringLiteral("session 2")));
    }

    void fulfillingAndAbandoningObligationsRemovesThemAndUpdatesStats()
    {
        QTemporaryDir dir;
        Presence p(dir.filePath(QStringLiteral("state")));
        QVERIFY(p.wake());

        p.promise(QStringLiteral("task 1"));
        p.promise(QStringLiteral("task 2"));
        p.promise(QStringLiteral("task 3"));
        QCOMPARE(p.obligations().size(), 3);

        const auto detailed = p.detailedObligations();
        QCOMPARE(detailed.size(), 3);
        QCOMPARE(detailed.at(0).toMap().value(QStringLiteral("description")).toString(), QStringLiteral("task 1"));

        // Fulfill task 1 (index 0)
        QVERIFY(p.fulfillIndex(0));
        QCOMPARE(p.obligations().size(), 2);
        QCOMPARE(p.obligations().at(0), QStringLiteral("task 2"));

        // Abandon task 3 (now index 1)
        QVERIFY(p.abandonIndex(1));
        QCOMPARE(p.obligations().size(), 1);
        QCOMPARE(p.obligations().at(0), QStringLiteral("task 2"));

        QVERIFY(p.observe(QStringLiteral("cpu-load"), 12.5));

        const QVariantMap stats = p.stats();
        QCOMPARE(stats.value(QStringLiteral("openIntentions")).toInt(), 1);
        QVERIFY(stats.value(QStringLiteral("journalIntact")).toBool());
    }
};

QTEST_MAIN(TestPresence)
#include "tst_presence.moc"
