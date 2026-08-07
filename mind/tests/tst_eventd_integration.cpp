// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/events/EventStore.h"
#include "cybou/ipc/EventClient.h"
#include "cybou/presence/Presence.h"

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

    QProcessEnvironment daemonEnvironment() const
    {
        QProcessEnvironment environment =
            QProcessEnvironment::systemEnvironment();
        environment.insert(
            QStringLiteral("XDG_STATE_HOME"),
            m_state->path());
        return environment;
    }

    void startDaemon()
    {
        m_daemon = std::make_unique<QProcess>();
        m_daemon->setProcessEnvironment(daemonEnvironment());
        m_daemon->setProgram(m_daemonPath);
        m_daemon->start();
        QVERIFY2(
            m_daemon->waitForStarted(3000),
            qPrintable(m_daemon->errorString()));
    }

private Q_SLOTS:
    void initTestCase()
    {
        const QStringList arguments =
            QCoreApplication::arguments();
        QVERIFY2(
            arguments.size() >= 2,
            "eventd integration test requires the cybou-eventd executable path");

        m_daemonPath = arguments.at(1);
        m_state = std::make_unique<QTemporaryDir>();
        QVERIFY(m_state->isValid());

        qputenv(
            "XDG_STATE_HOME",
            m_state->path().toUtf8());

        startDaemon();

        EventClient probe;
        QTRY_VERIFY_WITH_TIMEOUT(probe.isOpen(), 5000);
        QCOMPARE(probe.databaseSchemaVersion(), 2);
    }

    void cleanupTestCase()
    {
        if (!m_daemon) {
            return;
        }

        if (m_daemon->state() != QProcess::NotRunning) {
            m_daemon->terminate();
            if (!m_daemon->waitForFinished(2000)) {
                m_daemon->kill();
                m_daemon->waitForFinished(2000);
            }
        }
    }

    void acceptedSignalIsPostCommit()
    {
        EventClient client;
        QVERIFY(client.isOpen());

        QSignalSpy accepted(&client, &EventStore::accepted);

        const CognitiveEnvelope root =
            observation(QStringLiteral("integrationd"));
        const quint64 sequence = client.append(root);
        QVERIFY2(sequence > 0, qPrintable(client.lastError()));

        QTRY_COMPARE_WITH_TIMEOUT(accepted.count(), 1, 3000);

        const QList<QVariant> arguments = accepted.takeFirst();
        const CognitiveEnvelope observed =
            qvariant_cast<CognitiveEnvelope>(arguments.at(0));
        QCOMPARE(observed.messageId, root.messageId);
        QCOMPARE(arguments.at(1).toULongLong(), sequence);

        CognitiveEnvelope invalid;
        QCOMPARE(client.append(invalid), 0u);
        QTest::qWait(50);
        QCOMPARE(accepted.count(), 0);
    }

    void queriesRoundTripThroughEventd()
    {
        EventClient client;
        QVERIFY(client.isOpen());

        const CognitiveEnvelope root =
            observation(QStringLiteral("queryd"));
        const quint64 sequence = client.append(root);
        QVERIFY(sequence > 0);

        QVERIFY(client.contains(root.messageId));

        const auto persisted =
            client.contribution(root.messageId);
        QVERIFY(persisted.has_value());
        QCOMPARE(persisted->messageId, root.messageId);
        QCOMPARE(persisted->originOrgan, root.originOrgan);

        const auto recent = client.recent(0);
        QVERIFY(!recent.isEmpty());
        QVERIFY(client.count() >= 1);
        QCOMPARE(client.verify(), 0u);
    }

    void defaultPresenceUsesEventdAndSharesTheSession()
    {
        Presence first;
        Presence second;

        QVERIFY2(first.wake(), qPrintable(first.lastError()));
        QVERIFY2(second.wake(), qPrintable(second.lastError()));

        const QVariantMap firstIdentity = first.identityState();
        const QVariantMap secondIdentity = second.identityState();

        QCOMPARE(
            firstIdentity.value(QStringLiteral("uuid")).toString(),
            secondIdentity.value(QStringLiteral("uuid")).toString());
        QCOMPARE(
            firstIdentity.value(QStringLiteral("sessionCount")).toLongLong(),
            secondIdentity.value(QStringLiteral("sessionCount")).toLongLong());

        const QUuid intention =
            first.promise(QStringLiteral("eventd-backed commitment"));
        QVERIFY2(!intention.isNull(), qPrintable(first.lastError()));

        QCOMPARE(
            second.obligations(),
            QStringList{QStringLiteral("eventd-backed commitment")});
    }

    void secondEventdCannotOwnTheServiceName()
    {
        QProcess second;
        second.setProcessEnvironment(daemonEnvironment());
        second.setProgram(m_daemonPath);
        second.start();
        QVERIFY(second.waitForStarted(3000));
        QVERIFY2(
            second.waitForFinished(3000),
            "a second eventd must fail instead of becoming another owner");
        QVERIFY(second.exitCode() != 0);
    }

    void daemonFailureDoesNotFallBackToLocalSQLite()
    {
        QVERIFY(m_daemon);
        m_daemon->terminate();
        if (!m_daemon->waitForFinished(2000)) {
            m_daemon->kill();
            QVERIFY(m_daemon->waitForFinished(2000));
        }

        EventClient client;
        QVERIFY(!client.isOpen());

        Presence surface;
        QVERIFY(!surface.wake());
        QVERIFY(!surface.lastError().isEmpty());
    }
};

QTEST_MAIN(TestEventdIntegration)
#include "tst_eventd_integration.moc"
