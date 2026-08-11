// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT
//
// ObservationV1, the typed perception envelope frozen by ADR-0027.
//
// The checkpoint's P7.1 exit gate is that malformed and future schemas fail closed, that duplicate
// acquisition is idempotent by declared semantics, and that source unavailability has a typed
// result. The first two are properties of this payload and are covered here; the third belongs to
// the adapter, which cannot smuggle a failure in as an observation because a valueless observation
// is rejected below.

#include "cybou/protocol/Observation.h"

#include <QTest>

using namespace cybou;

namespace {

ObservationV1 systemGeneration()
{
    ObservationV1 observation;
    observation.sourceId = QStringLiteral("nixos.system.generation");
    observation.subject = QStringLiteral("current-generation");
    observation.value = QCborValue(142);
    observation.acquiredAt = QDateTime(QDate(2026, 8, 11), QTime(9, 0), Qt::UTC);
    observation.freshnessUntil = observation.acquiredAt.addSecs(300);
    observation.provenance = QStringLiteral("readlink /run/current-system");
    return observation;
}

} // namespace

class TestObservation : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    void roundTripPreservesEveryField()
    {
        const ObservationV1 original = systemGeneration();
        QVERIFY(original.isValid());

        QString error;
        const auto decoded = decodeObservation(encodeObservation(original), &error);
        QVERIFY2(decoded.has_value(), qPrintable(error));
        QVERIFY(error.isEmpty());

        QCOMPARE(decoded->sourceId, original.sourceId);
        QCOMPARE(decoded->subject, original.subject);
        QCOMPARE(decoded->value, original.value);
        QCOMPARE(decoded->acquiredAt, original.acquiredAt);
        QCOMPARE(decoded->freshnessUntil, original.freshnessUntil);
        QCOMPARE(decoded->provenance, original.provenance);
    }

    // Evidence read under guessed rules is worse than no evidence, so an unrecognised schema is
    // refused rather than best-effort parsed.
    void unknownSchemaFailsClosed()
    {
        ObservationV1 future = systemGeneration();
        future.schemaVersion = kCurrentObservationSchemaVersion + 1;

        QString error;
        QVERIFY(!decodeObservation(encodeObservation(future), &error).has_value());
        QVERIFY(error.contains(QStringLiteral("not supported")));

        // A payload that is not a map at all, and one with no version, are equally refused.
        QVERIFY(!decodeObservation(QCborValue(42).toCbor()).has_value());
        QVERIFY(!decodeObservation(QCborMap{}.toCborValue().toCbor()).has_value());
        QVERIFY(!decodeObservation(QByteArray()).has_value());
    }

    void structurallyIncompleteObservationsAreRejected()
    {
        const ObservationV1 valid = systemGeneration();

        ObservationV1 noSource = valid;
        noSource.sourceId = QStringLiteral("   ");
        QVERIFY(!noSource.isValid());

        ObservationV1 noSubject = valid;
        noSubject.subject.clear();
        QVERIFY(!noSubject.isValid());

        ObservationV1 noProvenance = valid;
        noProvenance.provenance.clear();
        QVERIFY(!noProvenance.isValid());

        // A failure to observe is not an observation of nothing. An adapter that cannot read its
        // source must report that as a typed failure, not contribute an empty value.
        ObservationV1 noValue = valid;
        noValue.value = QCborValue();
        QVERIFY(!noValue.isValid());

        // A horizon at or before acquisition describes something that was never current, so
        // nothing could ever legitimately act on it.
        ObservationV1 expiredOnArrival = valid;
        expiredOnArrival.freshnessUntil = expiredOnArrival.acquiredAt;
        QVERIFY(!expiredOnArrival.isValid());

        ObservationV1 backwards = valid;
        backwards.freshnessUntil = backwards.acquiredAt.addSecs(-1);
        QVERIFY(!backwards.isValid());

        // Rejection has to survive the wire, not only the accessor.
        QVERIFY(!decodeObservation(encodeObservation(noValue)).has_value());
        QVERIFY(!decodeObservation(encodeObservation(expiredOnArrival)).has_value());
    }

    // Freshness is declared by the adapter, which knows how fast its source changes, rather than
    // inferred by whoever reads the observation later.
    void freshnessIsDeclaredNotInferred()
    {
        const ObservationV1 observation = systemGeneration();

        QVERIFY(observation.isFreshAt(observation.acquiredAt));
        QVERIFY(observation.isFreshAt(observation.freshnessUntil.addSecs(-1)));
        QVERIFY(!observation.isFreshAt(observation.freshnessUntil));
        QVERIFY(!observation.isFreshAt(observation.freshnessUntil.addSecs(1)));
        QVERIFY(!observation.isFreshAt(QDateTime()));
    }

    // Re-reporting one reading - after an adapter restart, a retry, or a replayed queue - must
    // resolve to the same contribution, so Event1's duplicate rejection makes it a durable no-op.
    // Without this, "observed twice" and "observed once, reported twice" are indistinguishable.
    void repeatedAcquisitionHasOneIdentity()
    {
        const ObservationV1 observation = systemGeneration();
        const QUuid id = observationMessageId(
            observation.sourceId, observation.subject, observation.acquiredAt);

        QVERIFY(!id.isNull());
        QCOMPARE(
            observationMessageId(
                observation.sourceId, observation.subject, observation.acquiredAt),
            id);

        // The value is excluded on purpose: two different values for one subject at one instant is
        // a contradiction for the projection to surface, not two contributions to record.
        ObservationV1 disagreeing = observation;
        disagreeing.value = QCborValue(999);
        QCOMPARE(
            observationMessageId(
                disagreeing.sourceId, disagreeing.subject, disagreeing.acquiredAt),
            id);

        // Each of the three components genuinely participates.
        QVERIFY(
            observationMessageId(
                QStringLiteral("other.source"), observation.subject, observation.acquiredAt)
            != id);
        QVERIFY(
            observationMessageId(
                observation.sourceId, QStringLiteral("other-subject"), observation.acquiredAt)
            != id);
        QVERIFY(
            observationMessageId(
                observation.sourceId, observation.subject, observation.acquiredAt.addSecs(1))
            != id);

        // Timezone must not change identity: the same instant expressed differently is one
        // acquisition, and treating it as two would duplicate every observation across a DST change.
        ObservationV1 shifted = observation;
        shifted.acquiredAt = observation.acquiredAt.toOffsetFromUtc(3600);
        QCOMPARE(
            observationMessageId(shifted.sourceId, shifted.subject, shifted.acquiredAt), id);
    }

    // A source that splits its subject on the separator must not be able to collide with another.
    void identityComponentsCannotBeConfused()
    {
        QVERIFY(
            observationMessageId(
                QStringLiteral("a"), QStringLiteral("b"), systemGeneration().acquiredAt)
            != observationMessageId(
                QStringLiteral("a\x1f" "b"), QString(), systemGeneration().acquiredAt));
    }
};

QTEST_MAIN(TestObservation)
#include "tst_observation.moc"
