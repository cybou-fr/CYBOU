// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT
//
// The associative owner: what the service adds over the graph it holds.
//
// The graph's own properties are covered separately. What is new here is that the graph is derived
// from the biography rather than handed to it — which is what makes association a projection of
// what happened rather than a second, unaccountable memory.

#include "ContextService.h"

#include "cybou/fabric/FabricCodec.h"
#include "cybou/protocol/Observation.h"
#include "cybou/storage/Journal.h"

#include <QTemporaryDir>
#include <QTest>

using namespace cybou;

namespace {

CognitiveEnvelope observationOf(
    const QString &subject,
    const QString &value,
    const QDateTime &acquiredAt,
    PrivacyClass privacy = PrivacyClass::Local)
{
    ObservationV1 observation;
    observation.sourceId = QStringLiteral("nixos.system");
    observation.subject = subject;
    observation.value = QCborValue(value);
    observation.acquiredAt = acquiredAt;
    observation.freshnessUntil = acquiredAt.addSecs(3600);
    observation.provenance = QStringLiteral("test");

    CognitiveEnvelope envelope;
    envelope.messageId = QUuid::createUuid();
    envelope.correlationId = envelope.messageId;
    envelope.originOrgan = QStringLiteral("perceptiond");
    envelope.originNode = QStringLiteral("local");
    envelope.kind = ContributionKind::Observation;
    envelope.wallTime = acquiredAt;
    envelope.confidence = 1.0;
    envelope.privacy = privacy;
    envelope.payloadCbor = encodeObservation(observation);
    return envelope;
}

} // namespace

class TestContextService : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    // The graph is derived from what was accepted, not asserted independently. That is what makes
    // it a projection rather than a second memory with its own rules.
    void theGraphIsBuiltFromTheBiography()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        QVERIFY(journal.isOpen());
        const QDateTime now = QDateTime::currentDateTimeUtc();

        QVERIFY(journal.append(observationOf(
                    QStringLiteral("current-system"), QStringLiteral("abc"), now.addSecs(-60)))
                > 0);

        ContextService service(&journal, dir.filePath(QStringLiteral("cp.cbor")));
        QVERIFY2(service.isReady(), qPrintable(service.startupError()));
        QCOMPARE(service.Cursor(), 1u);

        const ContextBundle bundle =
            service.projection().activate({QStringLiteral("current-system")}, ActivationBudget{});
        QVERIFY(bundle.complete);

        QStringList retrieved;
        for (const ContextItem &item : bundle.items) {
            retrieved.append(item.conceptId);
        }
        QVERIFY2(
            retrieved.contains(QStringLiteral("abc")),
            "the observed value is reachable from its subject");
    }

    // A9: a concept inherits the privacy of the contribution it came from.
    //
    // A concept more permissive than its evidence would be a way to launder a private observation
    // into a retrievable one, which is the sort of leak that looks like a feature until someone
    // notices what is in the results.
    void aConceptInheritsThePrivacyOfItsEvidence()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        QVERIFY(journal.isOpen());
        const QDateTime now = QDateTime::currentDateTimeUtc();

        QVERIFY(journal.append(observationOf(
                    QStringLiteral("private-subject"), QStringLiteral("private-value"),
                    now.addSecs(-60), PrivacyClass::Local))
                > 0);

        ContextService service(&journal, dir.filePath(QStringLiteral("cp.cbor")));
        QVERIFY(service.isReady());

        const ContextBundle bundle =
            service.projection().activate({QStringLiteral("private-subject")}, ActivationBudget{});
        QVERIFY(!bundle.items.isEmpty());
        for (const ContextItem &item : bundle.items) {
            QCOMPARE(item.privacy, PrivacyClass::Local);
            QVERIFY2(!item.evidence.isEmpty(), "a concept must name what it was derived from");
        }
    }

    // A3: deleting the checkpoint and rebuilding gives an observationally equivalent result.
    //
    // The checkpoint is a cache of a cache. Losing it must cost speed and nothing else, which is the
    // same contract epistemicd's checkpoint has and the reason neither is allowed to become an
    // authority.
    void rebuildingWithoutACheckpointAnswersTheSame()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        QVERIFY(journal.isOpen());
        const QDateTime now = QDateTime::currentDateTimeUtc();
        for (int i = 0; i < 5; ++i) {
            QVERIFY(journal.append(observationOf(
                        QStringLiteral("subject-%1").arg(i % 2),
                        QStringLiteral("value-%1").arg(i),
                        now.addSecs(-600 + i * 60)))
                    > 0);
        }

        const QString checkpoint = dir.filePath(QStringLiteral("cp.cbor"));
        ContextService warm(&journal, checkpoint);
        QVERIFY(warm.isReady());
        const ContextBundle before =
            warm.projection().activate({QStringLiteral("subject-0")}, ActivationBudget{});

        QVERIFY(QFile::remove(checkpoint));

        ContextService cold(&journal, checkpoint);
        QVERIFY2(cold.isReady(), qPrintable(cold.startupError()));
        const ContextBundle after =
            cold.projection().activate({QStringLiteral("subject-0")}, ActivationBudget{});

        QCOMPARE(cold.Cursor(), warm.Cursor());
        QCOMPARE(after.items.size(), before.items.size());
        for (int i = 0; i < before.items.size(); ++i) {
            QCOMPARE(after.items.at(i).conceptId, before.items.at(i).conceptId);
            QCOMPARE(after.items.at(i).relevance, before.items.at(i).relevance);
        }
    }

    // A7: an erasure invalidates the associative projection.
    //
    // The graph is derived state, so it obeys the erasure epoch like every other projection. A
    // checkpoint predating an erasure may hold a concept whose evidence has since been redacted,
    // and finding out which one would need exactly the payload that is gone.
    void anErasureDiscardsTheCheckpointAndRebuilds()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        QVERIFY(journal.isOpen());
        const QDateTime now = QDateTime::currentDateTimeUtc();

        const CognitiveEnvelope doomed = observationOf(
            QStringLiteral("current-system"), QStringLiteral("forget-me"), now.addSecs(-120));
        QVERIFY(journal.append(doomed) > 0);
        QVERIFY(journal.append(observationOf(
                    QStringLiteral("other"), QStringLiteral("keep-me"), now.addSecs(-60)))
                > 0);

        const QString checkpoint = dir.filePath(QStringLiteral("cp.cbor"));
        {
            ContextService warm(&journal, checkpoint);
            QVERIFY(warm.isReady());
            QCOMPARE(warm.Cursor(), 2u);
        }
        QVERIFY(QFile::exists(checkpoint));

        QVERIFY(journal.requestErasure(doomed.messageId, QStringLiteral("UserRequested")) > 0);
        QVERIFY(journal.applyErasure(doomed.messageId));

        ContextService rebuilt(&journal, checkpoint);
        QVERIFY2(rebuilt.isReady(), qPrintable(rebuilt.startupError()));

        // The erased observation no longer decodes, so its concepts are simply not in the rebuilt
        // graph. Nothing about the forgotten value survives to be retrieved.
        const ContextBundle bundle =
            rebuilt.projection().activate({QStringLiteral("current-system")}, ActivationBudget{});
        for (const ContextItem &item : bundle.items) {
            QVERIFY2(
                item.conceptId != QStringLiteral("forget-me"),
                "an erased value must not survive in the association graph");
        }

        // And what was never erased is still there.
        const ContextBundle kept =
            rebuilt.projection().activate({QStringLiteral("other")}, ActivationBudget{});
        QStringList retrieved;
        for (const ContextItem &item : kept.items) {
            retrieved.append(item.conceptId);
        }
        QVERIFY(retrieved.contains(QStringLiteral("keep-me")));
    }

    // A6 at the service boundary: a projection that cannot answer refuses rather than returning an
    // empty bundle. Empty means nothing is related, which is a fact this service does not have.
    void anUnreadyProjectionRefusesRatherThanAnsweringEmpty()
    {
        QTemporaryDir dir;

        // No store at all, which is the same condition the constructor guards: a projection with
        // nothing to derive from has no facts, and must not present that as "nothing is related".
        ContextService service(nullptr, dir.filePath(QStringLiteral("cp.cbor")));
        QVERIFY2(!service.isReady(), "a service with no journal is not ready");
        QVERIFY(service.Activate({QStringLiteral("anything")}, 0, 0).isEmpty());
    }

    // A2 at the service boundary: a caller may ask for less than the budget, never for more. A
    // limit a caller can raise is not a limit.
    void aCallerCannotRaiseTheActivationBudget()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        QVERIFY(journal.isOpen());
        const QDateTime now = QDateTime::currentDateTimeUtc();
        for (int i = 0; i < 40; ++i) {
            QVERIFY(journal.append(observationOf(
                        QStringLiteral("subject"), QStringLiteral("value-%1").arg(i),
                        now.addSecs(-4000 + i * 60)))
                    > 0);
        }

        ContextService service(&journal, dir.filePath(QStringLiteral("cp.cbor")));
        QVERIFY(service.isReady());

        const QVariantMap generous = FabricCodec::decodeMap(
            service.Activate({QStringLiteral("subject")}, 10000, 100));
        QVERIFY2(
            generous.value(QStringLiteral("items")).toList().size() <= ActivationBudget{}.maxNodes,
            "asking for more than the default must not raise the ceiling");
    }
};

QTEST_MAIN(TestContextService)
#include "tst_context_service.moc"
