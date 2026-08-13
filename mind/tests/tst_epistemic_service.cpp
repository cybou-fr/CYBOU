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
    return QCborValue::fromCbor(encoded).toMap().value(QStringLiteral("status")).toString();
}

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

        const QCborMap knowledge =
            QCborValue::fromCbor(service.KnowledgeOf(QStringLiteral("current-system"))).toMap();
        QCOMPARE(
            knowledge.value(QStringLiteral("current")).toArray().at(0).toMap()
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
        const QCborMap knowledge =
            QCborValue::fromCbor(service.KnowledgeOf(QStringLiteral("current-system"))).toMap();
        QCOMPARE(knowledge.value(QStringLiteral("superseded")).toArray().size(), 1);
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
        const QCborMap knowledge =
            QCborValue::fromCbor(resumed.KnowledgeOf(QStringLiteral("current-system"))).toMap();
        QCOMPARE(knowledge.value(QStringLiteral("status")).toString(), QStringLiteral("observed"));
        QCOMPARE(knowledge.value(QStringLiteral("superseded")).toArray().size(), 1);
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
