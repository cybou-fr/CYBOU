// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/predictor/Predictor.h"
#include "cybou/storage/Journal.h"

#include <QTemporaryDir>
#include <QTest>

using namespace cybou;

class TestPredictor : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    void withNoHistoryItSaysNothing()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        Predictor predictor(&journal);

        const Forecast forecast = predictor.predict(QStringLiteral("nixos-rebuild"));
        QVERIFY(forecast.id.isNull());
        QCOMPARE(journal.count(), 0u);
    }

    void predictionIsGroundedInHistoricalEvidence()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        Predictor predictor(&journal);

        QVERIFY(predictor.observe(QStringLiteral("build"), 10.0));
        QVERIFY(predictor.observe(QStringLiteral("build"), 12.0));
        QVERIFY(predictor.observe(QStringLiteral("build"), 14.0));

        const Forecast forecast = predictor.predict(QStringLiteral("build"));
        QVERIFY(!forecast.id.isNull());
        QCOMPARE(forecast.samples, 3);
        QCOMPARE(forecast.estimate, 12.0);
        QCOMPARE(forecast.margin, 4.0 / 3.0);
        QCOMPARE(forecast.confidence, 0.5);

        const auto stored = journal.contribution(forecast.id);
        QVERIFY(stored.has_value());
        QVERIFY(stored->causationId.isNull());
        QCOMPARE(stored->evidence.size(), 3);
        QVERIFY(!stored->evidence.contains(stored->messageId));
        for (const QUuid &evidenceId : stored->evidence) {
            QVERIFY(journal.contains(evidenceId));
        }
    }

    void settlingCreatesOneCausalOutcome()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        Predictor predictor(&journal);

        QVERIFY(predictor.observe(QStringLiteral("build"), 10.0));
        const Forecast forecast = predictor.predict(QStringLiteral("build"));
        QVERIFY(predictor.settle(forecast.id, 13.0));
        QVERIFY(!predictor.settle(forecast.id, 14.0));

        const auto latest = journal.recent(1).first();
        QCOMPARE(latest.kind, ContributionKind::Outcome);
        QCOMPARE(latest.causationId, forecast.id);
        QVERIFY(latest.evidence.isEmpty());

        const Calibration calibration = predictor.calibration(QStringLiteral("build"));
        QCOMPARE(calibration.settled, 1);
        QCOMPARE(calibration.meanError, 3.0);
        QCOMPARE(calibration.bias, 3.0);
    }

    void beingWrongChangesTheNextForecast()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        Predictor predictor(&journal);

        QVERIFY(predictor.observe(QStringLiteral("build"), 10.0));
        const Forecast first = predictor.predict(QStringLiteral("build"));
        QVERIFY(predictor.settle(first.id, 20.0));

        const Forecast second = predictor.predict(QStringLiteral("build"));
        QCOMPARE(second.samples, 2);
        QCOMPARE(second.estimate, 15.0);
    }

    void subjectsDoNotContaminateEachOther()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        Predictor predictor(&journal);

        predictor.observe(QStringLiteral("build"), 100.0);
        predictor.observe(QStringLiteral("boot"), 4.0);
        predictor.observe(QStringLiteral("boot"), 6.0);

        const Forecast forecast = predictor.predict(QStringLiteral("boot"));
        QCOMPARE(forecast.samples, 2);
        QCOMPARE(forecast.estimate, 5.0);
    }
};

QTEST_MAIN(TestPredictor)
#include "tst_predictor.moc"
