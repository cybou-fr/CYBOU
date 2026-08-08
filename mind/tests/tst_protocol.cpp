// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/protocol/CanonicalEnvelope.h"
#include "cybou/protocol/CognitiveEnvelope.h"
#include "cybou/protocol/Lifecycle.h"

#include <QTest>

#include <limits>

using namespace cybou;

namespace {

CognitiveEnvelope observation()
{
    CognitiveEnvelope e;
    e.messageId = QUuid::createUuid();
    e.correlationId = e.messageId;
    e.originOrgan = QStringLiteral("perceptiond");
    e.wallTime = QDateTime::currentDateTimeUtc();
    e.kind = ContributionKind::Observation;
    return e;
}

CognitiveEnvelope derived(ContributionKind kind)
{
    CognitiveEnvelope e = observation();
    e.kind = kind;
    e.causationId = QUuid::createUuid();
    return e;
}

} // namespace

class TestProtocol : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    void unknownPrivacyFailsClosed()
    {
        QCOMPARE(privacyFromString(QString()), PrivacyClass::Local);
        QCOMPARE(privacyFromString(QStringLiteral("shareable")), PrivacyClass::Local);
        QCOMPARE(privacyFromString(QStringLiteral("public")), PrivacyClass::Public);
    }

    void privacyIsInheritedFromReferences()
    {
        CognitiveEnvelope e;
        e.privacy = PrivacyClass::Public;
        QCOMPARE(e.derivedPrivacy({PrivacyClass::Local}), PrivacyClass::Local);
        QCOMPARE(e.derivedPrivacy({PrivacyClass::Household, PrivacyClass::Node}),
                 PrivacyClass::Node);
    }

    void onlyObservationMayBeRoot()
    {
        CognitiveEnvelope root = observation();
        QVERIFY(root.isValid());

        CognitiveEnvelope intention = root;
        intention.kind = ContributionKind::Intention;
        QVERIFY(!intention.isValid());
    }

    void rootMustNotHaveReferences()
    {
        CognitiveEnvelope e = observation();
        e.causationId = QUuid::createUuid();
        QVERIFY(!e.isValid());

        e.causationId = {};
        e.evidence = {QUuid::createUuid()};
        QVERIFY(!e.isValid());
    }

    void derivedContributionNeedsABasis()
    {
        CognitiveEnvelope e = observation();
        e.kind = ContributionKind::Prediction;
        QVERIFY(!e.isValid());

        e.evidence = {QUuid::createUuid()};
        QVERIFY(e.isValid());
    }

    void selfCausationIsRejected()
    {
        CognitiveEnvelope e = derived(ContributionKind::Intention);
        e.causationId = e.messageId;
        QVERIFY(!e.isValid());
    }

    void selfEvidenceIsRejected()
    {
        CognitiveEnvelope e = derived(ContributionKind::Prediction);
        e.causationId = {};
        e.evidence = {e.messageId};
        QVERIFY(!e.isValid());
    }

    void duplicateAndNullEvidenceAreRejected()
    {
        CognitiveEnvelope e = derived(ContributionKind::Prediction);
        e.causationId = {};
        const QUuid id = QUuid::createUuid();
        e.evidence = {id, id};
        QVERIFY(!e.isValid());

        e.evidence = {QUuid()};
        QVERIFY(!e.isValid());
    }

    void causeMustNotBeDuplicatedAsEvidence()
    {
        CognitiveEnvelope e = derived(ContributionKind::Outcome);
        e.evidence = {e.causationId};
        QVERIFY(!e.isValid());
    }

    void correlationAndFiniteConfidenceAreRequired()
    {
        CognitiveEnvelope e = observation();
        e.correlationId = {};
        QVERIFY(!e.isValid());

        e = observation();
        e.confidence = std::numeric_limits<double>::quiet_NaN();
        QVERIFY(!e.isValid());
    }

    void supportedSchemaVersionsAreExplicit()
    {
        CognitiveEnvelope e = observation();
        QCOMPARE(e.schemaVersion, kCurrentEnvelopeSchemaVersion);

        e.schemaVersion = kLegacyEnvelopeSchemaVersion;
        QVERIFY(e.isValid());

        e.schemaVersion = 99;
        QVERIFY(!e.isValid());
    }

    void canonicalEvidenceIsOrderIndependent()
    {
        CognitiveEnvelope e = observation();
        e.kind = ContributionKind::Prediction;
        e.causationId = {};
        const QUuid first = QUuid::createUuid();
        const QUuid second = QUuid::createUuid();
        e.evidence = {first, second};

        CognitiveEnvelope reordered = e;
        reordered.evidence = {second, first};
        QCOMPARE(canonicalEnvelopeV2(e), canonicalEnvelopeV2(reordered));
    }

    void canonicalEncodingCoversSemanticFields()
    {
        CognitiveEnvelope e = observation();
        const QByteArray original = canonicalEnvelopeV2(e);

        e.privacy = PrivacyClass::Public;
        QVERIFY(canonicalEnvelopeV2(e) != original);

        e = observation();
        const QByteArray beforeCapability = canonicalEnvelopeV2(e);
        e.capabilityScope = QStringLiteral("system.observe");
        QVERIFY(canonicalEnvelopeV2(e) != beforeCapability);
    }

    void lifecycleTransitionsAreExplicit()
    {
        QVERIFY(canTransition(LifecycleMode::Awake, LifecycleMode::Idle));
        QVERIFY(canTransition(LifecycleMode::Idle, LifecycleMode::Consolidating));
        QVERIFY(canTransition(LifecycleMode::Consolidating, LifecycleMode::Awake));
        QVERIFY(!canTransition(LifecycleMode::Awake, LifecycleMode::Consolidating));
        QVERIFY(!canTransition(LifecycleMode::Awake, LifecycleMode::Awake));
        QVERIFY(canTransition(LifecycleMode::Suspended, LifecycleMode::Recovering));
    }

    void lifecycleRunRoundTrips()
    {
        LifecycleRun run;
        run.runId = QUuid::createUuid();
        run.kind = QStringLiteral("consolidation");
        run.policyId = QStringLiteral("explicit-user-request");
        run.requestedAt = QDateTime::currentDateTimeUtc();
        run.inputHighWaterMark = 42;
        run.requiredCapabilities = {QStringLiteral("journal")};
        run.optionalCapabilities = {QStringLiteral("prediction")};
        run.status = LifecycleRunStatus::Completed;
        run.completedWork = {QStringLiteral("journal.verify")};
        run.terminalCause = QStringLiteral("completed");
        QVERIFY(run.isValid());

        QString error;
        const LifecycleRun decoded = decodeLifecycleRun(encodeLifecycleRun(run), &error);
        QVERIFY2(error.isEmpty(), qPrintable(error));
        QCOMPARE(decoded.runId, run.runId);
        QCOMPARE(decoded.inputHighWaterMark, 42u);
        QCOMPARE(decoded.status, LifecycleRunStatus::Completed);
    }

    void lifecycleRunFailsClosed()
    {
        LifecycleRun run;
        run.runId = QUuid::createUuid();
        run.kind = QStringLiteral("consolidation");
        run.policyId = QStringLiteral("idle");
        run.requestedAt = QDateTime::currentDateTimeUtc();
        run.status = LifecycleRunStatus::Completed;
        QVERIFY(!run.isValid());

        QString error;
        decodeLifecycleRun(QByteArrayLiteral("not-cbor"), &error);
        QVERIFY(!error.isEmpty());
    }
};

QTEST_MAIN(TestProtocol)
#include "tst_protocol.moc"
