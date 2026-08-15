// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT

#include "cybou/crypto/KeyStore.h"
#include "cybou/storage/RetentionSweep.h"

#include <QCborMap>
#include <QTemporaryDir>
#include <QTest>

using namespace cybou;

namespace {

/// An observation with a real payload. A sweep over empty payloads proves nothing: erasing nothing
/// changes nothing, and every assertion about redaction would hold trivially.
CognitiveEnvelope observation(const QDateTime &retainUntil)
{
    CognitiveEnvelope e;
    e.messageId = QUuid::createUuid();
    e.correlationId = e.messageId;
    e.originOrgan = QStringLiteral("perceptiond");
    e.originNode = QStringLiteral("test-node");
    e.wallTime = QDateTime::currentDateTimeUtc();
    e.kind = ContributionKind::Observation;
    e.privacy = PrivacyClass::Node;

    QCborMap payload;
    payload.insert(QStringLiteral("value"), QStringLiteral("something worth forgetting"));
    e.payloadCbor = payload.toCborValue().toCbor();

    if (retainUntil.isValid()) {
        e.schemaVersion = kProtectedEnvelopeSchemaVersion;
        e.retentionClass = RetentionClass::Standard;
        e.retainUntil = retainUntil;
    }
    return e;
}

} // namespace

class TestRetentionSweep : public QObject
{
    Q_OBJECT

private slots:
    void erasesWhatHasExpiredAndNothingElse();
    void aContributionWithNoRetainUntilNeverExpires();
    void sweepingTwiceErasesNothingTheSecondTime();
    void aTruncatedSweepSaysSoRatherThanLookingClean();
    void anInterruptedErasureIsFinishedBeforeNewWorkStarts();
    void theBoundaryInstantIsInclusive();
    void theRecordOfAnErasureDoesNotItselfExpire();
};

void TestRetentionSweep::theRecordOfAnErasureDoesNotItselfExpire()
{
    QTemporaryDir dir;
    Journal journal(dir.filePath(QStringLiteral("j.db")));
    QVERIFY(journal.isOpen());
    KeyStore keys(dir.filePath(QStringLiteral("keys")));
    QVERIFY(keys.isUsable());

    const QDateTime now = QDateTime::currentDateTimeUtc();
    const CognitiveEnvelope target = observation(now.addDays(-1));
    QVERIFY(journal.append(target) > 0);

    RetentionSweep sweep(journal, keys);
    QCOMPARE(sweep.sweep(now).erased, 1);

    // The erasure record names the target as its cause. Inheriting the target's retention would
    // make it expire alongside what it recorded, and a later sweep would erase the evidence that
    // anything had ever been erased.
    QUuid erasureRecord;
    QDateTime erasureRetainUntil;
    for (const CognitiveEnvelope &e : journal.recent(50)) {
        if (e.kind == ContributionKind::ErasureApplied) {
            erasureRecord = e.messageId;
            erasureRetainUntil = e.retainUntil;
        }
    }
    QVERIFY2(!erasureRecord.isNull(), "the erasure left no record of itself");
    QVERIFY2(!erasureRetainUntil.isValid(), "an erasure record with a retention date will expire");

    // Far into the future, and it is still there with its payload intact.
    const SweepReport later = sweep.sweep(now.addYears(100));
    QVERIFY(later.isClean());
    QCOMPARE(later.expiredFound, 0);

    const auto record = journal.contribution(erasureRecord);
    QVERIFY(record.has_value());
    QVERIFY2(!record->payloadCbor.isEmpty(), "the erasure record's own payload was erased");
}

void TestRetentionSweep::erasesWhatHasExpiredAndNothingElse()
{
    QTemporaryDir dir;
    Journal journal(dir.filePath(QStringLiteral("j.db")));
    QVERIFY(journal.isOpen());
    KeyStore keys(dir.filePath(QStringLiteral("keys")));
    QVERIFY(keys.isUsable());

    const QDateTime now = QDateTime::currentDateTimeUtc();
    const CognitiveEnvelope stale = observation(now.addDays(-1));
    const CognitiveEnvelope fresh = observation(now.addDays(30));
    QVERIFY(journal.append(stale) > 0);
    QVERIFY(journal.append(fresh) > 0);

    RetentionSweep sweep(journal, keys);
    const SweepReport report = sweep.sweep(now);

    QVERIFY2(report.isClean(), qPrintable(sweep.lastError()));
    QCOMPARE(report.erased, 1);
    QVERIFY(report.complete);

    const auto erased = journal.contribution(stale.messageId);
    QVERIFY2(erased.has_value(), "the record survives; only its payload goes");
    QVERIFY(erased->payloadCbor.isEmpty());

    // The one still inside its window is untouched, payload and all.
    const auto kept = journal.contribution(fresh.messageId);
    QVERIFY(kept.has_value());
    QCOMPARE(kept->payloadCbor, fresh.payloadCbor);

    // And the chain is still whole, with the erased row's content skipped rather than verified.
    const VerificationResult result = journal.verifyFrom({});
    QVERIFY(result.intact());
    QCOMPARE(result.contentSkipped, 1u);
}

void TestRetentionSweep::aContributionWithNoRetainUntilNeverExpires()
{
    QTemporaryDir dir;
    Journal journal(dir.filePath(QStringLiteral("j.db")));
    QVERIFY(journal.isOpen());
    KeyStore keys(dir.filePath(QStringLiteral("keys")));
    QVERIFY(keys.isUsable());

    const CognitiveEnvelope permanent = observation(QDateTime());
    QVERIFY(!permanent.retainUntil.isValid());
    QVERIFY(journal.append(permanent) > 0);

    RetentionSweep sweep(journal, keys);

    // Far into the future: if a missing date read as "expired long ago", this is where it shows.
    const SweepReport report
        = sweep.sweep(QDateTime::currentDateTimeUtc().addYears(100));

    QVERIFY(report.isClean());
    QCOMPARE(report.expiredFound, 0);
    QCOMPARE(report.erased, 0);

    const auto kept = journal.contribution(permanent.messageId);
    QVERIFY(kept.has_value());
    QCOMPARE(kept->payloadCbor, permanent.payloadCbor);
}

void TestRetentionSweep::sweepingTwiceErasesNothingTheSecondTime()
{
    QTemporaryDir dir;
    Journal journal(dir.filePath(QStringLiteral("j.db")));
    QVERIFY(journal.isOpen());
    KeyStore keys(dir.filePath(QStringLiteral("keys")));
    QVERIFY(keys.isUsable());

    const QDateTime now = QDateTime::currentDateTimeUtc();
    QVERIFY(journal.append(observation(now.addDays(-1))) > 0);

    RetentionSweep sweep(journal, keys);
    QCOMPARE(sweep.sweep(now).erased, 1);
    const quint64 epochAfterFirst = journal.erasureEpoch();

    const SweepReport second = sweep.sweep(now);
    QVERIFY(second.isClean());
    QCOMPARE(second.expiredFound, 0);
    QCOMPARE(second.erased, 0);
    QCOMPARE(second.resumed, 0);
    QVERIFY(second.complete);

    // A second erasure of the same record would bump the epoch again and make every persisted
    // projection rebuild over nothing.
    QCOMPARE(journal.erasureEpoch(), epochAfterFirst);
}

void TestRetentionSweep::aTruncatedSweepSaysSoRatherThanLookingClean()
{
    QTemporaryDir dir;
    Journal journal(dir.filePath(QStringLiteral("j.db")));
    QVERIFY(journal.isOpen());
    KeyStore keys(dir.filePath(QStringLiteral("keys")));
    QVERIFY(keys.isUsable());

    const QDateTime now = QDateTime::currentDateTimeUtc();
    for (int i = 0; i < 5; ++i) {
        QVERIFY(journal.append(observation(now.addDays(-1))) > 0);
    }

    RetentionSweep sweep(journal, keys);
    const SweepReport partial = sweep.sweep(now, 2);

    QVERIFY(partial.isClean());
    QCOMPARE(partial.erased, 2);
    QVERIFY2(!partial.complete, "a sweep that stopped early must not report a finished pass");

    // The rest is still there, and a later sweep finds it rather than losing the cursor.
    const SweepReport rest = sweep.sweep(now, 64);
    QCOMPARE(rest.erased, 3);
    QVERIFY(rest.complete);
}

void TestRetentionSweep::anInterruptedErasureIsFinishedBeforeNewWorkStarts()
{
    QTemporaryDir dir;
    Journal journal(dir.filePath(QStringLiteral("j.db")));
    QVERIFY(journal.isOpen());
    KeyStore keys(dir.filePath(QStringLiteral("keys")));
    QVERIFY(keys.isUsable());

    const QDateTime now = QDateTime::currentDateTimeUtc();
    const CognitiveEnvelope interrupted = observation(now.addDays(-2));
    const CognitiveEnvelope untouched = observation(now.addDays(-1));
    QVERIFY(journal.append(interrupted) > 0);
    QVERIFY(journal.append(untouched) > 0);

    // A crash between step one and step three: intent recorded, nothing applied.
    QVERIFY(journal.requestErasure(interrupted.messageId, QStringLiteral("retention-expiry")) > 0);
    QCOMPARE(journal.incompleteErasures().size(), 1);

    RetentionSweep sweep(journal, keys);
    const SweepReport report = sweep.sweep(now);

    QVERIFY2(report.isClean(), qPrintable(sweep.lastError()));
    QCOMPARE(report.resumed, 1);

    // The half-erased record is counted as resumed, not as a fresh expiry: `expiredBefore` excludes
    // anything already requested, so it is not requested a second time.
    QCOMPARE(report.expiredFound, 1);
    QCOMPARE(report.erased, 1);
    QVERIFY(journal.incompleteErasures().isEmpty());

    for (const QUuid &id : {interrupted.messageId, untouched.messageId}) {
        const auto row = journal.contribution(id);
        QVERIFY(row.has_value());
        QVERIFY(row->payloadCbor.isEmpty());
    }
}

void TestRetentionSweep::theBoundaryInstantIsInclusive()
{
    QTemporaryDir dir;
    Journal journal(dir.filePath(QStringLiteral("j.db")));
    QVERIFY(journal.isOpen());
    KeyStore keys(dir.filePath(QStringLiteral("keys")));
    QVERIFY(keys.isUsable());

    const QDateTime deadline = QDateTime::currentDateTimeUtc();
    const CognitiveEnvelope exactly = observation(deadline);
    QVERIFY(journal.append(exactly) > 0);

    // One millisecond before the deadline, the window has not closed.
    RetentionSweep sweep(journal, keys);
    QCOMPARE(sweep.sweep(deadline.addMSecs(-1)).erased, 0);
    QVERIFY(journal.contribution(exactly.messageId)->payloadCbor.isEmpty() == false);

    // At the deadline itself it has. "Retain until T" that kept the record past T would make the
    // date advisory.
    QCOMPARE(sweep.sweep(deadline).erased, 1);
    QVERIFY(journal.contribution(exactly.messageId)->payloadCbor.isEmpty());
}

QTEST_MAIN(TestRetentionSweep)
#include "tst_retention_sweep.moc"
