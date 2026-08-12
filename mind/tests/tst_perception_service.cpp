// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT
//
// The first perception adapter: what it contributes, and what it refuses to.
//
// The checkpoint's P7.1 exit gate asks that duplicate acquisition be idempotent by declared
// semantics and that source unavailability have a typed result. Both are properties of this
// adapter's behaviour against a real Journal, which is what this exercises.

#include "PerceptionService.h"

#include "cybou/protocol/Observation.h"
#include "cybou/storage/Journal.h"

#include <QCborMap>
#include <QCborValue>
#include <QDir>
#include <QFile>
#include <QTemporaryDir>
#include <QTest>

using namespace cybou;

class TestPerceptionService : public QObject
{
    Q_OBJECT

private:
    static QString stageSystem(const QTemporaryDir &dir, const QString &build)
    {
        const QString store = dir.filePath(build);
        QDir().mkpath(store);
        const QString link = dir.filePath(QStringLiteral("current-system"));
        QFile::remove(link);
        QFile::link(store, link);
        return link;
    }

    static QList<CognitiveEnvelope> observationsIn(const Journal &journal)
    {
        QList<CognitiveEnvelope> found;
        for (const CognitiveEnvelope &e : journal.recent(0)) {
            if (decodeObservation(e.payloadCbor).has_value()) {
                found.append(e);
            }
        }
        return found;
    }

    static QList<CognitiveEnvelope> transitionsIn(const Journal &journal)
    {
        QList<CognitiveEnvelope> found;
        for (const CognitiveEnvelope &e : journal.recent(0)) {
            const QCborValue payload = QCborValue::fromCbor(e.payloadCbor);
            if (payload.isMap()
                && payload.toMap().value(QStringLiteral("@type")).toString()
                    == QStringLiteral("cybou.acquisition-state.v1")) {
                found.append(e);
            }
        }
        return found;
    }

private Q_SLOTS:
    void oneReadingBecomesOneObservation()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        QVERIFY(journal.isOpen());

        const QString link = stageSystem(dir, QStringLiteral("aaa-nixos-system-host-26.05"));
        PerceptionService service(&journal, SystemGenerationSource(link));
        service.acquireOnce();

        const QList<CognitiveEnvelope> observations = observationsIn(journal);
        QCOMPARE(observations.size(), 1);

        // The producer is this organ; what was observed is named separately. Conflating them is
        // what ADR-0027 forbids, and Event1 binds originOrgan to the executable precisely so this
        // claim is checkable rather than asserted.
        QCOMPARE(observations.first().originOrgan, QStringLiteral("perceptiond"));

        const auto observation = decodeObservation(observations.first().payloadCbor);
        QVERIFY(observation.has_value());
        QCOMPARE(observation->sourceId, QStringLiteral("nixos.system"));
        QCOMPARE(
            observation->value.toString(),
            QStringLiteral("aaa-nixos-system-host-26.05"));
        QVERIFY(!observation->provenance.isEmpty());
    }

    // Polling an unchanged system must cost nothing durable, or a source read every ten seconds
    // would fill the biography with restatements of one fact.
    void pollingAnUnchangedSystemAddsNothing()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        QVERIFY(journal.isOpen());

        const QString link = stageSystem(dir, QStringLiteral("bbb-nixos-system-host-26.05"));
        PerceptionService service(&journal, SystemGenerationSource(link));

        service.acquireOnce();
        const quint64 afterFirst = journal.count();
        QVERIFY(afterFirst > 0);

        for (int i = 0; i < 5; ++i) {
            service.acquireOnce();
        }

        // Identity includes the acquisition instant, so polls do *not* collapse on their own - an
        // earlier version of this adapter wrote one contribution per poll for exactly that reason,
        // and this test is what caught it. What holds them back is the freshness horizon: while the
        // previous contribution still speaks, an unchanged reading adds nothing.
        QCOMPARE(observationsIn(journal).size(), 1);
        QCOMPARE(journal.count(), afterFirst);
        QVERIFY(service.LastError().isEmpty());
    }

    // An unchanged fact is re-affirmed once its declared horizon lapses, and not before.
    //
    // Contributing on every poll restates one fact thousands of times a day. Contributing only on
    // change is the opposite error: within its horizon the previous observation speaks for the
    // present, but once that lapses nothing does, and a projection would have to call the fact stale
    // forever while the adapter sat watching it be true.
    void anUnchangedFactIsReaffirmedOncePerHorizon()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        QVERIFY(journal.isOpen());

        const QString link = stageSystem(dir, QStringLiteral("fff-nixos-system-host-26.05"));
        // One second, so the horizon can lapse inside a test rather than in five minutes.
        PerceptionService service(&journal, SystemGenerationSource(link, 1));

        service.acquireOnce();
        QCOMPARE(observationsIn(journal).size(), 1);

        // Inside the horizon: read repeatedly, contribute nothing.
        for (int i = 0; i < 4; ++i) {
            service.acquireOnce();
        }
        QCOMPARE(observationsIn(journal).size(), 1);

        QTest::qWait(1100);

        // The horizon has lapsed and the fact is still true, so it is said again - once.
        service.acquireOnce();
        QCOMPARE(observationsIn(journal).size(), 2);

        service.acquireOnce();
        QCOMPARE(observationsIn(journal).size(), 2);

        // Both say the same thing about the world; they differ in when it was checked, which is the
        // point of re-affirming rather than leaving the first to age out.
        const QList<CognitiveEnvelope> observations = observationsIn(journal);
        const auto older = decodeObservation(observations.at(0).payloadCbor);
        const auto newer = decodeObservation(observations.at(1).payloadCbor);
        QVERIFY(older.has_value() && newer.has_value());
        QCOMPARE(older->value, newer->value);
        QVERIFY(older->acquiredAt != newer->acquiredAt);
    }

    // A rebuilt system is a new fact, and must be recorded as one - otherwise nothing downstream
    // could ever supersede the earlier reading.
    void aRebuiltSystemIsANewObservation()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        QVERIFY(journal.isOpen());

        QString link = stageSystem(dir, QStringLiteral("ccc-nixos-system-host-26.05"));
        PerceptionService service(&journal, SystemGenerationSource(link));
        service.acquireOnce();
        QCOMPARE(observationsIn(journal).size(), 1);

        stageSystem(dir, QStringLiteral("ddd-nixos-system-host-26.05"));
        service.acquireOnce();

        const QList<CognitiveEnvelope> observations = observationsIn(journal);
        QCOMPARE(observations.size(), 2);

        QStringList values;
        for (const CognitiveEnvelope &e : observations) {
            values.append(decodeObservation(e.payloadCbor)->value.toString());
        }
        QVERIFY(values.contains(QStringLiteral("ccc-nixos-system-host-26.05")));
        QVERIFY(values.contains(QStringLiteral("ddd-nixos-system-host-26.05")));
    }

    // An unreadable source contributes no observation at all. A failure to observe is not an
    // observation of nothing, and the adapter must not be able to put its own failure into the
    // biography as a fact about the world.
    void anUnreadableSourceContributesNoObservation()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        QVERIFY(journal.isOpen());

        PerceptionService service(
            &journal, SystemGenerationSource(dir.filePath(QStringLiteral("absent"))));
        service.acquireOnce();

        QCOMPARE(observationsIn(journal).size(), 0);
        QVERIFY(!service.LastError().isEmpty());

        // The transition into unreadable is durable, because becoming unreadable is itself a fact.
        QCOMPARE(transitionsIn(journal).size(), 1);
    }

    // Only the change is durable. Repeating an unchanged failure every poll would write thousands
    // of contributions recording that nothing happened.
    void repeatedFailureRecordsOnlyTheTransition()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        QVERIFY(journal.isOpen());

        PerceptionService service(
            &journal, SystemGenerationSource(dir.filePath(QStringLiteral("absent"))));
        for (int i = 0; i < 6; ++i) {
            service.acquireOnce();
        }
        QCOMPARE(transitionsIn(journal).size(), 1);
        QCOMPARE(observationsIn(journal).size(), 0);

        // Recovery is the opposite transition, and is equally worth one record.
        stageSystem(dir, QStringLiteral("eee-nixos-system-host-26.05"));
        PerceptionService recovered(
            &journal,
            SystemGenerationSource(dir.filePath(QStringLiteral("current-system"))));
        // A fresh instance has observed nothing, so its first result is a change by definition -
        // assuming a starting state would either invent a transition or suppress a real one.
        recovered.acquireOnce();
        QCOMPARE(transitionsIn(journal).size(), 2);
        QCOMPARE(observationsIn(journal).size(), 1);
    }

    // A transition describes this adapter's ability to observe, not the subject it observes, so it
    // must never be readable as an ObservationV1. Otherwise a failure to look would present itself
    // to the epistemic projection as something looked at.
    void aTransitionIsNotAnObservation()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        QVERIFY(journal.isOpen());

        PerceptionService service(
            &journal, SystemGenerationSource(dir.filePath(QStringLiteral("absent"))));
        service.acquireOnce();

        const QList<CognitiveEnvelope> transitions = transitionsIn(journal);
        QCOMPARE(transitions.size(), 1);
        QVERIFY(!decodeObservation(transitions.first().payloadCbor).has_value());

        const QCborMap payload = QCborValue::fromCbor(transitions.first().payloadCbor).toMap();
        QCOMPARE(
            payload.value(QStringLiteral("sourceId")).toString(),
            QStringLiteral("nixos.system"));
        QCOMPARE(
            payload.value(QStringLiteral("status")).toString(),
            QStringLiteral("source-unavailable"));
        QVERIFY(!payload.value(QStringLiteral("since")).toString().isEmpty());
    }
};

QTEST_MAIN(TestPerceptionService)
#include "tst_perception_service.moc"
