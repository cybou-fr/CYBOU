// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT
//
// What is worth testing here is the promise the journal makes: the past cannot be rewritten
// without it being detectable. Everything else is SQLite's job.

#include "cybou/storage/Journal.h"

#include <QSqlDatabase>
#include <QSqlQuery>
#include <QTemporaryDir>
#include <QTest>

using namespace cybou;

namespace {
CognitiveEnvelope observation(const QString &organ = QStringLiteral("perceptiond"))
{
    CognitiveEnvelope e;
    e.messageId = QUuid::createUuid();
    e.correlationId = QUuid::createUuid();
    e.originOrgan = organ;
    e.wallTime = QDateTime::currentDateTimeUtc();
    e.kind = ContributionKind::Observation;
    return e;
}
}

class TestJournal : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    void appendsAndCounts()
    {
        QTemporaryDir dir;
        Journal j(dir.filePath(QStringLiteral("j.db")));
        QVERIFY2(j.isOpen(), qPrintable(j.lastError()));

        QCOMPARE(j.append(observation()), 1u);
        QCOMPARE(j.append(observation()), 2u);
        QCOMPARE(j.count(), 2u);
        QCOMPARE(j.verify(), 0u);
    }

    void refusesInvalidEnvelope()
    {
        QTemporaryDir dir;
        Journal j(dir.filePath(QStringLiteral("j.db")));

        CognitiveEnvelope bad;                       // no id, no organ, no time
        QCOMPARE(j.append(bad), 0u);
        QCOMPARE(j.count(), 0u);

        CognitiveEnvelope orphan = observation();
        orphan.kind = ContributionKind::Decision;    // a decision with no cause and no evidence
        QCOMPARE(j.append(orphan), 0u);
    }

    void rewritingThePastBreaksTheChain()
    {
        QTemporaryDir dir;
        const QString path = dir.filePath(QStringLiteral("j.db"));
        {
            Journal j(path);
            for (int i = 0; i < 5; ++i) {
                QVERIFY(j.append(observation()) > 0);
            }
            QCOMPARE(j.verify(), 0u);
        }

        // Tamper behind the journal's back, exactly as an editor or a script would.
        {
            QSqlDatabase db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"),
                                                        QStringLiteral("tamper"));
            db.setDatabaseName(path);
            QVERIFY(db.open());
            QSqlQuery q(db);
            QVERIFY(q.exec(QStringLiteral(
                "UPDATE contribution SET origin_organ = 'forged' WHERE seq = 3")));
            db.close();
        }
        QSqlDatabase::removeDatabase(QStringLiteral("tamper"));

        Journal j(path);
        QCOMPARE(j.verify(), 3u); // names the first row that no longer matches
    }

    void episodeReplaysInOrder()
    {
        QTemporaryDir dir;
        Journal j(dir.filePath(QStringLiteral("j.db")));

        const QUuid episodeId = QUuid::createUuid();
        QUuid previous;
        for (int i = 0; i < 3; ++i) {
            CognitiveEnvelope e = observation();
            e.correlationId = episodeId;
            e.causationId = previous;
            e.kind = i == 0 ? ContributionKind::Observation : ContributionKind::Prediction;
            previous = e.messageId;
            QVERIFY(j.append(e) > 0);
        }
        j.append(observation()); // noise in a different episode

        const auto chain = j.episode(episodeId);
        QCOMPARE(chain.size(), 3);
        QCOMPARE(chain.at(0).kind, ContributionKind::Observation);
        QCOMPARE(chain.at(1).causationId, chain.at(0).messageId);
        QCOMPARE(chain.at(2).causationId, chain.at(1).messageId);
    }

    void survivesReopening()
    {
        QTemporaryDir dir;
        const QString path = dir.filePath(QStringLiteral("j.db"));
        {
            Journal j(path);
            j.append(observation());
        }
        Journal j(path);
        QCOMPARE(j.count(), 1u);
        QCOMPARE(j.append(observation()), 2u);
        QCOMPARE(j.verify(), 0u);
    }
};

QTEST_MAIN(TestJournal)
#include "tst_journal.moc"
