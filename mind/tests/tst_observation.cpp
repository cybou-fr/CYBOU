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

    // ContributionKind::Observation already carries unrelated payload shapes, so kind alone cannot
    // identify an epistemic observation. Without the discriminator, a projection scanning
    // Observations would guess from each payload's shape - and would eventually guess wrong about
    // history that can no longer be changed.
    //
    // The payloads below are the ones predictord and presenced actually write today.
    void otherObservationPayloadsAreNotObservations()
    {
        QCborMap predictorOutcome;
        predictorOutcome.insert(QStringLiteral("subject"), QStringLiteral("mood"));
        predictorOutcome.insert(QStringLiteral("actual"), 0.5);

        QString error;
        QVERIFY(!decodeObservation(predictorOutcome.toCborValue().toCbor(), &error).has_value());
        QVERIFY2(
            error.contains(QStringLiteral("not an ObservationV1")),
            qPrintable(QStringLiteral("wrong reason: %1").arg(error)));

        QCborMap presenceUserObservation;
        presenceUserObservation.insert(
            QStringLiteral("event"), QStringLiteral("user-requested-intention"));
        presenceUserObservation.insert(QStringLiteral("description"), QStringLiteral("call back"));
        QVERIFY(!decodeObservation(presenceUserObservation.toCborValue().toCbor()).has_value());

        // A payload that happens to carry every ObservationV1 field but does not claim the type is
        // still not one. Claiming the type is what makes it readable as evidence, not resembling it.
        QCborMap lookalike =
            QCborValue::fromCbor(encodeObservation(systemGeneration())).toMap();
        lookalike.remove(QStringLiteral("@type"));
        QVERIFY(!decodeObservation(lookalike.toCborValue().toCbor()).has_value());

        QCborMap wrongType = QCborValue::fromCbor(encodeObservation(systemGeneration())).toMap();
        wrongType.insert(QStringLiteral("@type"), QStringLiteral("cybou.observation.v2"));
        QVERIFY(!decodeObservation(wrongType.toCborValue().toCbor()).has_value());
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

        // Bounded below as well. An observation says nothing about a time before it was acquired,
        // and an earlier version checked only the upper bound - so a reading taken at 09:00
        // reported as fresh at 04:00 the same day.
        QVERIFY(!observation.isFreshAt(observation.acquiredAt.addSecs(-1)));
        QVERIFY(!observation.isFreshAt(observation.acquiredAt.addSecs(-18000)));
    }

    // Re-reporting one reading - after an adapter restart, a retry, or a replayed queue - must
    // resolve to the same contribution, so Event1's duplicate rejection makes it a durable no-op.
    void repeatedAcquisitionHasOneIdentity()
    {
        const ObservationV1 o = systemGeneration();
        const QUuid id = observationMessageId(o.sourceId, o.subject, o.acquiredAt, o.value);

        QVERIFY(!id.isNull());
        QCOMPARE(observationMessageId(o.sourceId, o.subject, o.acquiredAt, o.value), id);

        // Every component participates.
        QVERIFY(
            observationMessageId(
                QStringLiteral("other.source"), o.subject, o.acquiredAt, o.value) != id);
        QVERIFY(
            observationMessageId(
                o.sourceId, QStringLiteral("other-subject"), o.acquiredAt, o.value) != id);
        QVERIFY(
            observationMessageId(o.sourceId, o.subject, o.acquiredAt.addSecs(1), o.value) != id);

        // One instant expressed in two zones is one acquisition. Otherwise every observation would
        // duplicate across a DST change.
        QCOMPARE(
            observationMessageId(
                o.sourceId, o.subject, o.acquiredAt.toOffsetFromUtc(3600), o.value),
            id);
    }

    // The value must participate, or a contradiction can never be recorded.
    //
    // An earlier version excluded it, reasoning that two different values for one subject at one
    // instant should become a contradiction for the projection to reconcile. They could not: both
    // mapped to one messageId, Event1 rejects a duplicate, and the second piece of evidence never
    // reached the Journal for anything to reconcile.
    void disagreeingValuesAreTwoContributions()
    {
        const ObservationV1 o = systemGeneration();

        ObservationV1 disagreeing = o;
        disagreeing.value = QCborValue(QStringLiteral("a completely different system"));

        QVERIFY(
            observationMessageId(o.sourceId, o.subject, o.acquiredAt, o.value)
            != observationMessageId(
                disagreeing.sourceId,
                disagreeing.subject,
                disagreeing.acquiredAt,
                disagreeing.value));

        // Types must not collapse either: the string "142" is not the integer 142.
        QVERIFY(
            observationMessageId(
                o.sourceId, o.subject, o.acquiredAt, QCborValue(142))
            != observationMessageId(
                o.sourceId, o.subject, o.acquiredAt, QCborValue(QStringLiteral("142"))));
    }

    // Field boundaries must come from the encoding, not from a byte the fields are trusted not to
    // contain. The previous test here checked a pair that never collided, so it passed while the
    // genuinely ambiguous pair below went uncovered.
    void fieldBoundariesCannotBeForged()
    {
        const QDateTime at = systemGeneration().acquiredAt;
        const QCborValue value(1);
        const QChar unitSeparator(0x1f);

        QVERIFY(
            observationMessageId(QStringLiteral("a"), QStringLiteral("b") + unitSeparator + QStringLiteral("c"), at, value)
            != observationMessageId(QStringLiteral("a") + unitSeparator + QStringLiteral("b"), QStringLiteral("c"), at, value));

        // The same holds for any other byte someone might reach for.
        QVERIFY(
            observationMessageId(QStringLiteral("a"), QStringLiteral("b:c"), at, value)
            != observationMessageId(QStringLiteral("a:b"), QStringLiteral("c"), at, value));

        // And for the empty-field case, where a naive join loses the boundary entirely.
        QVERIFY(
            observationMessageId(QStringLiteral("ab"), QString(), at, value)
            != observationMessageId(QStringLiteral("a"), QStringLiteral("b"), at, value));
    }

};

QTEST_MAIN(TestObservation)
#include "tst_observation.moc"
