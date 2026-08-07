// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/presence/Presence.h"
#include "cybou/runtime/StatePaths.h"
#include "cybou/storage/Journal.h"
#include "cybou/workspace/Workspace.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QSignalSpy>
#include <QTemporaryDir>
#include <QTest>

using namespace cybou;

namespace {

CognitiveEnvelope observation(const QString &organ = QStringLiteral("testd"))
{
    CognitiveEnvelope e;
    e.messageId = QUuid::createUuid();
    e.correlationId = e.messageId;
    e.originOrgan = organ;
    e.kind = ContributionKind::Observation;
    e.wallTime = QDateTime::currentDateTimeUtc();
    e.confidence = 1.0;
    e.privacy = PrivacyClass::Node;
    return e;
}

bool writeFile(const QString &path, const QByteArray &contents)
{
    QDir().mkpath(QFileInfo(path).absolutePath());
    QFile file(path);
    if (!file.open(QIODevice::WriteOnly)) {
        return false;
    }
    return file.write(contents) == contents.size();
}

QByteArray readFile(const QString &path)
{
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly)) {
        return {};
    }
    return file.readAll();
}

} // namespace

class TestM1Runtime : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    void journalPublishesOnlyCommittedContributions()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());

        Journal journal(dir.filePath(QStringLiteral("journal.db")));
        QVERIFY2(journal.isOpen(), qPrintable(journal.lastError()));

        int accepted = 0;
        quint64 acceptedSequence = 0;
        QUuid acceptedId;
        connect(
            &journal,
            &Journal::accepted,
            this,
            [&](const CognitiveEnvelope &envelope, quint64 sequence) {
                ++accepted;
                acceptedSequence = sequence;
                acceptedId = envelope.messageId;
            });

        const CognitiveEnvelope root = observation();
        QCOMPARE(journal.append(root), 1u);
        QCOMPARE(accepted, 1);
        QCOMPARE(acceptedSequence, 1u);
        QCOMPARE(acceptedId, root.messageId);

        CognitiveEnvelope invalid;
        QCOMPARE(journal.append(invalid), 0u);
        QCOMPARE(accepted, 1);
    }

    void workspaceFollowsDirectJournalAppendLive()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());

        Journal journal(dir.filePath(QStringLiteral("journal.db")));
        Workspace workspace(&journal, 8);

        int admitted = 0;
        connect(
            &workspace,
            &Workspace::contributed,
            this,
            [&](const CognitiveEnvelope &) { ++admitted; });

        const CognitiveEnvelope root = observation(QStringLiteral("sensord"));
        QCOMPARE(journal.append(root), 1u);

        QCOMPARE(admitted, 1);
        QCOMPARE(workspace.moment().size(), 1);
        QCOMPARE(workspace.moment().first().messageId, root.messageId);

        CognitiveEnvelope invalid;
        QCOMPARE(journal.append(invalid), 0u);
        QCOMPARE(admitted, 1);
        QCOMPARE(workspace.moment().size(), 1);
    }

    void workspaceAcceptIsIdempotent()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("journal.db")));
        Workspace workspace(&journal, 8);

        const CognitiveEnvelope root = observation();
        QCOMPARE(journal.append(root), 1u);
        QCOMPARE(workspace.moment().size(), 1);

        workspace.accept(root);
        QCOMPARE(workspace.moment().size(), 1);
    }

    void twoPresenceSurfacesShareOneRuntime()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());

        const QString state = dir.filePath(QStringLiteral("state"));
        Presence first(state);
        Presence second(state);

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

        QSignalSpy secondChanged(&second, &Presence::changed);
        const int before = second.contributions();

        const QUuid intention =
            first.promise(QStringLiteral("shared runtime commitment"));
        QVERIFY(!intention.isNull());

        QCOMPARE(second.contributions(), before + 2);
        QCOMPARE(
            second.obligations(),
            QStringList{QStringLiteral("shared runtime commitment")});
        // Observation + Intention were independently durably accepted.
        QCOMPARE(secondChanged.count(), 2);
    }

    void aNewRuntimeStartsOnlyAfterAllSurfacesLeave()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());

        const QString state = dir.filePath(QStringLiteral("state"));
        qint64 firstSession = 0;

        {
            Presence first(state);
            Presence second(state);
            QVERIFY(first.wake());
            QVERIFY(second.wake());
            firstSession =
                first.identityState()
                    .value(QStringLiteral("sessionCount"))
                    .toLongLong();
            QCOMPARE(
                second.identityState()
                    .value(QStringLiteral("sessionCount"))
                    .toLongLong(),
                firstSession);
        }

        Presence nextSession(state);
        QVERIFY2(nextSession.wake(), qPrintable(nextSession.lastError()));
        QCOMPARE(
            nextSession.identityState()
                .value(QStringLiteral("sessionCount"))
                .toLongLong(),
            firstSession + 1);
    }

    void persistentRootUsesXdgStateHome()
    {
#ifdef Q_OS_UNIX
        QTemporaryDir dir;
        QVERIFY(dir.isValid());

        const QByteArray previous = qgetenv("XDG_STATE_HOME");
        qputenv("XDG_STATE_HOME", dir.path().toUtf8());

        QCOMPARE(
            StatePaths::persistentRoot(),
            QDir(dir.path()).filePath(QStringLiteral("cybou")));

        if (previous.isNull()) {
            qunsetenv("XDG_STATE_HOME");
        } else {
            qputenv("XDG_STATE_HOME", previous);
        }
#else
        QVERIFY(!StatePaths::persistentRoot().isEmpty());
#endif
    }

    void legacyMigrationMergesWithoutDeletingDesktopMarker()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());

        const QString legacy = dir.filePath(QStringLiteral("legacy"));
        const QString target = dir.filePath(QStringLiteral("state"));

        QVERIFY(writeFile(
            QDir(legacy).filePath(QStringLiteral("journal.db")),
            QByteArrayLiteral("journal")));
        QVERIFY(writeFile(
            QDir(legacy).filePath(QStringLiteral("identity.json")),
            QByteArrayLiteral("identity")));
        QVERIFY(writeFile(
            QDir(target).filePath(QStringLiteral("desktop-layout-version")),
            QByteArrayLiteral("2\n")));

        QString error;
        QVERIFY2(
            StatePaths::migrateLegacy(legacy, target, &error),
            qPrintable(error));

        QCOMPARE(
            readFile(QDir(target).filePath(QStringLiteral("journal.db"))),
            QByteArrayLiteral("journal"));
        QCOMPARE(
            readFile(QDir(target).filePath(QStringLiteral("identity.json"))),
            QByteArrayLiteral("identity"));
        QCOMPARE(
            readFile(
                QDir(target).filePath(QStringLiteral("desktop-layout-version"))),
            QByteArrayLiteral("2\n"));

        QVERIFY(!QFileInfo::exists(
            QDir(legacy).filePath(QStringLiteral("journal.db"))));
    }

    void legacyMigrationFailsClosedOnCollision()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());

        const QString legacy = dir.filePath(QStringLiteral("legacy"));
        const QString target = dir.filePath(QStringLiteral("state"));

        QVERIFY(writeFile(
            QDir(legacy).filePath(QStringLiteral("journal.db")),
            QByteArrayLiteral("old")));
        QVERIFY(writeFile(
            QDir(target).filePath(QStringLiteral("journal.db")),
            QByteArrayLiteral("new")));

        QString error;
        QVERIFY(!StatePaths::migrateLegacy(legacy, target, &error));
        QVERIFY(error.contains(QStringLiteral("collision")));

        QCOMPARE(
            readFile(QDir(legacy).filePath(QStringLiteral("journal.db"))),
            QByteArrayLiteral("old"));
        QCOMPARE(
            readFile(QDir(target).filePath(QStringLiteral("journal.db"))),
            QByteArrayLiteral("new"));
    }
};

QTEST_MAIN(TestM1Runtime)
#include "tst_m1_runtime.moc"
