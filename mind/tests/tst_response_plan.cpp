// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT

#include "cybou/meaning/ResponsePlan.h"

#include <QTest>

using namespace cybou;

namespace {

PlanClaim claimOf(const QString &subject, const QString &value)
{
    PlanClaim claim;
    claim.subject = subject;
    claim.value = value;
    claim.evidence = {QUuid::createUuid()};
    return claim;
}

ReferenceResolution ambiguousReference()
{
    ReferenceResolution reference;
    reference.surfaceForm = QStringLiteral("that one");
    for (const auto &entry : {qMakePair(QStringLiteral("server-A"), 0.52),
                              qMakePair(QStringLiteral("server-B"), 0.48)}) {
        ReferenceCandidate candidate;
        candidate.entityId = entry.first;
        candidate.score = entry.second;
        candidate.evidence = {QUuid::createUuid()};
        reference.candidates.append(candidate);
    }
    reference.resolveIfUnambiguous(0.15);
    return reference;
}

} // namespace

class TestResponsePlan : public QObject
{
    Q_OBJECT

private slots:
    void anUnresolvedReferenceBecomesAQuestionNotAnAnswer();
    void realizationCarriesEveryClaimAndNothingElse();
    void twoLanguagesRenderOneSemanticObject();
    void aQualificationSurvivesRealization();
    void aPlanThatWouldSayNothingIsNotValid();
};

// C2 as the person experiences it: the ambiguous case produces a question, and the question names
// the options.
void TestResponsePlan::anUnresolvedReferenceBecomesAQuestionNotAnAnswer()
{
    CognitiveAct act;
    act.kind = ActKind::Ask;
    act.utterance = QUuid::createUuid();
    act.references = {ambiguousReference()};
    QVERIFY(!act.isFullyResolved());

    const ResponsePlan plan
        = planFor(act, {claimOf(QStringLiteral("server-A"), QStringLiteral("healthy"))});

    QCOMPARE(plan.goal, ResponseGoal::Clarify);
    QCOMPARE(plan.unresolved.size(), 1);

    // The status claim is not in the plan: answering about a guessed target is the failure this
    // whole path exists to prevent.
    QVERIFY2(plan.claims.isEmpty(), "an unresolved act must not carry an answer about a guess");

    const QString english = realize(plan, Language::English);
    QVERIFY(english.contains(QStringLiteral("server-A")));
    QVERIFY(english.contains(QStringLiteral("server-B")));
    QVERIFY2(!english.contains(QStringLiteral("healthy")),
             "the withheld answer must not reach the rendered text either");
}

// C5/C6. The realization says what the plan says.
void TestResponsePlan::realizationCarriesEveryClaimAndNothingElse()
{
    ResponsePlan plan;
    plan.goal = ResponseGoal::ExplainStatus;
    plan.claims = {claimOf(QStringLiteral("PostgreSQL"), QStringLiteral("healthy")),
                   claimOf(QStringLiteral("disk"), QStringLiteral("82% full"))};
    QVERIFY(plan.isValid());

    const QString text = realize(plan, Language::English);
    for (const PlanClaim &claim : plan.claims) {
        QVERIFY2(text.contains(claim.subject), qPrintable(claim.subject));
        QVERIFY2(text.contains(claim.value), qPrintable(claim.value));
    }

    // Dropping a claim would understate what Mind knows; the count is what notices it.
    QCOMPARE(text.count(QStringLiteral(" is ")), plan.claims.size());

    // And the renderer has no channel for anything else: removing a claim from the plan removes it
    // from the prose, which is C6 held by construction rather than by inspecting the output.
    ResponsePlan narrowed = plan;
    narrowed.claims.removeLast();
    const QString narrowedText = realize(narrowed, Language::English);
    QVERIFY(!narrowedText.contains(QStringLiteral("82% full")));
    QVERIFY(narrowedText.contains(QStringLiteral("PostgreSQL")));
}

// C7. One semantic object, two surfaces.
void TestResponsePlan::twoLanguagesRenderOneSemanticObject()
{
    ResponsePlan plan;
    plan.goal = ResponseGoal::ExplainStatus;
    plan.claims = {claimOf(QStringLiteral("PostgreSQL"), QStringLiteral("healthy"))};

    const QString english = realize(plan, Language::English);
    const QString russian = realize(plan, Language::Russian);

    QVERIFY2(english != russian, "two languages that render identically are one language");

    // Same claim in both, because the plan is the authority and the language is the surface.
    for (const QString &text : {english, russian}) {
        QVERIFY(text.contains(QStringLiteral("PostgreSQL")));
        QVERIFY(text.contains(QStringLiteral("healthy")));
    }

    // The clarification path is bilingual too, and both name the candidates.
    CognitiveAct act;
    act.kind = ActKind::Ask;
    act.utterance = QUuid::createUuid();
    act.references = {ambiguousReference()};
    const ResponsePlan clarify = planFor(act, {});
    for (const Language language : {Language::English, Language::Russian}) {
        const QString text = realize(clarify, language);
        QVERIFY(text.contains(QStringLiteral("server-A")));
        QVERIFY(text.contains(QStringLiteral("server-B")));
    }
}

// Fluency that discards a hedge asserts something stronger than the plan does.
void TestResponsePlan::aQualificationSurvivesRealization()
{
    ResponsePlan plan;
    plan.goal = ResponseGoal::ExplainStatus;
    plan.claims = {claimOf(QStringLiteral("backup"), QStringLiteral("complete"))};
    plan.qualifications = {QStringLiteral("verification has not run")};

    for (const Language language : {Language::English, Language::Russian}) {
        const QString text = realize(plan, language);
        QVERIFY(text.contains(QStringLiteral("complete")));
        QVERIFY2(text.contains(QStringLiteral("verification has not run")),
                 "a dropped qualification states more than the plan does");
    }
}

void TestResponsePlan::aPlanThatWouldSayNothingIsNotValid()
{
    ResponsePlan empty;
    empty.goal = ResponseGoal::ExplainStatus;
    QVERIFY2(!empty.isValid(), "an explanation with nothing to explain is not a response");

    ResponsePlan clarifyNothing;
    clarifyNothing.goal = ResponseGoal::Clarify;
    QVERIFY(!clarifyNothing.isValid());

    // A claim with no evidence is a sentence Mind cannot source.
    ResponsePlan unsourced;
    unsourced.goal = ResponseGoal::ExplainStatus;
    PlanClaim claim;
    claim.subject = QStringLiteral("disk");
    claim.value = QStringLiteral("healthy");
    unsourced.claims = {claim};
    QVERIFY2(!unsourced.isValid(), "a claim without evidence must not become prose");

    // And the valid case, so the refusals above are about what they say they are about.
    ResponsePlan good;
    good.goal = ResponseGoal::ExplainStatus;
    good.claims = {claimOf(QStringLiteral("disk"), QStringLiteral("healthy"))};
    QVERIFY(good.isValid());
}

QTEST_MAIN(TestResponsePlan)
#include "tst_response_plan.moc"
