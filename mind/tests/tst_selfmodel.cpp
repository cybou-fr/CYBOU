// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT
//
// The rule under test is ADR-0003's: nothing is shown that is not measured. These tests mostly
// check what the system refuses to claim.

#include "cybou/self/SelfModel.h"

#include <QSqlDatabase>
#include <QSqlQuery>
#include <QTemporaryDir>
#include <QTest>

using namespace cybou;

class TestSelfModel : public QObject
{
    Q_OBJECT

private:
    struct Fixture {
        QTemporaryDir dir;
        Journal journal;
        Identity identity;
        Intentions intentions;
        Predictor predictor;
        SelfModel self;

        Fixture()
            : journal(dir.filePath(QStringLiteral("j.db")))
            , identity(dir.filePath(QStringLiteral("identity.json")), &journal)
            , intentions(&journal)
            , predictor(&journal)
            , self(&journal, &identity, &intentions, &predictor)
        {
            identity.beginSession();
        }
    };

private Q_SLOTS:
    void itReportsWhatTheOtherOrgansKnow()
    {
        Fixture f;
        f.intentions.form(QStringLiteral("verify sound"));
        f.intentions.form(QStringLiteral("check network"));

        const SelfReport r = f.self.measure();
        QVERIFY(r.isValid());
        QCOMPARE(r.openIntentions, 2);
        QCOMPARE(r.sessions, 1u);
        QCOMPARE(r.architectureVersion, QStringLiteral("presence-0.1"));
        QVERIFY(r.journalIntact);
        QVERIFY(r.contributions > 0);
    }

    void untestedItDoesNotClaimAccuracy()
    {
        Fixture f;
        const SelfReport r = f.self.measure();

        QCOMPARE(r.settledPredictions, 0);
        QVERIFY(r.calibrations.isEmpty());

        // And it says so in words rather than staying conveniently quiet.
        QVERIFY(f.self.narrate(r).contains(QStringLiteral("not yet been tested")));
    }

    void accuracyAppearsOnlyAfterBeingChecked()
    {
        Fixture f;
        f.predictor.observe(QStringLiteral("build"), 10.0);

        const Forecast unsettled = f.predictor.predict(QStringLiteral("build"));
        QVERIFY(!unsettled.id.isNull());

        // A prediction nobody checked teaches nothing about accuracy.
        QCOMPARE(f.self.measure().settledPredictions, 0);

        f.predictor.settle(unsettled.id, 13.0);

        const SelfReport r = f.self.measure();
        QCOMPARE(r.settledPredictions, 1);
        QCOMPARE(r.calibrations.size(), 1);
        QCOMPARE(r.calibrations.at(0).subject, QStringLiteral("build"));
        QCOMPARE(r.calibrations.at(0).bias, 3.0);

        QVERIFY(f.self.narrate(r).contains(QStringLiteral("optimistic")));
    }

    void anAssessmentIsRemembered()
    {
        Fixture f;
        const quint64 before = f.journal.count();

        const SelfReport r = f.self.assess();
        QVERIFY(r.isValid());
        QCOMPARE(f.journal.count(), before + 1);
        QCOMPARE(f.journal.verify(), 0u);

        const auto latest = f.journal.recent(1);
        QCOMPARE(latest.size(), 1);
        QCOMPARE(latest.at(0).kind, ContributionKind::SelfAssessment);
        QCOMPARE(latest.at(0).originOrgan, QStringLiteral("selfd"));
    }

    void itDoesNotHideADamagedMemory()
    {
        Fixture f;
        f.intentions.form(QStringLiteral("something"));

        // Tamper with the chain the way corruption would.
        {
            QSqlDatabase db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"),
                                                        QStringLiteral("tamper"));
            db.setDatabaseName(f.dir.filePath(QStringLiteral("j.db")));
            QVERIFY(db.open());
            QSqlQuery q(db);
            QVERIFY(q.exec(QStringLiteral("UPDATE contribution SET origin_organ = 'forged' "
                                          "WHERE seq = 1")));
            db.close();
        }
        QSqlDatabase::removeDatabase(QStringLiteral("tamper"));

        Journal reopened(f.dir.filePath(QStringLiteral("j.db")),
                         QStringLiteral("reopened"));
        Identity identity(f.dir.filePath(QStringLiteral("identity.json")), &reopened);
        Intentions intentions(&reopened);
        Predictor predictor(&reopened);
        SelfModel self(&reopened, &identity, &intentions, &predictor);

        const SelfReport r = self.measure();
        QVERIFY(!r.journalIntact);
        QCOMPARE(r.firstBrokenAt, 1u);
        QVERIFY(self.narrate(r).contains(QStringLiteral("memory is damaged")));
    }

    void withoutAnOrganItSaysNothingRatherThanGuessing()
    {
        Fixture f;
        SelfModel crippled(&f.journal, &f.identity, nullptr, &f.predictor);

        const SelfReport r = crippled.measure();
        QVERIFY(!r.isValid());
        QCOMPARE(crippled.narrate(r), QStringLiteral("I cannot see myself clearly enough to say."));

        // And it refuses to record a self-assessment it could not actually make.
        const quint64 before = f.journal.count();
        QVERIFY(!crippled.assess().isValid());
        QCOMPARE(f.journal.count(), before);
    }
};

QTEST_MAIN(TestSelfModel)
#include "tst_selfmodel.moc"
