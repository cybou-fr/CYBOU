// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT
//
// The perception adapter across a real process boundary.
//
// Two things were proven separately and never together: that the adapter turns a reading into an
// ObservationV1 (against a Journal, in process) and that Event1 binds a claimed origin to the
// calling executable (with an impostor, over D-Bus). This joins them. A contribution that carries
// correct provenance only when nothing is checking it would be worth very little.

#include "OrganStaging.h"

#include "cybou/fabric/OrganBus.h"
#include "cybou/ipc/EventClient.h"
#include "cybou/protocol/Observation.h"

#include <QDBusInterface>
#include <QDir>
#include <QFile>
#include <QProcess>
#include <QProcessEnvironment>
#include <QTemporaryDir>
#include <QTest>

#include <memory>

using namespace cybou;

class TestPerceptiondIntegration : public QObject
{
    Q_OBJECT

private:
    QTemporaryDir m_root;
    cybou::testing::StagedInstall m_install;
    std::unique_ptr<QProcess> m_eventd;
    std::unique_ptr<QProcess> m_perceptiond;
    QString m_systemLink;

    QProcessEnvironment environment() const
    {
        auto env = QProcessEnvironment::systemEnvironment();
        env.insert(QStringLiteral("XDG_STATE_HOME"), m_root.filePath(QStringLiteral("state")));
        env.insert(QStringLiteral("XDG_RUNTIME_DIR"), m_root.filePath(QStringLiteral("runtime")));
        env.insert(QStringLiteral("CYBOU_PERCEPTION_SYSTEM_LINK"), m_systemLink);
        // Fast enough to observe inside a test. The shipped default is ten seconds, which is the
        // rate the source actually changes at.
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

    QList<CognitiveEnvelope> observations()
    {
        QList<CognitiveEnvelope> found;
        EventClient events;
        for (const CognitiveEnvelope &e : events.recent(0)) {
            if (decodeObservation(e.payloadCbor).has_value()) {
                found.append(e);
            }
        }
        return found;
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

        // Both binaries in one directory: eventd grants an organ identity only to an executable
        // beside itself, and the build tree scatters them. See OrganStaging.h.
        const QString eventdPath = m_install.stageFromEnvironment("CYBOU_EVENTD_PATH");
        QVERIFY2(!eventdPath.isEmpty(), "CYBOU_EVENTD_PATH is not set or cannot be staged");
        m_eventd = start(eventdPath);
        QVERIFY(m_eventd);

        EventClient events;
        QTRY_VERIFY_WITH_TIMEOUT(events.isOpen(), 5000);
    }

    void cleanupTestCase()
    {
        stop(m_perceptiond);
        stop(m_eventd);
    }

    // The whole point: a real process, over a real bus, contributing under an origin that eventd
    // verified rather than accepted.
    void theAdapterContributesWithVerifiedProvenance()
    {
        EventClient events;
        const quint64 before = events.count();

        const QString path = m_install.stageFromEnvironment("CYBOU_PERCEPTIOND_PATH");
        QVERIFY2(!path.isEmpty(), "CYBOU_PERCEPTIOND_PATH is not set or cannot be staged");
        m_perceptiond = start(path);
        QVERIFY(m_perceptiond);

        QTRY_VERIFY_WITH_TIMEOUT(!observations().isEmpty(), 10000);

        const CognitiveEnvelope contribution = observations().first();

        // Accepted at all means eventd resolved the caller's executable and agreed it was
        // perceptiond. A binary of the same name elsewhere is refused, which the impostor test in
        // the process suite covers separately.
        QCOMPARE(contribution.originOrgan, QStringLiteral("perceptiond"));

        const auto observation = decodeObservation(contribution.payloadCbor);
        QVERIFY(observation.has_value());
        QCOMPARE(observation->sourceId, QStringLiteral("nixos.system"));
        QCOMPARE(
            observation->value.toString(),
            QStringLiteral("abc-nixos-system-host-26.05"));

        // The producer and the source are different fields and must stay that way across the wire:
        // the envelope says who brought this in, the payload says what was observed.
        QVERIFY(observation->sourceId != contribution.originOrgan);

        QVERIFY(events.count() > before);
    }

    // Polling at ten times a second must not fill the biography. The shipped interval is a hundred
    // times slower, so if this holds here it holds in production by a wide margin.
    void continuousPollingDoesNotAccumulate()
    {
        QVERIFY(m_perceptiond);
        QTRY_VERIFY_WITH_TIMEOUT(!observations().isEmpty(), 10000);

        const int afterFirst = observations().size();
        QTest::qWait(2000);

        // Twenty more polls at the configured interval; the freshness horizon has not lapsed, so
        // none of them is worth recording.
        QCOMPARE(observations().size(), afterFirst);
    }

    // A restart re-reads and records once, then falls silent again.
    //
    // This test first asserted that a restart records nothing, on the reasoning that identity is
    // derived from the reading rather than from who read it. That is wrong: identity includes the
    // acquisition instant, and a fresh instance has no memory of what it last contributed, so its
    // first reading is a new one.
    //
    // Which is the right behaviour, not a defect to suppress. The re-affirmation rule exists so a
    // fact that stays true keeps saying so, and a session boundary is exactly when a fresh reading
    // is worth having - "the system was still this at startup" is evidence, where "it was still
    // this one poll later" is not. The rate is bounded by how often the process starts, and a
    // crash loop is bounded further by systemd's own restart limit.
    void restartRecordsOnceThenFallsSilent()
    {
        QTRY_VERIFY_WITH_TIMEOUT(!observations().isEmpty(), 10000);
        const int before = observations().size();

        stop(m_perceptiond);
        m_perceptiond = start(m_install.stageFromEnvironment("CYBOU_PERCEPTIOND_PATH"));
        QVERIFY(m_perceptiond);
        QTRY_COMPARE_WITH_TIMEOUT(observations().size(), before + 1, 5000);

        // And then it is quiet again: the new instance now holds a contribution that speaks for the
        // present, so its own polling adds nothing.
        QTest::qWait(1500);
        QCOMPARE(observations().size(), before + 1);
    }
};

QTEST_MAIN(TestPerceptiondIntegration)
#include "tst_perceptiond_integration.moc"
