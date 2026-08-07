// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

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

        CognitiveEnvelope observation(const QString &event)
        {
            CognitiveEnvelope e;
            e.messageId = QUuid::createUuid();
            e.correlationId = e.messageId;
            e.originOrgan = QStringLiteral("testd");
            e.kind = ContributionKind::Observation;
            e.wallTime = QDateTime::currentDateTimeUtc();
            e.privacy = PrivacyClass::Node;
            e.payloadCbor = event.toUtf8();
            return e;
        }

        QUuid appendObservation(const QString &event)
        {
            const CognitiveEnvelope e = observation(event);
            return journal.append(e) > 0 ? e.messageId : QUuid();
        }
    };

private Q_SLOTS:
    void itReportsWhatTheOtherOrgansKnow()
    {
        Fixture fixture;
        const QUuid firstCause = fixture.appendObservation(QStringLiteral("first request"));
        const QUuid secondCause = fixture.appendObservation(QStringLiteral("second request"));
        fixture.intentions.form(QStringLiteral("verify sound"), QString(), firstCause);
        fixture.intentions.form(QStringLiteral("check network"), QString(), secondCause);

        const SelfReport report = fixture.self.measure();
        QVERIFY(report.isValid());
        QCOMPARE(report.openIntentions, 2);
        QCOMPARE(report.sessions, 1u);
        QVERIFY(report.journalIntact);
    }

    void assessmentNeedsAnExistingCause()
    {
        Fixture fixture;
        const quint64 before = fixture.journal.count();
        QVERIFY(!fixture.self.assess(QUuid::createUuid()).isValid());
        QCOMPARE(fixture.journal.count(), before);
    }

    void assessmentIsCausallyGrounded()
    {
        Fixture fixture;
        const QUuid inspection = fixture.appendObservation(
            QStringLiteral("self-inspection-requested"));
        QVERIFY(!inspection.isNull());

        const SelfReport report = fixture.self.assess(inspection);
        QVERIFY(report.isValid());

        const auto latest = fixture.journal.recent(1).first();
        QCOMPARE(latest.kind, ContributionKind::SelfAssessment);
        QCOMPARE(latest.causationId, inspection);
        QVERIFY(latest.causationId != latest.messageId);
        QVERIFY(latest.evidence.isEmpty());
    }

    void accuracyAppearsOnlyAfterBeingChecked()
    {
        Fixture fixture;
        QVERIFY(fixture.predictor.observe(QStringLiteral("build"), 10.0));
        const Forecast forecast = fixture.predictor.predict(QStringLiteral("build"));
        QVERIFY(fixture.predictor.settle(forecast.id, 13.0));

        const SelfReport report = fixture.self.measure();
        QCOMPARE(report.settledPredictions, 1);
        QCOMPARE(report.calibrations.size(), 1);
        QCOMPARE(report.calibrations.at(0).bias, 3.0);
        QVERIFY(fixture.self.narrate(report).contains(QStringLiteral("optimistic")));
    }

    void itDoesNotHideADamagedMemory()
    {
        Fixture fixture;

        {
            QSqlDatabase db = QSqlDatabase::addDatabase(
                QStringLiteral("QSQLITE"), QStringLiteral("tamper-self"));
            db.setDatabaseName(fixture.dir.filePath(QStringLiteral("j.db")));
            QVERIFY(db.open());
            QSqlQuery query(db);
            QVERIFY(query.exec(QStringLiteral(
                "UPDATE contribution SET origin_organ = 'forged' WHERE seq = 1")));
            db.close();
        }
        QSqlDatabase::removeDatabase(QStringLiteral("tamper-self"));

        Journal reopened(
            fixture.dir.filePath(QStringLiteral("j.db")), QStringLiteral("reopened"));
        Identity identity(fixture.dir.filePath(QStringLiteral("identity.json")), &reopened);
        Intentions intentions(&reopened);
        Predictor predictor(&reopened);
        SelfModel self(&reopened, &identity, &intentions, &predictor);

        const SelfReport report = self.measure();
        QVERIFY(!report.journalIntact);
        QCOMPARE(report.firstBrokenAt, 1u);
        QVERIFY(self.narrate(report).contains(QStringLiteral("memory is damaged")));
    }

    void withoutAnOrganItRefusesToAssess()
    {
        Fixture fixture;
        SelfModel crippled(
            &fixture.journal, &fixture.identity, nullptr, &fixture.predictor);
        const QUuid inspection = fixture.appendObservation(QStringLiteral("inspection"));

        QVERIFY(!crippled.measure().isValid());
        QVERIFY(!crippled.assess(inspection).isValid());
    }
};

QTEST_MAIN(TestSelfModel)
#include "tst_selfmodel.moc"
