// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "OrganStaging.h"

#include "cybou/fabric/OrganBus.h"
#include "cybou/protocol/Health.h"
#include "cybou/protocol/Homeostasis.h"

#include <QDBusConnection>
#include <QDBusInterface>
#include <QDBusPendingCall>
#include <QDBusPendingReply>
#include <QDBusReply>
#include <QDir>
#include <QElapsedTimer>
#include <QProcess>
#include <QProcessEnvironment>
#include <QSet>
#include <QTemporaryDir>
#include <QTest>

#include <memory>
#include <algorithm>
#include <vector>

using namespace cybou;

class TestHealthdIntegration : public QObject
{
    Q_OBJECT

private:
    QTemporaryDir m_root;
    std::vector<std::unique_ptr<QProcess>> m_dependencies;
    std::unique_ptr<QProcess> m_healthd;
    cybou::testing::StagedInstall m_install;
    QProcess *m_predictord{nullptr};
    QProcess *m_workspaced{nullptr};

    QProcessEnvironment environment() const
    {
        auto environment = QProcessEnvironment::systemEnvironment();
        environment.insert(QStringLiteral("XDG_STATE_HOME"), m_root.filePath(QStringLiteral("state")));
        environment.insert(QStringLiteral("XDG_RUNTIME_DIR"), m_root.filePath(QStringLiteral("runtime")));
        environment.insert(QStringLiteral("CYBOU_HEALTH_DISABLE_AUTO_REFRESH"), QStringLiteral("1"));
        return environment;
    }

    QDBusInterface interfaceFor(const BusEndpoint &endpoint) const
    {
        return QDBusInterface(
            QString::fromLatin1(endpoint.service),
            QString::fromLatin1(endpoint.objectPath),
            QString::fromLatin1(endpoint.interfaceName),
            QDBusConnection::sessionBus());
    }

    bool waitForInterface(const BusEndpoint &endpoint, bool expected = true) const
    {
        QElapsedTimer timer;
        timer.start();
        while (timer.elapsed() < 5000) {
            if (interfaceFor(endpoint).isValid() == expected) {
                return true;
            }
            QTest::qWait(25);
        }
        return false;
    }

    QProcess *startDependency(const char *variable, const BusEndpoint &endpoint)
    {
        auto process = std::make_unique<QProcess>();
        process->setProgram(m_install.stageFromEnvironment(variable));
        process->setProcessEnvironment(environment());
        process->start();
        if (!process->waitForStarted(3000)) {
            return nullptr;
        }
        QProcess *result = process.get();
        m_dependencies.push_back(std::move(process));
        return waitForInterface(endpoint) ? result : nullptr;
    }

    void startHealthd(int refreshHoldMs = 0)
    {
        m_healthd = std::make_unique<QProcess>();
        m_healthd->setProgram(m_install.stageFromEnvironment("CYBOU_HEALTHD_PATH"));
        QProcessEnvironment env = environment();
        if (refreshHoldMs > 0) {
            env.insert(
                QStringLiteral("CYBOU_HEALTH_REFRESH_HOLD_MS"),
                QString::number(refreshHoldMs));
        }
        m_healthd->setProcessEnvironment(env);
        m_healthd->start();
        QVERIFY2(m_healthd->waitForStarted(3000), qPrintable(m_healthd->errorString()));
        QVERIFY(waitForInterface(kHealthEndpoint));
    }

    void stopProcess(QProcess *process)
    {
        if (!process || process->state() == QProcess::NotRunning) {
            return;
        }
        process->terminate();
        if (!process->waitForFinished(2000)) {
            process->kill();
            process->waitForFinished(2000);
        }
    }

    CapabilitySnapshot snapshot() const
    {
        QDBusReply<QByteArray> reply = interfaceFor(kHealthEndpoint).call(QStringLiteral("Snapshot"));
        if (!reply.isValid()) {
            return {};
        }
        QString error;
        const CapabilitySnapshot result = decodeCapabilitySnapshot(reply.value(), &error);
        if (!error.isEmpty()) {
            return {};
        }
        return result;
    }

    bool refresh() const
    {
        QDBusReply<bool> reply = interfaceFor(kHealthEndpoint).call(QStringLiteral("Refresh"));
        return reply.isValid() && reply.value();
    }

    HomeostasisSnapshot measurements() const
    {
        QDBusReply<QByteArray> reply = interfaceFor(kHealthEndpoint).call(
            QStringLiteral("Measurements"));
        if (!reply.isValid()) {
            return {};
        }
        QString error;
        return decodeHomeostasisSnapshot(reply.value(), &error);
    }

    qulonglong eventCount() const
    {
        QDBusReply<qulonglong> reply = interfaceFor(kEventEndpoint).call(QStringLiteral("Count"));
        return reply.isValid() ? reply.value() : 0;
    }

private Q_SLOTS:
    // A refresh requested while one is running must be served, not refused.
    //
    // healthd refreshes on a 30 s timer and on every bus owner change, each run taking up to its
    // deadline. A caller arriving during one is refused outright, and under process churn - a suite
    // starting and stopping nine organs generates owner changes continuously - an explicit caller
    // can be locked out for seconds together. That is the process-suite flakiness: tests failing on
    // the refusal rather than on anything they were written to check.
    //
    // Refusing is also wrong on its own terms. "I am busy" is not "I could not", and a caller
    // cannot tell them apart from a bare false.
    //
    // The overlap is constructed, not hoped for. An earlier version of this test issued two
    // concurrent calls and assumed the second would land mid-refresh; instrumentation showed it
    // never did, because with few organs running a refresh finishes first. The hold knob makes the
    // running refresh outlast the second request by construction.
    void aRefreshDuringAnotherIsServedNotRefused()
    {
        stopProcess(m_healthd.get());
        m_healthd.reset();
        startHealthd(1500);

        QDBusInterface first = interfaceFor(kHealthEndpoint);
        QDBusInterface second = interfaceFor(kHealthEndpoint);

        QDBusPendingCall firstCall = first.asyncCall(QStringLiteral("Refresh"));
        // Comfortably inside the hold, and long enough that the first call has certainly been
        // dispatched and begun collecting.
        QTest::qWait(300);
        QDBusPendingCall secondCall = second.asyncCall(QStringLiteral("Refresh"));

        firstCall.waitForFinished();
        secondCall.waitForFinished();

        const QDBusPendingReply<bool> firstReply(firstCall);
        const QDBusPendingReply<bool> secondReply(secondCall);
        QVERIFY2(firstReply.isValid(), qPrintable(firstReply.error().message()));
        QVERIFY2(secondReply.isValid(), qPrintable(secondReply.error().message()));

        QVERIFY2(firstReply.value(), "the first refresh did not succeed, so the test proves nothing");
        QVERIFY2(
            secondReply.value(),
            "a refresh arriving during another was refused rather than served");

        QVERIFY(snapshot().isValid());

        stopProcess(m_healthd.get());
        m_healthd.reset();
        startHealthd();
    }

    void initTestCase()
    {
        QVERIFY(m_root.isValid());
        QVERIFY(QDir().mkpath(m_root.filePath(QStringLiteral("runtime"))));

        QVERIFY(startDependency("CYBOU_EVENTD_PATH", kEventEndpoint));
        QVERIFY(startDependency("CYBOU_LIFECYCLED_PATH", kLifecycleEndpoint));
        QVERIFY(startDependency("CYBOU_IDENTITYD_PATH", kIdentityEndpoint));
        QVERIFY(startDependency("CYBOU_INTENTIOND_PATH", kIntentionEndpoint));
        m_predictord = startDependency("CYBOU_PREDICTORD_PATH", kPredictorEndpoint);
        QVERIFY(m_predictord);
        QVERIFY(startDependency("CYBOU_SELFD_PATH", kSelfEndpoint));
        m_workspaced = startDependency("CYBOU_WORKSPACED_PATH", kWorkspaceEndpoint);
        QVERIFY(m_workspaced);
        QVERIFY(startDependency("CYBOU_PRESENCED_PATH", kPresenceEndpoint));
        startHealthd();
    }

    void cleanupTestCase()
    {
        stopProcess(m_healthd.get());
        for (const auto &process : m_dependencies) {
            stopProcess(process.get());
        }
    }

    void optionalOwnerLossIsCapabilitySpecificAndPersistent()
    {
        const qulonglong beforeRefresh = eventCount();
        QVERIFY(refresh());
        QCOMPARE(eventCount(), beforeRefresh);
        CapabilitySnapshot healthy = snapshot();
        QVERIFY(healthy.isValid());
        QCOMPARE(healthy.aggregateState, CapabilityState::Available);
        QVERIFY(healthy.deficits.isEmpty());
        const HomeostasisSnapshot initialMeasurements = measurements();
        QVERIFY(initialMeasurements.isValid());
        QVERIFY(initialMeasurements.authorizes(QStringLiteral("event-backlog-v1")));
        const auto accepted = std::find_if(
            initialMeasurements.measurements.cbegin(), initialMeasurements.measurements.cend(),
            [](const HomeostaticMeasurement &measurement) {
                return measurement.metricId == QStringLiteral("event.accepted.count");
            });
        QVERIFY(accepted != initialMeasurements.measurements.cend());
        QCOMPARE(accepted->status, MeasurementStatus::Current);
        QCOMPARE(accepted->value, static_cast<double>(beforeRefresh));
        const auto backlog = std::find_if(
            initialMeasurements.measurements.cbegin(), initialMeasurements.measurements.cend(),
            [](const HomeostaticMeasurement &measurement) {
                return measurement.metricId == QStringLiteral("event.backlog.count");
            });
        QVERIFY(backlog != initialMeasurements.measurements.cend());
        QCOMPARE(backlog->status, MeasurementStatus::Current);
        QVERIFY(backlog->hasValue);

        stopProcess(m_predictord);
        QVERIFY(waitForInterface(kPredictorEndpoint, false));
        QVERIFY(refresh());
        const CapabilitySnapshot degraded = snapshot();
        QVERIFY(degraded.isValid());
        QCOMPARE(degraded.aggregateState, CapabilityState::Limited);
        QStringList affected;
        for (const CapabilityDeficit &deficit : degraded.deficits) {
            affected.append(deficit.capabilityId);
        }
        QVERIFY(affected.contains(QStringLiteral("prediction")));
        QVERIFY(affected.contains(QStringLiteral("consolidation")));
        QVERIFY(!affected.contains(QStringLiteral("accepted-biography")));
        QVERIFY(!affected.contains(QStringLiteral("identity-continuity")));
        QVERIFY(!affected.contains(QStringLiteral("commitment-access")));
        QVERIFY(!affected.contains(QStringLiteral("attention-workspace")));

        stopProcess(m_healthd.get());
        startHealthd();
        const CapabilitySnapshot recoveredOwner = snapshot();
        QVERIFY(recoveredOwner.isValid());
        QCOMPARE(recoveredOwner.snapshotId, degraded.snapshotId);
        QCOMPARE(recoveredOwner.deficits.size(), degraded.deficits.size());

        m_predictord = startDependency("CYBOU_PREDICTORD_PATH", kPredictorEndpoint);
        QVERIFY(m_predictord);
        QVERIFY(refresh());
        const CapabilitySnapshot recovering = snapshot();
        QVERIFY(recovering.isValid());
        QCOMPARE(recovering.aggregateState, CapabilityState::Recovering);
        QVERIFY(!recovering.deficits.isEmpty());
        for (const CapabilityDeficit &deficit : recovering.deficits) {
            if (deficit.dependencyId == QStringLiteral("predictord")) {
                QCOMPARE(deficit.state, CapabilityState::Recovering);
                QCOMPARE(deficit.recoveryPolicy, RecoveryPolicy::Reconcile);
            }
        }

        QVERIFY(refresh());
        const CapabilitySnapshot restored = snapshot();
        QVERIFY(restored.isValid());
        QCOMPARE(restored.aggregateState, CapabilityState::Available);
        QVERIFY(restored.deficits.isEmpty());
    }

    void duplicateOwnerIsRejected()
    {
        QProcess duplicate;
        duplicate.setProgram(m_install.stageFromEnvironment("CYBOU_HEALTHD_PATH"));
        duplicate.setProcessEnvironment(environment());
        duplicate.start();
        QVERIFY(duplicate.waitForStarted(3000));
        QVERIFY(duplicate.waitForFinished(5000));
        QVERIFY(duplicate.exitCode() != 0);
    }

    void simultaneousOwnerLossPreservesEveryCause()
    {
        stopProcess(m_predictord);
        stopProcess(m_workspaced);
        QVERIFY(waitForInterface(kPredictorEndpoint, false));
        QVERIFY(waitForInterface(kWorkspaceEndpoint, false));
        QVERIFY(refresh());
        const CapabilitySnapshot degraded = snapshot();
        QVERIFY(degraded.isValid());
        QStringList dependencies;
        for (const CapabilityDeficit &deficit : degraded.deficits) {
            if (deficit.capabilityId == QStringLiteral("consolidation"))
                dependencies.append(deficit.dependencyId);
        }
        QCOMPARE(
            QSet<QString>(dependencies.begin(), dependencies.end()),
            QSet<QString>({QStringLiteral("predictord"), QStringLiteral("workspaced")}));

        m_predictord = startDependency("CYBOU_PREDICTORD_PATH", kPredictorEndpoint);
        m_workspaced = startDependency("CYBOU_WORKSPACED_PATH", kWorkspaceEndpoint);
        QVERIFY(m_predictord);
        QVERIFY(m_workspaced);
        QVERIFY(refresh());
        QVERIFY(refresh());
        QCOMPARE(snapshot().aggregateState, CapabilityState::Available);
    }
};

QTEST_MAIN(TestHealthdIntegration)
#include "tst_healthd_integration.moc"
