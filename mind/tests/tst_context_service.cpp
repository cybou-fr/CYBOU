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
#include "cybou/context/ContextDelivery.h"
#include "cybou/storage/Journal.h"

#include <QTemporaryDir>
#include <QCborMap>
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

    // Package C. What a consumer receives, and what it must not learn.
    //
    // The private item is held back, and its identity is absent from the consumer reply entirely --
    // not present with a "held-back" label. That an episode exists is often the sensitive part, and
    // announcing its identity while withholding its content discloses the fact of it to the party
    // policy just refused.
    void aConsumerNeverLearnsWhatWasHeldBackFromIt()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        QVERIFY(journal.isOpen());
        const QDateTime now = QDateTime::currentDateTimeUtc();

        // One ordinary subject and one whose very name is the sensitive part.
        QVERIFY(journal.append(observationOf(QStringLiteral("subject"), QStringLiteral("ordinary"),
                                             now.addSecs(-120), PrivacyClass::Public))
                > 0);
        QVERIFY(journal.append(observationOf(QStringLiteral("subject"),
                                             QStringLiteral("medical-episode"), now.addSecs(-60),
                                             PrivacyClass::Local))
                > 0);

        ContextService service(&journal, dir.filePath(QStringLiteral("cp.cbor")));
        QVERIFY(service.isReady());

        const QVariantMap prepared
            = FabricCodec::decodeMap(service.Prepare({QStringLiteral("subject")}, 0, 0));
        const QString requestId = prepared.value(QStringLiteral("requestId")).toString();
        const int activated = prepared.value(QStringLiteral("items")).toList().size();
        QVERIFY(activated > 1);

        const QVariantMap payload = FabricCodec::decodeMap(service.Deliver(
            requestId, QStringLiteral("third-party-plugin"),
            static_cast<int>(ConsumerTrust::Untrusted), false, false, {}, {}));

        // Nothing in the consumer reply carries a held-back identity, and there is no dispositions
        // list at all for one to hide in.
        QVERIFY2(!payload.contains(QStringLiteral("decisions")),
                 "the consumer reply must not carry the full plan");
        QVERIFY2(!QString::fromUtf8(FabricCodec::encode(payload))
                      .contains(QStringLiteral("medical-episode")),
                 "a held-back concept id reached the consumer");

        const int withheld = payload.value(QStringLiteral("withheldCount")).toInt();
        QVERIFY2(withheld > 0, "the fixture must actually hold something back");

        // The consumer is still owed the knowledge that its answer was narrowed: partial is not
        // empty, and a consumer that believed it had everything would reason as though nothing was
        // withheld. A count is that knowledge without the identities.
        QCOMPARE(payload.value(QStringLiteral("delivered")).toList().size() + withheld, activated);

        // The person sees all of it, through the inspector, with reasons.
        const QVariantMap inspected = FabricCodec::decodeMap(service.Inspect(requestId));
        const QVariantList decisions = inspected.value(QStringLiteral("decisions")).toList();
        QCOMPARE(decisions.size(), activated);

        bool sawHeldBack = false;
        for (const QVariant &entry : decisions) {
            const QVariantMap decision = entry.toMap();
            if (decision.value(QStringLiteral("disposition")).toString()
                == QStringLiteral("held-back-by-policy")) {
                sawHeldBack = true;
                QVERIFY(!decision.value(QStringLiteral("reason")).toString().isEmpty());
            }
        }
        QVERIFY2(sawHeldBack, "the person must be able to see what was withheld");
    }


    // Concept ids from a prepared reply, for use as the person's selection. Delivering with an
    // empty selection leaves every item NotSelected, which makes any assertion about delivered
    // payload vacuously true.
    static QStringList conceptsOf(const QVariantMap &prepared)
    {
        QStringList ids;
        for (const QVariant &entry : prepared.value(QStringLiteral("items")).toList()) {
            ids.append(entry.toMap().value(QStringLiteral("concept")).toString());
        }
        return ids;
    }

    // Package D. A recorded disclosure precedes the disclosure it records.
    //
    // The consumer never receives the payload until a ContextDisclosed contribution committing to
    // it is already in the Journal. Handing the payload over first and asking for the record
    // afterwards would make the record optional in practice: the disclosure has happened either
    // way, and only a well-behaved consumer would produce the evidence.
    void aRetainingConsumerGetsNothingUntilTheDisclosureIsRecorded()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        QVERIFY(journal.isOpen());
        QVERIFY(journal.append(observationOf(QStringLiteral("subject"), QStringLiteral("value"),
                                             QDateTime::currentDateTimeUtc(),
                                             PrivacyClass::Public))
                > 0);

        ContextService service(&journal, dir.filePath(QStringLiteral("cp.cbor")));
        QVERIFY(service.isReady());
        const QVariantMap prepared
            = FabricCodec::decodeMap(service.Prepare({QStringLiteral("subject")}, 0, 0));
        const QString requestId = prepared.value(QStringLiteral("requestId")).toString();

        // A consumer that retains what it receives owes a record.
        const QVariantMap pending = FabricCodec::decodeMap(service.Deliver(
            requestId, QStringLiteral("local-model"), static_cast<int>(ConsumerTrust::Bounded),
            true, false, conceptsOf(prepared), {}));

        QVERIFY(pending.value(QStringLiteral("disclosurePending")).toBool());
        QVERIFY2(!pending.contains(QStringLiteral("delivered")),
                 "the payload must not cross before the record exists");
        QVERIFY(pending.value(QStringLiteral("deliveredCount")).toInt() > 0);

        const QString digest = pending.value(QStringLiteral("disclosureDigest")).toString();
        QVERIFY(!digest.isEmpty());

        // Nothing recorded yet, so nothing is released.
        QVERIFY(service.Release(requestId, QUuid::createUuid().toString(QUuid::WithoutBraces))
                    .isEmpty());

        // A record that commits to the wrong thing is refused, which is what stops the exchange
        // being satisfied by any contribution at all.
        const auto disclosure = [&](const QString &request, const QString &destination,
                                    const QString &commitment) {
            CognitiveEnvelope e;
            e.messageId = QUuid::createUuid();
            e.correlationId = e.messageId;
            e.originOrgan = QStringLiteral("perceptiond");
            e.originNode = QStringLiteral("local");
            e.kind = ContributionKind::ContextDisclosed;
            e.wallTime = QDateTime::currentDateTimeUtc();
            e.privacy = PrivacyClass::Local;
            QCborMap payload;
            payload.insert(QStringLiteral("requestId"), request);
            payload.insert(QStringLiteral("destination"), destination);
            payload.insert(QStringLiteral("digest"), commitment);
            e.payloadCbor = payload.toCborValue().toCbor();
            return e;
        };

        const CognitiveEnvelope wrongDigest
            = disclosure(requestId, QStringLiteral("local-model"), QStringLiteral("00"));
        QVERIFY2(journal.append(wrongDigest) > 0, qPrintable(journal.lastError()));
        QVERIFY2(service
                     .Release(requestId,
                              wrongDigest.messageId.toString(QUuid::WithoutBraces))
                     .isEmpty(),
                 "a record committing to something else must not release this payload");

        const CognitiveEnvelope wrongConsumer
            = disclosure(requestId, QStringLiteral("someone-else"), digest);
        QVERIFY(journal.append(wrongConsumer) > 0);
        QVERIFY(service
                    .Release(requestId, wrongConsumer.messageId.toString(QUuid::WithoutBraces))
                    .isEmpty());

        // An ordinary contribution is not a disclosure, however convenient its id would be.
        const CognitiveEnvelope notADisclosure = observationOf(
            QStringLiteral("subject"), QStringLiteral("other"), QDateTime::currentDateTimeUtc());
        QVERIFY(journal.append(notADisclosure) > 0);
        QVERIFY(service
                    .Release(requestId, notADisclosure.messageId.toString(QUuid::WithoutBraces))
                    .isEmpty());
        // Refused *as not a disclosure*. Asserting only that it was refused would pass on the
        // payload-match check instead, leaving the kind check itself uncovered: an observation has
        // no requestId in its payload, so it would fail the later comparison anyway.
        QVERIFY2(service.LastError().contains(QStringLiteral("not a disclosure")),
                 qPrintable(service.LastError()));

        // The right record releases it.
        const CognitiveEnvelope correct
            = disclosure(requestId, QStringLiteral("local-model"), digest);
        QVERIFY(journal.append(correct) > 0);
        const QVariantMap released = FabricCodec::decodeMap(
            service.Release(requestId, correct.messageId.toString(QUuid::WithoutBraces)));

        const QVariantList delivered = released.value(QStringLiteral("delivered")).toList();
        QCOMPARE(delivered.size(), pending.value(QStringLiteral("deliveredCount")).toInt());
        QVERIFY(!delivered.isEmpty());
        QVERIFY(!delivered.first().toMap().value(QStringLiteral("evidence")).toList().isEmpty());
    }

    // The commitment covers what was released and nothing else.
    //
    // A digest over the whole plan would tie a permanent record to material the consumer never
    // received. Concept spaces are small enough to brute-force, so such a record would be standing
    // evidence about what was withheld, written into the one place that is never erased.
    //
    // Both deliveries run against one journal and one prepared request, so the evidence identities
    // are identical and the only thing that varies is why the private item did not travel. Two
    // separate journals would mint different evidence uuids and the digests would differ for a
    // reason that has nothing to do with withholding.
    void theDisclosureDigestDoesNotDependOnWhatWasWithheld()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        QVERIFY(journal.isOpen());
        const QDateTime now = QDateTime::currentDateTimeUtc();
        QVERIFY(journal.append(observationOf(QStringLiteral("subject"), QStringLiteral("shared"),
                                             now.addSecs(-120), PrivacyClass::Public))
                > 0);
        QVERIFY(journal.append(observationOf(QStringLiteral("subject"), QStringLiteral("private"),
                                             now.addSecs(-60), PrivacyClass::Local))
                > 0);

        ContextService service(&journal, dir.filePath(QStringLiteral("cp.cbor")));
        QVERIFY(service.isReady());
        const QVariantMap prepared
            = FabricCodec::decodeMap(service.Prepare({QStringLiteral("subject")}, 0, 0));
        const QString requestId = prepared.value(QStringLiteral("requestId")).toString();
        const QStringList all = conceptsOf(prepared);

        // Held back by policy: an untrusted consumer may not have the private item.
        const QVariantMap byPolicy = FabricCodec::decodeMap(service.Deliver(
            requestId, QStringLiteral("local-model"), static_cast<int>(ConsumerTrust::Untrusted),
            true, false, all, {}));

        QStringList selected = all;
        QStringList excluded;
        for (const QString &node : all) {
            if (node.contains(QStringLiteral("private"))) {
                excluded.append(node);
                selected.removeAll(node);
            }
        }
        QVERIFY2(!excluded.isEmpty(), "the fixture must contain a private concept");

        // Same item absent, different reason: the person removed it, and the consumer is trusted
        // enough that policy would have allowed it.
        const QVariantMap byPerson = FabricCodec::decodeMap(service.Deliver(
            requestId, QStringLiteral("local-model"), static_cast<int>(ConsumerTrust::Bounded),
            true, false, selected, excluded));

        const QString first = byPolicy.value(QStringLiteral("disclosureDigest")).toString();
        const QString second = byPerson.value(QStringLiteral("disclosureDigest")).toString();

        QVERIFY2(!first.isEmpty(), "a pending disclosure must carry a commitment");
        QVERIFY2(byPolicy.value(QStringLiteral("withheldCount")).toInt() > 0,
                 "the fixture must actually withhold something");
        QCOMPARE(byPolicy.value(QStringLiteral("deliveredCount")).toInt(),
                 byPerson.value(QStringLiteral("deliveredCount")).toInt());

        // Same delivered set, different reason for the absence, same commitment.
        QCOMPARE(second, first);
    }

    // A consumer that owes no record is not made to invent one: an inspector that renders and
    // forgets gets its answer directly.
    void aConsumerThatOwesNoRecordIsNotMadeToWaitForOne()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        QVERIFY(journal.isOpen());
        QVERIFY(journal.append(observationOf(QStringLiteral("subject"), QStringLiteral("value"),
                                             QDateTime::currentDateTimeUtc(),
                                             PrivacyClass::Public))
                > 0);

        ContextService service(&journal, dir.filePath(QStringLiteral("cp.cbor")));
        QVERIFY(service.isReady());
        const QVariantMap prepared
            = FabricCodec::decodeMap(service.Prepare({QStringLiteral("subject")}, 0, 0));
        const QString requestId = prepared.value(QStringLiteral("requestId")).toString();

        const QVariantMap payload = FabricCodec::decodeMap(service.Deliver(
            requestId, QStringLiteral("inspector"), static_cast<int>(ConsumerTrust::Bounded),
            false, false, conceptsOf(prepared), {}));

        QVERIFY(!payload.value(QStringLiteral("disclosurePending")).toBool());
        QVERIFY(!payload.value(QStringLiteral("delivered")).toList().isEmpty());

        // And there is nothing to release, because nothing was withheld pending a record.
        QVERIFY(service.Release(requestId, QUuid::createUuid().toString(QUuid::WithoutBraces))
                    .isEmpty());
        QVERIFY(service.LastError().contains(QStringLiteral("owes no disclosure")));
    }

    // Inspection reports a delivery that happened, not a plan recomputed at inspection time.
    void inspectRefusesARequestThatWasNeverDelivered()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        QVERIFY(journal.isOpen());
        QVERIFY(journal.append(observationOf(QStringLiteral("subject"), QStringLiteral("v"),
                                             QDateTime::currentDateTimeUtc()))
                > 0);

        ContextService service(&journal, dir.filePath(QStringLiteral("cp.cbor")));
        QVERIFY(service.isReady());
        const QString requestId
            = FabricCodec::decodeMap(service.Prepare({QStringLiteral("subject")}, 0, 0))
                  .value(QStringLiteral("requestId"))
                  .toString();

        QVERIFY2(service.Inspect(requestId).isEmpty(),
                 "there is no delivery to inspect yet");
        QVERIFY(service.LastError().contains(QStringLiteral("no delivery")));

        QVERIFY(!service
                     .Deliver(requestId, QStringLiteral("c"),
                              static_cast<int>(ConsumerTrust::Full), false, false, {}, {})
                     .isEmpty());
        QVERIFY2(!service.Inspect(requestId).isEmpty(), "now there is");

        QVERIFY(service.Inspect(QUuid::createUuid().toString(QUuid::WithoutBraces)).isEmpty());
    }

    // The request id is minted before activation, so the bundle actually carries one. A null id
    // makes every DeliveryRecord built from it invalid, which no library test would notice.
    void aPreparedBundleCarriesItsOwnRequestIdentity()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        QVERIFY(journal.isOpen());
        QVERIFY(journal.append(observationOf(QStringLiteral("subject"), QStringLiteral("v"),
                                             QDateTime::currentDateTimeUtc()))
                > 0);

        ContextService service(&journal, dir.filePath(QStringLiteral("cp.cbor")));
        QVERIFY(service.isReady());

        const QString first
            = FabricCodec::decodeMap(service.Prepare({QStringLiteral("subject")}, 0, 0))
                  .value(QStringLiteral("requestId"))
                  .toString();
        const QString second
            = FabricCodec::decodeMap(service.Prepare({QStringLiteral("subject")}, 0, 0))
                  .value(QStringLiteral("requestId"))
                  .toString();

        QVERIFY(!QUuid::fromString(first).isNull());
        QVERIFY(!QUuid::fromString(second).isNull());
        QVERIFY2(first != second, "two requests must not share one identity");

        // The id on the wire is the bundle's own. Minting one for the reply while activating
        // without it would leave every bundle null-identified and every delivery record invalid,
        // with the wire looking perfectly correct.
        const QVariantMap reply
            = FabricCodec::decodeMap(service.Prepare({QStringLiteral("subject")}, 0, 0));
        const QString wireId = reply.value(QStringLiteral("requestId")).toString();
        QVERIFY(!QUuid::fromString(wireId).isNull());
        QVERIFY2(!service
                      .Deliver(wireId, QStringLiteral("c"),
                               static_cast<int>(ConsumerTrust::Full), false, false, {}, {})
                      .isEmpty(),
                 "the id the caller was handed must name the bundle that was frozen");
    }

    // Package B. What was inspected is what gets delivered, or nothing is.
    void deliverRefusesARequestTheProjectionHasMovedPast()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        QVERIFY(journal.isOpen());
        const QDateTime now = QDateTime::currentDateTimeUtc();
        QVERIFY(journal.append(observationOf(QStringLiteral("subject"), QStringLiteral("first"),
                                             now.addSecs(-60)))
                > 0);

        ContextService service(&journal, dir.filePath(QStringLiteral("cp.cbor")));
        QVERIFY(service.isReady());

        const QString requestId
            = FabricCodec::decodeMap(service.Prepare({QStringLiteral("subject")}, 0, 0))
                  .value(QStringLiteral("requestId"))
                  .toString();

        // Delivering against it works, which is what makes the refusal below mean something.
        QVERIFY(!service
                     .Deliver(requestId, QStringLiteral("c"),
                              static_cast<int>(ConsumerTrust::Full), false, false, {}, {})
                     .isEmpty());

        // The world moves on between inspection and delivery.
        const CognitiveEnvelope later
            = observationOf(QStringLiteral("subject"), QStringLiteral("second"), now);
        const quint64 sequence = journal.append(later);
        QVERIFY(sequence > 0);
        service.admitAccepted(later, sequence);

        QVERIFY2(service
                     .Deliver(requestId, QStringLiteral("c"),
                              static_cast<int>(ConsumerTrust::Full), false, false, {}, {})
                     .isEmpty(),
                 "a stale request must be refused, never silently re-activated");
        QVERIFY(service.LastError().contains(QStringLiteral("stale")));

        // An id nobody prepared is refused, and refused *as unknown*. Asserting only that it was
        // refused would pass on the staleness check instead: an unprepared id yields a default
        // record whose cursor happens not to match, so the test would hold while the lookup itself
        // was gone.
        QVERIFY(service
                    .Deliver(QUuid::createUuid().toString(QUuid::WithoutBraces),
                             QStringLiteral("c"), static_cast<int>(ConsumerTrust::Full), false,
                             false, {}, {})
                    .isEmpty());
        QVERIFY2(service.LastError().contains(QStringLiteral("no prepared request")),
                 qPrintable(service.LastError()));
    }

    // An unrecognised trust level is refused rather than defaulted. Defaulting upward would hand a
    // caller full context by sending a number nobody implemented.
    void deliverRefusesATrustLevelItDoesNotKnow()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        QVERIFY(journal.isOpen());
        QVERIFY(journal.append(observationOf(QStringLiteral("subject"), QStringLiteral("value"),
                                             QDateTime::currentDateTimeUtc()))
                > 0);

        ContextService service(&journal, dir.filePath(QStringLiteral("cp.cbor")));
        QVERIFY(service.isReady());
        const QString requestId
            = FabricCodec::decodeMap(service.Prepare({QStringLiteral("subject")}, 0, 0))
                  .value(QStringLiteral("requestId"))
                  .toString();

        QVERIFY(!service
                     .Deliver(requestId, QStringLiteral("c"),
                              static_cast<int>(ConsumerTrust::Full), false, false, {}, {})
                     .isEmpty());

        for (int unknown : {-1, static_cast<int>(ConsumerTrust::Full) + 1, 99}) {
            QVERIFY2(service
                         .Deliver(requestId, QStringLiteral("c"), unknown, false, false, {}, {})
                         .isEmpty(),
                     qPrintable(QStringLiteral("trust %1 was answered").arg(unknown)));
        }

        // And a nameless destination is refused too: a delivery nobody can name is one nobody can
        // later inspect or hold to a policy.
        QVERIFY(service
                    .Deliver(requestId, QString(), static_cast<int>(ConsumerTrust::Full), false,
                             false, {}, {})
                    .isEmpty());
    }

    // Whether a record is owed follows the consumer, and contextd reports it without writing it.
    void deliverReportsWhetherARecordIsOwed()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        QVERIFY(journal.isOpen());
        QVERIFY(journal.append(observationOf(QStringLiteral("subject"), QStringLiteral("value"),
                                             QDateTime::currentDateTimeUtc()))
                > 0);

        ContextService service(&journal, dir.filePath(QStringLiteral("cp.cbor")));
        QVERIFY(service.isReady());
        const QString requestId
            = FabricCodec::decodeMap(service.Prepare({QStringLiteral("subject")}, 0, 0))
                  .value(QStringLiteral("requestId"))
                  .toString();
        const QByteArray headBefore = journal.head();

        const auto owed = [&](bool retains, bool external) {
            return FabricCodec::decodeMap(service.Deliver(
                       requestId, QStringLiteral("c"),
                       static_cast<int>(ConsumerTrust::Bounded), retains, external, {}, {}))
                .value(QStringLiteral("recordRequired"))
                .toBool();
        };

        QVERIFY2(!owed(false, false), "an inspector that forgets owes nothing");
        QVERIFY2(owed(true, false), "a local consumer that retains owes a record");
        QVERIFY2(owed(false, true), "crossing an external boundary owes one on its own account");

        // Reported, never written: contextd owns no writes, and these deliveries did not grow the
        // Journal by a single contribution.
        QCOMPARE(journal.head(), headBefore);
    }
};

QTEST_MAIN(TestContextService)
#include "tst_context_service.moc"
