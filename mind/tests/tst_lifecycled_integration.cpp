// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/fabric/FabricCodec.h"
#include "cybou/fabric/OrganBus.h"
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
        startDaemon();
    }

    void cleanupTestCase() { stopDaemon(); }

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
};

QTEST_MAIN(TestLifecycledIntegration)
#include "tst_lifecycled_integration.moc"
