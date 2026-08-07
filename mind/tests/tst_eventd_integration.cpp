// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/events/EventStore.h"
#include "cybou/ipc/EventClient.h"

#include <QCoreApplication>
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

private Q_SLOTS:
    void initTestCase()
    {
        const QStringList arguments =
            QCoreApplication::arguments();
        QVERIFY(arguments.size() >= 2);

        m_daemonPath = arguments.at(1);
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
        QCOMPARE(client.verify(), 0u);
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
