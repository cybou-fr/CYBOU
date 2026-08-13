// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT
//
// Journal scale baseline.
//
// The checkpoint records "no performance envelope for Journal and compound projections" as a P0
// risk, on the grounds that correctness tests can hide growth-driven failure. This is the first
// answer to that: deterministic fixtures at a chosen size, measurements printed as a table, and
// ceilings loose enough that ordinary machine variance does not fail the build but a regression in
// the shape of a cost does.
//
// The size is CYBOU_SCALE_CONTRIBUTIONS, defaulting to 10k so it can run in the ordinary checks.
// 100k and 1m are the same code with a larger number, run as a separate gate and as a manual
// benchmark respectively.
//
// What is deliberately NOT asserted: absolute times against a fixed budget. This suite runs on
// whatever machine builds the package, so a wall-clock ceiling tight enough to be meaningful would
// be flaky, and one loose enough to be stable would be meaningless. The numbers are printed so a
// budget can be set from evidence, which is the point of a baseline.

#include "cybou/storage/Journal.h"

#include <QElapsedTimer>
#include <QFileInfo>
#include "cybou/intentions/Intentions.h"
#include "cybou/predictor/Predictor.h"

#include <QTemporaryDir>
#include <QTest>

using namespace cybou;

namespace {

int contributionCount()
{
    bool ok = false;
    const int configured = qEnvironmentVariableIntValue("CYBOU_SCALE_CONTRIBUTIONS", &ok);
    return ok && configured > 0 ? configured : 10000;
}

// Deterministic by construction: the same index always produces the same envelope, so two runs
// build byte-identical journals and a measured difference is a real difference.
CognitiveEnvelope contributionAt(int index)
{
    CognitiveEnvelope e;
    e.messageId = QUuid::createUuidV5(
        QUuid(QStringLiteral("6ba7b810-9dad-11d1-80b4-00c04fd430c8")),
        QStringLiteral("cybou-scale-%1").arg(index).toUtf8());
    e.correlationId = e.messageId;
    e.originOrgan = QStringLiteral("scale-fixture");
    e.originNode = QStringLiteral("local");
    e.kind = ContributionKind::Observation;
    // A fixed instant keeps the fixture reproducible; the Journal does not order by wall time.
    e.wallTime = QDateTime::fromSecsSinceEpoch(1767225600, Qt::UTC).addSecs(index);
    e.monotonicTime = static_cast<quint64>(index);
    e.logicalClock = static_cast<quint64>(index);
    e.confidence = 1.0;
    e.privacy = PrivacyClass::Local;

    QCborMap payload;
    payload.insert(QStringLiteral("subject"), QStringLiteral("scale"));
    payload.insert(QStringLiteral("index"), index);
    e.payloadCbor = payload.toCborValue().toCbor();
    return e;
}

// The same biography, shaped like one.
//
// Every contribution above is a root observation: no causation, no evidence, nothing to look up on
// the way in. Mind never produces that. A prediction cites the observations it rests on, an outcome
// settles a prediction, and Event1 checks each of those references and inherits privacy from them
// before it accepts anything - work the flat fixture never pays for, and work that grows with how
// densely a life is connected rather than with how long it is.
//
// Five-contribution episodes, deterministic like the flat fixture:
//   +0 root observation      +1 prediction citing it     +2 intention citing both
//   +3 outcome settling +1   +4 observation citing +0..+2
//
// Privacy is uniform, because Event1 requires a contribution's class to equal the most restrictive
// of its references exactly. Mixing classes would measure rejection, not acceptance; the per-
// reference lookup that dominates the cost is identical either way.
CognitiveEnvelope graphContributionAt(int index)
{
    CognitiveEnvelope e = contributionAt(index);
    const int offset = index % 5;
    const int root = index - offset;
    const auto idAt = [](int i) { return contributionAt(i).messageId; };

    // Two rules shape this, and the first version broke both. An Observation is a root kind and may
    // carry no causation and no evidence at all, so a derived contribution has to be some other
    // kind. And evidence may never repeat the causationId - citing the thing you were caused by is
    // a duplicate reference, not a second one.
    switch (offset) {
    case 0:
        return e;
    case 1:
        e.kind = ContributionKind::Prediction;
        e.evidence = {idAt(root)};
        break;
    case 2:
        e.kind = ContributionKind::Intention;
        e.causationId = idAt(root + 1);
        e.evidence = {idAt(root)};
        break;
    case 3:
        // Settles the prediction rather than the root: Event1 permits exactly one terminal Outcome
        // per cause, so every episode has to settle a different one.
        e.kind = ContributionKind::Outcome;
        e.causationId = idAt(root + 1);
        break;
    default:
        e.kind = ContributionKind::Prediction;
        e.evidence = {idAt(root), idAt(root + 2)};
        break;
    }

    e.correlationId = idAt(root);
    return e;
}

void report(const QString &measure, const QString &value)
{
    qInfo().noquote() << QStringLiteral("  %1 %2").arg(measure, -44, QLatin1Char('.')).arg(value);
}

void reportMs(const QString &measure, qint64 ms)
{
    report(measure, QStringLiteral("%1 ms").arg(ms));
}

} // namespace

class TestJournalScale : public QObject
{
    Q_OBJECT

private:
    QTemporaryDir m_dir;
    QString m_path;
    int m_count{0};

private Q_SLOTS:
    void initTestCase()
    {
        QVERIFY(m_dir.isValid());
        m_path = m_dir.filePath(QStringLiteral("scale.db"));
        m_count = contributionCount();
        qInfo().noquote() << QStringLiteral("Journal scale baseline at %1 contributions")
                                 .arg(m_count);
    }

    // Built through appendBatch so the fixture costs one fsync per batch rather than per row.
    // Append cost under production durability is measured separately below, because that is the
    // number that actually describes the write path.
    void buildFixture()
    {
        Journal journal(m_path);
        QVERIFY2(journal.isOpen(), qPrintable(journal.lastError()));

        constexpr int kBatch = 1000;
        QElapsedTimer elapsed;
        elapsed.start();

        for (int start = 0; start < m_count; start += kBatch) {
            QList<CognitiveEnvelope> batch;
            const int end = std::min(start + kBatch, m_count);
            batch.reserve(end - start);
            for (int index = start; index < end; ++index) {
                batch.append(contributionAt(index));
            }
            QVERIFY2(journal.appendBatch(batch) > 0, qPrintable(journal.lastError()));
        }

        const qint64 buildMs = elapsed.elapsed();
        QCOMPARE(journal.count(), static_cast<quint64>(m_count));
        reportMs(QStringLiteral("fixture build (batched, %1 per commit)").arg(kBatch), buildMs);
    }

    // The honest write-path number: single contributions, each its own transaction and fsync,
    // exactly as Event1 accepts them. Measured over a small sample because the point is the
    // per-contribution cost, not the total.
    void appendUnderProductionDurability()
    {
        Journal journal(m_path);
        QVERIFY(journal.isOpen());

        constexpr int kSample = 200;
        QElapsedTimer elapsed;
        elapsed.start();
        for (int index = 0; index < kSample; ++index) {
            QVERIFY2(
                journal.append(contributionAt(m_count + index)) > 0,
                qPrintable(journal.lastError()));
        }
        const qint64 appendMs = elapsed.elapsed();

        reportMs(QStringLiteral("append x%1 (one fsync each)").arg(kSample), appendMs);
        report(
            QStringLiteral("append per contribution"),
            QStringLiteral("%1 ms").arg(double(appendMs) / kSample, 0, 'f', 3));
    }

    // The path every organ takes to rebuild its state. recent(0) is unbounded by design and is how
    // intentiond, predictord and selfd replay their whole history, so this is the cost of one organ
    // starting up - paid by each of them, on every start.
    void fullReplay()
    {
        Journal journal(m_path);
        QVERIFY(journal.isOpen());

        QElapsedTimer elapsed;
        elapsed.start();
        const QList<CognitiveEnvelope> all = journal.recent(0);
        const qint64 replayMs = elapsed.elapsed();

        QCOMPARE(all.size(), journal.count());
        reportMs(QStringLiteral("full replay recent(0)"), replayMs);
        report(
            QStringLiteral("full replay per contribution"),
            QStringLiteral("%1 us").arg(double(replayMs) * 1000.0 / all.size(), 0, 'f', 2));
    }

    // The paged replacement for recent(0). In-process this is not expected to be faster - the same
    // rows are read and decoded, plus one query per page - so the number here is a check that
    // paging costs little, not a claimed speedup. The win paging buys is memory, and across D-Bus
    // the absence of a single reply carrying the entire biography.
    void pagedReplay()
    {
        Journal journal(m_path);
        QVERIFY(journal.isOpen());

        int seen = 0;
        QElapsedTimer elapsed;
        elapsed.start();
        QVERIFY(journal.replayAll([&seen](const CognitiveEnvelope &) { ++seen; }, 1000));
        const qint64 pagedMs = elapsed.elapsed();

        QCOMPARE(static_cast<quint64>(seen), journal.count());
        reportMs(QStringLiteral("paged replay, 1000 per page"), pagedMs);
        report(
            QStringLiteral("paged replay per contribution"),
            QStringLiteral("%1 us").arg(double(pagedMs) * 1000.0 / seen, 0, 'f', 2));
    }

    // Verify rechains the whole journal and is reachable from selfd's ordinary self-assessment
    // path, so its cost is not confined to a maintenance task.
    void fullVerification()
    {
        Journal journal(m_path);
        QVERIFY(journal.isOpen());

        QElapsedTimer elapsed;
        elapsed.start();
        const quint64 brokenAt = journal.verify();
        const qint64 verifyMs = elapsed.elapsed();

        QCOMPARE(brokenAt, 0u);
        reportMs(QStringLiteral("full verify"), verifyMs);
    }

    // The same check anchored at a checkpoint. This is the number the 460k cliff turns on: full
    // verification scales with the whole history, incremental verification scales with what has
    // arrived since it was last run.
    void incrementalVerification()
    {
        Journal journal(m_path);
        QVERIFY(journal.isOpen());

        const VerifiedCheckpoint anchor = journal.checkpointAtHead();
        QVERIFY(!anchor.isEmpty());

        // A realistic suffix: what a session might add between two verifications.
        for (int index = 0; index < 500; ++index) {
            QVERIFY(journal.append(contributionAt(m_count + 100000 + index)) > 0);
        }

        QElapsedTimer elapsed;
        elapsed.start();
        const VerificationResult result = journal.verifyFrom(anchor);
        const qint64 incrementalMs = elapsed.elapsed();

        QCOMPARE(result.status, VerificationStatus::VerifiedThrough);
        reportMs(QStringLiteral("incremental verify, 500 new"), incrementalMs);
    }

    // Indexed lookups must not care how much history precedes them. This is the one assertion that
    // is about shape rather than speed, and it is machine-independent: a scan would make the last
    // contribution dramatically more expensive to reach than the first.
    // What a derived organ pays to answer a question, before and after it has read the biography.
    //
    // Predictor used to scan the whole Journal on every read, so a self-assessment cost the length
    // of a life each time it was asked. It now advances a cursor, and the second read pays only for
    // what arrived since the first. The fixture's contributions are all foreign to predictord, so
    // this measures the traversal rather than the accumulation - which is the part that grew.
    void aDerivedProjectionPaysForTheBiographyOnlyOnce()
    {
        Journal journal(m_path);
        QVERIFY(journal.isOpen());
        Predictor predictor(&journal);

        QElapsedTimer timer;
        timer.start();
        const QList<Calibration> cold = predictor.allCalibrations().value();
        const qint64 coldMs = timer.elapsed();

        timer.restart();
        const QList<Calibration> warm = predictor.allCalibrations().value();
        const qint64 warmMs = timer.elapsed();

        QCOMPARE(warm.size(), cold.size());
        reportMs(QStringLiteral("predictor first read (whole biography)"), coldMs);
        reportMs(QStringLiteral("predictor second read (nothing new)"), warmMs);

        // Deliberately not a ratio against coldMs: this suite runs on whatever hardware it lands
        // on, and the claim being pinned is absolute anyway. A read that answers from the cursor
        // does no work proportional to history, so it cannot take a meaningful number of
        // milliseconds no matter how slow the machine is.
        QVERIFY2(
            warmMs <= 50,
            qPrintable(QStringLiteral("second read took %1 ms; it should do no work").arg(warmMs)));
    }

    // The obligation question, asked twice.
    //
    // Presence asks what Mind is committed to on every Snapshot, and open() used to replay the
    // whole biography each time. The fixture holds no intentions at all, so this measures the
    // traversal rather than the assembly - which is the part that grew with the length of a life.
    void askingWhatIsCommittedPaysForTheBiographyOnlyOnce()
    {
        Journal journal(m_path);
        QVERIFY(journal.isOpen());
        Intentions intentions(&journal);

        QElapsedTimer timer;
        timer.start();
        const QList<Intention> cold = intentions.open().value();
        const qint64 coldMs = timer.elapsed();

        timer.restart();
        const QList<Intention> warm = intentions.open().value();
        const qint64 warmMs = timer.elapsed();

        QCOMPARE(warm.size(), cold.size());
        reportMs(QStringLiteral("open() first read (whole biography)"), coldMs);
        reportMs(QStringLiteral("open() second read (nothing new)"), warmMs);

        QVERIFY2(
            warmMs <= 50,
            qPrintable(QStringLiteral("second read took %1 ms; it should do no work").arg(warmMs)));
    }

    // What the numbers above cost once the biography has a shape.
    //
    // This is not a variant of the same measurement: it is the honest one. The flat fixture
    // measures a Journal nothing writes, and every budget derived from it is optimistic by whatever
    // this difference turns out to be.
    void aConnectedBiographyCostsMoreThanAFlatOne()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString path = dir.filePath(QStringLiteral("graph.db"));

        Journal journal(path);
        QVERIFY2(journal.isOpen(), qPrintable(journal.lastError()));

        constexpr int kBatch = 1000;
        QElapsedTimer elapsed;
        elapsed.start();
        for (int start = 0; start < m_count; start += kBatch) {
            QList<CognitiveEnvelope> batch;
            const int end = std::min(start + kBatch, m_count);
            batch.reserve(end - start);
            for (int index = start; index < end; ++index) {
                batch.append(graphContributionAt(index));
            }
            QVERIFY2(journal.appendBatch(batch) > 0, qPrintable(journal.lastError()));
        }
        const qint64 buildMs = elapsed.elapsed();
        QCOMPARE(journal.count(), static_cast<quint64>(m_count));

        // Appends past the fixture continue the same episode pattern, so an Outcome in the sample
        // settles a prediction the sample itself wrote rather than one the fixture already settled.
        constexpr int kSample = 200;
        elapsed.restart();
        for (int index = 0; index < kSample; ++index) {
            QVERIFY2(
                journal.append(graphContributionAt(m_count + index)) > 0,
                qPrintable(journal.lastError()));
        }
        const qint64 appendMs = elapsed.elapsed();

        elapsed.restart();
        const auto all = journal.recent(0);
        const qint64 replayMs = elapsed.elapsed();
        QCOMPARE(all.size(), static_cast<int>(journal.count()));

        elapsed.restart();
        // verify() answers where the chain broke, so zero is the intact answer rather than a count
        // of nothing verified. Asserting it like a count is a mistake this comment exists to stop.
        const quint64 brokenAt = journal.verify();
        const qint64 verifyMs = elapsed.elapsed();
        QVERIFY2(
            brokenAt == 0,
            qPrintable(QStringLiteral("chain broken at %1 of %2")
                           .arg(brokenAt)
                           .arg(journal.count())));

        // Evidence survived the round trip. Without this the graph could be silently flat and every
        // number below would describe the fixture this test exists to stop measuring.
        int withEvidence = 0;
        for (const CognitiveEnvelope &e : all) {
            if (!e.evidence.isEmpty()) {
                ++withEvidence;
            }
        }
        QVERIFY2(withEvidence > all.size() / 2, qPrintable(QStringLiteral(
            "only %1 of %2 contributions carry evidence").arg(withEvidence).arg(all.size())));

        reportMs(QStringLiteral("connected fixture build (batched)"), buildMs);
        report(
            QStringLiteral("connected append per contribution"),
            QStringLiteral("%1 ms").arg(double(appendMs) / kSample, 0, 'f', 3));
        reportMs(QStringLiteral("connected full replay recent(0)"), replayMs);
        reportMs(QStringLiteral("connected full Verify"), verifyMs);
    }

    void indexedLookupDoesNotScaleWithHistory()
    {
        Journal journal(m_path);
        QVERIFY(journal.isOpen());

        const CognitiveEnvelope first = contributionAt(0);
        const CognitiveEnvelope last = contributionAt(m_count - 1);

        constexpr int kRepeats = 200;
        QElapsedTimer elapsed;

        elapsed.start();
        for (int i = 0; i < kRepeats; ++i) {
            QVERIFY(journal.contribution(first.messageId).has_value());
        }
        const qint64 firstMs = elapsed.elapsed();

        elapsed.restart();
        for (int i = 0; i < kRepeats; ++i) {
            QVERIFY(journal.contribution(last.messageId).has_value());
        }
        const qint64 lastMs = elapsed.elapsed();

        reportMs(QStringLiteral("lookup oldest x%1").arg(kRepeats), firstMs);
        reportMs(QStringLiteral("lookup newest x%1").arg(kRepeats), lastMs);

        // Generous: this catches a lost index, not a slow machine.
        QVERIFY2(
            lastMs <= std::max<qint64>(50, firstMs * 10),
            qPrintable(QStringLiteral("reaching the newest contribution cost %1 ms against %2 ms "
                                      "for the oldest, which is the shape of a scan")
                           .arg(lastMs)
                           .arg(firstMs)));
    }

    // The consolidation backlog was a per-row decode before P6.8. It is now one aggregate query,
    // and this is where that difference becomes visible rather than theoretical.
    void consolidationBacklogIsAggregate()
    {
        Journal journal(m_path);
        QVERIFY(journal.isOpen());

        QElapsedTimer elapsed;
        elapsed.start();
        const quint64 backlog = journal.countAfterExcludingCapability(
            0, QStringLiteral("lifecycle.consolidation"));
        const qint64 backlogMs = elapsed.elapsed();

        QCOMPARE(backlog, journal.count());
        reportMs(QStringLiteral("consolidation backlog count"), backlogMs);
    }

    void journalFootprint()
    {
        Journal journal(m_path);
        QVERIFY(journal.isOpen());
        const quint64 rows = journal.count();

        qint64 bytes = QFileInfo(m_path).size();
        for (const QString &suffix : {QStringLiteral("-wal"), QStringLiteral("-shm")}) {
            bytes += QFileInfo(m_path + suffix).size();
        }

        report(
            QStringLiteral("journal size"),
            QStringLiteral("%1 KiB").arg(bytes / 1024));
        report(
            QStringLiteral("journal size per contribution"),
            QStringLiteral("%1 bytes").arg(double(bytes) / rows, 0, 'f', 1));
    }
};

QTEST_MAIN(TestJournalScale)
#include "tst_journal_scale.moc"
