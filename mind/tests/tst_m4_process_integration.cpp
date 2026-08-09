// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/fabric/OrganClients.h"
#include "cybou/ipc/EventClient.h"
#include "cybou/presence/Presence.h"

#include <QDebug>
#include <QDir>
#include <QFile>
#include <QProcess>
#include <QProcessEnvironment>
#include <QSet>
#include <QTemporaryDir>
#include <QTest>

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
    QString m_presencedPath;

    std::unique_ptr<QProcess> m_eventd;
    std::unique_ptr<QProcess> m_identityd;
    std::unique_ptr<QProcess> m_intentiond;
    std::unique_ptr<QProcess> m_predictord;
    std::unique_ptr<QProcess> m_selfd;
    std::unique_ptr<QProcess> m_workspaced;
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
        const QString &path)
    {
        auto process = std::make_unique<QProcess>();
        process->setProgram(path);
        process->setProcessEnvironment(environment());
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
        m_presencedPath =
            qEnvironmentVariable("CYBOU_PRESENCED_PATH");

        QVERIFY2(!m_eventdPath.isEmpty(), "CYBOU_EVENTD_PATH is not set");
        QVERIFY2(!m_identitydPath.isEmpty(), "CYBOU_IDENTITYD_PATH is not set");
        QVERIFY2(!m_intentiondPath.isEmpty(), "CYBOU_INTENTIOND_PATH is not set");
        QVERIFY2(!m_predictordPath.isEmpty(), "CYBOU_PREDICTORD_PATH is not set");
        QVERIFY2(!m_selfdPath.isEmpty(), "CYBOU_SELFD_PATH is not set");
        QVERIFY2(!m_workspacedPath.isEmpty(), "CYBOU_WORKSPACED_PATH is not set");
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

        m_presenced = start(m_presencedPath);
        QVERIFY(m_presenced);
        PresenceClient presence;
        QTRY_VERIFY_WITH_TIMEOUT(presence.ready(), 5000);
    }

    void cleanupTestCase()
    {
        stop(m_presenced);
        stop(m_selfd);
        stop(m_workspaced);
        stop(m_predictord);
        stop(m_intentiond);
        stop(m_identityd);
        stop(m_eventd);
    }

    void sevenDistinctProcessesOwnTheRuntime()
    {
        QSet<qint64> pids{
            m_eventd->processId(),
            m_identityd->processId(),
            m_intentiond->processId(),
            m_predictord->processId(),
            m_selfd->processId(),
            m_workspaced->processId(),
            m_presenced->processId(),
        };

        QCOMPARE(pids.size(), 7);
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
    }
};

QTEST_MAIN(TestM4Processes)
#include "tst_m4_process_integration.moc"
