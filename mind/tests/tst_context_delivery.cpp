// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT

#include "cybou/context/ContextDelivery.h"

#include <QTest>

using namespace cybou;

namespace {

ContextItem item(const QString &id, SensitivityClass sensitivity)
{
    ContextItem it;
    it.conceptId = id;
    it.relevance = 0.5;
    it.privacy = PrivacyClass::Public;
    it.sensitivity = sensitivity;
    it.activationReason = QStringLiteral("seed");
    it.evidence = {QUuid::createUuid()};
    return it;
}

ContextBundle bundleOf(const QList<ContextItem> &items, bool complete = true)
{
    ContextBundle bundle;
    bundle.requestId = QUuid::createUuid();
    bundle.items = items;
    bundle.complete = complete;
    return bundle;
}

QSet<QString> idsOf(const ContextBundle &bundle)
{
    QSet<QString> out;
    for (const ContextItem &it : bundle.items) {
        out.insert(it.conceptId);
    }
    return out;
}

} // namespace

class TestContextDelivery : public QObject
{
    Q_OBJECT

private slots:
    /// B6. A held-back item is shown as held back, never silently omitted.
    void everyActivatedItemGetsExactlyOneDisposition();

    /// B2. Policy produces a different delivered set without altering the bundle.
    void policyNarrowsDeliveryWithoutAlteringTheBundle();

    /// B1. Available and delivered are separately answerable, and differ.
    void availableAndDeliveredAreIndependentlyInspectable();

    /// B7. Being on the same machine is not a permission.
    void aLocalConsumerIsNotTrustedForBeingLocal();

    /// B4. An inspector that forgets leaves no durable trace; a consumer that retains does.
    void recordingFollowsRetentionRatherThanDistance();

    /// B4. The record names what was sent and carries no content.
    void deliveryRecordCarriesProvenanceAndNoContent();

    /// A person's exclusion is distinguishable from a policy hold-back.
    void personalExclusionIsNotReportedAsPolicy();

    /// A plan built over a truncated retrieval is itself partial.
    void planOverIncompleteBundleReportsIncomplete();

    /// The label a person reads distinguishes the dispositions the code distinguishes.
    void everyDispositionHasItsOwnLabel();

    /// The commitment covers the delivered items and nothing else.
    void theDigestCoversDeliveredItemsOnly();

    /// ADR-0033 A9 as a property of the delivery path.
    void secretsAndCredentialsReachNoConsumerAtAll();

    /// Scope keeps its own job at the device boundary.
    void scopeStillDecidesWhatMayLeaveTheDevice();
};

void TestContextDelivery::everyDispositionHasItsOwnLabel()
{
    const QList<Disposition> all{
        Disposition::Delivered,
        Disposition::HeldBackByPolicy,
        Disposition::ExcludedByPerson,
        Disposition::NotSelected,
    };

    QSet<QString> labels;
    for (Disposition disposition : all) {
        const QString label = dispositionToString(disposition);
        QVERIFY(!label.isEmpty());
        QVERIFY2(label != QStringLiteral("unknown"), qPrintable(label));
        labels.insert(label);
    }

    // Two dispositions sharing a label would make a hold-back and a non-selection read the same,
    // which is the distinction B6 exists to keep visible.
    QCOMPARE(labels.size(), all.size());

    QCOMPARE(dispositionToString(Disposition::HeldBackByPolicy),
             QStringLiteral("held-back-by-policy"));
}

void TestContextDelivery::everyActivatedItemGetsExactlyOneDisposition()
{
    const ContextBundle bundle = bundleOf({
        item(QStringLiteral("lemon"), SensitivityClass::Ordinary),
        item(QStringLiteral("episode"), SensitivityClass::Secret),
        item(QStringLiteral("honey"), SensitivityClass::Personal),
    });

    DeliveryPolicy policy;
    policy.ceilingForUntrusted = SensitivityClass::Personal;

    const DeliveryPlan plan = DeliveryPlan::build(
        bundle, policy, {QStringLiteral("mistral"), ConsumerTrust::Untrusted, false, true}, idsOf(bundle), {});

    QCOMPARE(plan.size(), bundle.items.size());

    QSet<QString> seen;
    for (const DeliveryDecision &decision : plan.decisions()) {
        QVERIFY2(!seen.contains(decision.conceptId), qPrintable(decision.conceptId));
        seen.insert(decision.conceptId);
        QVERIFY2(!decision.reason.isEmpty(), qPrintable(decision.conceptId));
    }
    QCOMPARE(seen, idsOf(bundle));

    // The private item is present and named as held back, rather than absent.
    const QList<DeliveryDecision> held = plan.withDisposition(Disposition::HeldBackByPolicy);
    QCOMPARE(held.size(), 1);
    QCOMPARE(held.first().conceptId, QStringLiteral("episode"));
}

void TestContextDelivery::policyNarrowsDeliveryWithoutAlteringTheBundle()
{
    const ContextBundle bundle = bundleOf({
        item(QStringLiteral("lemon"), SensitivityClass::Ordinary),
        item(QStringLiteral("episode"), SensitivityClass::Secret),
    });
    const QList<ContextItem> before = bundle.items;

    DeliveryPolicy permissive;
    permissive.ceilingForUntrusted = SensitivityClass::Credential;
    DeliveryPolicy strict;
    strict.ceilingForUntrusted = SensitivityClass::Ordinary;

    const Destination remote{QStringLiteral("mistral"), ConsumerTrust::Untrusted, false, true};
    const DeliveryPlan open
        = DeliveryPlan::build(bundle, permissive, remote, idsOf(bundle), {});
    const DeliveryPlan closed = DeliveryPlan::build(bundle, strict, remote, idsOf(bundle), {});

    QCOMPARE(open.deliveredIds().size(), 2);
    QCOMPARE(closed.deliveredIds(), QList<QString>{QStringLiteral("lemon")});

    // Two policies, two delivered sets, one unchanged bundle.
    QCOMPARE(bundle.items.size(), before.size());
    for (int i = 0; i < before.size(); ++i) {
        QCOMPARE(bundle.items.at(i).conceptId, before.at(i).conceptId);
        QCOMPARE(bundle.items.at(i).sensitivity, before.at(i).sensitivity);
    }
}

void TestContextDelivery::availableAndDeliveredAreIndependentlyInspectable()
{
    const ContextBundle bundle = bundleOf({
        item(QStringLiteral("lemon"), SensitivityClass::Ordinary),
        item(QStringLiteral("honey"), SensitivityClass::Ordinary),
        item(QStringLiteral("episode"), SensitivityClass::Secret),
    });

    DeliveryPolicy policy;
    policy.ceilingForUntrusted = SensitivityClass::Ordinary;

    // The person selected only one of the two permitted items.
    const DeliveryPlan plan = DeliveryPlan::build(
        bundle, policy, {QStringLiteral("mistral"), ConsumerTrust::Untrusted, false, true}, {QStringLiteral("lemon")}, {});

    const QList<QString> available = plan.availableIds();
    const QList<QString> delivered = plan.deliveredIds();

    QCOMPARE(available.size(), 2);
    QVERIFY(available.contains(QStringLiteral("honey")));
    QVERIFY(!available.contains(QStringLiteral("episode")));

    QCOMPARE(delivered, QList<QString>{QStringLiteral("lemon")});

    // The gap between them is the point: honey was permitted and not sent.
    QVERIFY(available.size() > delivered.size());
}

void TestContextDelivery::aLocalConsumerIsNotTrustedForBeingLocal()
{
    // Personal rather than Secret: no ceiling reaches Secret, so a secret item would be refused
    // to every consumer alike and could not show that trust is what differs here.
    const ContextBundle bundle = bundleOf({
        item(QStringLiteral("episode"), SensitivityClass::Personal),
        item(QStringLiteral("lemon"), SensitivityClass::Ordinary),
    });

    DeliveryPolicy policy;

    // A local plugin, on this machine, with no network anywhere in sight.
    const Destination plugin{QStringLiteral("third-party-plugin"), ConsumerTrust::Untrusted, false, false};
    const DeliveryPlan restricted
        = DeliveryPlan::build(bundle, policy, plugin, idsOf(bundle), {});

    QCOMPARE(restricted.deliveredIds(), QList<QString>{QStringLiteral("lemon")});
    QCOMPARE(restricted.withDisposition(Disposition::HeldBackByPolicy).size(), 1);

    // The same items, the same machine, a consumer the person actually trusts.
    const Destination surface{QStringLiteral("inspector"), ConsumerTrust::Full, false, false};
    QCOMPARE(DeliveryPlan::build(bundle, policy, surface, idsOf(bundle), {}).deliveredIds().size(), 2);

    // Trust decided this, not distance: neither consumer is remote.
    QVERIFY(!plugin.externalBoundary);
    QVERIFY(!surface.externalBoundary);
}

void TestContextDelivery::recordingFollowsRetentionRatherThanDistance()
{
    const ContextBundle bundle = bundleOf({item(QStringLiteral("lemon"), SensitivityClass::Ordinary)});
    DeliveryPolicy policy;

    // An inspector renders and forgets. Recording every render would grow the Journal with use and
    // prove nothing about what the person's data is doing.
    const Destination inspector{QStringLiteral("inspector"), ConsumerTrust::Full, false, false};
    QVERIFY(!requiresRecord(inspector));
    QVERIFY(!recordFor(DeliveryPlan::build(bundle, policy, inspector, idsOf(bundle), {})).has_value());

    // A local model that learns from what it receives is the consequential case, ADR-0033's
    // invalidation has nothing to follow without this record, and it never touches a network.
    const Destination learner{QStringLiteral("local-model"), ConsumerTrust::Bounded, true, false};
    QVERIFY(requiresRecord(learner));
    const auto learned = recordFor(DeliveryPlan::build(bundle, policy, learner, idsOf(bundle), {}));
    QVERIFY(learned.has_value());
    QVERIFY(learned->retained);
    QVERIFY(!learned->externalBoundary);
    QCOMPARE(learned->deliveredConceptIds, QList<QString>{QStringLiteral("lemon")});

    // Crossing an external boundary is recorded on its own account, retention or not.
    const Destination external{QStringLiteral("remote"), ConsumerTrust::Untrusted, false, true};
    QVERIFY(requiresRecord(external));
    QVERIFY(recordFor(DeliveryPlan::build(bundle, policy, external, idsOf(bundle), {})).has_value());
}

void TestContextDelivery::deliveryRecordCarriesProvenanceAndNoContent()
{
    ContextItem lemon = item(QStringLiteral("lemon"), SensitivityClass::Ordinary);
    const QUuid lemonEvidence = lemon.evidence.first();

    const ContextBundle bundle
        = bundleOf({lemon, item(QStringLiteral("episode"), SensitivityClass::Secret)});

    DeliveryPolicy policy;
    policy.ceilingForUntrusted = SensitivityClass::Ordinary;

    const DeliveryPlan plan = DeliveryPlan::build(
        bundle, policy, {QStringLiteral("mistral"), ConsumerTrust::Untrusted, false, true}, idsOf(bundle), {});
    const auto maybeRecord = recordFor(plan);
    QVERIFY(maybeRecord.has_value());
    const DeliveryRecord record = *maybeRecord;

    QVERIFY(record.isValid());
    QCOMPARE(record.requestId, bundle.requestId);
    QCOMPARE(record.destinationId, QStringLiteral("mistral"));
    QVERIFY(record.externalBoundary);
    QCOMPARE(record.deliveredConceptIds, QList<QString>{QStringLiteral("lemon")});
    QVERIFY(record.evidence.contains(lemonEvidence));

    // What was withheld is counted, so a reader can tell a full delivery from a narrowed one.
    QCOMPARE(record.heldBackCount, 1);

    // The record names the withheld item nowhere: a record of a hold-back that carried the held
    // item's identity outward would have delivered it after all.
    QVERIFY(!record.deliveredConceptIds.contains(QStringLiteral("episode")));
}

void TestContextDelivery::personalExclusionIsNotReportedAsPolicy()
{
    const ContextBundle bundle = bundleOf({item(QStringLiteral("lemon"), SensitivityClass::Ordinary)});

    DeliveryPolicy policy;
    policy.ceilingForUntrusted = SensitivityClass::Ordinary;

    const DeliveryPlan plan = DeliveryPlan::build(
        bundle,
        policy,
        {QStringLiteral("mistral"), ConsumerTrust::Untrusted, false, true},
        idsOf(bundle),
        {QStringLiteral("lemon")});

    QVERIFY(plan.deliveredIds().isEmpty());
    QVERIFY(plan.withDisposition(Disposition::HeldBackByPolicy).isEmpty());

    const QList<DeliveryDecision> excluded = plan.withDisposition(Disposition::ExcludedByPerson);
    QCOMPARE(excluded.size(), 1);
    QCOMPARE(excluded.first().conceptId, QStringLiteral("lemon"));

    // Blaming policy for a person's own choice would make policy look stricter than it is.
    QVERIFY(recordFor(plan).has_value());
    QCOMPARE(recordFor(plan)->heldBackCount, 0);
}

void TestContextDelivery::planOverIncompleteBundleReportsIncomplete()
{
    const ContextBundle truncated
        = bundleOf({item(QStringLiteral("lemon"), SensitivityClass::Ordinary)}, false);

    const DeliveryPlan plan = DeliveryPlan::build(
        truncated, {}, {QStringLiteral("inspector"), ConsumerTrust::Full, false, false}, idsOf(truncated), {});

    QVERIFY(!plan.complete());
    QCOMPARE(plan.deliveredIds().size(), 1);

    const ContextBundle whole
        = bundleOf({item(QStringLiteral("lemon"), SensitivityClass::Ordinary)}, true);
    QVERIFY(DeliveryPlan::build(
                whole, {}, {QStringLiteral("inspector"), ConsumerTrust::Full, false, false}, idsOf(whole), {})
                .complete());
}


// The commitment covers what was released and nothing else.
//
// Tested here, on plans built by hand, because through the service the property is invisible: a
// concept held back and the same concept excluded by the person hash identically, so no pair of
// deliveries can distinguish a digest that covers the whole plan from one that does not.
void TestContextDelivery::theDigestCoversDeliveredItemsOnly()
{
    const QUuid shared = QUuid::createUuid();
    const QUuid secret = QUuid::createUuid();

    DeliveryDecision sent;
    sent.conceptId = QStringLiteral("lemon");
    sent.disposition = Disposition::Delivered;
    sent.evidence = {shared};

    DeliveryDecision held;
    held.conceptId = QStringLiteral("medical-episode");
    held.disposition = Disposition::HeldBackByPolicy;
    held.evidence = {secret};

    DeliveryDecision refused;
    refused.conceptId = QStringLiteral("another-episode");
    refused.disposition = Disposition::ExcludedByPerson;
    refused.evidence = {QUuid::createUuid()};

    const QByteArray alone = deliveryDigest({sent});
    QVERIFY(!alone.isEmpty());

    // Withheld material must not move the commitment, whatever it is or why it stayed.
    QCOMPARE(deliveryDigest({sent, held}), alone);
    QCOMPARE(deliveryDigest({held, sent, refused}), alone);

    // And it must still commit to what was delivered: a digest that ignored everything would
    // satisfy every assertion above.
    DeliveryDecision other = sent;
    other.conceptId = QStringLiteral("honey");
    QVERIFY(deliveryDigest({other}) != alone);

    DeliveryDecision reprovenanced = sent;
    reprovenanced.evidence = {QUuid::createUuid()};
    QVERIFY2(deliveryDigest({reprovenanced}) != alone,
             "a commitment that ignored provenance would not identify what was disclosed");
}

// ADR-0033's A9, enforced where the material actually moves.
//
// No trust level has a ceiling that reaches Secret or Credential, so those classifications are
// refused to every consumer -- including the most trusted one, and including a purely local
// consumer with no boundary to cross. A training pipeline cannot receive them to begin with, which
// is a stronger guarantee than asking it to remember not to use them.
void TestContextDelivery::secretsAndCredentialsReachNoConsumerAtAll()
{
    const ContextBundle bundle = bundleOf({
        item(QStringLiteral("password"), SensitivityClass::Credential),
        item(QStringLiteral("diagnosis"), SensitivityClass::Secret),
        item(QStringLiteral("preference"), SensitivityClass::Personal),
    });

    const DeliveryPolicy policy;
    for (const ConsumerTrust trust :
         {ConsumerTrust::Untrusted, ConsumerTrust::Bounded, ConsumerTrust::Full}) {
        for (const bool external : {false, true}) {
            const Destination destination{QStringLiteral("any"), trust, true, external};
            const DeliveryPlan plan
                = DeliveryPlan::build(bundle, policy, destination, idsOf(bundle), {});

            const QList<QString> delivered = plan.deliveredIds();
            QVERIFY2(!delivered.contains(QStringLiteral("password")),
                     qPrintable(consumerTrustToString(trust)));
            QVERIFY2(!delivered.contains(QStringLiteral("diagnosis")),
                     qPrintable(consumerTrustToString(trust)));
        }
    }

    // And the rule is not "refuse everything": a fully trusted consumer still receives ordinary
    // personal material, or the assertions above would hold for the wrong reason.
    const Destination trusted{QStringLiteral("inspector"), ConsumerTrust::Full, false, false};
    const DeliveryPlan plan = DeliveryPlan::build(bundle, policy, trusted, idsOf(bundle), {});
    QCOMPARE(plan.deliveredIds(), QList<QString>{QStringLiteral("preference")});

    // The person is still told what was withheld, and why.
    const QList<DeliveryDecision> held = plan.withDisposition(Disposition::HeldBackByPolicy);
    QCOMPARE(held.size(), 2);
    for (const DeliveryDecision &decision : held) {
        QVERIFY(decision.reason.contains(QStringLiteral("does not permit")));
    }
}

// Sensitivity says who may be shown a thing; scope says where it may go. Both, not either.
//
// Every other fixture here pins privacy to Public so that a refusal can only be about
// classification. This one is the opposite case, and without it the scope rule could be deleted
// with the whole suite still green.
void TestContextDelivery::scopeStillDecidesWhatMayLeaveTheDevice()
{
    ContextItem homebound = item(QStringLiteral("thermostat"), SensitivityClass::Ordinary);
    homebound.privacy = PrivacyClass::Local;
    const ContextBundle bundle = bundleOf({homebound});

    const DeliveryPolicy policy;

    // Ordinary material, fully trusted consumer, and it still may not cross the boundary.
    const Destination away{QStringLiteral("remote"), ConsumerTrust::Full, false, true};
    const DeliveryPlan crossing = DeliveryPlan::build(bundle, policy, away, idsOf(bundle), {});
    QVERIFY2(crossing.deliveredIds().isEmpty(), "a Local-scoped item must not leave the device");
    QCOMPARE(crossing.withDisposition(Disposition::HeldBackByPolicy).size(), 1);

    // The same item, the same consumer, staying here.
    const Destination here{QStringLiteral("inspector"), ConsumerTrust::Full, false, false};
    QCOMPARE(DeliveryPlan::build(bundle, policy, here, idsOf(bundle), {}).deliveredIds(),
             QList<QString>{QStringLiteral("thermostat")});
}

QTEST_MAIN(TestContextDelivery)
#include "tst_context_delivery.moc"
