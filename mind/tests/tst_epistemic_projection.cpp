// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT
//
// The epistemic projection: what Mind believes, and how it says it does not know.
//
// The distinctions carry the weight here. Never having looked, having looked and the answer having
// aged, and two sources disagreeing are three different states, and presenting any of them as
// another is exactly the failure ADR-0025 calls perception being treated as truth.

#include "cybou/epistemic/EpistemicProjection.h"

#include <QCborArray>
#include <QCborMap>
#include <QSet>
#include <QTest>

using namespace cybou;

namespace {

const QDateTime kNoon = QDateTime(QDate(2026, 8, 13), QTime(12, 0), Qt::UTC);

CognitiveEnvelope observationOf(
    const QString &sourceId,
    const QString &subject,
    const QCborValue &value,
    const QDateTime &acquiredAt,
    int freshnessSeconds = 300)
{
    ObservationV1 observation;
    observation.sourceId = sourceId;
    observation.subject = subject;
    observation.value = value;
    observation.acquiredAt = acquiredAt;
    observation.freshnessUntil = acquiredAt.addSecs(freshnessSeconds);
    observation.provenance = QStringLiteral("test");

    CognitiveEnvelope envelope;
    envelope.messageId = QUuid::createUuid();
    envelope.correlationId = envelope.messageId;
    envelope.originOrgan = QStringLiteral("perceptiond");
    envelope.kind = ContributionKind::Observation;
    envelope.wallTime = acquiredAt;
    envelope.confidence = 1.0;
    envelope.privacy = PrivacyClass::Local;
    envelope.payloadCbor = encodeObservation(observation);
    return envelope;
}

} // namespace

class TestEpistemicProjection : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    // Never looked is its own answer, and must not be reachable by accident from any other.
    void whatWasNeverObservedIsUnknown()
    {
        EpistemicProjection projection;

        const SubjectKnowledge knowledge =
            projection.knowledgeOf(QStringLiteral("current-system"), kNoon);
        QCOMPARE(knowledge.status, EpistemicStatus::Unknown);
        QVERIFY(knowledge.current.isEmpty());
        QVERIFY(knowledge.superseded.isEmpty());

        // Asking about an unfamiliar subject answers rather than fails: not knowing is a normal
        // state of a mind, not an error condition.
        QCOMPARE(
            projection.knowledgeOf(QStringLiteral("never-heard-of"), kNoon).status,
            EpistemicStatus::Unknown);
    }

    void aFreshObservationIsObserved()
    {
        EpistemicProjection projection;
        QVERIFY(projection.admit(observationOf(
            QStringLiteral("nixos.system"),
            QStringLiteral("current-system"),
            QCborValue(QStringLiteral("aaa")),
            kNoon)));

        const SubjectKnowledge knowledge =
            projection.knowledgeOf(QStringLiteral("current-system"), kNoon.addSecs(60));
        QCOMPARE(knowledge.status, EpistemicStatus::Observed);
        QCOMPARE(knowledge.current.size(), 1);
        QCOMPARE(knowledge.current.first().value.toString(), QStringLiteral("aaa"));
    }

    // Ageing out does not erase what was learned. "Was this, last checked then" is more useful than
    // silence, and discarding it would lose evidence that was actually gathered.
    void anAgedObservationBecomesStaleAndKeepsItsValue()
    {
        EpistemicProjection projection;
        projection.admit(observationOf(
            QStringLiteral("nixos.system"),
            QStringLiteral("current-system"),
            QCborValue(QStringLiteral("aaa")),
            kNoon,
            300));

        QCOMPARE(
            projection.knowledgeOf(QStringLiteral("current-system"), kNoon.addSecs(299)).status,
            EpistemicStatus::Observed);

        const SubjectKnowledge stale =
            projection.knowledgeOf(QStringLiteral("current-system"), kNoon.addSecs(301));
        QCOMPARE(stale.status, EpistemicStatus::Stale);
        QCOMPARE(stale.current.size(), 1);
        QCOMPARE(stale.current.first().value.toString(), QStringLiteral("aaa"));

        // Stale is not unknown. Somebody looked; the answer has aged.
        QVERIFY(stale.status != EpistemicStatus::Unknown);
    }

    // A source replacing its own earlier reading is supersession, not disagreement. This is the
    // case ADR-0027 chose the first source for: the system is rebuilt while an earlier observation
    // still claims to be current.
    // A source that says two different things about one instant of acquisition has contradicted
    // itself, and that is not the same event as changing its mind.
    //
    // The projection used to let the later arrival replace the earlier one, so an arrival order
    // that carries no meaning decided the answer and the contradiction disappeared. ObservationV1
    // gives these distinct identities precisely so the Journal keeps both; keeping only one here
    // threw away what the Journal was careful to preserve.
    void oneSourceContradictingItselfAtOneInstantIsDisputed()
    {
        EpistemicProjection projection;
        const QDateTime now = QDateTime::currentDateTimeUtc();
        const QDateTime acquired = now.addSecs(-30);

        QVERIFY(projection.admit(observationOf(
            QStringLiteral("sensor"), QStringLiteral("temperature"), QCborValue(20), acquired)));
        QVERIFY(projection.admit(observationOf(
            QStringLiteral("sensor"), QStringLiteral("temperature"), QCborValue(25), acquired)));

        const SubjectKnowledge knowledge =
            projection.knowledgeOf(QStringLiteral("temperature"), now);

        // One source is enough. Requiring two would let a single unreliable source look certain,
        // which is the opposite of what disputing exists for.
        QCOMPARE(knowledge.status, EpistemicStatus::Disputed);

        // Both readings are offered. A dispute a caller cannot see both sides of is an unexplained
        // refusal rather than an answer.
        QCOMPARE(knowledge.current.size(), 2);
        QSet<int> values;
        for (const EpistemicClaim &claim : knowledge.current) {
            QCOMPARE(claim.status, EpistemicStatus::Disputed);
            values.insert(claim.value.toInteger());
        }
        QVERIFY(values.contains(20));
        QVERIFY(values.contains(25));

        // And neither was filed as superseded: nothing happened between them to supersede.
        QVERIFY(knowledge.superseded.isEmpty());
    }

    // checkpoint == replay, asserted over a whole population rather than one case at a time.
    //
    // The dispute-across-a-checkpoint defect existed because every individual behaviour had a test
    // and the *property* had none: each new rule was checked live, and whether it survived being
    // written down and read back was checked only for the rules someone thought to pair with a
    // restart. This drives a deterministic mix of every rule the projection has - re-affirmation,
    // supersession, late arrival, cross-source disagreement, self-contradiction, staleness - and
    // asserts a restored projection answers identically at several instants.
    //
    // Any future rule carried in state that snapshot() does not write will fail here without anyone
    // remembering to pair it with a checkpoint.
    void aRestoredProjectionAnswersIdenticallyToTheOneItReplaces()
    {
        EpistemicProjection live;
        const QDateTime base = QDateTime::currentDateTimeUtc().addSecs(-3600);

        // Deterministic by construction: the same index always produces the same observation, so a
        // failure here is reproducible rather than a story about one unlucky run.
        for (int i = 0; i < 60; ++i) {
            const QString subject = QStringLiteral("subject-%1").arg(i % 6);
            const QString source = QStringLiteral("source-%1").arg(i % 3);
            const int horizon = 60 + (i % 4) * 120;

            // Every fifth pair shares an acquisition instant with a different value, which is the
            // self-contradiction case; the rest advance in time, and every third repeats its value
            // so re-affirmation is exercised too.
            const QDateTime acquired = base.addSecs((i / 5) * 45);
            const QCborValue value = QCborValue(
                i % 3 == 0 ? QStringLiteral("steady") : QStringLiteral("value-%1").arg(i));

            live.admit(observationOf(source, subject, value, acquired, horizon));

        }

        // Self-contradiction needs its own subjects, and getting here took two corrections the
        // guard below forced. Varying subject by i%6 and source by i%3 inside five-entry time
        // blocks makes two claims sharing a source, a subject *and* an instant arithmetically
        // impossible. Injecting a contradicting claim alongside the loop fixed that and still
        // produced no dispute, because a later acquisition for the same source and subject
        // superseded the pair before the end - correctly, since the dispute was about one instant.
        //
        // So a contested subject is one nothing else touches: two values, one instant, one source.
        for (int i = 0; i < 3; ++i) {
            const QString subject = QStringLiteral("contested-%1").arg(i);
            const QDateTime acquired = base.addSecs(30);
            live.admit(observationOf(
                QStringLiteral("source-0"), subject, QCborValue(QStringLiteral("a")), acquired,
                600));
            live.admit(observationOf(
                QStringLiteral("source-0"), subject, QCborValue(QStringLiteral("b")), acquired,
                600));
        }

        EpistemicProjection restored;
        QString error;
        QVERIFY2(restored.restore(live.snapshot(), &error), qPrintable(error));

        // Asked at several instants, because status is derived from when the question is put and a
        // checkpoint that agreed only at one moment would still be wrong at every other.
        for (const int offset : {0, 120, 600, 3600, 7200}) {
            const QDateTime at = base.addSecs(offset);
            const QList<SubjectKnowledge> before = live.knowledgeAt(at);
            const QList<SubjectKnowledge> after = restored.knowledgeAt(at);
            QCOMPARE(after.size(), before.size());

            for (int i = 0; i < before.size(); ++i) {
                QCOMPARE(after.at(i).subject, before.at(i).subject);
                QCOMPARE(after.at(i).status, before.at(i).status);
                QCOMPARE(after.at(i).current.size(), before.at(i).current.size());
                QCOMPARE(after.at(i).superseded.size(), before.at(i).superseded.size());

                for (int c = 0; c < before.at(i).current.size(); ++c) {
                    const EpistemicClaim &was = before.at(i).current.at(c);
                    const EpistemicClaim &now = after.at(i).current.at(c);
                    QCOMPARE(now.contributionId, was.contributionId);
                    QCOMPARE(now.sourceId, was.sourceId);
                    QCOMPARE(now.provenance, was.provenance);
                    QCOMPARE(now.value, was.value);
                    QCOMPARE(now.acquiredAt, was.acquiredAt);
                    QCOMPARE(now.freshUntil, was.freshUntil);
                    QCOMPARE(now.status, was.status);
                }
            }
        }

        // And the population actually exercised what it claims to: a checkpoint that agreed about
        // nothing interesting would pass everything above.
        int disputed = 0;
        int superseded = 0;
        for (const SubjectKnowledge &knowledge : restored.knowledgeAt(base.addSecs(120))) {
            if (knowledge.status == EpistemicStatus::Disputed) {
                ++disputed;
            }
            superseded += knowledge.superseded.size();
        }
        QVERIFY2(disputed > 0, "the fixture never produced a dispute");
        QVERIFY2(superseded > 0, "the fixture never produced a supersession");
    }

    // A dispute must survive a checkpoint, because a checkpoint may never be weaker than the replay
    // it stands in for.
    //
    // It did not. Self-contradiction was carried in side tables that snapshot() never wrote, so a
    // dispute lasted until the next restart and then quietly became agreement - checkpoint stopped
    // equalling replay in exactly the case the projection exists to report. There were tests for
    // the dispute and tests for the checkpoint, and none for the two together.
    void aSelfContradictionSurvivesACheckpoint()
    {
        EpistemicProjection projection;
        const QDateTime now = QDateTime::currentDateTimeUtc();
        const QDateTime acquired = now.addSecs(-30);

        projection.admit(observationOf(
            QStringLiteral("sensor"), QStringLiteral("temperature"), QCborValue(20), acquired));
        projection.admit(observationOf(
            QStringLiteral("sensor"), QStringLiteral("temperature"), QCborValue(25), acquired));
        QCOMPARE(
            projection.knowledgeOf(QStringLiteral("temperature"), now).status,
            EpistemicStatus::Disputed);

        EpistemicProjection restored;
        QString error;
        QVERIFY2(restored.restore(projection.snapshot(), &error), qPrintable(error));

        const SubjectKnowledge knowledge =
            restored.knowledgeOf(QStringLiteral("temperature"), now);
        QCOMPARE(knowledge.status, EpistemicStatus::Disputed);
        QCOMPARE(knowledge.current.size(), 2);

        QSet<int> values;
        for (const EpistemicClaim &claim : knowledge.current) {
            values.insert(claim.value.toInteger());
        }
        QVERIFY(values.contains(20));
        QVERIFY(values.contains(25));
    }

    // Two readings of one instant may declare different freshness horizons, and each is aged by its
    // own. Once the shorter one lapses the dispute is over: a claim that no longer speaks cannot
    // argue with one that does, which is the same rule that stops a lapsed source disputing a
    // fresh one.
    void aLapsedHalfOfASelfContradictionStopsDisputing()
    {
        EpistemicProjection projection;
        const QDateTime now = QDateTime::currentDateTimeUtc();
        const QDateTime acquired = now.addSecs(-120);

        // Same source, same instant, different values - and deliberately different horizons.
        projection.admit(observationOf(
            QStringLiteral("sensor"), QStringLiteral("temperature"), QCborValue(20), acquired, 60));
        projection.admit(observationOf(
            QStringLiteral("sensor"), QStringLiteral("temperature"), QCborValue(25), acquired, 600));

        // At acquisition both spoke, so this was a dispute.
        QCOMPARE(
            projection.knowledgeOf(QStringLiteral("temperature"), acquired.addSecs(30)).status,
            EpistemicStatus::Disputed);

        // Now only the longer-lived reading still speaks, and it speaks alone.
        const SubjectKnowledge knowledge =
            projection.knowledgeOf(QStringLiteral("temperature"), now);
        QCOMPARE(knowledge.status, EpistemicStatus::Observed);
        QCOMPARE(knowledge.current.size(), 1);
        QCOMPARE(knowledge.current.first().value.toInteger(), 25);
    }

    // The dispute is about one instant, so evidence about a later one settles it. Otherwise a
    // source that stuttered once would be permanently distrusted, which no amount of subsequent
    // good behaviour could repair.
    void aLaterAcquisitionResolvesASourcesSelfContradiction()
    {
        EpistemicProjection projection;
        const QDateTime now = QDateTime::currentDateTimeUtc();
        const QDateTime acquired = now.addSecs(-60);

        projection.admit(observationOf(
            QStringLiteral("sensor"), QStringLiteral("temperature"), QCborValue(20), acquired));
        projection.admit(observationOf(
            QStringLiteral("sensor"), QStringLiteral("temperature"), QCborValue(25), acquired));
        QCOMPARE(
            projection.knowledgeOf(QStringLiteral("temperature"), now).status,
            EpistemicStatus::Disputed);

        projection.admit(observationOf(
            QStringLiteral("sensor"),
            QStringLiteral("temperature"),
            QCborValue(22),
            now.addSecs(-10)));

        const SubjectKnowledge knowledge =
            projection.knowledgeOf(QStringLiteral("temperature"), now);
        QCOMPARE(knowledge.status, EpistemicStatus::Observed);
        QCOMPARE(knowledge.current.size(), 1);
        QCOMPARE(knowledge.current.first().value.toInteger(), 22);
    }

    // A claim must be able to name the contribution it came from. Without it the projection asserts
    // things on its own authority: a reader sees what is believed and cannot reach the evidence,
    // and "perception is not truth" stops being something anyone can check.
    void everyClaimNamesTheContributionAndProvenanceBehindIt()
    {
        EpistemicProjection projection;
        const QDateTime now = QDateTime::currentDateTimeUtc();
        const CognitiveEnvelope envelope = observationOf(
            QStringLiteral("nixos.system"),
            QStringLiteral("current-system"),
            QCborValue(QStringLiteral("abc")),
            now.addSecs(-10));
        QVERIFY(projection.admit(envelope));

        const SubjectKnowledge knowledge =
            projection.knowledgeOf(QStringLiteral("current-system"), now);
        QCOMPARE(knowledge.current.size(), 1);
        QCOMPARE(knowledge.current.first().contributionId, envelope.messageId);
        QCOMPARE(knowledge.current.first().provenance, QStringLiteral("test"));
    }

    // And it must survive a checkpoint, because a restored projection may never be weaker than the
    // replay it stands in for.
    void evidenceSurvivesACheckpoint()
    {
        EpistemicProjection projection;
        const QDateTime now = QDateTime::currentDateTimeUtc();
        const CognitiveEnvelope envelope = observationOf(
            QStringLiteral("nixos.system"),
            QStringLiteral("current-system"),
            QCborValue(QStringLiteral("abc")),
            now.addSecs(-10));
        QVERIFY(projection.admit(envelope));

        EpistemicProjection restored;
        QString error;
        QVERIFY2(restored.restore(projection.snapshot(), &error), qPrintable(error));

        const SubjectKnowledge knowledge =
            restored.knowledgeOf(QStringLiteral("current-system"), now);
        QCOMPARE(knowledge.current.size(), 1);
        QCOMPARE(knowledge.current.first().contributionId, envelope.messageId);
        QCOMPARE(knowledge.current.first().provenance, QStringLiteral("test"));
    }

    void oneSourceChangingItsMindSupersedes()
    {
        EpistemicProjection projection;
        projection.admit(observationOf(
            QStringLiteral("nixos.system"),
            QStringLiteral("current-system"),
            QCborValue(QStringLiteral("aaa")),
            kNoon));
        projection.admit(observationOf(
            QStringLiteral("nixos.system"),
            QStringLiteral("current-system"),
            QCborValue(QStringLiteral("bbb")),
            kNoon.addSecs(60)));

        const SubjectKnowledge knowledge =
            projection.knowledgeOf(QStringLiteral("current-system"), kNoon.addSecs(90));
        QCOMPARE(knowledge.status, EpistemicStatus::Observed);
        QCOMPARE(knowledge.current.size(), 1);
        QCOMPARE(knowledge.current.first().value.toString(), QStringLiteral("bbb"));

        // The earlier reading is kept as history rather than deleted: it is how a reader sees that
        // something changed, rather than only that it is now different.
        QCOMPARE(knowledge.superseded.size(), 1);
        QCOMPARE(knowledge.superseded.first().value.toString(), QStringLiteral("aaa"));
        QCOMPARE(knowledge.superseded.first().status, EpistemicStatus::Superseded);
    }

    // Re-affirmation is not a change. The adapter restates an unchanged fact once per freshness
    // horizon, and filing each restatement as a supersession would make a still world look busy.
    void restatingTheSameValueIsNotASupersession()
    {
        EpistemicProjection projection;
        for (int i = 0; i < 4; ++i) {
            projection.admit(observationOf(
                QStringLiteral("nixos.system"),
                QStringLiteral("current-system"),
                QCborValue(QStringLiteral("aaa")),
                kNoon.addSecs(i * 300)));
        }

        const SubjectKnowledge knowledge =
            projection.knowledgeOf(QStringLiteral("current-system"), kNoon.addSecs(1000));
        QCOMPARE(knowledge.status, EpistemicStatus::Observed);
        QVERIFY(knowledge.superseded.isEmpty());
        QCOMPARE(projection.observationCount(), 4);
    }

    // Two sources currently claiming different things is a contradiction, and the projection must
    // surface it rather than pick. Choosing a winner by recency or by source would be inventing
    // knowledge it does not have.
    void twoSourcesDisagreeingIsDisputed()
    {
        EpistemicProjection projection;
        projection.admit(observationOf(
            QStringLiteral("nixos.system"),
            QStringLiteral("current-system"),
            QCborValue(QStringLiteral("aaa")),
            kNoon));
        projection.admit(observationOf(
            QStringLiteral("nixos.profile"),
            QStringLiteral("current-system"),
            QCborValue(QStringLiteral("bbb")),
            kNoon.addSecs(10)));

        const SubjectKnowledge knowledge =
            projection.knowledgeOf(QStringLiteral("current-system"), kNoon.addSecs(20));
        QCOMPARE(knowledge.status, EpistemicStatus::Disputed);
        QCOMPARE(knowledge.current.size(), 2);
        for (const EpistemicClaim &claim : knowledge.current) {
            QCOMPARE(claim.status, EpistemicStatus::Disputed);
        }

        // Neither was quietly discarded, and neither was promoted.
        QStringList values;
        for (const EpistemicClaim &claim : knowledge.current) {
            values.append(claim.value.toString());
        }
        QVERIFY(values.contains(QStringLiteral("aaa")));
        QVERIFY(values.contains(QStringLiteral("bbb")));
    }

    // A dispute needs both claims to currently speak. Once one has aged out it is the past, and the
    // past does not argue with the present.
    void aLapsedClaimDoesNotDispute()
    {
        EpistemicProjection projection;
        projection.admit(observationOf(
            QStringLiteral("nixos.system"),
            QStringLiteral("current-system"),
            QCborValue(QStringLiteral("aaa")),
            kNoon,
            60));
        projection.admit(observationOf(
            QStringLiteral("nixos.profile"),
            QStringLiteral("current-system"),
            QCborValue(QStringLiteral("bbb")),
            kNoon.addSecs(120),
            300));

        // At 130 s the first has lapsed and only the second speaks.
        const SubjectKnowledge knowledge =
            projection.knowledgeOf(QStringLiteral("current-system"), kNoon.addSecs(130));
        QCOMPARE(knowledge.status, EpistemicStatus::Observed);
        QCOMPARE(knowledge.current.size(), 1);
        QCOMPARE(knowledge.current.first().value.toString(), QStringLiteral("bbb"));
    }

    // Replay and restart deliver contributions in Journal order, which is acceptance order, not
    // acquisition order. An older reading arriving late must not unseat a newer one.
    void anOlderReadingArrivingLateDoesNotUnseatANewerOne()
    {
        EpistemicProjection projection;
        projection.admit(observationOf(
            QStringLiteral("nixos.system"),
            QStringLiteral("current-system"),
            QCborValue(QStringLiteral("newer")),
            kNoon.addSecs(600)));
        projection.admit(observationOf(
            QStringLiteral("nixos.system"),
            QStringLiteral("current-system"),
            QCborValue(QStringLiteral("older")),
            kNoon));

        const SubjectKnowledge knowledge =
            projection.knowledgeOf(QStringLiteral("current-system"), kNoon.addSecs(700));
        QCOMPARE(knowledge.current.size(), 1);
        QCOMPARE(knowledge.current.first().value.toString(), QStringLiteral("newer"));
    }

    // The projection derives from observations only. Everything else in the biography - including
    // the acquisition-state records the adapter itself writes - must pass through untouched, or a
    // failure to look would present itself as something looked at.
    void nonObservationsAreNotAdmitted()
    {
        EpistemicProjection projection;

        QCborMap transition;
        transition.insert(QStringLiteral("@type"), QStringLiteral("cybou.acquisition-state.v1"));
        transition.insert(QStringLiteral("sourceId"), QStringLiteral("nixos.system"));
        transition.insert(QStringLiteral("status"), QStringLiteral("source-unavailable"));

        CognitiveEnvelope envelope;
        envelope.messageId = QUuid::createUuid();
        envelope.correlationId = envelope.messageId;
        envelope.originOrgan = QStringLiteral("perceptiond");
        envelope.kind = ContributionKind::Observation;
        envelope.wallTime = kNoon;
        envelope.confidence = 1.0;
        envelope.privacy = PrivacyClass::Local;
        envelope.payloadCbor = transition.toCborValue().toCbor();

        QVERIFY(!projection.admit(envelope));

        QCborMap predictorOutcome;
        predictorOutcome.insert(QStringLiteral("subject"), QStringLiteral("current-system"));
        predictorOutcome.insert(QStringLiteral("actual"), 1.0);
        envelope.payloadCbor = predictorOutcome.toCborValue().toCbor();
        QVERIFY(!projection.admit(envelope));

        // A source that went unreadable leaves what it last said unchanged: no new evidence is not
        // counter-evidence.
        QCOMPARE(
            projection.knowledgeOf(QStringLiteral("current-system"), kNoon).status,
            EpistemicStatus::Unknown);
        QCOMPARE(projection.observationCount(), 0);
    }

    // A checkpoint must be indistinguishable from the replay it stands in for. If it is not, it is
    // not a cache of the Journal - it is a second biography, and nothing would say which one is
    // right.
    void aCheckpointAnswersExactlyAsAReplayWould()
    {
        const QList<CognitiveEnvelope> history{
            observationOf(
                QStringLiteral("nixos.system"),
                QStringLiteral("current-system"),
                QCborValue(QStringLiteral("aaa")),
                kNoon),
            observationOf(
                QStringLiteral("nixos.system"),
                QStringLiteral("current-system"),
                QCborValue(QStringLiteral("bbb")),
                kNoon.addSecs(60)),
            observationOf(
                QStringLiteral("nixos.profile"),
                QStringLiteral("current-system"),
                QCborValue(QStringLiteral("ccc")),
                kNoon.addSecs(70)),
            observationOf(
                QStringLiteral("nixos.system"),
                QStringLiteral("kernel"),
                QCborValue(QStringLiteral("6.12")),
                kNoon.addSecs(30),
                10),
        };

        EpistemicProjection replayed;
        for (const CognitiveEnvelope &envelope : history) {
            replayed.admit(envelope);
        }

        EpistemicProjection restored;
        QString error;
        QVERIFY2(restored.restore(replayed.snapshot(), &error), qPrintable(error));
        QVERIFY(error.isEmpty());

        // Checked at two instants, because status is derived rather than stored: one where the
        // kernel reading is still fresh and one where it has aged out. A checkpoint that froze a
        // status would agree at the first and disagree at the second.
        for (const QDateTime &at : {kNoon.addSecs(35), kNoon.addSecs(600)}) {
            const QList<SubjectKnowledge> a = replayed.knowledgeAt(at);
            const QList<SubjectKnowledge> b = restored.knowledgeAt(at);
            QCOMPARE(b.size(), a.size());
            for (int i = 0; i < a.size(); ++i) {
                QCOMPARE(b.at(i).subject, a.at(i).subject);
                QCOMPARE(b.at(i).status, a.at(i).status);
                QCOMPARE(b.at(i).current.size(), a.at(i).current.size());
                QCOMPARE(b.at(i).superseded.size(), a.at(i).superseded.size());
                for (int j = 0; j < a.at(i).current.size(); ++j) {
                    QCOMPARE(b.at(i).current.at(j).value, a.at(i).current.at(j).value);
                    QCOMPARE(b.at(i).current.at(j).sourceId, a.at(i).current.at(j).sourceId);
                    QCOMPARE(b.at(i).current.at(j).status, a.at(i).current.at(j).status);
                }
            }
        }

        // The dispute and the supersession both survived, which is what makes this a real test
        // rather than a comparison of two empty projections.
        QCOMPARE(
            restored.knowledgeOf(QStringLiteral("current-system"), kNoon.addSecs(80)).status,
            EpistemicStatus::Disputed);
        QCOMPARE(
            restored.knowledgeOf(QStringLiteral("current-system"), kNoon.addSecs(80))
                .superseded.size(),
            1);
    }

    // A corrupt or unrecognised checkpoint is discarded whole. Rebuilding from the Journal is always
    // available and always correct, so a projection half-built from a bad cache buys nothing and
    // risks being quietly wrong.
    void aBadCheckpointIsRefusedRatherThanPartlyApplied()
    {
        EpistemicProjection projection;
        projection.admit(observationOf(
            QStringLiteral("nixos.system"),
            QStringLiteral("current-system"),
            QCborValue(QStringLiteral("aaa")),
            kNoon));

        const SubjectKnowledge before =
            projection.knowledgeOf(QStringLiteral("current-system"), kNoon.addSecs(10));

        QString error;
        QVERIFY(!projection.restore(QByteArray(), &error));
        QVERIFY(!error.isEmpty());
        QVERIFY(!projection.restore(QCborValue(42).toCbor()));

        QCborMap future = QCborValue::fromCbor(projection.snapshot()).toMap();
        future.insert(
            QStringLiteral("schemaVersion"), kCurrentProjectionSchemaVersion + 1);
        QVERIFY(!projection.restore(future.toCborValue().toCbor(), &error));
        QVERIFY(error.contains(QStringLiteral("not supported")));

        QCborMap malformed = QCborValue::fromCbor(projection.snapshot()).toMap();
        QCborMap firstSubject =
            malformed.value(QStringLiteral("subjects")).toArray().at(0).toMap();
        // A claim that is a bare string rather than a map: structurally wrong in a way a
        // best-effort parser would happily skip past.
        firstSubject.insert(QStringLiteral("current"), QCborArray{QCborValue(QStringLiteral("x"))});
        malformed.insert(QStringLiteral("subjects"), QCborArray{firstSubject});
        QVERIFY(!projection.restore(malformed.toCborValue().toCbor()));

        // Every refusal left what was already known untouched.
        const SubjectKnowledge after =
            projection.knowledgeOf(QStringLiteral("current-system"), kNoon.addSecs(10));
        QCOMPARE(after.status, before.status);
        QCOMPARE(after.current.size(), before.current.size());
        QCOMPARE(after.current.first().value, before.current.first().value);
    }

    // Rebuilding from the same history must produce the same answer, or the projection could not be
    // a cache of the Journal - losing it would mean losing knowledge rather than losing a shortcut.
    void rebuildingFromTheSameHistoryGivesTheSameAnswer()
    {
        const QList<CognitiveEnvelope> history{
            observationOf(
                QStringLiteral("nixos.system"),
                QStringLiteral("current-system"),
                QCborValue(QStringLiteral("aaa")),
                kNoon),
            observationOf(
                QStringLiteral("nixos.system"),
                QStringLiteral("current-system"),
                QCborValue(QStringLiteral("bbb")),
                kNoon.addSecs(60)),
            observationOf(
                QStringLiteral("nixos.profile"),
                QStringLiteral("other-subject"),
                QCborValue(42),
                kNoon.addSecs(30)),
        };

        EpistemicProjection first;
        EpistemicProjection second;
        for (const CognitiveEnvelope &envelope : history) {
            first.admit(envelope);
            second.admit(envelope);
        }

        const QDateTime at = kNoon.addSecs(90);
        const QList<SubjectKnowledge> a = first.knowledgeAt(at);
        const QList<SubjectKnowledge> b = second.knowledgeAt(at);
        QCOMPARE(a.size(), b.size());
        QCOMPARE(a.size(), 2);
        for (int i = 0; i < a.size(); ++i) {
            QCOMPARE(a.at(i).subject, b.at(i).subject);
            QCOMPARE(a.at(i).status, b.at(i).status);
            QCOMPARE(a.at(i).current.size(), b.at(i).current.size());
            QCOMPARE(a.at(i).superseded.size(), b.at(i).superseded.size());
        }
    }
};

QTEST_MAIN(TestEpistemicProjection)
#include "tst_epistemic_projection.moc"
