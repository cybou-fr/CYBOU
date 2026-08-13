// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/intentions/Intentions.h"
#include "cybou/storage/Journal.h"

#include <QTemporaryDir>
#include <QTest>

using namespace cybou;

namespace {

CognitiveEnvelope requestObservation(const QString &description = QStringLiteral("request"))
{
    CognitiveEnvelope e;
    e.messageId = QUuid::createUuid();
    e.correlationId = e.messageId;
    e.originOrgan = QStringLiteral("presenced");
    e.kind = ContributionKind::Observation;
    e.wallTime = QDateTime::currentDateTimeUtc();
    e.privacy = PrivacyClass::Node;
    e.payloadCbor = description.toUtf8();
    return e;
}

} // namespace

// A Journal whose paged reads can be made to fail, so the one case that matters here - reading
// history does not work - can actually be produced.
class UnreadableAfter : public Journal
{
public:
    using Journal::Journal;

    ContributionPage after(quint64 afterSequence, int limit) const override
    {
        if (m_readsFail) {
            ContributionPage page;
            page.ok = false;
            return page;
        }
        return Journal::after(afterSequence, limit);
    }

    void failReads(bool fail) { m_readsFail = fail; }

private:
    bool m_readsFail{false};
};

class TestIntentions : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    void formingNeedsAnExistingCause()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        Intentions intentions(&journal);

        QVERIFY(intentions.form(
            QStringLiteral("verify sound"), QString(), QUuid::createUuid()).isNull());
        QCOMPARE(journal.count(), 0u);
    }

    void formedIntentionIsCausallyGrounded()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        Intentions intentions(&journal);

        const CognitiveEnvelope request = requestObservation();
        QVERIFY(journal.append(request) > 0);

        const QUuid id = intentions.form(
            QStringLiteral("verify sound after reboot"),
            QStringLiteral("next session"),
            request.messageId);
        QVERIFY(!id.isNull());

        const auto stored = journal.contribution(id);
        QVERIFY(stored.has_value());
        QCOMPARE(stored->kind, ContributionKind::Intention);
        QCOMPARE(stored->causationId, request.messageId);
        QVERIFY(stored->causationId != stored->messageId);
        QVERIFY(stored->evidence.isEmpty());
    }

    void closingValidatesTheTargetAndCannotRepeat()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        Intentions intentions(&journal);

        const CognitiveEnvelope request = requestObservation();
        QVERIFY(journal.append(request) > 0);
        QVERIFY(!intentions.close(request.messageId, Resolution::Fulfilled));

        const QUuid id = intentions.form(
            QStringLiteral("check the network"), QString(), request.messageId);
        QVERIFY(!id.isNull());
        QVERIFY(intentions.close(id, Resolution::Fulfilled));
        QVERIFY(!intentions.close(id, Resolution::Abandoned));
        QVERIFY(intentions.open().value().isEmpty());

        const auto outcome = journal.recent(1).first();
        QCOMPARE(outcome.kind, ContributionKind::Outcome);
        QCOMPARE(outcome.causationId, id);
        QVERIFY(outcome.evidence.isEmpty());
    }

    void survivesRestartAndReplaysTheWholeEpisode()
    {
        QTemporaryDir dir;
        const QString path = dir.filePath(QStringLiteral("j.db"));

        QUuid requestId;
        QUuid intentionId;
        {
            Journal journal(path);
            Intentions intentions(&journal);
            const CognitiveEnvelope request = requestObservation();
            requestId = request.messageId;
            QVERIFY(journal.append(request) > 0);
            intentionId = intentions.form(
                QStringLiteral("verify sound and network after reboot"),
                QString(),
                request.messageId);
            QVERIFY(!intentionId.isNull());
        }

        {
            Journal journal(path, QStringLiteral("second"));
            Intentions intentions(&journal);
            QCOMPARE(intentions.open().value().size(), 1);
            QVERIFY(intentions.close(intentionId, Resolution::Obsolete));

            const auto episode = journal.episode(requestId);
            QCOMPARE(episode.size(), 3);
            QCOMPARE(episode.at(0).kind, ContributionKind::Observation);
            QCOMPARE(episode.at(1).kind, ContributionKind::Intention);
            QCOMPARE(episode.at(2).kind, ContributionKind::Outcome);
        }
    }

    // open() answers from a cursor now, so a read costs what arrived since the last one rather than
    // the length of the biography. That is only safe if a later read still sees what happened in
    // between, and every existing test here would pass against a cache that never advanced.
    //
    // The second instance is the case that matters: this one did not write the closure and has no
    // way to learn of it except by reading the Journal again.
    void aLaterReadSeesACommitmentClosedElsewhere()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        QVERIFY(journal.isOpen());

        const CognitiveEnvelope request = requestObservation();
        QVERIFY(journal.append(request) > 0);

        Intentions intentions(&journal);
        const QUuid id = intentions.form(
            QStringLiteral("water the plants"), QStringLiteral("daily"), request.messageId);
        QVERIFY(!id.isNull());
        QCOMPARE(intentions.open().value().size(), 1);

        Intentions other(&journal);
        QVERIFY(other.close(id, Resolution::Fulfilled));

        QVERIFY(intentions.open().value().isEmpty());
    }

    // A read that could not be assembled must not look like a Mind that owes nothing.
    //
    // open() used to return an empty list both when there were no obligations and when the Journal
    // could not be read. Those are different answers, and collapsing them chose the most
    // reassuring one: Presence would show "nothing outstanding" precisely when it had no idea.
    void anUnreadableJournalIsNotAnEmptyObligationSet()
    {
        QTemporaryDir dir;
        UnreadableAfter journal(dir.filePath(QStringLiteral("j.db")));
        QVERIFY(journal.isOpen());

        const CognitiveEnvelope request = requestObservation();
        QVERIFY(journal.append(request) > 0);

        Intentions intentions(&journal);
        QVERIFY(!intentions
                     .form(QStringLiteral("outstanding"), QStringLiteral("test"), request.messageId)
                     .isNull());
        QCOMPARE(intentions.open().value().size(), 1);

        Intentions cold(&journal);
        journal.failReads(true);
        const auto answer = cold.open();
        QVERIFY2(!answer.has_value(), "a failed read must not answer with an empty set");
        QVERIFY(!cold.lastError().isEmpty());

        journal.failReads(false);
        QCOMPARE(cold.open().value().size(), 1);
    }

    // A closed commitment is not merely filtered out of the answer, it stops being carried at all.
    //
    // The first cursor version kept every intention ever formed and filtered on each call, so a
    // read cost the number of commitments a life had ever had rather than the number it currently
    // holds - the same unbounded shape as the replay it replaced, one level up.
    void closingACommitmentStopsCarryingIt()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        QVERIFY(journal.isOpen());

        const CognitiveEnvelope request = requestObservation();
        QVERIFY(journal.append(request) > 0);

        Intentions intentions(&journal);
        QList<QUuid> ids;
        for (int i = 0; i < 5; ++i) {
            const QUuid id = intentions.form(
                QStringLiteral("obligation %1").arg(i),
                QStringLiteral("test"),
                request.messageId);
            QVERIFY(!id.isNull());
            ids.append(id);
        }
        QCOMPARE(intentions.open().value().size(), 5);

        for (const QUuid &id : ids) {
            QVERIFY(intentions.close(id, Resolution::Fulfilled));
        }
        QVERIFY(intentions.open().value().isEmpty());

        // A fresh instance replays the same history and agrees. If closure were only a filter, the
        // two would still agree - so what this pins is that both are empty rather than that they
        // match, which they would either way.
        Intentions replayed(&journal);
        QVERIFY(replayed.open().value().isEmpty());

        // And the order of what remains is still acceptance order after removals.
        const QUuid kept = intentions.form(
            QStringLiteral("still open"), QStringLiteral("test"), request.messageId);
        QVERIFY(!kept.isNull());
        const QList<Intention> open = intentions.open().value();
        QCOMPARE(open.size(), 1);
        QCOMPARE(open.first().description, QStringLiteral("still open"));
    }

    // And a commitment formed elsewhere appears, for the same reason in the other direction.
    void aLaterReadSeesACommitmentFormedElsewhere()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        QVERIFY(journal.isOpen());

        const CognitiveEnvelope request = requestObservation();
        QVERIFY(journal.append(request) > 0);

        Intentions intentions(&journal);
        QVERIFY(intentions.open().value().isEmpty());

        Intentions other(&journal);
        QVERIFY(!other.form(
                     QStringLiteral("call the dentist"),
                     QStringLiteral("weekday"),
                     request.messageId)
                     .isNull());

        const QList<Intention> open = intentions.open().value();
        QCOMPARE(open.size(), 1);
        QCOMPARE(open.first().description, QStringLiteral("call the dentist"));
    }

    void oldestObligationComesFirst()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        Intentions intentions(&journal);

        for (const QString &description : {
                 QStringLiteral("first"),
                 QStringLiteral("second"),
                 QStringLiteral("third")}) {
            const CognitiveEnvelope request = requestObservation(description);
            QVERIFY(journal.append(request) > 0);
            QVERIFY(!intentions.form(description, QString(), request.messageId).isNull());
        }

        const auto open = intentions.open().value();
        QCOMPARE(open.size(), 3);
        QCOMPARE(open.at(0).description, QStringLiteral("first"));
        QCOMPARE(open.at(2).description, QStringLiteral("third"));
    }
};

QTEST_MAIN(TestIntentions)
#include "tst_intentions.moc"
