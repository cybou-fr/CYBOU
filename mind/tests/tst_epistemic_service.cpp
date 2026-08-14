// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT
//
// The epistemic owner: what the process adds over the projection it holds.
//
// The projection's reasoning is covered separately. What is new here is the cursor - how much of
// the biography has been taken in - and the checkpoint that lets a restart resume instead of
// replaying from zero, which ADR-0027 requires because a full replay exhausts the Presence budget
// near 560k contributions.

#include "EpistemicService.h"

#include "cybou/fabric/FabricCodec.h"
#include "cybou/protocol/Observation.h"
#include "cybou/storage/Journal.h"

#include <QCborArray>
#include <QCborMap>
#include <QTemporaryDir>
#include <QTest>

using namespace cybou;

namespace {

CognitiveEnvelope observationOf(const QString &value, const QDateTime &acquiredAt)
{
    ObservationV1 observation;
    observation.sourceId = QStringLiteral("nixos.system");
    observation.subject = QStringLiteral("current-system");
    observation.value = QCborValue(value);
    observation.acquiredAt = acquiredAt;
    observation.freshnessUntil = acquiredAt.addSecs(3600);
    observation.provenance = QStringLiteral("test");

    CognitiveEnvelope envelope;
    envelope.messageId = QUuid::createUuid();
    envelope.correlationId = envelope.messageId;
    envelope.originOrgan = QStringLiteral("perceptiond");
    envelope.originNode = QStringLiteral("local");
    envelope.kind = ContributionKind::Observation;
    envelope.wallTime = acquiredAt;
    envelope.confidence = 1.0;
    envelope.privacy = PrivacyClass::Local;
    envelope.payloadCbor = encodeObservation(observation);
    return envelope;
}

QString statusIn(const QByteArray &encoded)
{
    return FabricCodec::decodeMap(encoded).value(QStringLiteral("status")).toString();
}


// A Journal that can be told to fail its paged reads.
//
// Everything else here drives a real Journal, which is right for behaviour but useless for this:
// the one case that matters is the one where reading history does not work, and a healthy Journal
// will not produce it.
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

} // namespace

class TestEpistemicService : public QObject
{
    Q_OBJECT

private:
    QTemporaryDir m_dir;
    QString journalPath() const { return m_dir.filePath(QStringLiteral("j.db")); }
    QString checkpointPath() const { return m_dir.filePath(QStringLiteral("epistemic.cbor")); }

private Q_SLOTS:
    void initTestCase() { QVERIFY(m_dir.isValid()); }

    void itDerivesFromWhatWasAlreadyAccepted()
    {
        Journal journal(journalPath());
        QVERIFY(journal.isOpen());
        const QDateTime now = QDateTime::currentDateTimeUtc();
        QVERIFY(journal.append(observationOf(QStringLiteral("aaa"), now)) > 0);

        EpistemicService service(&journal, checkpointPath());
        QVERIFY2(service.isReady(), qPrintable(service.startupError()));

        // Catching up on start is what makes the projection a function of the whole biography
        // rather than of whatever happens to arrive while it is running.
        QCOMPARE(service.Cursor(), 1u);
        QCOMPARE(
            statusIn(service.KnowledgeOf(QStringLiteral("current-system"))),
            QStringLiteral("observed"));
    }

    // An unfamiliar subject answers rather than fails, across the wire as well as in the library.
    void anUnknownSubjectIsAnsweredNotRefused()
    {
        Journal journal(journalPath());
        QVERIFY(journal.isOpen());
        EpistemicService service(&journal, checkpointPath());
        QVERIFY(service.isReady());

        QCOMPARE(
            statusIn(service.KnowledgeOf(QStringLiteral("nothing-observed-this"))),
            QStringLiteral("unknown"));
    }

    void liveAcceptanceAdvancesTheCursor()
    {
        Journal journal(journalPath());
        QVERIFY(journal.isOpen());
        EpistemicService service(&journal, checkpointPath());
        QVERIFY(service.isReady());
        const qulonglong before = service.Cursor();

        const CognitiveEnvelope envelope =
            observationOf(QStringLiteral("bbb"), QDateTime::currentDateTimeUtc());
        const quint64 sequence = journal.append(envelope);
        QVERIFY(sequence > 0);
        service.admitAccepted(envelope, sequence);

        QCOMPARE(service.Cursor(), sequence);
        QVERIFY(service.Cursor() > before);

        const QVariantMap knowledge =
            FabricCodec::decodeMap(service.KnowledgeOf(QStringLiteral("current-system")));
        QCOMPARE(
            knowledge.value(QStringLiteral("current")).toList().at(0).toMap()
                .value(QStringLiteral("value")).toString(),
            QStringLiteral("bbb"));
    }

    // An announcement already behind the cursor was admitted. Re-admitting is harmless because
    // admission is idempotent, but the cursor must never move backwards - that would claim history
    // had not been read when it had.
    void anAnnouncementBehindTheCursorDoesNotRewindIt()
    {
        Journal journal(journalPath());
        QVERIFY(journal.isOpen());
        EpistemicService service(&journal, checkpointPath());
        QVERIFY(service.isReady());

        const qulonglong cursor = service.Cursor();
        QVERIFY(cursor > 0);

        const auto stale = journal.atSequence(1);
        QVERIFY(stale.has_value());
        service.admitAccepted(*stale, 1);

        QCOMPARE(service.Cursor(), cursor);
    }

    // A gap means something was accepted that never reached us. The projection must be a function
    // of the whole history, not of what happened to be delivered, so it reads the gap rather than
    // skipping over it.
    void aGapInAnnouncementsIsReadRatherThanSkipped()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        QVERIFY(journal.isOpen());
        const QDateTime now = QDateTime::currentDateTimeUtc();

        EpistemicService service(&journal, dir.filePath(QStringLiteral("cp.cbor")));
        QVERIFY(service.isReady());
        QCOMPARE(service.Cursor(), 0u);

        // Two accepted, only the second announced.
        QVERIFY(journal.append(observationOf(QStringLiteral("first"), now.addSecs(-90))) > 0);
        const CognitiveEnvelope second =
            observationOf(QStringLiteral("second"), now.addSecs(-30));
        const quint64 sequence = journal.append(second);
        QCOMPARE(sequence, 2u);

        service.admitAccepted(second, sequence);

        QCOMPARE(service.Cursor(), 2u);
        // The skipped contribution was read, so it appears as the superseded earlier value rather
        // than being absent from history altogether.
        const QVariantMap knowledge =
            FabricCodec::decodeMap(service.KnowledgeOf(QStringLiteral("current-system")));
        QCOMPARE(knowledge.value(QStringLiteral("supersededCount")).toULongLong(), 1u);
    }


    // A gap that cannot be read must not be stepped over.
    //
    // admitAccepted used to call catchUp() and discard the answer. When the read failed the cursor
    // stayed put, which made the announced sequence still look admissible - so it was admitted, the
    // cursor jumped past the unread stretch, and those contributions were skipped permanently. The
    // projection would then be a function of what happened to be delivered, which is exactly what
    // the cursor exists to prevent.
    void anUnreadableGapIsNotSteppedOver()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        UnreadableAfter journal(dir.filePath(QStringLiteral("j.db")));
        QVERIFY(journal.isOpen());
        const QDateTime now = QDateTime::currentDateTimeUtc();

        QVERIFY(journal.append(observationOf(QStringLiteral("first"), now.addSecs(-300))) > 0);

        EpistemicService service(&journal, dir.filePath(QStringLiteral("cp.cbor")));
        QVERIFY(service.isReady());
        QCOMPARE(service.Cursor(), 1u);

        // Four more accepted while nothing was listening, then the fifth announced.
        for (int i = 2; i <= 5; ++i) {
            QVERIFY(
                journal.append(observationOf(
                    QStringLiteral("value-%1").arg(i), now.addSecs(-300 + i * 10)))
                > 0);
        }
        const auto announced = journal.atSequence(5);
        QVERIFY(announced.has_value());

        journal.failReads(true);
        service.admitAccepted(*announced, 5);

        // Behind is recoverable. Ahead of history it never read is not.
        QCOMPARE(service.Cursor(), 1u);
        QVERIFY(!service.LastError().isEmpty());

        journal.failReads(false);
        QVERIFY(service.catchUp());
        QCOMPARE(service.Cursor(), 5u);

        // And the skipped contributions really were admitted, in acquisition order, rather than
        // merely counted: the newest value wins and the rest are filed as superseded.
        const QVariantMap knowledge =
            FabricCodec::decodeMap(service.KnowledgeOf(QStringLiteral("current-system")));
        QCOMPARE(
            knowledge.value(QStringLiteral("current")).toList().at(0).toMap()
                .value(QStringLiteral("value")).toString(),
            QStringLiteral("value-5"));
        QCOMPARE(knowledge.value(QStringLiteral("supersededCount")).toULongLong(), 4u);
    }

    // A checkpoint whose cursor will not parse is not a checkpoint with a cursor of zero. Restoring
    // the projection beside a zeroed cursor would claim nothing had been admitted while holding a
    // projection that says otherwise.
    void aCheckpointWithAnUnparseableCursorIsRefusedWhole()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString journalPath = dir.filePath(QStringLiteral("j.db"));
        const QString checkpoint = dir.filePath(QStringLiteral("cp.cbor"));
        const QDateTime now = QDateTime::currentDateTimeUtc();

        Journal journal(journalPath);
        QVERIFY(journal.isOpen());
        QVERIFY(journal.append(observationOf(QStringLiteral("aaa"), now.addSecs(-60))) > 0);

        {
            EpistemicService warm(&journal, checkpoint);
            QVERIFY(warm.isReady());
            QCOMPARE(warm.Cursor(), 1u);
        }

        QFile file(checkpoint);
        QVERIFY(file.open(QIODevice::ReadOnly));
        QCborMap stored = QCborValue::fromCbor(file.readAll()).toMap();
        file.close();
        stored.insert(QStringLiteral("cursor"), QStringLiteral("not-a-number"));
        QVERIFY(file.open(QIODevice::WriteOnly | QIODevice::Truncate));
        file.write(stored.toCborValue().toCbor());
        file.close();

        // Rebuilt from the Journal rather than resumed from half a checkpoint, and it answers the
        // same as a warm start would.
        EpistemicService rebuilt(&journal, checkpoint);
        QVERIFY2(rebuilt.isReady(), qPrintable(rebuilt.startupError()));
        QCOMPARE(rebuilt.Cursor(), 1u);
        QCOMPARE(
            statusIn(rebuilt.KnowledgeOf(QStringLiteral("current-system"))),
            QStringLiteral("observed"));
    }

    // Supersession grows for the life of the Journal, and Presence reads this projection on every
    // Snapshot. Returning it inline would swap the full-Journal scan P7.3 removed for an
    // ever-growing reply - the same unbounded cost moved somewhere less visible.
    //
    // So the current projection carries only a count, and the history is asked for by the page.
    void whatWasSupersededIsPagedRatherThanReturnedInline()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        QVERIFY(journal.isOpen());
        const QDateTime now = QDateTime::currentDateTimeUtc();

        // Ten real changes, so ten supersessions.
        for (int i = 0; i < 11; ++i) {
            QVERIFY(
                journal.append(observationOf(
                    QStringLiteral("value-%1").arg(i), now.addSecs(-600 + i * 10)))
                > 0);
        }

        EpistemicService service(&journal, dir.filePath(QStringLiteral("cp.cbor")));
        QVERIFY2(service.isReady(), qPrintable(service.startupError()));

        const QVariantMap knowledge =
            FabricCodec::decodeMap(service.KnowledgeOf(QStringLiteral("current-system")));
        QVERIFY2(
            !knowledge.contains(QStringLiteral("superseded")),
            "the current projection must not carry the whole history");
        QCOMPARE(knowledge.value(QStringLiteral("supersededCount")).toULongLong(), 10u);

        // First page, oldest first, and it says there is more.
        const QVariantMap first = FabricCodec::decodeMap(
            service.KnowledgeHistory(QStringLiteral("current-system"), 0, 4));
        QCOMPARE(first.value(QStringLiteral("superseded")).toList().size(), 4);
        QCOMPARE(first.value(QStringLiteral("total")).toULongLong(), 10u);
        QVERIFY(first.value(QStringLiteral("hasMore")).toBool());
        QCOMPARE(
            first.value(QStringLiteral("superseded")).toList().at(0).toMap()
                .value(QStringLiteral("value")).toString(),
            QStringLiteral("value-0"));

        // Walking to the end reports no more, rather than looping on a cursor that never advances.
        const QVariantMap last = FabricCodec::decodeMap(
            service.KnowledgeHistory(QStringLiteral("current-system"), 8, 4));
        QCOMPARE(last.value(QStringLiteral("superseded")).toList().size(), 2);
        QVERIFY(!last.value(QStringLiteral("hasMore")).toBool());

        // An asking-for-everything caller does not get to decide how large the reply is.
        const QVariantMap capped = FabricCodec::decodeMap(
            service.KnowledgeHistory(QStringLiteral("current-system"), 0, 0));
        QCOMPARE(capped.value(QStringLiteral("superseded")).toList().size(), 10);

        // Past the end answers empty rather than failing: there is simply nothing after there.
        const QVariantMap beyond = FabricCodec::decodeMap(
            service.KnowledgeHistory(QStringLiteral("current-system"), 99, 4));
        QVERIFY(beyond.value(QStringLiteral("superseded")).toList().isEmpty());
        QVERIFY(!beyond.value(QStringLiteral("hasMore")).toBool());
    }

    // E8: a checkpoint built before an erasure is discarded, not repaired.
    //
    // It may hold a claim whose evidence has since been redacted, and there is no way to find out
    // which claim that is - establishing it would need exactly the payload that is gone. So the
    // whole checkpoint goes and the projection rebuilds from the Journal, which is always available
    // and always correct.
    void aCheckpointFromBeforeAnErasureIsDiscardedAndRebuilt()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString journalPath = dir.filePath(QStringLiteral("j.db"));
        const QString checkpoint = dir.filePath(QStringLiteral("cp.cbor"));
        const QDateTime now = QDateTime::currentDateTimeUtc();

        Journal journal(journalPath);
        QVERIFY(journal.isOpen());
        const CognitiveEnvelope observation =
            observationOf(QStringLiteral("aaa"), now.addSecs(-60));
        QVERIFY(journal.append(observation) > 0);

        {
            EpistemicService warm(&journal, checkpoint);
            QVERIFY(warm.isReady());
            QCOMPARE(
                statusIn(warm.KnowledgeOf(QStringLiteral("current-system"))),
                QStringLiteral("observed"));
        }
        QVERIFY(QFile::exists(checkpoint));
        QCOMPARE(journal.erasureEpoch(), 0u);

        // Forget the observation the projection was built from.
        QVERIFY(journal.requestErasure(observation.messageId, QStringLiteral("UserRequested")) > 0);
        QVERIFY(journal.applyErasure(observation.messageId));
        QCOMPARE(journal.erasureEpoch(), 1u);

        // The stale checkpoint would have answered "observed" from a claim whose evidence is gone.
        EpistemicService rebuilt(&journal, checkpoint);
        QVERIFY2(rebuilt.isReady(), qPrintable(rebuilt.startupError()));
        QCOMPARE(
            statusIn(rebuilt.KnowledgeOf(QStringLiteral("current-system"))),
            QStringLiteral("unknown"));
    }

    // E9: a projection that cannot answer must not answer "unknown".
    //
    // "unknown" means nobody looked. A projection that failed to catch up has not looked *yet*, and
    // reporting the first for the second collapses the reassuring reading onto the least
    // informative one - in the organ whose whole job is telling those apart.
    void anUnreadyProjectionRefusesRatherThanReportingUnknown()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        UnreadableAfter journal(dir.filePath(QStringLiteral("j.db")));
        QVERIFY(journal.isOpen());
        QVERIFY(journal.append(observationOf(
                    QStringLiteral("aaa"), QDateTime::currentDateTimeUtc().addSecs(-60)))
                > 0);

        journal.failReads(true);
        EpistemicService service(&journal, dir.filePath(QStringLiteral("cp.cbor")));

        QVERIFY2(!service.isReady(), "a projection that could not catch up is not ready");
        QVERIFY(!service.startupError().isEmpty());

        // In process the guard returns an empty payload rather than a D-Bus error, and empty is
        // exactly what a caller must not mistake for an answer: a fabric reply always carries its
        // version envelope, so empty bytes are unambiguously a refusal.
        QVERIFY2(
            service.KnowledgeOf(QStringLiteral("current-system")).isEmpty(),
            "an unready projection must not answer at all");
        QVERIFY(service.Knowledge().isEmpty());

        // And once reading works, it answers properly rather than staying stuck.
        journal.failReads(false);
        QVERIFY(service.catchUp());
    }

    // E13, re-checked with erasure underneath it: checkpoint still equals replay after payloads
    // have been redacted. The property was established in P7.9; erasure is the first thing that
    // changes what a replay produces, so it is the first thing that could break the equality.
    void checkpointStillEqualsReplayAcrossAnErasure()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString journalPath = dir.filePath(QStringLiteral("j.db"));
        const QString checkpoint = dir.filePath(QStringLiteral("cp.cbor"));
        const QDateTime now = QDateTime::currentDateTimeUtc();

        Journal journal(journalPath);
        QVERIFY(journal.isOpen());
        const CognitiveEnvelope first = observationOf(QStringLiteral("aaa"), now.addSecs(-300));
        const CognitiveEnvelope second = observationOf(QStringLiteral("bbb"), now.addSecs(-120));
        QVERIFY(journal.append(first) > 0);
        QVERIFY(journal.append(second) > 0);

        {
            EpistemicService warm(&journal, checkpoint);
            QVERIFY(warm.isReady());
        }

        QVERIFY(journal.requestErasure(first.messageId, QStringLiteral("UserRequested")) > 0);
        QVERIFY(journal.applyErasure(first.messageId));

        // Resumed from a checkpoint that predates the erasure: it must be discarded, and what
        // remains must equal what a projection built from scratch would say.
        EpistemicService resumed(&journal, checkpoint);
        QVERIFY2(resumed.isReady(), qPrintable(resumed.startupError()));

        EpistemicService fromScratch(&journal, dir.filePath(QStringLiteral("other.cbor")));
        QVERIFY(fromScratch.isReady());

        QCOMPARE(
            resumed.KnowledgeOf(QStringLiteral("current-system")),
            fromScratch.KnowledgeOf(QStringLiteral("current-system")));
        QCOMPARE(resumed.Knowledge(), fromScratch.Knowledge());
    }

    // The point of the checkpoint: a restart resumes rather than replaying from zero.
    void aRestartResumesFromTheCheckpoint()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString journal = dir.filePath(QStringLiteral("j.db"));
        const QString checkpoint = dir.filePath(QStringLiteral("cp.cbor"));
        const QDateTime now = QDateTime::currentDateTimeUtc();

        {
            Journal store(journal);
            QVERIFY(store.isOpen());
            // Both acquisitions are in the past. An observation says nothing about a time before
            // it was acquired, so a future-dated one does not speak for the present - dating the
            // later reading forward is what made an earlier version of this test see "stale".
            QVERIFY(store.append(observationOf(QStringLiteral("aaa"), now.addSecs(-120))) > 0);
            QVERIFY(store.append(observationOf(QStringLiteral("bbb"), now.addSecs(-60))) > 0);

            EpistemicService service(&store, checkpoint);
            QVERIFY(service.isReady());
            QCOMPARE(service.Cursor(), 2u);
        }

        QVERIFY(QFile::exists(checkpoint));

        Journal reopened(journal);
        QVERIFY(reopened.isOpen());
        EpistemicService resumed(&reopened, checkpoint);
        QVERIFY2(resumed.isReady(), qPrintable(resumed.startupError()));

        // Same answer as before the restart, and the supersession survived - which is what makes
        // this a resumption rather than a fresh projection that happens to look similar.
        QCOMPARE(resumed.Cursor(), 2u);
        const QVariantMap knowledge =
            FabricCodec::decodeMap(resumed.KnowledgeOf(QStringLiteral("current-system")));
        QCOMPARE(knowledge.value(QStringLiteral("status")).toString(), QStringLiteral("observed"));
        QCOMPARE(knowledge.value(QStringLiteral("supersededCount")).toULongLong(), 1u);
    }

    // Losing the checkpoint costs a replay and nothing else. If it cost knowledge, the checkpoint
    // would be an authority rather than a cache, and the Journal would have a rival.
    void losingTheCheckpointCostsAReplayAndNothingElse()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString journal = dir.filePath(QStringLiteral("j.db"));
        const QString checkpoint = dir.filePath(QStringLiteral("cp.cbor"));
        const QDateTime now = QDateTime::currentDateTimeUtc();

        Journal store(journal);
        QVERIFY(store.isOpen());
        QVERIFY(store.append(observationOf(QStringLiteral("aaa"), now.addSecs(-120))) > 0);
        QVERIFY(store.append(observationOf(QStringLiteral("bbb"), now.addSecs(-60))) > 0);

        EpistemicService warm(&store, checkpoint);
        QVERIFY(warm.isReady());
        const QByteArray withCheckpoint = warm.KnowledgeOf(QStringLiteral("current-system"));

        QVERIFY(QFile::remove(checkpoint));

        EpistemicService cold(&store, checkpoint);
        QVERIFY2(cold.isReady(), qPrintable(cold.startupError()));
        QCOMPARE(cold.Cursor(), warm.Cursor());
        QCOMPARE(cold.KnowledgeOf(QStringLiteral("current-system")), withCheckpoint);
    }

    // A corrupt checkpoint is discarded, not partly trusted, and the replay that follows produces
    // the same answer. Anything else would let a damaged cache quietly become what Mind believes.
    void aCorruptCheckpointIsDiscardedAndRebuilt()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString journal = dir.filePath(QStringLiteral("j.db"));
        const QString checkpoint = dir.filePath(QStringLiteral("cp.cbor"));
        const QDateTime now = QDateTime::currentDateTimeUtc();

        Journal store(journal);
        QVERIFY(store.isOpen());
        QVERIFY(store.append(observationOf(QStringLiteral("aaa"), now)) > 0);

        EpistemicService warm(&store, checkpoint);
        QVERIFY(warm.isReady());
        const QByteArray expected = warm.KnowledgeOf(QStringLiteral("current-system"));

        QFile file(checkpoint);
        QVERIFY(file.open(QIODevice::WriteOnly));
        file.write("not a checkpoint at all");
        file.close();

        EpistemicService rebuilt(&store, checkpoint);
        QVERIFY2(rebuilt.isReady(), qPrintable(rebuilt.startupError()));
        QCOMPARE(rebuilt.Cursor(), warm.Cursor());
        QCOMPARE(rebuilt.KnowledgeOf(QStringLiteral("current-system")), expected);
    }
};

QTEST_MAIN(TestEpistemicService)
#include "tst_epistemic_service.moc"
