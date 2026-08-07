// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/storage/Journal.h"

#include <QSqlDatabase>
#include <QSqlQuery>
#include <QTemporaryDir>
#include <QTest>

using namespace cybou;

namespace {

CognitiveEnvelope observation(
    PrivacyClass privacy = PrivacyClass::Node,
    const QString &organ = QStringLiteral("perceptiond"))
{
    CognitiveEnvelope e;
    e.messageId = QUuid::createUuid();
    e.correlationId = e.messageId;
    e.originOrgan = organ;
    e.wallTime = QDateTime::currentDateTimeUtc();
    e.kind = ContributionKind::Observation;
    e.privacy = privacy;
    return e;
}

CognitiveEnvelope derived(
    ContributionKind kind,
    const CognitiveEnvelope &cause,
    const QString &organ = QStringLiteral("modeld"))
{
    CognitiveEnvelope e;
    e.messageId = QUuid::createUuid();
    e.correlationId = cause.correlationId;
    e.causationId = cause.messageId;
    e.originOrgan = organ;
    e.kind = kind;
    e.wallTime = QDateTime::currentDateTimeUtc();
    e.privacy = cause.privacy;
    return e;
}

} // namespace

class TestJournal : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    void appendsAndFindsContributions()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        const CognitiveEnvelope root = observation();

        QCOMPARE(journal.append(root), 1u);
        QVERIFY(journal.contains(root.messageId));
        QVERIFY(journal.contribution(root.messageId).has_value());
        QCOMPARE(journal.verify(), 0u);
    }

    void missingCauseAndEvidenceAreRejected()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));

        CognitiveEnvelope withMissingCause = observation();
        withMissingCause.kind = ContributionKind::Decision;
        withMissingCause.causationId = QUuid::createUuid();
        QCOMPARE(journal.append(withMissingCause), 0u);
        QVERIFY(journal.lastError().contains(QStringLiteral("causal")));

        CognitiveEnvelope withMissingEvidence = observation();
        withMissingEvidence.kind = ContributionKind::Prediction;
        withMissingEvidence.evidence = {QUuid::createUuid()};
        QCOMPARE(journal.append(withMissingEvidence), 0u);
        QVERIFY(journal.lastError().contains(QStringLiteral("evidence")));
    }

    void referencesOnlyPointIntoTheExistingPast()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));

        const CognitiveEnvelope root = observation();
        QVERIFY(journal.append(root) > 0);

        CognitiveEnvelope decision = derived(ContributionKind::Decision, root);
        QVERIFY(journal.append(decision) > 0);

        const auto stored = journal.contribution(decision.messageId);
        QVERIFY(stored.has_value());
        QCOMPARE(stored->causationId, root.messageId);
    }

    void evidenceAndCapabilityRoundTrip()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));

        const CognitiveEnvelope first = observation();
        const CognitiveEnvelope second = observation();
        QVERIFY(journal.append(first) > 0);
        QVERIFY(journal.append(second) > 0);

        CognitiveEnvelope prediction = observation();
        prediction.kind = ContributionKind::Prediction;
        prediction.evidence = {first.messageId, second.messageId};
        prediction.capabilityScope = QStringLiteral("system.observe");
        QVERIFY(journal.append(prediction) > 0);

        const auto stored = journal.contribution(prediction.messageId);
        QVERIFY(stored.has_value());
        QCOMPARE(stored->evidence, prediction.evidence);
        QCOMPARE(stored->capabilityScope, prediction.capabilityScope);
    }

    void privacyCannotBeWeakened()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));

        const CognitiveEnvelope local = observation(PrivacyClass::Local);
        QVERIFY(journal.append(local) > 0);

        CognitiveEnvelope publicConclusion = derived(ContributionKind::Learning, local);
        publicConclusion.privacy = PrivacyClass::Public;
        QCOMPARE(journal.append(publicConclusion), 0u);
    }

    void rewritingThePastBreaksTheChain()
    {
        QTemporaryDir dir;
        const QString path = dir.filePath(QStringLiteral("j.db"));
        {
            Journal journal(path);
            for (int i = 0; i < 5; ++i) {
                QVERIFY(journal.append(observation()) > 0);
            }
        }

        {
            QSqlDatabase db = QSqlDatabase::addDatabase(
                QStringLiteral("QSQLITE"), QStringLiteral("tamper"));
            db.setDatabaseName(path);
            QVERIFY(db.open());
            QSqlQuery query(db);
            QVERIFY(query.exec(QStringLiteral(
                "UPDATE contribution SET origin_organ = 'forged' WHERE seq = 3")));
            db.close();
        }
        QSqlDatabase::removeDatabase(QStringLiteral("tamper"));

        Journal journal(path);
        QCOMPARE(journal.verify(), 3u);
    }

    void episodeReplaysInOrder()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));

        const CognitiveEnvelope root = observation();
        QVERIFY(journal.append(root) > 0);
        const CognitiveEnvelope hypothesis = derived(ContributionKind::Hypothesis, root);
        QVERIFY(journal.append(hypothesis) > 0);
        const CognitiveEnvelope decision = derived(ContributionKind::Decision, hypothesis);
        QVERIFY(journal.append(decision) > 0);

        const auto episode = journal.episode(root.correlationId);
        QCOMPARE(episode.size(), 3);
        QCOMPARE(episode.at(0).messageId, root.messageId);
        QCOMPARE(episode.at(1).causationId, root.messageId);
        QCOMPARE(episode.at(2).causationId, hypothesis.messageId);
    }
};

QTEST_MAIN(TestJournal)
#include "tst_journal.moc"
