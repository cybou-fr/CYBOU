// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/fabric/FabricCodec.h"
#include "cybou/fabric/OrganBus.h"
#include "cybou/events/EnvelopeCodec.h"
#include "cybou/protocol/Lifecycle.h"

#include <QDBusConnection>
#include <QDBusInterface>
#include <QDBusReply>
#include <QDir>
#include <QProcess>
#include <QProcessEnvironment>
#include <QTemporaryDir>
#include <QTest>

#include <memory>

using namespace cybou;

class TestLifecycledIntegration : public QObject
{
    Q_OBJECT

private:
    QTemporaryDir m_root;
    QString m_daemonPath;
    std::unique_ptr<QProcess> m_daemon;
    std::unique_ptr<QProcess> m_eventd;
    std::unique_ptr<QProcess> m_predictord;
    std::unique_ptr<QProcess> m_workspaced;

    QProcessEnvironment environment() const
    {
        auto env = QProcessEnvironment::systemEnvironment();
        env.insert(QStringLiteral("XDG_STATE_HOME"), m_root.filePath(QStringLiteral("state")));
        env.insert(QStringLiteral("XDG_RUNTIME_DIR"), m_root.filePath(QStringLiteral("runtime")));
        return env;
    }

    void startDaemon()
    {
        m_daemon = std::make_unique<QProcess>();
        m_daemon->setProgram(m_daemonPath);
        m_daemon->setProcessEnvironment(environment());
        m_daemon->start();
        QVERIFY2(m_daemon->waitForStarted(3000), qPrintable(m_daemon->errorString()));
        QTRY_VERIFY_WITH_TIMEOUT(interface().isValid(), 5000);
    }

    std::unique_ptr<QProcess> startAuxiliary(const char *variable)
    {
        auto process = std::make_unique<QProcess>();
        process->setProgram(qEnvironmentVariable(variable));
        process->setProcessEnvironment(environment());
        process->start();
        if (!process->waitForStarted(3000)) return {};
        return process;
    }

    void stopDaemon()
    {
        if (!m_daemon || m_daemon->state() == QProcess::NotRunning) return;
        m_daemon->terminate();
        if (!m_daemon->waitForFinished(2000)) {
            m_daemon->kill();
            m_daemon->waitForFinished(2000);
        }
        m_daemon.reset();
    }

    QDBusInterface interface() const
    {
        return QDBusInterface(
            QString::fromLatin1(kLifecycleEndpoint.service),
            QString::fromLatin1(kLifecycleEndpoint.objectPath),
            QString::fromLatin1(kLifecycleEndpoint.interfaceName),
            QDBusConnection::sessionBus());
    }

private Q_SLOTS:
    void initTestCase()
    {
        QVERIFY(m_root.isValid());
        m_daemonPath = qEnvironmentVariable("CYBOU_LIFECYCLED_PATH");
        QVERIFY2(!m_daemonPath.isEmpty(), "CYBOU_LIFECYCLED_PATH is not set");
        QVERIFY(QDir().mkpath(m_root.filePath(QStringLiteral("runtime"))));
        m_eventd = startAuxiliary("CYBOU_EVENTD_PATH");
        QVERIFY(m_eventd);
        QTRY_VERIFY_WITH_TIMEOUT(
            QDBusInterface(
                QStringLiteral("org.cybou.Mind.Event1"),
                QStringLiteral("/org/cybou/Mind/Event1"),
                QStringLiteral("org.cybou.Mind.Event1"),
                QDBusConnection::sessionBus()).isValid(),
            5000);
        m_predictord = startAuxiliary("CYBOU_PREDICTORD_PATH");
        QVERIFY(m_predictord);
        m_workspaced = startAuxiliary("CYBOU_WORKSPACED_PATH");
        QVERIFY(m_workspaced);
        for (const auto &endpoint : {kPredictorEndpoint, kWorkspaceEndpoint}) {
            QTRY_VERIFY_WITH_TIMEOUT(
                QDBusInterface(
                    QString::fromLatin1(endpoint.service),
                    QString::fromLatin1(endpoint.objectPath),
                    QString::fromLatin1(endpoint.interfaceName),
                    QDBusConnection::sessionBus()).isValid(),
                5000);
        }
        startDaemon();
    }

    void cleanupTestCase()
    {
        stopDaemon();
        for (auto *process : {m_workspaced.get(), m_predictord.get(), m_eventd.get()}) {
            if (process && process->state() != QProcess::NotRunning) {
                process->terminate();
                process->waitForFinished(2000);
            }
        }
    }

    void activeRunRecoversAcrossProcessRestart()
    {
        QDBusReply<bool> idle = interface().call(QStringLiteral("Transition"), QStringLiteral("idle"));
        QVERIFY(idle.isValid() && idle.value());

        LifecycleRun run;
        run.runId = QUuid::createUuid();
        run.kind = QStringLiteral("consolidation");
        run.policyId = QStringLiteral("integration-test");
        run.requestedAt = QDateTime::currentDateTimeUtc();
        run.inputHighWaterMark = 17;
        run.requiredCapabilities = {QStringLiteral("journal")};

        QDBusReply<bool> begun = interface().call(
            QStringLiteral("BeginRun"), encodeLifecycleRun(run));
        QVERIFY(begun.isValid() && begun.value());

        stopDaemon();
        startDaemon();

        QDBusReply<QByteArray> stateReply = interface().call(QStringLiteral("State"));
        QVERIFY(stateReply.isValid());
        QString error;
        const QVariantMap state = FabricCodec::decodeMap(stateReply.value(), &error);
        QVERIFY2(error.isEmpty(), qPrintable(error));
        QCOMPARE(state.value(QStringLiteral("mode")).toString(), QStringLiteral("recovering"));
        QCOMPARE(state.value(QStringLiteral("status")).toString(), QStringLiteral("active"));
        QCOMPARE(
            state.value(QStringLiteral("runId")).toString(),
            run.runId.toString(QUuid::WithoutBraces));

        QDBusReply<bool> resumed = interface().call(QStringLiteral("ResumeRun"));
        QVERIFY(resumed.isValid() && resumed.value());
        QDBusReply<QString> operationKey = interface().call(
            QStringLiteral("WorkOperationKey"), QStringLiteral("journal"));
        QVERIFY(operationKey.isValid() && !operationKey.value().isEmpty());
        QDBusReply<bool> acknowledged = interface().call(
            QStringLiteral("AcknowledgeWork"),
            QStringLiteral("journal"), operationKey.value(), qulonglong(17));
        QVERIFY(acknowledged.isValid() && acknowledged.value());
        QDBusReply<bool> duplicate = interface().call(
            QStringLiteral("AcknowledgeWork"),
            QStringLiteral("journal"), operationKey.value(), qulonglong(17));
        QVERIFY(duplicate.isValid() && duplicate.value());
        QDBusReply<bool> finished = interface().call(
            QStringLiteral("FinishRun"),
            QStringLiteral("interrupted"), QStringLiteral("manual acknowledgement has no Event1 result"));
        QVERIFY(finished.isValid() && finished.value());
        QDBusReply<bool> awake = interface().call(QStringLiteral("Transition"), QStringLiteral("awake"));
        QVERIFY(awake.isValid() && awake.value());
    }

    void duplicateOwnerIsRejected()
    {
        QProcess duplicate;
        duplicate.setProgram(m_daemonPath);
        duplicate.setProcessEnvironment(environment());
        duplicate.start();
        QVERIFY(duplicate.waitForStarted(3000));
        QVERIFY(duplicate.waitForFinished(5000));
        QVERIFY(duplicate.exitCode() != 0);
    }

    void dispatchesPredictorAndWorkspaceIdempotently()
    {
        QDBusReply<bool> idle = interface().call(QStringLiteral("Transition"), QStringLiteral("idle"));
        QVERIFY(idle.isValid() && idle.value());

        QDBusInterface events(
            QStringLiteral("org.cybou.Mind.Event1"),
            QStringLiteral("/org/cybou/Mind/Event1"),
            QStringLiteral("org.cybou.Mind.Event1"),
            QDBusConnection::sessionBus());
        QTRY_VERIFY_WITH_TIMEOUT(events.isValid(), 5000);

        CognitiveEnvelope input;
        input.messageId = QUuid::createUuid();
        input.correlationId = input.messageId;
        input.originOrgan = QStringLiteral("integration-test");
        input.originNode = QStringLiteral("local");
        input.kind = ContributionKind::Observation;
        input.wallTime = QDateTime::currentDateTimeUtc();
        input.privacy = PrivacyClass::Local;
        QDBusReply<QByteArray> submitted = events.call(
            QStringLiteral("Submit"), EnvelopeCodec::encode(input));
        QVERIFY(submitted.isValid() && !submitted.value().isEmpty());

        QDBusReply<qulonglong> count = events.call(QStringLiteral("Count"));
        QVERIFY(count.isValid() && count.value() > 0);

        QDBusReply<QString> requested = interface().call(
            QStringLiteral("RequestRun"), QStringLiteral("consolidation"),
            QStringLiteral("dispatch-test"), count.value(),
            QStringList{QStringLiteral("predictor")},
            QStringList{QStringLiteral("workspace")});
        QVERIFY(requested.isValid() && !requested.value().isEmpty());
        QDBusReply<bool> first = interface().call(QStringLiteral("Dispatch"));
        QDBusReply<QString> dispatchError =
            interface().call(QStringLiteral("LastError"));
        QVERIFY2(
            first.isValid() && first.value(),
            qPrintable(dispatchError.isValid() ? dispatchError.value() : first.error().message()));
        QDBusReply<qulonglong> afterFirst = events.call(QStringLiteral("Count"));
        QVERIFY(afterFirst.isValid());
        QCOMPARE(afterFirst.value(), count.value() + 2);
        QDBusReply<bool> duplicate = interface().call(QStringLiteral("Dispatch"));
        QVERIFY(duplicate.isValid() && duplicate.value());
        QDBusReply<qulonglong> afterDuplicate = events.call(QStringLiteral("Count"));
        QVERIFY(afterDuplicate.isValid());
        QCOMPARE(afterDuplicate.value(), afterFirst.value());

        QDBusReply<QByteArray> stateReply = interface().call(QStringLiteral("State"));
        QString error;
        const QVariantMap state = FabricCodec::decodeMap(stateReply.value(), &error);
        QVERIFY(error.isEmpty());
        const QStringList completed =
            state.value(QStringLiteral("completedWork")).toStringList();
        QCOMPARE(
            QSet<QString>(completed.begin(), completed.end()),
            QSet<QString>({QStringLiteral("predictor"), QStringLiteral("workspace")}));
        QDBusReply<bool> finished = interface().call(
            QStringLiteral("FinishRun"), QStringLiteral("completed"),
            QStringLiteral("accepted owner receipts"));
        QVERIFY(finished.isValid() && finished.value());
        QDBusReply<qulonglong> afterTerminal = events.call(QStringLiteral("Count"));
        QVERIFY(afterTerminal.isValid());
        QCOMPARE(afterTerminal.value(), afterFirst.value() + 1);
        QDBusReply<QByteArray> terminalStateReply = interface().call(QStringLiteral("State"));
        const QVariantMap terminalState = FabricCodec::decodeMap(terminalStateReply.value(), &error);
        QVERIFY(error.isEmpty());
        QVERIFY(!QUuid(terminalState.value(QStringLiteral("terminalContributionId")).toString()).isNull());
    }
};

QTEST_MAIN(TestLifecycledIntegration)
#include "tst_lifecycled_integration.moc"
