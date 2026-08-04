// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT
//
// The claim ADR-0003 makes falsifiable: the system predicts, checks itself against what
// happened, and can state how wrong it usually is. These tests are what "alive rather than
// animated" reduces to in practice.

#include "cybou/predictor/Predictor.h"

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
        Journal j(dir.filePath(QStringLiteral("j.db")));
        Predictor p(&j);

        const Forecast f = p.predict(QStringLiteral("nixos-rebuild"));
        QVERIFY(f.id.isNull());
        QCOMPARE(f.samples, 0);
        QCOMPARE(f.confidence, 0.0);

        // Nothing was written: a guess with no basis is not a contribution.
        QCOMPARE(j.count(), 0u);
    }

    void itPredictsFromWhatItHasLived()
    {
        QTemporaryDir dir;
        Journal j(dir.filePath(QStringLiteral("j.db")));
        Predictor p(&j);

        QVERIFY(p.observe(QStringLiteral("build"), 10.0));
        QVERIFY(p.observe(QStringLiteral("build"), 12.0));
        QVERIFY(p.observe(QStringLiteral("build"), 14.0));

        const Forecast f = p.predict(QStringLiteral("build"));
        QVERIFY(!f.id.isNull());
        QCOMPARE(f.samples, 3);
        QCOMPARE(f.estimate, 12.0);
        QCOMPARE(f.margin, 4.0 / 3.0); // mean absolute deviation
        QCOMPARE(f.confidence, 0.5);   // 3 / (3 + 3)
    }

    void confidenceGrowsWithEvidence()
    {
        QTemporaryDir dir;
        Journal j(dir.filePath(QStringLiteral("j.db")));
        Predictor p(&j);

        p.observe(QStringLiteral("build"), 10.0);
        const double few = p.predict(QStringLiteral("build")).confidence;

        for (int i = 0; i < 20; ++i) {
            p.observe(QStringLiteral("build"), 10.0);
        }
        const double many = p.predict(QStringLiteral("build")).confidence;

        QVERIFY(many > few);
        QVERIFY(many < 1.0); // and it never becomes certainty
    }

    void subjectsDoNotContaminateEachOther()
    {
        QTemporaryDir dir;
        Journal j(dir.filePath(QStringLiteral("j.db")));
        Predictor p(&j);

        p.observe(QStringLiteral("build"), 100.0);
        p.observe(QStringLiteral("boot"), 4.0);
        p.observe(QStringLiteral("boot"), 6.0);

        const Forecast f = p.predict(QStringLiteral("boot"));
        QCOMPARE(f.samples, 2);
        QCOMPARE(f.estimate, 5.0);
    }

    void itMeasuresHowWrongItWas()
    {
        QTemporaryDir dir;
        Journal j(dir.filePath(QStringLiteral("j.db")));
        Predictor p(&j);

        p.observe(QStringLiteral("build"), 10.0);
        p.observe(QStringLiteral("build"), 10.0);

        const Forecast f = p.predict(QStringLiteral("build"));
        QCOMPARE(f.estimate, 10.0);

        QVERIFY(p.settle(f.id, 13.0)); // reality ran three units long

        const Calibration c = p.calibration(QStringLiteral("build"));
        QCOMPARE(c.settled, 1);
        QCOMPARE(c.meanError, 3.0);
        QCOMPARE(c.bias, 3.0); // positive: the system was optimistic
    }

    void beingWrongChangesTheNextForecast()
    {
        QTemporaryDir dir;
        Journal j(dir.filePath(QStringLiteral("j.db")));
        Predictor p(&j);

        p.observe(QStringLiteral("build"), 10.0);
        const Forecast first = p.predict(QStringLiteral("build"));
        QCOMPARE(first.estimate, 10.0);

        p.settle(first.id, 20.0);

        // The outcome is experience too, so the next forecast has moved toward reality.
        // This is the whole point: without it, prediction is decoration.
        const Forecast second = p.predict(QStringLiteral("build"));
        QCOMPARE(second.samples, 2);
        QCOMPARE(second.estimate, 15.0);
    }

    void theEpisodeReplaysClaimAndResult()
    {
        QTemporaryDir dir;
        Journal j(dir.filePath(QStringLiteral("j.db")));
        Predictor p(&j);

        p.observe(QStringLiteral("build"), 10.0);
        const Forecast f = p.predict(QStringLiteral("build"));
        p.settle(f.id, 11.0);

        const auto episode = j.episode(f.id);
        QCOMPARE(episode.size(), 2);
        QCOMPARE(episode.at(0).kind, ContributionKind::Prediction);
        QCOMPARE(episode.at(1).kind, ContributionKind::Outcome);
        QCOMPARE(episode.at(1).causationId, f.id);
        QCOMPARE(j.verify(), 0u);
    }

    void settlingSomethingItNeverPredictedIsRefused()
    {
        QTemporaryDir dir;
        Journal j(dir.filePath(QStringLiteral("j.db")));
        Predictor p(&j);

        QVERIFY(!p.settle(QUuid::createUuid(), 5.0));
        QVERIFY(!p.lastError().isEmpty());
        QCOMPARE(j.count(), 0u);
    }
};

QTEST_MAIN(TestPredictor)
#include "tst_predictor.moc"
