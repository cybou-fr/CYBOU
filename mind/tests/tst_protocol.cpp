// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT
//
// The two rules the protocol enforces by type rather than by convention are exactly the two
// that are worth testing: causal traceability, and privacy that fails closed.

#include "cybou/protocol/CognitiveEnvelope.h"

#include <QTest>

using namespace cybou;

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

    void privacyIsInheritedFromEvidence()
    {
        // A public conclusion drawn from local evidence must not stay public: this is the
        // leak-through-generalisation case from docs/14.
        CognitiveEnvelope e;
        e.privacy = PrivacyClass::Public;
        QCOMPARE(e.derivedPrivacy({PrivacyClass::Local}), PrivacyClass::Local);
        QCOMPARE(e.derivedPrivacy({PrivacyClass::Public}), PrivacyClass::Public);
        QCOMPARE(e.derivedPrivacy({PrivacyClass::Household, PrivacyClass::Node}),
                 PrivacyClass::Node);
    }

    void nonObservationNeedsCausationOrEvidence()
    {
        CognitiveEnvelope e;
        e.messageId = QUuid::createUuid();
        e.originOrgan = QStringLiteral("predictord");
        e.wallTime = QDateTime::currentDateTimeUtc();
        e.kind = ContributionKind::Prediction;

        QVERIFY2(!e.isValid(), "a prediction with no cause and no evidence must be rejected");

        e.causationId = QUuid::createUuid();
        QVERIFY(e.isValid());
    }

    void rootObservationIsValidWithoutCause()
    {
        CognitiveEnvelope e;
        e.messageId = QUuid::createUuid();
        e.originOrgan = QStringLiteral("perceptiond");
        e.wallTime = QDateTime::currentDateTimeUtc();
        e.kind = ContributionKind::Observation;
        QVERIFY(e.isValid());
    }

    void confidenceIsBounded()
    {
        CognitiveEnvelope e;
        e.messageId = QUuid::createUuid();
        e.originOrgan = QStringLiteral("perceptiond");
        e.wallTime = QDateTime::currentDateTimeUtc();
        e.confidence = 1.4;
        QVERIFY(!e.isValid());
    }
};

QTEST_MAIN(TestProtocol)
#include "tst_protocol.moc"
