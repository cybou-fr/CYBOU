// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT

#include "cybou/context/ContextDelivery.h"

#include <QTest>

using namespace cybou;

namespace {

ContextItem item(const QString &id, PrivacyClass privacy)
{
    ContextItem it;
    it.conceptId = id;
    it.relevance = 0.5;
    it.privacy = privacy;
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

    /// A local destination is not filtered: policy exists for what leaves the machine.
    void localDeliveryIsNotFilteredByRemotePolicy();

    /// B4. The record names what was sent and carries no content.
    void deliveryRecordCarriesProvenanceAndNoContent();

    /// A person's exclusion is distinguishable from a policy hold-back.
    void personalExclusionIsNotReportedAsPolicy();

    /// A plan built over a truncated retrieval is itself partial.
    void planOverIncompleteBundleReportsIncomplete();

    /// The label a person reads distinguishes the dispositions the code distinguishes.
    void everyDispositionHasItsOwnLabel();
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
        item(QStringLiteral("lemon"), PrivacyClass::Public),
        item(QStringLiteral("episode"), PrivacyClass::Local),
        item(QStringLiteral("honey"), PrivacyClass::Household),
    });

    DeliveryPolicy policy;
    policy.maxPrivacyForRemote = PrivacyClass::Household;

    const DeliveryPlan plan = DeliveryPlan::build(
        bundle, policy, {QStringLiteral("mistral"), true}, idsOf(bundle), {});

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
        item(QStringLiteral("lemon"), PrivacyClass::Public),
        item(QStringLiteral("episode"), PrivacyClass::Local),
    });
    const QList<ContextItem> before = bundle.items;

    DeliveryPolicy permissive;
    permissive.maxPrivacyForRemote = PrivacyClass::Local;
    DeliveryPolicy strict;
    strict.maxPrivacyForRemote = PrivacyClass::Public;

    const Destination remote{QStringLiteral("mistral"), true};
    const DeliveryPlan open
        = DeliveryPlan::build(bundle, permissive, remote, idsOf(bundle), {});
    const DeliveryPlan closed = DeliveryPlan::build(bundle, strict, remote, idsOf(bundle), {});

    QCOMPARE(open.deliveredIds().size(), 2);
    QCOMPARE(closed.deliveredIds(), QList<QString>{QStringLiteral("lemon")});

    // Two policies, two delivered sets, one unchanged bundle.
    QCOMPARE(bundle.items.size(), before.size());
    for (int i = 0; i < before.size(); ++i) {
        QCOMPARE(bundle.items.at(i).conceptId, before.at(i).conceptId);
        QCOMPARE(bundle.items.at(i).privacy, before.at(i).privacy);
    }
}

void TestContextDelivery::availableAndDeliveredAreIndependentlyInspectable()
{
    const ContextBundle bundle = bundleOf({
        item(QStringLiteral("lemon"), PrivacyClass::Public),
        item(QStringLiteral("honey"), PrivacyClass::Public),
        item(QStringLiteral("episode"), PrivacyClass::Local),
    });

    DeliveryPolicy policy;
    policy.maxPrivacyForRemote = PrivacyClass::Public;

    // The person selected only one of the two permitted items.
    const DeliveryPlan plan = DeliveryPlan::build(
        bundle, policy, {QStringLiteral("mistral"), true}, {QStringLiteral("lemon")}, {});

    const QList<QString> available = plan.availableIds();
    const QList<QString> delivered = plan.deliveredIds();

    QCOMPARE(available.size(), 2);
    QVERIFY(available.contains(QStringLiteral("honey")));
    QVERIFY(!available.contains(QStringLiteral("episode")));

    QCOMPARE(delivered, QList<QString>{QStringLiteral("lemon")});

    // The gap between them is the point: honey was permitted and not sent.
    QVERIFY(available.size() > delivered.size());
}

void TestContextDelivery::localDeliveryIsNotFilteredByRemotePolicy()
{
    const ContextBundle bundle = bundleOf({
        item(QStringLiteral("episode"), PrivacyClass::Local),
    });

    DeliveryPolicy strict;
    strict.maxPrivacyForRemote = PrivacyClass::Public;

    const DeliveryPlan local = DeliveryPlan::build(
        bundle, strict, {QStringLiteral("inspector"), false}, idsOf(bundle), {});
    QCOMPARE(local.deliveredIds(), QList<QString>{QStringLiteral("episode")});

    // The same policy and the same item, going somewhere else.
    const DeliveryPlan remote = DeliveryPlan::build(
        bundle, strict, {QStringLiteral("mistral"), true}, idsOf(bundle), {});
    QVERIFY(remote.deliveredIds().isEmpty());
    QCOMPARE(remote.withDisposition(Disposition::HeldBackByPolicy).size(), 1);
}

void TestContextDelivery::deliveryRecordCarriesProvenanceAndNoContent()
{
    ContextItem lemon = item(QStringLiteral("lemon"), PrivacyClass::Public);
    const QUuid lemonEvidence = lemon.evidence.first();

    const ContextBundle bundle
        = bundleOf({lemon, item(QStringLiteral("episode"), PrivacyClass::Local)});

    DeliveryPolicy policy;
    policy.maxPrivacyForRemote = PrivacyClass::Public;

    const DeliveryPlan plan = DeliveryPlan::build(
        bundle, policy, {QStringLiteral("mistral"), true}, idsOf(bundle), {});
    const DeliveryRecord record = recordFor(plan);

    QVERIFY(record.isValid());
    QCOMPARE(record.requestId, bundle.requestId);
    QCOMPARE(record.destinationId, QStringLiteral("mistral"));
    QVERIFY(record.remote);
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
    const ContextBundle bundle = bundleOf({item(QStringLiteral("lemon"), PrivacyClass::Public)});

    DeliveryPolicy policy;
    policy.maxPrivacyForRemote = PrivacyClass::Public;

    const DeliveryPlan plan = DeliveryPlan::build(
        bundle,
        policy,
        {QStringLiteral("mistral"), true},
        idsOf(bundle),
        {QStringLiteral("lemon")});

    QVERIFY(plan.deliveredIds().isEmpty());
    QVERIFY(plan.withDisposition(Disposition::HeldBackByPolicy).isEmpty());

    const QList<DeliveryDecision> excluded = plan.withDisposition(Disposition::ExcludedByPerson);
    QCOMPARE(excluded.size(), 1);
    QCOMPARE(excluded.first().conceptId, QStringLiteral("lemon"));

    // Blaming policy for a person's own choice would make policy look stricter than it is.
    QCOMPARE(recordFor(plan).heldBackCount, 0);
}

void TestContextDelivery::planOverIncompleteBundleReportsIncomplete()
{
    const ContextBundle truncated
        = bundleOf({item(QStringLiteral("lemon"), PrivacyClass::Public)}, false);

    const DeliveryPlan plan = DeliveryPlan::build(
        truncated, {}, {QStringLiteral("inspector"), false}, idsOf(truncated), {});

    QVERIFY(!plan.complete());
    QCOMPARE(plan.deliveredIds().size(), 1);

    const ContextBundle whole
        = bundleOf({item(QStringLiteral("lemon"), PrivacyClass::Public)}, true);
    QVERIFY(DeliveryPlan::build(
                whole, {}, {QStringLiteral("inspector"), false}, idsOf(whole), {})
                .complete());
}

QTEST_MAIN(TestContextDelivery)
#include "tst_context_delivery.moc"
