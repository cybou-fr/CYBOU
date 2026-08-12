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
