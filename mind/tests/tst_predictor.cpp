// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/predictor/Predictor.h"
#include "PredictorService.h"
#include "cybou/fabric/FabricCodec.h"
#include "cybou/storage/Journal.h"

#include <QMap>
#include <QTemporaryDir>
#include <QTest>

using namespace cybou;

// A Journal whose paged reads can be made to fail. The only case that matters below is the one a
// healthy Journal will not produce.
class UnreadableAfter : public Journal
{
public:
    using Journal::Journal;

    ContributionPage after(quint64 afterSequence, int limit) const override
    {
        if (m_readsFail) {
            ContributionPage page;
            page.ok = false;
            return page;
        }
        return Journal::after(afterSequence, limit);
    }

    void failReads(bool fail) { m_readsFail = fail; }

private:
    bool m_readsFail{false};
};

class TestPredictor : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    // allCalibrations had no coverage at all, which is how it kept a defect that made
    // self-assessment cost the length of the biography multiplied by the number of subjects: it
    // replayed history to find the subjects, then replayed it again for each one. It now
    // accumulates every subject in a single pass, and this pins the arithmetic that refactor could
    // have changed.
    void everySubjectIsCalibratedFromOneReading()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        Predictor predictor(&journal);

        // Two subjects, settled with errors of known sign and size.
        QVERIFY(predictor.observe(QStringLiteral("build"), 10.0));
        const Forecast build = predictor.predict(QStringLiteral("build"));
        QVERIFY(predictor.settle(build.id, 14.0));

        QVERIFY(predictor.observe(QStringLiteral("boot"), 20.0));
        const Forecast boot = predictor.predict(QStringLiteral("boot"));
        QVERIFY(predictor.settle(boot.id, 18.0));

        const QList<Calibration> all = predictor.allCalibrations().value();
        QCOMPARE(all.size(), 2);

        // Each subject must carry its own arithmetic. A single-pass accumulator that shared state
        // between subjects would still return two entries, so the values are what proves it.
        QMap<QString, Calibration> bySubject;
        for (const Calibration &calibration : all) {
            bySubject.insert(calibration.subject, calibration);
        }
        QVERIFY(bySubject.contains(QStringLiteral("build")));
        QVERIFY(bySubject.contains(QStringLiteral("boot")));
        QCOMPARE(bySubject[QStringLiteral("build")].settled, 1);
        QCOMPARE(bySubject[QStringLiteral("boot")].settled, 1);

        // And must agree with what the single-subject query says, which is the contract the
        // refactor had to preserve.
        for (const QString &subject : {QStringLiteral("build"), QStringLiteral("boot")}) {
            const Calibration direct = predictor.calibration(subject).value();
            QCOMPARE(bySubject[subject].settled, direct.settled);
            QCOMPARE(bySubject[subject].meanError, direct.meanError);
            QCOMPARE(bySubject[subject].bias, direct.bias);
        }

        // Opposite-signed errors must not cancel across subjects.
        QVERIFY(
            bySubject[QStringLiteral("build")].bias
            != bySubject[QStringLiteral("boot")].bias);

        // A subject that was never settled contributes nothing.
        QVERIFY(predictor.observe(QStringLiteral("unsettled"), 1.0));
        QCOMPARE(predictor.allCalibrations().value().size(), 2);
    }

    // A calibration that could not be read is not a calibration of zero.
    //
    // Both of these used to return an empty value on a failed read, so a Journal that could not be
    // read reported every subject as never settled and perfectly unbiased. That is the most
    // flattering possible way to fail, and indistinguishable from a Mind that has simply not been
    // wrong yet.
    void anUnreadableJournalIsNotAPerfectRecord()
    {
        QTemporaryDir dir;
        UnreadableAfter journal(dir.filePath(QStringLiteral("j.db")));
        QVERIFY(journal.isOpen());

        Predictor predictor(&journal);
        QVERIFY(predictor.observe(QStringLiteral("build"), 10.0));
        const Forecast forecast = predictor.predict(QStringLiteral("build"));
        QVERIFY(predictor.settle(forecast.id, 14.0));
        QCOMPARE(predictor.calibration(QStringLiteral("build")).value().settled, 1);

        Predictor cold(&journal);
        journal.failReads(true);
        QVERIFY2(
            !cold.calibration(QStringLiteral("build")).has_value(),
            "a failed read must not answer as an unsettled subject");
        QVERIFY2(
            !cold.allCalibrations().has_value(),
            "a failed read must not answer as a Mind with no subjects");

        journal.failReads(false);
        QCOMPARE(cold.calibration(QStringLiteral("build")).value().settled, 1);
        QCOMPARE(cold.allCalibrations().value().size(), 1);
    }

    // The projection is incremental now: a read costs what has been accepted since the last read,
    // not the length of the biography. That is only safe if a later read still sees what arrived in
    // between - a cache that answers from a cursor it never advances is a cache that lies.
    //
    // The contribution here comes from a second writer against the same Journal, which is the case
    // a per-instance cache would get wrong: this Predictor did not append it and has no other way
    // to learn of it.
    void aLaterReadSeesWhatArrivedAfterTheEarlierOne()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        Predictor predictor(&journal);

        QVERIFY(predictor.observe(QStringLiteral("latency"), 10.0));
        const Forecast first = predictor.predict(QStringLiteral("latency"));
        QCOMPARE(first.samples, 1);

        Predictor other(&journal);
        QVERIFY(other.observe(QStringLiteral("latency"), 20.0));

        const Forecast second = predictor.predict(QStringLiteral("latency"));
        QCOMPARE(second.samples, 2);
        QCOMPARE(second.estimate, 15.0);
    }

    // Settling through one instance must be visible to a calibration read from another, for the
    // same reason. selfd asks predictord over the bus while predictord is settling its own
    // forecasts, so the two are genuinely different call paths into one Journal.
    void calibrationSeesAnOutcomeSettledAfterTheFirstRead()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        Predictor predictor(&journal);

        QVERIFY(predictor.observe(QStringLiteral("latency"), 10.0));
        QCOMPARE(predictor.calibration(QStringLiteral("latency")).value().settled, 0);

        Predictor other(&journal);
        const Forecast forecast = other.predict(QStringLiteral("latency"));
        QVERIFY(other.settle(forecast.id, 14.0));

        const Calibration calibration = predictor.calibration(QStringLiteral("latency")).value();
        QCOMPARE(calibration.settled, 1);
        QCOMPARE(calibration.meanError, 4.0);
        QCOMPARE(calibration.bias, 4.0);
    }

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

        const Calibration calibration = predictor.calibration(QStringLiteral("build")).value();
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

    void consolidationUsesStateAsOfHighWaterMark()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        PredictorService service(&journal);
        QVERIFY(service.Observe(QStringLiteral("before"), 1.0));
        QString error;
        QVariantMap forecast = FabricCodec::decodeMap(
            service.Predict(QStringLiteral("before"), {}), &error);
        QVERIFY(error.isEmpty());
        QVERIFY(service.Settle(forecast.value(QStringLiteral("id")).toString(), 2.0));
        const quint64 mark = journal.count();

        QVERIFY(service.Observe(QStringLiteral("after"), 3.0));
        forecast = FabricCodec::decodeMap(service.Predict(QStringLiteral("after"), {}), &error);
        QVERIFY(service.Settle(forecast.value(QStringLiteral("id")).toString(), 4.0));

        const QVariantMap receipt = FabricCodec::decodeMap(service.Consolidate(
            QUuid::createUuid().toString(QUuid::WithoutBraces),
            QStringLiteral("bounded-predictor"), mark), &error);
        QVERIFY2(error.isEmpty(), qPrintable(error));
        QCOMPARE(receipt.value(QStringLiteral("calibrationCount")).toInt(), 1);
    }
};

QTEST_MAIN(TestPredictor)
#include "tst_predictor.moc"
