// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT

#include "cybou/protocol/CanonicalEnvelope.h"
#include "cybou/protocol/CognitiveEnvelope.h"
#include "cybou/protocol/Sensitivity.h"

#include <QSet>
#include <QTest>

using namespace cybou;

class TestSensitivity : public QObject
{
    Q_OBJECT

private slots:
    void aConclusionInheritsTheStrongestClassificationItRestsOn();
    void anUnclassifiedContributionIsNotAssumedHarmless();
    void secretsAndCredentialsAreNeverTrainingTargets();
    void everyClassificationHasItsOwnLabel();
    void schemaFourAddsOneByteAndSchemaThreeKeepsItsOwn();
};

void TestSensitivity::schemaFourAddsOneByteAndSchemaThreeKeepsItsOwn()
{
    CognitiveEnvelope e;
    e.messageId = QUuid::createUuid();
    e.correlationId = e.messageId;
    e.originOrgan = QStringLiteral("perceptiond");
    e.originNode = QStringLiteral("local");
    e.wallTime = QDateTime::currentDateTimeUtc();
    e.kind = ContributionKind::Observation;
    e.sensitivity = SensitivityClass::Credential;

    e.schemaVersion = kProtectedEnvelopeSchemaVersion;
    const QByteArray three = canonicalNonErasableEnvelopeV3(e);

    e.schemaVersion = kClassifiedEnvelopeSchemaVersion;
    const QByteArray four = canonicalNonErasableEnvelopeV3(e);

    // Exactly one byte more, and only for schema 4. Hashing the byte unconditionally would grow the
    // schema-3 form and rewrite the digest of every row already written; never hashing it would
    // leave the classification outside the chain entirely. Writing and verifying both change
    // together under either mistake, so neither can be caught by a round trip -- only by measuring
    // the canonical form itself.
    QCOMPARE(four.size(), three.size() + 1);

    // Not a prefix relation: the schema version is itself part of the canonical bytes, so the two
    // forms differ at that field as well as in length. The size and the trailing byte are what
    // carry the property.
    QCOMPARE(static_cast<quint8>(four.at(four.size() - 1)),
             static_cast<quint8>(SensitivityClass::Credential));

    // And the classification actually reaches the digest: a form that ignored it would make two
    // differently-classified contributions hash alike.
    e.sensitivity = SensitivityClass::Ordinary;
    QVERIFY(canonicalNonErasableEnvelopeV3(e) != four);
}

void TestSensitivity::aConclusionInheritsTheStrongestClassificationItRestsOn()
{
    // A conclusion that restated its evidence at a weaker classification would launder it.
    QCOMPARE(derivedSensitivity(SensitivityClass::Ordinary, {SensitivityClass::Secret}),
             SensitivityClass::Secret);
    QCOMPARE(
        derivedSensitivity(SensitivityClass::Ordinary,
                           {SensitivityClass::Personal, SensitivityClass::Credential,
                            SensitivityClass::Sensitive}),
        SensitivityClass::Credential);

    // Declaring more than the evidence requires is allowed: propagation is a floor, not a target.
    QCOMPARE(derivedSensitivity(SensitivityClass::Secret, {SensitivityClass::Ordinary}),
             SensitivityClass::Secret);

    // With nothing to inherit from, the declaration stands.
    QCOMPARE(derivedSensitivity(SensitivityClass::Personal, {}), SensitivityClass::Personal);
    QCOMPARE(derivedSensitivity(SensitivityClass::Ordinary, {}), SensitivityClass::Ordinary);

    // Order must not matter, or the same evidence would classify differently by arrival.
    QCOMPARE(derivedSensitivity(SensitivityClass::Ordinary,
                                {SensitivityClass::Secret, SensitivityClass::Personal}),
             derivedSensitivity(SensitivityClass::Ordinary,
                                {SensitivityClass::Personal, SensitivityClass::Secret}));
}

void TestSensitivity::anUnclassifiedContributionIsNotAssumedHarmless()
{
    // Every row written before this axis existed carries no classification. Reading absence as
    // Ordinary would make the whole history look safe on the day the point is to notice what is
    // not.
    QVERIFY2(kUnclassifiedSensitivity != SensitivityClass::Ordinary,
             "an unclassified contribution must not default to harmless");
    QCOMPARE(kUnclassifiedSensitivity, SensitivityClass::Personal);

    // And the default must not be so strong that everything unmigrated becomes untrainable and
    // undeliverable, which would make the axis unusable rather than safe.
    QVERIFY(mayBeTrainingTarget(kUnclassifiedSensitivity));
}

void TestSensitivity::secretsAndCredentialsAreNeverTrainingTargets()
{
    // ADR-0033's A9, as a predicate over the type rather than a policy flag someone can clear.
    QVERIFY(!mayBeTrainingTarget(SensitivityClass::Secret));
    QVERIFY(!mayBeTrainingTarget(SensitivityClass::Credential));

    // The permitted cases matter as much: a rule that refused everything would satisfy the two
    // assertions above and forbid all learning, which is a refusal for the wrong reason.
    QVERIFY(mayBeTrainingTarget(SensitivityClass::Ordinary));
    QVERIFY(mayBeTrainingTarget(SensitivityClass::Personal));
    QVERIFY(mayBeTrainingTarget(SensitivityClass::Sensitive));

    // A conclusion drawn from a credential inherits it, so it cannot be trained on either. This is
    // the path A9 actually has to close: nobody trains on the password, they train on something
    // derived from it.
    QVERIFY(!mayBeTrainingTarget(
        derivedSensitivity(SensitivityClass::Ordinary, {SensitivityClass::Credential})));
}

void TestSensitivity::everyClassificationHasItsOwnLabel()
{
    const QList<SensitivityClass> all{
        SensitivityClass::Ordinary, SensitivityClass::Personal, SensitivityClass::Sensitive,
        SensitivityClass::Secret,   SensitivityClass::Credential,
    };

    QSet<QString> labels;
    for (const SensitivityClass sensitivity : all) {
        const QString label = sensitivityToString(sensitivity);
        QVERIFY2(label != QStringLiteral("unknown"), qPrintable(label));
        labels.insert(label);
    }

    // Two classifications sharing a label would make a credential and an ordinary fact read alike
    // wherever a person or an audit looks at them.
    QCOMPARE(labels.size(), all.size());
    QCOMPARE(sensitivityToString(SensitivityClass::Credential), QStringLiteral("credential"));
}

QTEST_MAIN(TestSensitivity)
#include "tst_sensitivity.moc"
