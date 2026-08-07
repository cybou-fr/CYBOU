// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: MIT
//
// The claim under test is continuity: the same subject across restarts, and a refusal to
// quietly become a different one.

#include "cybou/identity/Identity.h"
#include "cybou/storage/Journal.h"

#include <QFile>
#include <QTemporaryDir>
#include <QTest>

using namespace cybou;

class TestIdentity : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    void bornOnceThenContinues()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        const QString statePath = dir.filePath(QStringLiteral("identity.json"));

        QUuid firstId;
        {
            Identity id(statePath, &journal);
            QVERIFY2(id.beginSession(), qPrintable(id.lastError()));
            QVERIFY(id.wasBorn());
            QCOMPARE(id.state().sessionCount, 1u);
            firstId = id.state().identityId;
            QVERIFY(!firstId.isNull());
        }

        // A reboot: new process, same state on disk.
        {
            Identity id(statePath, &journal);
            QVERIFY(id.beginSession());
            QVERIFY2(!id.wasBorn(), "a second run must continue, not create a new identity");
            QCOMPARE(id.state().identityId, firstId);
            QCOMPARE(id.state().sessionCount, 2u);
        }

        // Both events are in the biography, and the chain is intact.
        QCOMPARE(journal.count(), 2u);
        QCOMPARE(journal.verify(), 0u);
    }

    void originSurvivesRestarts()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        const QString statePath = dir.filePath(QStringLiteral("identity.json"));

        QDateTime origin;
        {
            Identity id(statePath, &journal);
            QVERIFY(id.beginSession());
            origin = id.state().origin;
        }
        {
            Identity id(statePath, &journal);
            QVERIFY(id.beginSession());
            QCOMPARE(id.state().origin, origin);
        }
    }

    void refusesToReplaceCorruptState()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        const QString statePath = dir.filePath(QStringLiteral("identity.json"));

        {
            Identity id(statePath, &journal);
            QVERIFY(id.beginSession());
        }

        QFile f(statePath);
        QVERIFY(f.open(QIODevice::WriteOnly | QIODevice::Truncate));
        f.write("{ this is not identity }");
        f.close();

        // Starting a fresh life here would erase the one thing this organ exists to keep.
        Identity id(statePath, &journal);
        QVERIFY2(!id.beginSession(), "corrupt state must be refused, not overwritten");
        QVERIFY(!id.lastError().isEmpty());
    }

    void identityEpisodeIsReadable()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("j.db")));
        const QString statePath = dir.filePath(QStringLiteral("identity.json"));

        Identity id(statePath, &journal);
        QVERIFY(id.beginSession());

        const auto life = journal.episode(id.state().identityId);
        QCOMPARE(life.size(), 1);
        QCOMPARE(life.at(0).originOrgan, QStringLiteral("identityd"));
    }
};

QTEST_MAIN(TestIdentity)
#include "tst_identity.moc"
