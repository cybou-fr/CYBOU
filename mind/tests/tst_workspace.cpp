// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/workspace/Workspace.h"
#include "WorkspaceService.h"
#include "cybou/fabric/FabricCodec.h"
#include "cybou/storage/Journal.h"

#include <QSignalSpy>
#include <QTemporaryDir>
#include <QTest>

using namespace cybou;

namespace {

CognitiveEnvelope observation(
    const QString &organ, const QUuid &correlation, const QDateTime &when)
{
    CognitiveEnvelope e;
    e.messageId = QUuid::createUuid();
    e.correlationId = correlation.isNull() ? e.messageId : correlation;
    e.originOrgan = organ;
    e.kind = ContributionKind::Observation;
    e.wallTime = when;
    e.privacy = PrivacyClass::Node;
    return e;
}

CognitiveEnvelope derived(
    ContributionKind kind,
    const QString &organ,
    const CognitiveEnvelope &cause,
    const QDateTime &when)
{
    CognitiveEnvelope e;
    e.messageId = QUuid::createUuid();
    e.correlationId = cause.correlationId;
    e.causationId = cause.messageId;
    e.originOrgan = organ;
    e.kind = kind;
    e.wallTime = when;
    e.privacy = cause.privacy;
    return e;
}

} // namespace

class TestWorkspace : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    void nothingEntersAttentionWithoutBeingRemembered()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        Workspace workspace(&journal);

        CognitiveEnvelope broken;
        QVERIFY(!workspace.publish(broken));
        QVERIFY(workspace.moment().isEmpty());
    }

    void contributionsGroupIntoCoalitions()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        Workspace workspace(&journal);

        const QDateTime now = QDateTime::currentDateTimeUtc();
        const QUuid thread = QUuid::createUuid();
        const CognitiveEnvelope root = observation(QStringLiteral("sensord"), thread, now);
        QVERIFY(workspace.publish(root));
        QVERIFY(workspace.publish(derived(
            ContributionKind::Hypothesis, QStringLiteral("modeld"), root, now)));
        QVERIFY(workspace.publish(observation(
            QStringLiteral("sensord"), QUuid(), now)));

        const auto coalitions = workspace.coalitions(now);
        QCOMPARE(coalitions.size(), 2);
        const Coalition shared = coalitions.at(0).correlationId == thread
                                     ? coalitions.at(0)
                                     : coalitions.at(1);
        QCOMPARE(shared.members.size(), 2);
        QCOMPARE(shared.organs().size(), 2);
    }

    void urgencyOutranksAge()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        Workspace workspace(&journal);

        const QDateTime now = QDateTime::currentDateTimeUtc();
        QVERIFY(workspace.publish(observation(
            QStringLiteral("sensord"), QUuid(), now.addSecs(-60))));

        const CognitiveEnvelope symptom = observation(
            QStringLiteral("sensord"), QUuid(), now);
        QVERIFY(workspace.publish(symptom));
        const CognitiveEnvelope need = derived(
            ContributionKind::NeedSignal, QStringLiteral("healthd"), symptom, now);
        QVERIFY(workspace.publish(need));

        QCOMPARE(workspace.focus(now).correlationId, symptom.correlationId);
    }

    void attentionDecays()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        Workspace workspace(&journal);

        const QDateTime now = QDateTime::currentDateTimeUtc();
        const CognitiveEnvelope root = observation(QStringLiteral("sensord"), QUuid(), now);
        QVERIFY(workspace.publish(root));
        QVERIFY(workspace.publish(derived(
            ContributionKind::Decision, QStringLiteral("plannerd"), root, now)));

        const double fresh = workspace.focus(now).salience;
        const double later = workspace.focus(now.addSecs(600)).salience;
        QVERIFY(later < fresh);
        QVERIFY(later > 0.0);
    }

    void theMomentIsBoundedButNothingIsLost()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        Workspace workspace(&journal, 4);

        const QDateTime now = QDateTime::currentDateTimeUtc();
        for (int i = 0; i < 10; ++i) {
            QVERIFY(workspace.publish(observation(
                QStringLiteral("sensord"), QUuid(), now)));
        }

        QCOMPARE(workspace.moment().size(), 4);
        QCOMPARE(journal.count(), 10u);
    }

    void aShiftOfAttentionIsAnnouncedOnce()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        Workspace workspace(&journal);
        QSignalSpy spy(&workspace, &Workspace::focusChanged);

        const QDateTime now = QDateTime::currentDateTimeUtc();
        const QUuid thread = QUuid::createUuid();
        const CognitiveEnvelope root = observation(QStringLiteral("sensord"), thread, now);
        QVERIFY(workspace.publish(root));
        QCOMPARE(spy.count(), 1);

        QVERIFY(workspace.publish(derived(
            ContributionKind::Hypothesis, QStringLiteral("modeld"), root, now)));
        QCOMPARE(spy.count(), 1);

        const CognitiveEnvelope urgentRoot = observation(
            QStringLiteral("sensord"), QUuid(), now);
        QVERIFY(workspace.publish(urgentRoot));
        QVERIFY(workspace.publish(derived(
            ContributionKind::NeedSignal,
            QStringLiteral("healthd"),
            urgentRoot,
            now)));
        QVERIFY(spy.count() >= 2);
    }

    void consolidationUsesStateAsOfHighWaterMark()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        WorkspaceService service(&journal);
        const QDateTime now = QDateTime::currentDateTimeUtc();
        QVERIFY(journal.append(observation(QStringLiteral("before"), QUuid(), now)) > 0);
        const quint64 mark = journal.count();
        QVERIFY(journal.append(observation(QStringLiteral("after"), QUuid(), now.addSecs(1))) > 0);

        QString error;
        const QVariantMap receipt = FabricCodec::decodeMap(service.Consolidate(
            QUuid::createUuid().toString(QUuid::WithoutBraces),
            QStringLiteral("bounded-workspace"), mark), &error);
        QVERIFY2(error.isEmpty(), qPrintable(error));
        QCOMPARE(receipt.value(QStringLiteral("coalitionCount")).toInt(), 1);
    }
};

QTEST_MAIN(TestWorkspace)
#include "tst_workspace.moc"
