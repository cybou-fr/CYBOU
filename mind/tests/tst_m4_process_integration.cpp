// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/fabric/OrganClients.h"
#include "cybou/fabric/FabricCodec.h"
#include "cybou/ipc/EventClient.h"
#include "cybou/presence/Presence.h"

#include <QDebug>
#include <QDir>
#include <QFile>
#include <QElapsedTimer>
#include <QMap>
#include <QProcess>
#include <QProcessEnvironment>
#include <QSet>
#include <QTemporaryDir>
#include <QTest>
#include <QTimer>

#include <memory>

using namespace cybou;

class TestM4Processes : public QObject
{
    Q_OBJECT

private:
    std::unique_ptr<QTemporaryDir> m_root;

    QString m_eventdPath;
    QString m_identitydPath;
    QString m_intentiondPath;
    QString m_predictordPath;
    QString m_selfdPath;
    QString m_workspacedPath;
    QString m_lifecycledPath;
    QString m_healthdPath;
    QString m_presencedPath;

    std::unique_ptr<QProcess> m_eventd;
    std::unique_ptr<QProcess> m_identityd;
    std::unique_ptr<QProcess> m_intentiond;
    std::unique_ptr<QProcess> m_predictord;
    std::unique_ptr<QProcess> m_selfd;
    std::unique_ptr<QProcess> m_workspaced;
    std::unique_ptr<QProcess> m_lifecycled;
    std::unique_ptr<QProcess> m_healthd;
    std::unique_ptr<QProcess> m_presenced;

    QProcessEnvironment environment() const
    {
        QProcessEnvironment env =
            QProcessEnvironment::systemEnvironment();
        env.insert(
            QStringLiteral("XDG_STATE_HOME"),
            m_root->filePath(QStringLiteral("state")));
        env.insert(
            QStringLiteral("XDG_RUNTIME_DIR"),
            m_root->filePath(QStringLiteral("runtime")));
        return env;
    }

    std::unique_ptr<QProcess> start(
        const QString &path,
        const QMap<QString, QString> &overrides = {})
    {
        auto process = std::make_unique<QProcess>();
        process->setProgram(path);
        QProcessEnvironment env = environment();
        for (auto it = overrides.cbegin(); it != overrides.cend(); ++it)
            env.insert(it.key(), it.value());
        process->setProcessEnvironment(env);
        process->start();

        if (!process->waitForStarted(3000)) {
            qWarning() << process->errorString();
            return {};
        }

        return process;
    }

    static void stop(std::unique_ptr<QProcess> &process)
    {
        if (!process
            || process->state() == QProcess::NotRunning) {
            return;
        }

        process->terminate();
        if (!process->waitForFinished(2000)) {
            process->kill();
            process->waitForFinished(2000);
        }
    }

private Q_SLOTS:
    void initTestCase()
    {
        m_eventdPath =
            qEnvironmentVariable("CYBOU_EVENTD_PATH");
        m_identitydPath =
            qEnvironmentVariable("CYBOU_IDENTITYD_PATH");
        m_intentiondPath =
            qEnvironmentVariable("CYBOU_INTENTIOND_PATH");
        m_predictordPath =
            qEnvironmentVariable("CYBOU_PREDICTORD_PATH");
        m_selfdPath =
            qEnvironmentVariable("CYBOU_SELFD_PATH");
        m_workspacedPath =
            qEnvironmentVariable("CYBOU_WORKSPACED_PATH");
        m_lifecycledPath =
            qEnvironmentVariable("CYBOU_LIFECYCLED_PATH");
        m_healthdPath =
            qEnvironmentVariable("CYBOU_HEALTHD_PATH");
        m_presencedPath =
            qEnvironmentVariable("CYBOU_PRESENCED_PATH");

        QVERIFY2(!m_eventdPath.isEmpty(), "CYBOU_EVENTD_PATH is not set");
        QVERIFY2(!m_identitydPath.isEmpty(), "CYBOU_IDENTITYD_PATH is not set");
        QVERIFY2(!m_intentiondPath.isEmpty(), "CYBOU_INTENTIOND_PATH is not set");
        QVERIFY2(!m_predictordPath.isEmpty(), "CYBOU_PREDICTORD_PATH is not set");
        QVERIFY2(!m_selfdPath.isEmpty(), "CYBOU_SELFD_PATH is not set");
        QVERIFY2(!m_workspacedPath.isEmpty(), "CYBOU_WORKSPACED_PATH is not set");
        QVERIFY2(!m_lifecycledPath.isEmpty(), "CYBOU_LIFECYCLED_PATH is not set");
        QVERIFY2(!m_healthdPath.isEmpty(), "CYBOU_HEALTHD_PATH is not set");
        QVERIFY2(!m_presencedPath.isEmpty(), "CYBOU_PRESENCED_PATH is not set");

        m_root = std::make_unique<QTemporaryDir>();
        QVERIFY(m_root->isValid());

        QDir().mkpath(
            m_root->filePath(QStringLiteral("runtime")));

        qputenv(
            "XDG_STATE_HOME",
            m_root->filePath(QStringLiteral("state")).toUtf8());
        qputenv(
            "XDG_RUNTIME_DIR",
            m_root->filePath(QStringLiteral("runtime")).toUtf8());

        m_eventd = start(m_eventdPath);
        QVERIFY(m_eventd);

        EventClient events;
        QTRY_VERIFY_WITH_TIMEOUT(events.isOpen(), 5000);

        m_identityd = start(m_identitydPath);
        QVERIFY(m_identityd);
        IdentityClient identity;
        QTRY_VERIFY_WITH_TIMEOUT(identity.ready(), 5000);

        m_intentiond = start(m_intentiondPath);
        QVERIFY(m_intentiond);
        IntentionClient intentions;
        QTRY_VERIFY_WITH_TIMEOUT(intentions.ready(), 5000);

        m_predictord = start(m_predictordPath);
        QVERIFY(m_predictord);
        PredictorClient predictor;
        QTRY_VERIFY_WITH_TIMEOUT(predictor.ready(), 5000);

        m_workspaced = start(m_workspacedPath);
        QVERIFY(m_workspaced);
        WorkspaceClient workspace;
        QTRY_VERIFY_WITH_TIMEOUT(workspace.ready(), 5000);

        m_selfd = start(m_selfdPath);
        QVERIFY(m_selfd);
        SelfClient self;
        QTRY_VERIFY_WITH_TIMEOUT(self.ready(), 5000);

        m_lifecycled = start(m_lifecycledPath);
        QVERIFY(m_lifecycled);
        LifecycleClient lifecycle;
        QTRY_VERIFY_WITH_TIMEOUT(lifecycle.ready(), 5000);

        m_healthd = start(m_healthdPath);
        QVERIFY(m_healthd);
        HealthClient health;
        QTRY_VERIFY_WITH_TIMEOUT(health.ready(), 5000);

        m_presenced = start(m_presencedPath);
        QVERIFY(m_presenced);
        PresenceClient presence;
        QTRY_VERIFY_WITH_TIMEOUT(presence.ready(), 5000);
        RpcClient healthRpc(kHealthEndpoint);
        QVERIFY(healthRpc.callBool(QStringLiteral("Refresh")));
    }

    void cleanupTestCase()
    {
        stop(m_presenced);
        stop(m_healthd);
        stop(m_lifecycled);
        stop(m_selfd);
        stop(m_workspaced);
        stop(m_predictord);
        stop(m_intentiond);
        stop(m_identityd);
        stop(m_eventd);
    }

    void nineDistinctProcessesOwnTheRuntime()
    {
        QSet<qint64> pids{
            m_eventd->processId(),
            m_identityd->processId(),
            m_intentiond->processId(),
            m_predictord->processId(),
            m_selfd->processId(),
            m_workspaced->processId(),
            m_lifecycled->processId(),
            m_healthd->processId(),
            m_presenced->processId(),
        };

        QCOMPARE(pids.size(), 9);
    }

    void qmlProxyDoesNotCreateAnotherIdentitySession()
    {
        IdentityClient identity;
        const qulonglong before =
            identity.state()
                .value(QStringLiteral("sessionCount"))
                .toULongLong();

        Presence first;
        Presence second;

        QVERIFY2(first.wake(), qPrintable(first.lastError()));
        QVERIFY2(second.wake(), qPrintable(second.lastError()));

        const qulonglong after =
            identity.state()
                .value(QStringLiteral("sessionCount"))
                .toULongLong();

        QCOMPARE(after, before);
        QCOMPARE(
            first.identityState()
                .value(QStringLiteral("uuid"))
                .toString(),
            second.identityState()
                .value(QStringLiteral("uuid"))
                .toString());
    }

    void lifecycleStateProjectsThroughPresence()
    {
        Presence presence;
        QVERIFY2(presence.wake(), qPrintable(presence.lastError()));
        QCOMPARE(presence.lifecycleMode(), QStringLiteral("awake"));

        RpcClient lifecycle(kLifecycleEndpoint);
        QVERIFY(lifecycle.callBool(QStringLiteral("Transition"), {QStringLiteral("idle")}));
        QTRY_COMPARE_WITH_TIMEOUT(presence.lifecycleMode(), QStringLiteral("idle"), 5000);
        QCOMPARE(
            presence.lifecycleState().value(QStringLiteral("mode")).toString(),
            QStringLiteral("idle"));
        QCOMPARE(
            presence.lifecycleProjection().value(QStringLiteral("progressClass")).toString(),
            QStringLiteral("inactive"));
        QCOMPARE(
            presence.lifecycleProjection().value(QStringLiteral("freshnessClass")).toString(),
            QStringLiteral("unknown"));
        QCOMPARE(
            presence.lifecycleProjection().value(QStringLiteral("progressPercent")).toInt(),
            0);
        QCOMPARE(
            presence.organHealth().value(QStringLiteral("lifecycled")).toString(),
            QStringLiteral("healthy"));

        const QString runId = lifecycle.callString(
            QStringLiteral("RequestRun"),
            {QStringLiteral("consolidation"), QStringLiteral("ui-interruption-test"),
             QVariant::fromValue<qulonglong>(1), QStringList{}, QStringList{}});
        QVERIFY(!runId.isEmpty());
        QTRY_COMPARE_WITH_TIMEOUT(presence.lifecycleStatus(), QStringLiteral("active"), 5000);
        presence.interruptLifecycle(QStringLiteral("process integration test"));
        QTRY_VERIFY_WITH_TIMEOUT(!presence.lifecycleCommandPending(), 5000);
        QTRY_COMPARE_WITH_TIMEOUT(presence.lifecycleStatus(), QStringLiteral("interrupted"), 5000);
        QCOMPARE(presence.lifecycleMode(), QStringLiteral("recovering"));
        QVERIFY2(presence.lastError().isEmpty(), qPrintable(presence.lastError()));

        QVERIFY(lifecycle.callBool(QStringLiteral("ResumeRun")) == false);
        QVERIFY(lifecycle.callBool(QStringLiteral("Transition"), {QStringLiteral("awake")}));
        QTRY_COMPARE_WITH_TIMEOUT(presence.lifecycleMode(), QStringLiteral("awake"), 5000);
    }

    void schedulingExecutionRejectsStaleEvidenceAndIsIdempotent()
    {
        EventClient eventClient;
        QVERIFY(eventClient.ensureConsumer(QStringLiteral("lifecycle.consolidation"), 0));
        while (eventClient.consumerBacklog(QStringLiteral("lifecycle.consolidation")).value_or(0)
               < 32) {
            CognitiveEnvelope pressure;
            pressure.messageId = QUuid::createUuid();
            pressure.correlationId = pressure.messageId;
            pressure.originOrgan = QStringLiteral("scheduling-test");
            pressure.originNode = QStringLiteral("local");
            pressure.kind = ContributionKind::Observation;
            pressure.wallTime = QDateTime::currentDateTimeUtc();
            pressure.confidence = 1.0;
            pressure.privacy = PrivacyClass::Local;
            QVERIFY(eventClient.append(pressure) > 0);
        }
        RpcClient health(kHealthEndpoint);
        QVERIFY(health.callBool(QStringLiteral("Refresh")));
        HealthClient healthClient;
        const HomeostasisSnapshot homeostasis = healthClient.measurements();
        QVERIFY(homeostasis.isValid());
        QCOMPARE(homeostasis.authorizedPolicyIds,
                 QStringList({QStringLiteral("event-backlog-v1")}));

        RpcClient lifecycle(kLifecycleEndpoint);
        QVERIFY(lifecycle.callBool(QStringLiteral("Transition"), {QStringLiteral("idle")}));
        const QByteArray before = lifecycle.callBytes(QStringLiteral("State"));

        QString error;
        const QVariantMap evaluation = FabricCodec::decodeMap(
            lifecycle.callBytes(QStringLiteral("EvaluateScheduling")), &error);
        QVERIFY2(error.isEmpty(), qPrintable(error));
        QCOMPARE(evaluation.value(QStringLiteral("decision")).toString(),
                 QStringLiteral("run"));
        QCOMPARE(lifecycle.callBytes(QStringLiteral("State")), before);

        Presence surface;
        QVERIFY2(surface.wake(), qPrintable(surface.lastError()));
        QCOMPARE(surface.lifecycleScheduling().value(QStringLiteral("decision")).toString(),
                 QStringLiteral("run"));

        QVERIFY(health.callBool(QStringLiteral("Refresh")));
        const QString staleResult = lifecycle.callString(
            QStringLiteral("ExecuteSchedulingDecision"),
            {evaluation.value(QStringLiteral("capabilitySnapshotId")).toString(),
             evaluation.value(QStringLiteral("homeostasisSnapshotId")).toString()});
        QVERIFY(staleResult.isEmpty());
        QCOMPARE(lifecycle.callBytes(QStringLiteral("State")), before);

        const QVariantMap current = FabricCodec::decodeMap(
            lifecycle.callBytes(QStringLiteral("EvaluateScheduling")), &error);
        QVERIFY2(error.isEmpty(), qPrintable(error));
        QCOMPARE(current.value(QStringLiteral("decision")).toString(), QStringLiteral("run"));
        const QVariantList evidence{
            current.value(QStringLiteral("capabilitySnapshotId")).toString(),
            current.value(QStringLiteral("homeostasisSnapshotId")).toString()};
        const QString runId = lifecycle.callString(
            QStringLiteral("ExecuteSchedulingDecision"), evidence);
        QVERIFY(!runId.isEmpty());
        QCOMPARE(lifecycle.callString(QStringLiteral("ExecuteSchedulingDecision"), evidence), runId);
        const QVariantMap active = LifecycleClient().state();
        QCOMPARE(active.value(QStringLiteral("runId")).toString(), runId);
        QCOMPARE(active.value(QStringLiteral("mode")).toString(), QStringLiteral("consolidating"));
        QVERIFY(lifecycle.callBool(QStringLiteral("Dispatch")));
        QVERIFY(lifecycle.callBool(
            QStringLiteral("FinishRun"),
            {QStringLiteral("completed"), QStringLiteral("authorized scheduling test")}));
        QCOMPARE(lifecycle.callString(QStringLiteral("ExecuteSchedulingDecision"), evidence), runId);

        QVERIFY(lifecycle.callBool(QStringLiteral("Transition"), {QStringLiteral("idle")}));
        const QString laterRun = lifecycle.callString(
            QStringLiteral("RequestRun"),
            {QStringLiteral("maintenance"), QStringLiteral("idempotency-window-test"),
             QVariant::fromValue<qulonglong>(eventClient.count()), QStringList{}, QStringList{}});
        QVERIFY(!laterRun.isEmpty());
        QVERIFY(lifecycle.callBool(
            QStringLiteral("FinishRun"),
            {QStringLiteral("interrupted"), QStringLiteral("idempotency window cleanup")}));
        QCOMPARE(lifecycle.callString(QStringLiteral("ExecuteSchedulingDecision"), evidence), runId);
        QVERIFY(lifecycle.callBool(QStringLiteral("Transition"), {QStringLiteral("awake")}));
    }

    void proxyRecreationDoesNotMutateOrDuplicateLifecycleRun()
    {
        RpcClient lifecycle(kLifecycleEndpoint);
        EventClient events;
        QVERIFY(lifecycle.callBool(QStringLiteral("Transition"), {QStringLiteral("idle")}));
        const qulonglong contributionsBefore = events.count();
        const QString runId = lifecycle.callString(
            QStringLiteral("RequestRun"),
            {QStringLiteral("consolidation"), QStringLiteral("proxy-recreation-test"),
             QVariant::fromValue<qulonglong>(contributionsBefore),
             QStringList{}, QStringList{}});
        QVERIFY(!runId.isEmpty());

        for (int i = 0; i < 3; ++i) {
            Presence proxy;
            QVERIFY2(proxy.wake(), qPrintable(proxy.lastError()));
            QCOMPARE(proxy.lifecycleState().value(QStringLiteral("runId")).toString(), runId);
            QCOMPARE(proxy.lifecycleStatus(), QStringLiteral("active"));
        }

        QCOMPARE(events.count(), contributionsBefore);
        const QVariantMap state = LifecycleClient().state();
        QCOMPARE(state.value(QStringLiteral("runId")).toString(), runId);
        QCOMPARE(state.value(QStringLiteral("status")).toString(), QStringLiteral("active"));
        QVERIFY(lifecycle.callBool(
            QStringLiteral("FinishRun"),
            {QStringLiteral("interrupted"), QStringLiteral("proxy recreation test cleanup")}));
        QVERIFY(lifecycle.callBool(QStringLiteral("Transition"), {QStringLiteral("awake")}));
    }

    void lifecycleTimeoutDoesNotBlockProxyEventLoopOrMutateRun()
    {
        RpcClient lifecycle(kLifecycleEndpoint);
        QVERIFY(lifecycle.callBool(QStringLiteral("Transition"), {QStringLiteral("idle")}));
        const QString runId = lifecycle.callString(
            QStringLiteral("RequestRun"),
            {QStringLiteral("consolidation"), QStringLiteral("async-timeout-test"),
             QVariant::fromValue<qulonglong>(1), QStringList{}, QStringList{}});
        QVERIFY(!runId.isEmpty());

        stop(m_presenced);
        m_presenced = start(
            m_presencedPath,
            {{QStringLiteral("CYBOU_PRESENCE_INTERRUPT_DELAY_MS"), QStringLiteral("6000")}});
        QVERIFY(m_presenced);
        PresenceClient delayedPresence;
        QTRY_VERIFY_WITH_TIMEOUT(delayedPresence.ready(), 5000);
        Presence proxy;
        QVERIFY2(proxy.wake(), qPrintable(proxy.lastError()));

        bool heartbeat = false;
        QTimer::singleShot(100, this, [&heartbeat]() { heartbeat = true; });
        QElapsedTimer elapsed;
        elapsed.start();
        proxy.interruptLifecycle(QStringLiteral("timeout test"));
        QVERIFY2(elapsed.elapsed() < 100, "async lifecycle command blocked its caller");
        QTRY_VERIFY_WITH_TIMEOUT(heartbeat, 1000);
        QTRY_VERIFY_WITH_TIMEOUT(!proxy.lifecycleCommandPending(), 6500);
        QVERIFY(!proxy.lastError().isEmpty());
        QVERIFY(proxy.lastError().contains(QStringLiteral("unknown-outcome")));
        QCOMPARE(LifecycleClient().state().value(QStringLiteral("runId")).toString(), runId);
        QCOMPARE(LifecycleClient().state().value(QStringLiteral("status")).toString(), QStringLiteral("active"));

        stop(m_presenced);
        m_presenced = start(m_presencedPath);
        QVERIFY(m_presenced);
        PresenceClient presence;
        QTRY_VERIFY_WITH_TIMEOUT(presence.ready(), 5000);
        QVERIFY(lifecycle.callBool(
            QStringLiteral("FinishRun"),
            {QStringLiteral("interrupted"), QStringLiteral("timeout test cleanup")}));
        QVERIFY(lifecycle.callBool(QStringLiteral("Transition"), {QStringLiteral("awake")}));
    }

    void commandsCrossRealOrganProcesses()
    {
        Presence first;
        Presence second;

        QVERIFY(first.wake());
        QVERIFY(second.wake());

        const int before = first.contributions();

        const QUuid intention =
            first.promise(
                QStringLiteral("prove M4 process routing"));
        QVERIFY2(
            !intention.isNull(),
            qPrintable(first.lastError()));

        QTRY_VERIFY_WITH_TIMEOUT(
            second.obligations().contains(
                QStringLiteral("prove M4 process routing")),
            5000);

        QVERIFY(first.contributions() >= before + 2);

        QVERIFY(first.observe(
            QStringLiteral("m4-build"),
            10.0));
        QVERIFY(first.observe(
            QStringLiteral("m4-build"),
            12.0));

        const QVariantMap prediction =
            first.predict(QStringLiteral("m4-build"));
        QVERIFY2(
            !prediction.isEmpty(),
            qPrintable(first.lastError()));

        QTRY_VERIFY_WITH_TIMEOUT(
            !second.coalitions().isEmpty(),
            5000);

        QVERIFY(
            !second.moment()
                 .value(QStringLiteral("focus"))
                 .toString()
                 .isEmpty());
    }

    void restartingIdentitydDoesNotIncrementTheUserSession()
    {
        IdentityClient identity;
        const QVariantMap before = identity.state();
        const qulonglong beforeCount =
            before.value(QStringLiteral("sessionCount"))
                .toULongLong();

        stop(m_identityd);
        m_identityd = start(m_identitydPath);
        QVERIFY(m_identityd);

        IdentityClient restarted;
        QTRY_VERIFY_WITH_TIMEOUT(restarted.ready(), 5000);

        const QVariantMap after = restarted.state();
        QCOMPARE(
            after.value(QStringLiteral("uuid")).toString(),
            before.value(QStringLiteral("uuid")).toString());
        QCOMPARE(
            after.value(QStringLiteral("sessionCount"))
                .toULongLong(),
            beforeCount);
    }

    void restartingPresencedDoesNotRestartMind()
    {
        IdentityClient identity;
        const QVariantMap before = identity.state();

        const qint64 eventPid = m_eventd->processId();
        const qint64 identityPid = m_identityd->processId();
        const qint64 intentionPid = m_intentiond->processId();
        const qint64 predictorPid = m_predictord->processId();
        const qint64 selfPid = m_selfd->processId();
        const qint64 workspacePid = m_workspaced->processId();

        stop(m_presenced);
        m_presenced = start(m_presencedPath);
        QVERIFY(m_presenced);

        PresenceClient backend;
        QTRY_VERIFY_WITH_TIMEOUT(backend.ready(), 5000);

        Presence surface;
        QVERIFY2(surface.wake(), qPrintable(surface.lastError()));

        QCOMPARE(m_eventd->processId(), eventPid);
        QCOMPARE(m_identityd->processId(), identityPid);
        QCOMPARE(m_intentiond->processId(), intentionPid);
        QCOMPARE(m_predictord->processId(), predictorPid);
        QCOMPARE(m_selfd->processId(), selfPid);
        QCOMPARE(m_workspaced->processId(), workspacePid);

        const QVariantMap after = identity.state();
        QCOMPARE(
            after.value(QStringLiteral("uuid")).toString(),
            before.value(QStringLiteral("uuid")).toString());
        QCOMPARE(
            after.value(QStringLiteral("sessionCount"))
                .toULongLong(),
            before.value(QStringLiteral("sessionCount"))
                .toULongLong());
    }

    void simulatedLoginBoundaryPreservesIdentityAndIntentions()
    {
        Presence beforeSurface;
        QVERIFY(beforeSurface.wake());
        const QString commitment = QStringLiteral("survive a logical login boundary");
        QVERIFY(!beforeSurface.promise(commitment).isNull());

        IdentityClient identity;
        const QVariantMap before = identity.state();

        stop(m_presenced);
        stop(m_workspaced);
        stop(m_selfd);
        stop(m_predictord);
        stop(m_intentiond);
        stop(m_identityd);
        stop(m_eventd);

        const QString sessionMarker = m_root->filePath(
            QStringLiteral("runtime/cybou/identity-session"));
        QVERIFY(QFile::remove(sessionMarker));

        m_eventd = start(m_eventdPath);
        QVERIFY(m_eventd);
        EventClient events;
        QTRY_VERIFY_WITH_TIMEOUT(events.isOpen(), 5000);

        m_identityd = start(m_identitydPath);
        QVERIFY(m_identityd);
        IdentityClient restartedIdentity;
        QTRY_VERIFY_WITH_TIMEOUT(restartedIdentity.ready(), 5000);

        m_intentiond = start(m_intentiondPath);
        QVERIFY(m_intentiond);
        IntentionClient restartedIntentions;
        QTRY_VERIFY_WITH_TIMEOUT(restartedIntentions.ready(), 5000);

        m_predictord = start(m_predictordPath);
        QVERIFY(m_predictord);
        PredictorClient restartedPredictor;
        QTRY_VERIFY_WITH_TIMEOUT(restartedPredictor.ready(), 5000);

        m_workspaced = start(m_workspacedPath);
        QVERIFY(m_workspaced);
        WorkspaceClient restartedWorkspace;
        QTRY_VERIFY_WITH_TIMEOUT(restartedWorkspace.ready(), 5000);

        m_selfd = start(m_selfdPath);
        QVERIFY(m_selfd);
        SelfClient restartedSelf;
        QTRY_VERIFY_WITH_TIMEOUT(restartedSelf.ready(), 5000);

        m_presenced = start(m_presencedPath);
        QVERIFY(m_presenced);
        PresenceClient restartedPresence;
        QTRY_VERIFY_WITH_TIMEOUT(restartedPresence.ready(), 5000);
        RpcClient health(kHealthEndpoint);
        QVERIFY(health.callBool(QStringLiteral("Refresh")));
        QVERIFY(health.callBool(QStringLiteral("Refresh")));

        IdentityClient afterIdentity;
        QTRY_VERIFY_WITH_TIMEOUT(afterIdentity.ready(), 5000);
        const QVariantMap after = afterIdentity.state();
        QCOMPARE(
            after.value(QStringLiteral("uuid")).toString(),
            before.value(QStringLiteral("uuid")).toString());
        QCOMPARE(
            after.value(QStringLiteral("sessionCount")).toULongLong(),
            before.value(QStringLiteral("sessionCount")).toULongLong() + 1);

        Presence afterSurface;
        QTRY_VERIFY_WITH_TIMEOUT(afterSurface.wake(), 5000);
        QTRY_VERIFY_WITH_TIMEOUT(afterSurface.obligations().contains(commitment), 5000);
    }

    void oneOrganFailureDoesNotKillTheOthers()
    {
        Presence surface;
        QVERIFY(surface.wake());

        stop(m_predictord);

        RpcClient health(kHealthEndpoint);
        QVERIFY(health.callBool(QStringLiteral("Refresh")));
        QTRY_COMPARE_WITH_TIMEOUT(
            surface.aggregateCapabilityState(),
            QStringLiteral("limited"),
            5000);
        QCOMPARE(
            surface.capabilityStates().value(QStringLiteral("prediction")).toString(),
            QStringLiteral("unavailable"));
        QVERIFY(surface.runtimeReachable());
        QVERIFY(surface.isAwake());
        QVERIFY(surface.hasCapability(QStringLiteral("identity-continuity")));
        QVERIFY(surface.hasCapability(QStringLiteral("commitment-access")));
        QVERIFY(surface.hasCapability(QStringLiteral("attention-workspace")));
        QVERIFY(!surface.hasCapability(QStringLiteral("prediction")));
        QVERIFY(!surface.identityState().isEmpty());
        QVERIFY(!surface.promise(QStringLiteral("continue without prediction")).isNull());

        QVERIFY(m_eventd->state() != QProcess::NotRunning);
        QVERIFY(m_identityd->state() != QProcess::NotRunning);
        QVERIFY(m_intentiond->state() != QProcess::NotRunning);
        QVERIFY(m_selfd->state() != QProcess::NotRunning);
        QVERIFY(m_workspaced->state() != QProcess::NotRunning);
        QVERIFY(m_presenced->state() != QProcess::NotRunning);

        QVERIFY(!surface.observe(
            QStringLiteral("predictor-down"),
            1.0));

        m_predictord = start(m_predictordPath);
        QVERIFY(m_predictord);

        PredictorClient predictor;
        QTRY_VERIFY_WITH_TIMEOUT(predictor.ready(), 5000);
        QVERIFY(health.callBool(QStringLiteral("Refresh")));
        QTRY_VERIFY_WITH_TIMEOUT(
            surface.hasCapability(QStringLiteral("prediction")),
            5000);
    }
};

QTEST_MAIN(TestM4Processes)
#include "tst_m4_process_integration.moc"
