// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT
//
// The epistemic owner across real process boundaries.
//
// In process, the projection has been proven to derive the right answer from contributions handed
// to it. That leaves the parts only a process boundary can show: that a live acceptance announced
// over D-Bus actually reaches it, that its checkpoint under the real state root survives a restart,
// and that it never writes to Event1. ADR-0027 fixes the last of those, and an owner that reads and
// writes is exactly the arrangement the ADR exists to prevent - so it is worth testing rather than
// asserting in prose.

#include "OrganStaging.h"

#include "cybou/fabric/FabricCodec.h"
#include "cybou/fabric/OrganBus.h"
#include "cybou/ipc/EventClient.h"
#include "cybou/protocol/Observation.h"

#include <QVariantMap>
#include <QDBusConnection>
#include <QDBusInterface>
#include <QDBusReply>
#include <QDir>
#include <QFile>
#include <QProcess>
#include <QProcessEnvironment>
#include <QTemporaryDir>
#include <QTest>

#include <memory>

using namespace cybou;

class TestEpistemicdIntegration : public QObject
{
    Q_OBJECT

private:
    QTemporaryDir m_root;
    cybou::testing::StagedInstall m_install;
    std::unique_ptr<QProcess> m_eventd;
    std::unique_ptr<QProcess> m_perceptiond;
    std::unique_ptr<QProcess> m_epistemicd;
    QString m_systemLink;

    QProcessEnvironment environment() const
    {
        auto env = QProcessEnvironment::systemEnvironment();
        env.insert(QStringLiteral("XDG_STATE_HOME"), m_root.filePath(QStringLiteral("state")));
        env.insert(QStringLiteral("XDG_RUNTIME_DIR"), m_root.filePath(QStringLiteral("runtime")));
        env.insert(QStringLiteral("CYBOU_PERCEPTION_SYSTEM_LINK"), m_systemLink);
        env.insert(QStringLiteral("CYBOU_PERCEPTION_INTERVAL_MS"), QStringLiteral("100"));
        return env;
    }

    std::unique_ptr<QProcess> start(const QString &path)
    {
        auto process = std::make_unique<QProcess>();
        process->setProgram(path);
        process->setProcessEnvironment(environment());
        process->start();
        return process->waitForStarted(3000) ? std::move(process) : nullptr;
    }

    static void stop(std::unique_ptr<QProcess> &process)
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

    QDBusInterface epistemic() const
    {
        return QDBusInterface(
            QString::fromLatin1(kEpistemicEndpoint.service),
            QString::fromLatin1(kEpistemicEndpoint.objectPath),
            QString::fromLatin1(kEpistemicEndpoint.interfaceName),
            QDBusConnection::sessionBus());
    }

    QVariantMap knowledgeOf(const QString &subject) const
    {
        QDBusInterface iface = epistemic();
        const QDBusReply<QByteArray> reply =
            iface.call(QStringLiteral("KnowledgeOf"), subject);
        if (!reply.isValid()) {
            return {};
        }
        // Decoded through the fabric codec, like every other organ's projection. A test that
        // decoded raw CBOR would keep passing if this organ drifted off the shared envelope again.
        return FabricCodec::decodeMap(reply.value());
    }

    QString statusOf(const QString &subject) const
    {
        return knowledgeOf(subject).value(QStringLiteral("status")).toString();
    }

    qulonglong cursor() const
    {
        QDBusInterface iface = epistemic();
        const QDBusReply<qulonglong> reply = iface.call(QStringLiteral("Cursor"));
        return reply.isValid() ? reply.value() : 0;
    }

private Q_SLOTS:
    void initTestCase()
    {
        QVERIFY(m_root.isValid());
        QVERIFY(m_install.isValid());
        QVERIFY(QDir().mkpath(m_root.filePath(QStringLiteral("runtime"))));

        const QString store = m_root.filePath(QStringLiteral("abc-nixos-system-host-26.05"));
        QVERIFY(QDir().mkpath(store));
        m_systemLink = m_root.filePath(QStringLiteral("current-system"));
        QVERIFY(QFile::link(store, m_systemLink));

        // One directory for all three: eventd grants an organ identity only to an executable beside
        // itself, and the build tree scatters them. See OrganStaging.h.
        const QString eventdPath = m_install.stageFromEnvironment("CYBOU_EVENTD_PATH");
        QVERIFY2(!eventdPath.isEmpty(), "CYBOU_EVENTD_PATH is not set or cannot be staged");
        m_eventd = start(eventdPath);
        QVERIFY(m_eventd);

        EventClient events;
        QTRY_VERIFY_WITH_TIMEOUT(events.isOpen(), 5000);
    }

    void cleanupTestCase()
    {
        stop(m_epistemicd);
        stop(m_perceptiond);
        stop(m_eventd);
    }

    // Nothing observed is a normal state and must answer as one, over the wire as well as in the
    // library. An owner that started before anything was perceived is the ordinary case at boot.
    void anEmptyBiographyAnswersUnknown()
    {
        const QString path = m_install.stageFromEnvironment("CYBOU_EPISTEMICD_PATH");
        QVERIFY2(!path.isEmpty(), "CYBOU_EPISTEMICD_PATH is not set or cannot be staged");
        m_epistemicd = start(path);
        QVERIFY(m_epistemicd);

        QTRY_VERIFY_WITH_TIMEOUT(epistemic().isValid(), 5000);
        QCOMPARE(statusOf(QStringLiteral("current-system")), QStringLiteral("unknown"));
        QCOMPARE(cursor(), 0u);
    }

    // The live path: perceptiond observes, Event1 accepts and announces, and the projection knows -
    // with no restart and no polling in between. Everything else here is checkpoint mechanics; this
    // is the one that proves the two organs are connected at all.
    void aLiveObservationBecomesKnowledgeWithoutARestart()
    {
        QVERIFY(m_epistemicd);
        QCOMPARE(statusOf(QStringLiteral("current-system")), QStringLiteral("unknown"));

        const QString path = m_install.stageFromEnvironment("CYBOU_PERCEPTIOND_PATH");
        QVERIFY2(!path.isEmpty(), "CYBOU_PERCEPTIOND_PATH is not set or cannot be staged");
        m_perceptiond = start(path);
        QVERIFY(m_perceptiond);

        QTRY_COMPARE_WITH_TIMEOUT(
            statusOf(QStringLiteral("current-system")), QStringLiteral("observed"), 10000);

        const QVariantMap knowledge = knowledgeOf(QStringLiteral("current-system"));
        QCOMPARE(knowledge.value(QStringLiteral("current")).toList().size(), 1);
        QCOMPARE(
            knowledge.value(QStringLiteral("current")).toList().at(0).toMap()
                .value(QStringLiteral("value")).toString(),
            QStringLiteral("abc-nixos-system-host-26.05"));

        // The claim is about the subject perception reported, under the source perception named -
        // not under the organ that carried it. Those are different fields and stay different across
        // two process boundaries.
        QCOMPARE(
            knowledge.value(QStringLiteral("current")).toList().at(0).toMap()
                .value(QStringLiteral("sourceId")).toString(),
            QStringLiteral("nixos.system"));

        QVERIFY(cursor() > 0);
    }

    // ADR-0027 says this organ owns the derived projection and never writes to Event1. A component
    // that both decides what is true and records what is true has no one to check it, so the
    // boundary is worth a test rather than a comment.
    void theProjectionNeverContributes()
    {
        QVERIFY(m_epistemicd);
        stop(m_perceptiond);

        EventClient events;
        const quint64 before = events.count();
        QVERIFY(before > 0);

        // Ask it everything it can answer. Reading is the only thing it does, and reading must not
        // leave a trace in the biography it reads.
        QVERIFY(!knowledgeOf(QStringLiteral("current-system")).isEmpty());
        QDBusInterface iface = epistemic();
        const QDBusReply<QByteArray> all = iface.call(QStringLiteral("Knowledge"));
        QVERIFY(all.isValid());
        QString decodeError;
        FabricCodec::decodeList(all.value(), &decodeError);
        QVERIFY2(decodeError.isEmpty(), qPrintable(decodeError));
        QTest::qWait(1000);

        QCOMPARE(events.count(), before);
    }

    // The checkpoint earns its place only if a restart resumes instead of replaying from zero. With
    // perceptiond stopped there is nothing new to take in, so a non-zero cursor immediately after
    // start can only have come from the persisted one.
    void aRestartResumesFromThePersistedCheckpoint()
    {
        QVERIFY(m_epistemicd);
        const qulonglong before = cursor();
        QVERIFY(before > 0);
        const QVariantMap knowledgeBefore = knowledgeOf(QStringLiteral("current-system"));

        stop(m_epistemicd);
        QTRY_VERIFY_WITH_TIMEOUT(!epistemic().isValid(), 5000);

        m_epistemicd = start(m_install.stageFromEnvironment("CYBOU_EPISTEMICD_PATH"));
        QVERIFY(m_epistemicd);
        QTRY_VERIFY_WITH_TIMEOUT(epistemic().isValid(), 5000);

        QCOMPARE(cursor(), before);
        QCOMPARE(
            knowledgeOf(QStringLiteral("current-system"))
                .value(QStringLiteral("current")).toList().at(0).toMap()
                .value(QStringLiteral("value")).toString(),
            knowledgeBefore.value(QStringLiteral("current")).toList().at(0).toMap()
                .value(QStringLiteral("value")).toString());
    }

    // What was accepted while the owner was down is taken in on the next start. The projection is a
    // function of the whole biography, not of what happened to arrive while it was listening - and
    // an organ that only learns from live announcements would silently disagree with the Journal
    // after every restart.
    void whatWasAcceptedWhileItWasDownIsTakenInOnTheNextStart()
    {
        QVERIFY(m_epistemicd);
        const qulonglong before = cursor();

        stop(m_epistemicd);
        QTRY_VERIFY_WITH_TIMEOUT(!epistemic().isValid(), 5000);

        // A different system, observed while nothing was listening.
        const QString store = m_root.filePath(QStringLiteral("def-nixos-system-host-26.11"));
        QVERIFY(QDir().mkpath(store));
        QVERIFY(QFile::remove(m_systemLink));
        QVERIFY(QFile::link(store, m_systemLink));

        m_perceptiond = start(m_install.stageFromEnvironment("CYBOU_PERCEPTIOND_PATH"));
        QVERIFY(m_perceptiond);

        EventClient events;
        QTRY_VERIFY_WITH_TIMEOUT(events.count() > before, 10000);
        stop(m_perceptiond);

        m_epistemicd = start(m_install.stageFromEnvironment("CYBOU_EPISTEMICD_PATH"));
        QVERIFY(m_epistemicd);
        QTRY_VERIFY_WITH_TIMEOUT(epistemic().isValid(), 5000);

        QVERIFY(cursor() > before);
        const QVariantMap knowledge = knowledgeOf(QStringLiteral("current-system"));
        QCOMPARE(knowledge.value(QStringLiteral("status")).toString(), QStringLiteral("observed"));
        QCOMPARE(
            knowledge.value(QStringLiteral("current")).toList().at(0).toMap()
                .value(QStringLiteral("value")).toString(),
            QStringLiteral("def-nixos-system-host-26.11"));

        // The earlier reading is not forgotten, it is outranked. Keeping what was superseded is how
        // the projection can later be asked why it changed its mind.
        QCOMPARE(knowledge.value(QStringLiteral("superseded")).toList().size(), 1);
    }
};

QTEST_MAIN(TestEpistemicdIntegration)
#include "tst_epistemicd_integration.moc"
