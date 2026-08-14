// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT
//
// What Mind associates, and what that is not allowed to mean.
//
// ADR-0029 deliberately does not freeze the ranking formula — that is expected to improve. It
// freezes the properties around it, and these are those properties. A change to how relevance is
// scored should leave every one of these passing; if it does not, the change is not an improvement
// to ranking but an alteration of what associative memory is.

#include "cybou/context/AssociativeProjection.h"

#include <QTest>

using namespace cybou;

namespace {

ConceptNode node(const QString &id, ConceptKind kind = ConceptKind::Subject)
{
    ConceptNode created;
    created.id = id;
    created.kind = kind;
    created.evidence = {QUuid::createUuid()};
    return created;
}

Association edge(
    const QString &from,
    const QString &to,
    double strength,
    RelationType type = RelationType::UsedWith,
    AssociationOrigin origin = AssociationOrigin::Observed)
{
    Association a;
    a.from = from;
    a.to = to;
    a.strength = strength;
    a.type = type;
    a.origin = origin;
    a.evidence = {QUuid::createUuid()};
    return a;
}

/// The example from ADR-0029, which is worth using because it is the one a person would recognise.
AssociativeProjection lemonGraph()
{
    AssociativeProjection graph;
    for (const QString &id :
         {QStringLiteral("lemon"), QStringLiteral("citrus"), QStringLiteral("sour"),
          QStringLiteral("yellow"), QStringLiteral("honey"), QStringLiteral("ginger"),
          QStringLiteral("tea")}) {
        graph.addConcept(node(id));
    }
    graph.addAssociation(edge(QStringLiteral("lemon"), QStringLiteral("citrus"), 0.94,
                              RelationType::IsA));
    graph.addAssociation(edge(QStringLiteral("lemon"), QStringLiteral("sour"), 0.90,
                              RelationType::HasValue));
    graph.addAssociation(edge(QStringLiteral("lemon"), QStringLiteral("yellow"), 0.88,
                              RelationType::HasValue));
    graph.addAssociation(edge(QStringLiteral("lemon"), QStringLiteral("honey"), 0.83));
    graph.addAssociation(edge(QStringLiteral("honey"), QStringLiteral("ginger"), 0.78));
    graph.addAssociation(edge(QStringLiteral("ginger"), QStringLiteral("tea"), 0.71));
    return graph;
}

} // namespace

class TestAssociativeProjection : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    // A1: the same graph, seeds and budget produce the same bundle.
    //
    // A memory that answers differently each time it is asked the same question cannot be audited,
    // and every other property here is worth less without this one. Hash iteration order is the
    // usual way this is lost, which is why edges are sorted before they are walked.
    void theSameQuestionGetsTheSameAnswer()
    {
        const AssociativeProjection graph = lemonGraph();
        const ActivationBudget budget;

        const ContextBundle first = graph.activate({QStringLiteral("lemon")}, budget);
        for (int i = 0; i < 8; ++i) {
            const ContextBundle again = graph.activate({QStringLiteral("lemon")}, budget);
            QCOMPARE(again.items.size(), first.items.size());
            for (int n = 0; n < first.items.size(); ++n) {
                QCOMPARE(again.items.at(n).conceptId, first.items.at(n).conceptId);
                QCOMPARE(again.items.at(n).relevance, first.items.at(n).relevance);
                QCOMPARE(again.items.at(n).activationReason, first.items.at(n).activationReason);
            }
        }
    }

    // The retrieval a person would recognise: thinking of lemon brings honey with it, and brings it
    // less strongly than citrus.
    void thinkingOfOneThingBringsWhatIsRelatedToIt()
    {
        const ContextBundle bundle =
            lemonGraph().activate({QStringLiteral("lemon")}, ActivationBudget{});
        QVERIFY(bundle.complete);

        QStringList retrieved;
        for (const ContextItem &item : bundle.items) {
            retrieved.append(item.conceptId);
        }
        QVERIFY(retrieved.contains(QStringLiteral("citrus")));
        QVERIFY(retrieved.contains(QStringLiteral("honey")));
        QVERIFY2(
            retrieved.contains(QStringLiteral("ginger")),
            "reached at depth two, through honey");

        // The seed itself ranks first, and a thing reached through two hops ranks below one reached
        // directly - relevance decays with distance rather than being flat.
        QCOMPARE(bundle.items.first().conceptId, QStringLiteral("lemon"));
        const auto relevanceOf = [&bundle](const QString &id) {
            for (const ContextItem &item : bundle.items) {
                if (item.conceptId == id) {
                    return item.relevance;
                }
            }
            return -1.0;
        };
        QVERIFY(relevanceOf(QStringLiteral("citrus")) > relevanceOf(QStringLiteral("honey")));
        QVERIFY(relevanceOf(QStringLiteral("honey")) > relevanceOf(QStringLiteral("ginger")));
    }

    // A2: every budget dimension is a hard stop, and A6: a truncated retrieval says so.
    //
    // Returning a short list and calling it complete is the failure this whole substrate keeps
    // finding in itself - partial reported as whole. Here it would mean "nothing else is relevant"
    // when the truth is "I stopped looking".
    void activationIsBoundedAndSaysWhenItStopped()
    {
        const AssociativeProjection graph = lemonGraph();

        ActivationBudget tight;
        tight.maxNodes = 3;
        const ContextBundle limited = graph.activate({QStringLiteral("lemon")}, tight);
        QVERIFY(limited.items.size() <= 3);
        QVERIFY2(!limited.complete, "a retrieval that hit its budget is not complete");

        ActivationBudget shallow;
        shallow.maxDepth = 1;
        const ContextBundle nearby = graph.activate({QStringLiteral("lemon")}, shallow);
        QStringList retrieved;
        for (const ContextItem &item : nearby.items) {
            retrieved.append(item.conceptId);
        }
        QVERIFY(retrieved.contains(QStringLiteral("honey")));
        QVERIFY2(
            !retrieved.contains(QStringLiteral("ginger")),
            "depth one must not reach what is two hops away");

        ActivationBudget fewEdges;
        fewEdges.maxEdges = 2;
        QVERIFY(!graph.activate({QStringLiteral("lemon")}, fewEdges).complete);
    }

    // A12: every item can answer "why was I retrieved?".
    //
    // Structurally, from the graph, without a model being asked to compose a plausible story. A
    // memory that needs one to explain its own retrieval has already given away the property this
    // layer exists to keep - and a generated explanation of a retrieval is not evidence about it.
    void everyRetrievedItemCanSayWhy()
    {
        const ContextBundle bundle =
            lemonGraph().activate({QStringLiteral("lemon")}, ActivationBudget{});

        for (const ContextItem &item : bundle.items) {
            QVERIFY2(
                !item.activationReason.isEmpty(),
                qPrintable(QStringLiteral("%1 was retrieved without a reason").arg(item.conceptId)));
            QVERIFY2(!item.evidence.isEmpty(), "a concept must name the contributions behind it");
        }

        // And the reason is the actual path, naming the relation and its origin - the answer to
        // "why did you think of honey?" is a traversal, not a sentence about lemons.
        for (const ContextItem &item : bundle.items) {
            if (item.conceptId == QStringLiteral("honey")) {
                QVERIFY(item.activationReason.contains(QStringLiteral("lemon")));
                QVERIFY(item.activationReason.contains(QStringLiteral("used-with")));
                QVERIFY(item.activationReason.contains(QStringLiteral("observed")));
            }
        }
    }

    // An association's origin is carried, never flattened. "lemon is yellow" and "lemon makes people
    // kinder" may both be in the graph; they must not be indistinguishable, and this organ does not
    // adjudicate between them - for epistemic force a caller asks epistemicd.
    void anAssociationRemembersWhereItCameFrom()
    {
        AssociativeProjection graph;
        graph.addConcept(node(QStringLiteral("lemon")));
        graph.addConcept(node(QStringLiteral("yellow")));
        graph.addConcept(node(QStringLiteral("kindness")));

        QVERIFY(graph.addAssociation(edge(
            QStringLiteral("lemon"), QStringLiteral("yellow"), 0.9, RelationType::HasValue,
            AssociationOrigin::Observed)));
        QVERIFY(graph.addAssociation(edge(
            QStringLiteral("lemon"), QStringLiteral("kindness"), 0.9, RelationType::CoOccursWith,
            AssociationOrigin::ModelSuggested)));

        const ContextBundle bundle =
            graph.activate({QStringLiteral("lemon")}, ActivationBudget{});
        for (const ContextItem &item : bundle.items) {
            if (item.conceptId == QStringLiteral("yellow")) {
                QVERIFY(item.activationReason.contains(QStringLiteral("observed")));
            }
            if (item.conceptId == QStringLiteral("kindness")) {
                QVERIFY2(
                    item.activationReason.contains(QStringLiteral("model-suggested")),
                    "a model's suggestion must not read like an observation");
            }
        }
    }

    // An edge to a concept that does not exist is refused, because it would produce a retrieval
    // nobody could explain - and explanation is what this graph is for.
    void anEdgeToNowhereIsRefused()
    {
        AssociativeProjection graph;
        graph.addConcept(node(QStringLiteral("lemon")));
        QVERIFY(!graph.addAssociation(
            edge(QStringLiteral("lemon"), QStringLiteral("nonexistent"), 0.9)));
        QVERIFY(!graph.addAssociation(
            edge(QStringLiteral("nonexistent"), QStringLiteral("lemon"), 0.9)));
        QCOMPARE(graph.conceptCount(), 1);
    }

    // Seeding with something unknown answers empty and complete: nothing is related to it, which is
    // a different fact from having stopped looking.
    void anUnknownSeedIsAnsweredNotRefused()
    {
        const ContextBundle bundle =
            lemonGraph().activate({QStringLiteral("bicycle")}, ActivationBudget{});
        QVERIFY(bundle.items.isEmpty());
        QVERIFY2(bundle.complete, "nothing related is complete; it is not a truncated search");
    }
};

QTEST_MAIN(TestAssociativeProjection)
#include "tst_associative_projection.moc"
