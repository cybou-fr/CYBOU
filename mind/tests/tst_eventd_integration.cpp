// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "OrganStaging.h"

#include "cybou/events/EventStore.h"
#include "cybou/ipc/EventClient.h"

#include <QProcess>
#include <QProcessEnvironment>
#include <QSignalSpy>
#include <QTemporaryDir>
#include <QTest>

#include <memory>

using namespace cybou;

namespace {

CognitiveEnvelope observation(const QString &organ)
{
    CognitiveEnvelope envelope;
    envelope.messageId = QUuid::createUuid();
    envelope.correlationId = envelope.messageId;
    envelope.originOrgan = organ;
    envelope.kind = ContributionKind::Observation;
    envelope.wallTime = QDateTime::currentDateTimeUtc();
    envelope.confidence = 1.0;
    envelope.privacy = PrivacyClass::Node;
    return envelope;
}

} // namespace

class TestEventdIntegration : public QObject
{
    Q_OBJECT

private:
    std::unique_ptr<QTemporaryDir> m_state;
    std::unique_ptr<QProcess> m_daemon;
    QString m_daemonPath;
    cybou::testing::StagedInstall m_install;

private Q_SLOTS:
    void initTestCase()
    {
        m_daemonPath =
            m_install.stageFromEnvironment("CYBOU_EVENTD_PATH");
        QVERIFY2(
            !m_daemonPath.isEmpty(),
            "CYBOU_EVENTD_PATH is not set");
        m_state = std::make_unique<QTemporaryDir>();
        QVERIFY(m_state->isValid());

        qputenv(
            "XDG_STATE_HOME",
            m_state->path().toUtf8());

        m_daemon = std::make_unique<QProcess>();
        m_daemon->setProgram(m_daemonPath);
        m_daemon->setProcessEnvironment(
            QProcessEnvironment::systemEnvironment());
        m_daemon->start();
        QVERIFY(m_daemon->waitForStarted(3000));

        EventClient probe;
        QTRY_VERIFY_WITH_TIMEOUT(probe.isOpen(), 5000);
        QCOMPARE(probe.databaseSchemaVersion(), 2);
    }

    void cleanupTestCase()
    {
        if (m_daemon
            && m_daemon->state() != QProcess::NotRunning) {
            m_daemon->terminate();
            if (!m_daemon->waitForFinished(2000)) {
                m_daemon->kill();
                m_daemon->waitForFinished(2000);
            }
        }
    }

    void acceptedIsPostCommit()
    {
        EventClient client;
        QVERIFY(client.isOpen());

        QSignalSpy accepted(
            &client,
            &EventStore::accepted);

        const CognitiveEnvelope root =
            observation(QStringLiteral("integrationd"));
        const quint64 sequence =
            client.append(root);
        QVERIFY2(sequence > 0, qPrintable(client.lastError()));

        QTRY_COMPARE_WITH_TIMEOUT(
            accepted.count(),
            1,
            3000);

        CognitiveEnvelope invalid;
        QCOMPARE(client.append(invalid), 0u);
        QTest::qWait(50);
        QCOMPARE(accepted.count(), 1);
    }

    // ADR-0028: Submit is not a door to destroying biography.
    //
    // Every organ can already submit contributions, so if an erasure kind were accepted here then
    // any of them could erase anything, and the authorization boundary M9 is meant to build would
    // have been pre-emptied by an enum value. A proposal is not permission to execute.
    void submittingAnErasureKindIsRefused()
    {
        EventClient client;
        QVERIFY(client.isOpen());
        const quint64 before = client.count();

        // A real cause, appended first. The first version of this test invented a random
        // causationId, so every forged envelope was refused for naming a cause that does not exist
        // - and the test passed with the erasure check disabled, proving nothing at all. It has to
        // be an envelope the Journal would otherwise accept.
        const CognitiveEnvelope cause = observation(QStringLiteral("integrationd"));
        QVERIFY(client.append(cause) > 0);
        const quint64 afterCause = client.count();

        for (const ContributionKind kind :
             {ContributionKind::ErasureRequested, ContributionKind::ErasureApplied}) {
            CognitiveEnvelope forged = observation(QStringLiteral("integrationd"));
            forged.kind = kind;
            forged.causationId = cause.messageId;

            QCOMPARE(client.append(forged), 0u);
        }

        QCOMPARE(client.count(), afterCause);
        QVERIFY(afterCause > before);
    }

    void queriesRoundTrip()
    {
        EventClient client;
        const CognitiveEnvelope root =
            observation(QStringLiteral("queryd"));

        QVERIFY(client.append(root) > 0);
        QVERIFY(client.contains(root.messageId));

        const auto persisted =
            client.contribution(root.messageId);
        QVERIFY(persisted.has_value());
        QCOMPARE(persisted->messageId, root.messageId);
        const auto bySequence = client.atSequence(client.count());
        QVERIFY(bySequence.has_value());
        QCOMPARE(bySequence->messageId, root.messageId);
        QCOMPARE(client.verify(), 0u);
    }

    void consumerOffsetIsMonotonicAndSurvivesOwnerRestart()
    {
        EventClient client;
        const quint64 initialHead = client.count();
        QVERIFY(client.ensureConsumer(QStringLiteral("integration.consumer"), initialHead));
        QCOMPARE(client.consumerBacklog(QStringLiteral("integration.consumer")).value(), 0u);

        QVERIFY(client.append(observation(QStringLiteral("consumer-test"))) > initialHead);
        QCOMPARE(client.consumerBacklog(QStringLiteral("integration.consumer")).value(), 1u);
        const quint64 head = client.count();
        QVERIFY(client.advanceConsumer(QStringLiteral("integration.consumer"), head));
        QVERIFY(client.advanceConsumer(QStringLiteral("integration.consumer"), head));
        QVERIFY(!client.advanceConsumer(QStringLiteral("integration.consumer"), head - 1));
        QVERIFY(!client.advanceConsumer(QStringLiteral("integration.consumer"), head + 1));

        m_daemon->terminate();
        QVERIFY(m_daemon->waitForFinished(3000));
        m_daemon = std::make_unique<QProcess>();
        m_daemon->setProgram(m_daemonPath);
        m_daemon->setProcessEnvironment(QProcessEnvironment::systemEnvironment());
        m_daemon->start();
        QVERIFY(m_daemon->waitForStarted(3000));
        EventClient recovered;
        QTRY_VERIFY_WITH_TIMEOUT(recovered.isOpen(), 5000);
        QCOMPARE(recovered.consumerBacklog(QStringLiteral("integration.consumer")).value(), 0u);
        QVERIFY(recovered.ensureConsumer(QStringLiteral("integration.consumer"), 0));
        QVERIFY(!recovered.ensureConsumer(QStringLiteral("Invalid Consumer"), 0));
    }

    void secondOwnerIsRejected()
    {
        QProcess second;
        second.setProgram(m_daemonPath);
        second.setProcessEnvironment(
            QProcessEnvironment::systemEnvironment());
        second.start();
        QVERIFY(second.waitForStarted(3000));
        QVERIFY(second.waitForFinished(3000));
        QVERIFY(second.exitCode() != 0);
    }
};

QTEST_MAIN(TestEventdIntegration)
#include "tst_eventd_integration.moc"
