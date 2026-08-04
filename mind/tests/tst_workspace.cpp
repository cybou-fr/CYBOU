// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT
//
// Attention has to be a real constraint, or it is not attention. These tests check that the
// moment is bounded, that urgency outranks age, and that falling out of the moment is not the
// same as being forgotten.

#include "cybou/workspace/Workspace.h"

#include <QSignalSpy>
#include <QTemporaryDir>
#include <QTest>

using namespace cybou;

namespace {

CognitiveEnvelope make(ContributionKind kind, const QString &organ, const QUuid &correlation,
                       const QDateTime &when)
{
    CognitiveEnvelope e;
    e.messageId = QUuid::createUuid();
    e.correlationId = correlation.isNull() ? e.messageId : correlation;
    e.causationId = e.messageId;
    e.originOrgan = organ;
    e.kind = kind;
    e.wallTime = when;
    e.confidence = 1.0;
    e.privacy = PrivacyClass::Node;
    return e;
}

} // namespace

class TestWorkspace : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    void nothingEntersAttentionWithoutBeingRemembered()
    {
        QTemporaryDir dir;
        Journal j(dir.filePath(QStringLiteral("j.db")));
        Workspace w(&j);

        CognitiveEnvelope broken; // no messageId, no organ: the journal will refuse it
        QVERIFY(!w.publish(broken));
        QVERIFY(w.moment().isEmpty());
        QCOMPARE(j.count(), 0u);
    }

    void contributionsGroupIntoCoalitions()
    {
        QTemporaryDir dir;
        Journal j(dir.filePath(QStringLiteral("j.db")));
        Workspace w(&j);

        const QDateTime t = QDateTime::currentDateTimeUtc();
        const QUuid thread = QUuid::createUuid();

        w.publish(make(ContributionKind::Observation, QStringLiteral("sensord"), thread, t));
        w.publish(make(ContributionKind::Hypothesis, QStringLiteral("modeld"), thread, t));
        w.publish(make(ContributionKind::Observation, QStringLiteral("sensord"), QUuid(), t));

        const auto all = w.coalitions(t);
        QCOMPARE(all.size(), 2);

        const Coalition shared = all.at(0).correlationId == thread ? all.at(0) : all.at(1);
        QCOMPARE(shared.members.size(), 2);
        QCOMPARE(shared.organs().size(), 2);
        // Members read oldest first, so a coalition can be replayed as a story.
        QCOMPARE(shared.members.at(0).kind, ContributionKind::Observation);
    }

    void urgencyOutranksAge()
    {
        QTemporaryDir dir;
        Journal j(dir.filePath(QStringLiteral("j.db")));
        Workspace w(&j);

        const QDateTime now = QDateTime::currentDateTimeUtc();

        // An observation from a minute ago, and a need signal from just now.
        w.publish(make(ContributionKind::Observation, QStringLiteral("sensord"), QUuid(),
                       now.addSecs(-60)));
        const auto need = make(ContributionKind::NeedSignal, QStringLiteral("healthd"), QUuid(), now);
        w.publish(need);

        const Coalition f = w.focus(now);
        QVERIFY(f.isValid());
        QCOMPARE(f.correlationId, need.correlationId);
    }

    void severalVoicesOutrankOneRepeatingItself()
    {
        QTemporaryDir dir;
        Journal j(dir.filePath(QStringLiteral("j.db")));
        Workspace w(&j);

        const QDateTime now = QDateTime::currentDateTimeUtc();
        const QUuid alone = QUuid::createUuid();
        const QUuid shared = QUuid::createUuid();

        // Three contributions from one organ...
        for (int i = 0; i < 3; ++i) {
            w.publish(make(ContributionKind::Observation, QStringLiteral("sensord"), alone, now));
        }
        // ...against three organs touching the same concern.
        w.publish(make(ContributionKind::Observation, QStringLiteral("sensord"), shared, now));
        w.publish(make(ContributionKind::Observation, QStringLiteral("modeld"), shared, now));
        w.publish(make(ContributionKind::Observation, QStringLiteral("memoryd"), shared, now));

        QCOMPARE(w.focus(now).correlationId, shared);
    }

    void attentionDecays()
    {
        QTemporaryDir dir;
        Journal j(dir.filePath(QStringLiteral("j.db")));
        Workspace w(&j);

        const QDateTime now = QDateTime::currentDateTimeUtc();
        const auto old = make(ContributionKind::Decision, QStringLiteral("plannerd"), QUuid(), now);
        w.publish(old);

        const double fresh = w.focus(now).salience;
        const double later = w.focus(now.addSecs(600)).salience;

        QVERIFY(later < fresh);
        QVERIFY(later > 0.0); // it fades rather than vanishing
    }

    void theMomentIsBoundedButNothingIsLost()
    {
        QTemporaryDir dir;
        Journal j(dir.filePath(QStringLiteral("j.db")));
        Workspace w(&j, 4);

        const QDateTime now = QDateTime::currentDateTimeUtc();
        for (int i = 0; i < 10; ++i) {
            w.publish(make(ContributionKind::Observation, QStringLiteral("sensord"), QUuid(), now));
        }

        QCOMPARE(w.moment().size(), 4);
        QCOMPARE(j.count(), 10u); // out of attention, still in the biography
        QCOMPARE(j.verify(), 0u);
    }

    void itWakesUpWithSomethingOnItsMind()
    {
        QTemporaryDir dir;
        const QString path = dir.filePath(QStringLiteral("j.db"));

        QUuid lastThread;
        {
            Journal j(path);
            Workspace w(&j, 4);
            const QDateTime now = QDateTime::currentDateTimeUtc();
            for (int i = 0; i < 6; ++i) {
                const auto e = make(ContributionKind::Observation, QStringLiteral("sensord"),
                                    QUuid(), now);
                w.publish(e);
                lastThread = e.correlationId;
            }
        }

        Journal j(path, QStringLiteral("second"));
        Workspace w(&j, 4);
        QVERIFY(w.moment().isEmpty()); // nothing until it looks

        w.rehydrate();
        QCOMPARE(w.moment().size(), 4);
        QCOMPARE(w.moment().first().correlationId, lastThread); // newest first
    }

    void aShiftOfAttentionIsAnnouncedOnce()
    {
        QTemporaryDir dir;
        Journal j(dir.filePath(QStringLiteral("j.db")));
        Workspace w(&j);
        QSignalSpy spy(&w, &Workspace::focusChanged);

        const QDateTime now = QDateTime::currentDateTimeUtc();
        const QUuid thread = QUuid::createUuid();

        w.publish(make(ContributionKind::Observation, QStringLiteral("sensord"), thread, now));
        QCOMPARE(spy.count(), 1);

        // More of the same concern: attention has not moved, so nothing is announced.
        w.publish(make(ContributionKind::Observation, QStringLiteral("modeld"), thread, now));
        QCOMPARE(spy.count(), 1);

        // Something urgent and unrelated does move it.
        w.publish(make(ContributionKind::NeedSignal, QStringLiteral("healthd"), QUuid(), now));
        QCOMPARE(spy.count(), 2);
    }
};

QTEST_MAIN(TestWorkspace)
#include "tst_workspace.moc"
