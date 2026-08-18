// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT

#include "cybou/meaning/CognitiveAct.h"

#include <QSet>
#include <QTest>

using namespace cybou;

namespace {

ReferenceResolution referenceTo(const QList<QPair<QString, double>> &scored)
{
    ReferenceResolution reference;
    reference.surfaceForm = QStringLiteral("that one");
    for (const auto &entry : scored) {
        ReferenceCandidate candidate;
        candidate.entityId = entry.first;
        candidate.score = entry.second;
        candidate.evidence = {QUuid::createUuid()};
        reference.candidates.append(candidate);
    }
    return reference;
}

CognitiveAct actOf(ActKind kind, const ReferenceResolution &reference)
{
    CognitiveAct act;
    act.kind = kind;
    act.utterance = QUuid::createUuid();
    act.property = QStringLiteral("status");
    act.references = {reference};
    return act;
}

} // namespace

class TestCognitiveAct : public QObject
{
    Q_OBJECT

private slots:
    void aNearTieDoesNotResolve();
    void aMutatingActMayNotProceedOnAnUnresolvedReference();
    void aPersonMayResolveWhatRankingCouldNot();
    void everyActKindHasItsOwnLabel();
    void anActWithoutItsUtteranceIsNotValid();
};

// ADR-0031: ranking uncertainty is not permission to pick one.
void TestCognitiveAct::aNearTieDoesNotResolve()
{
    const double margin = 0.15;

    ReferenceResolution tie
        = referenceTo({{QStringLiteral("server-A"), 0.52}, {QStringLiteral("server-B"), 0.48}});
    tie.resolveIfUnambiguous(margin);
    QVERIFY2(!tie.isResolved(), "0.52 against 0.48 is a question, not an answer");
    QVERIFY2(tie.resolved().isEmpty(), "an unresolved reference names nothing");

    // The alternatives survive resolution, so a person can see what it was choosing between.
    QCOMPARE(tie.candidates.size(), 2);

    // A clear leader does resolve, or the assertion above would hold for the wrong reason.
    ReferenceResolution clear
        = referenceTo({{QStringLiteral("server-A"), 0.91}, {QStringLiteral("server-B"), 0.12}});
    clear.resolveIfUnambiguous(margin);
    QVERIFY(clear.isResolved());
    QCOMPARE(clear.resolved(), QStringLiteral("server-A"));

    // A single candidate is unambiguous by construction.
    ReferenceResolution only = referenceTo({{QStringLiteral("server-A"), 0.30}});
    only.resolveIfUnambiguous(margin);
    QCOMPARE(only.resolved(), QStringLiteral("server-A"));

    // Nothing to choose from resolves to nothing rather than to the first thing.
    ReferenceResolution none = referenceTo({});
    none.resolveIfUnambiguous(margin);
    QVERIFY(!none.isResolved());
}

// C2. The distinction the boundary exists for: asking about an ambiguous thing is fine; acting on
// one is not.
void TestCognitiveAct::aMutatingActMayNotProceedOnAnUnresolvedReference()
{
    ReferenceResolution ambiguous
        = referenceTo({{QStringLiteral("server-A"), 0.52}, {QStringLiteral("server-B"), 0.48}});
    ambiguous.resolveIfUnambiguous(0.15);
    QVERIFY(!ambiguous.isResolved());

    const CognitiveAct request = actOf(ActKind::Request, ambiguous);
    QVERIFY2(!request.mayProceed(), "a mutating act must not run against an unresolved referent");

    // The same ambiguity in a question is not an error: it is a question that needs clarifying.
    const CognitiveAct question = actOf(ActKind::Ask, ambiguous);
    QVERIFY(!question.isFullyResolved());
    QVERIFY2(question.mayProceed(), "asking about an ambiguous thing is still a legitimate ask");

    // And a resolved request proceeds, or the refusal above would prove only that requests never
    // proceed.
    ReferenceResolution resolved
        = referenceTo({{QStringLiteral("server-A"), 0.91}, {QStringLiteral("server-B"), 0.12}});
    resolved.resolveIfUnambiguous(0.15);
    QVERIFY(actOf(ActKind::Request, resolved).mayProceed());

    // One unresolved reference among several is still unresolved: a caller that checked only the
    // first would proceed on a target nobody chose.
    CognitiveAct mixed = actOf(ActKind::Request, resolved);
    mixed.references.append(ambiguous);
    QVERIFY(!mixed.isFullyResolved());
    QVERIFY(!mixed.mayProceed());
}

// A correction is evidence, so it resolves what ranking would not -- but only among what was
// actually surfaced.
void TestCognitiveAct::aPersonMayResolveWhatRankingCouldNot()
{
    ReferenceResolution ambiguous
        = referenceTo({{QStringLiteral("server-A"), 0.52}, {QStringLiteral("server-B"), 0.48}});
    ambiguous.resolveIfUnambiguous(0.15);
    QVERIFY(!ambiguous.isResolved());

    QVERIFY(ambiguous.resolveByPerson(QStringLiteral("server-B")));
    QCOMPARE(ambiguous.resolved(), QStringLiteral("server-B"));
    QVERIFY(actOf(ActKind::Request, ambiguous).mayProceed());

    // A target the interpretation never considered is refused: accepting it would let a correction
    // introduce a different act while wearing the name of this one.
    ReferenceResolution other
        = referenceTo({{QStringLiteral("server-A"), 0.52}, {QStringLiteral("server-B"), 0.48}});
    QVERIFY2(!other.resolveByPerson(QStringLiteral("database")),
             "a correction may choose among candidates, not invent one");
    QVERIFY(!other.isResolved());
}

void TestCognitiveAct::everyActKindHasItsOwnLabel()
{
    const QList<ActKind> all{ActKind::Ask,     ActKind::Inform,  ActKind::Request,
                             ActKind::Correct, ActKind::Confirm, ActKind::Reject};

    QSet<QString> labels;
    for (const ActKind kind : all) {
        const QString label = actKindToString(kind);
        QVERIFY2(label != QStringLiteral("unknown"), qPrintable(label));
        labels.insert(label);
    }
    QCOMPARE(labels.size(), all.size());

    // Exactly one kind mutates today. If that ever grows, this is where the growth is noticed
    // rather than discovered by something acting when it should have asked.
    int mutating = 0;
    for (const ActKind kind : all) {
        if (isMutating(kind)) {
            ++mutating;
        }
    }
    QCOMPARE(mutating, 1);
    QVERIFY(isMutating(ActKind::Request));
}

// An interpretation with no source expression is an assertion the parser made on its own authority.
void TestCognitiveAct::anActWithoutItsUtteranceIsNotValid()
{
    CognitiveAct act;
    act.kind = ActKind::Ask;
    QVERIFY(!act.isValid());

    act.utterance = QUuid::createUuid();
    QVERIFY(act.isValid());
}

QTEST_MAIN(TestCognitiveAct)
#include "tst_cognitive_act.moc"
