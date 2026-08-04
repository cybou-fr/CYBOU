// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT
//
// The promise: an intention outlives the process, and closing it is a fact in the biography
// rather than a deletion.

#include "cybou/intentions/Intentions.h"

#include <QTemporaryDir>
#include <QTest>

using namespace cybou;

class TestIntentions : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    void formedIntentionIsOpen()
    {
        QTemporaryDir dir;
        Journal j(dir.filePath(QStringLiteral("j.db")));
        Intentions intentions(&j);

        const QUuid id = intentions.form(QStringLiteral("verify sound after reboot"),
                                         QStringLiteral("next session"));
        QVERIFY(!id.isNull());

        const auto open = intentions.open();
        QCOMPARE(open.size(), 1);
        QCOMPARE(open.at(0).id, id);
        QCOMPARE(open.at(0).description, QStringLiteral("verify sound after reboot"));
        QCOMPARE(open.at(0).trigger, QStringLiteral("next session"));
    }

    void closingRemovesItFromTheOpenList()
    {
        QTemporaryDir dir;
        Journal j(dir.filePath(QStringLiteral("j.db")));
        Intentions intentions(&j);

        const QUuid id = intentions.form(QStringLiteral("check the network"));
        QVERIFY(intentions.close(id, Resolution::Fulfilled, QStringLiteral("link is up")));

        QVERIFY(intentions.open().isEmpty());

        // Closed, not deleted: both the intention and its outcome are still in the biography.
        QCOMPARE(j.count(), 2u);
        QCOMPARE(j.verify(), 0u);
    }

    void survivesRestart()
    {
        QTemporaryDir dir;
        const QString path = dir.filePath(QStringLiteral("j.db"));

        QUuid id;
        {
            Journal j(path);
            Intentions intentions(&j);
            id = intentions.form(QStringLiteral("verify sound and network after reboot"));
            QVERIFY(!id.isNull());
        }

        // A reboot happens here: new process, new objects, same journal.
        {
            Journal j(path);
            Intentions intentions(&j);
            const auto open = intentions.open();
            QCOMPARE(open.size(), 1);
            QCOMPARE(open.at(0).id, id);
        }
    }

    void oldestObligationComesFirst()
    {
        QTemporaryDir dir;
        Journal j(dir.filePath(QStringLiteral("j.db")));
        Intentions intentions(&j);

        intentions.form(QStringLiteral("first"));
        intentions.form(QStringLiteral("second"));
        intentions.form(QStringLiteral("third"));

        const auto open = intentions.open();
        QCOMPARE(open.size(), 3);
        QCOMPARE(open.at(0).description, QStringLiteral("first"));
        QCOMPARE(open.at(2).description, QStringLiteral("third"));
    }

    void theReasonIsReadableAfterClosing()
    {
        QTemporaryDir dir;
        Journal j(dir.filePath(QStringLiteral("j.db")));
        Intentions intentions(&j);

        const QUuid id = intentions.form(QStringLiteral("watch the build"));
        intentions.close(id, Resolution::Obsolete, QStringLiteral("the build was cancelled"));

        // The whole episode replays: why it existed, and how it ended.
        const auto episode = j.episode(id);
        QCOMPARE(episode.size(), 2);
        QCOMPARE(episode.at(0).kind, ContributionKind::Intention);
        QCOMPARE(episode.at(1).kind, ContributionKind::Outcome);
        QCOMPARE(episode.at(1).causationId, id);
    }
};

QTEST_MAIN(TestIntentions)
#include "tst_intentions.moc"
