// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/runtime/StatePaths.h"
#include "cybou/storage/Journal.h"
#include "cybou/workspace/Workspace.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QTemporaryDir>
#include <QTest>

using namespace cybou;

namespace {

CognitiveEnvelope observation(
    const QString &organ = QStringLiteral("testd"))
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

bool writeFile(
    const QString &path,
    const QByteArray &contents)
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
        Journal journal(dir.filePath(QStringLiteral("journal.db")));
        QVERIFY(journal.isOpen());

        int accepted = 0;
        connect(
            &journal,
            &EventStore::accepted,
            this,
            [&](const CognitiveEnvelope &, quint64) {
                ++accepted;
            });

        QCOMPARE(journal.append(observation()), 1u);
        QCOMPARE(accepted, 1);

        CognitiveEnvelope invalid;
        QCOMPARE(journal.append(invalid), 0u);
        QCOMPARE(accepted, 1);
    }

    void workspaceFollowsAcceptedContributionLive()
    {
        QTemporaryDir dir;
        Journal journal(dir.filePath(QStringLiteral("journal.db")));
        Workspace workspace(&journal, 8);

        const CognitiveEnvelope root =
            observation(QStringLiteral("sensord"));
        QCOMPARE(journal.append(root), 1u);

        QCOMPARE(workspace.moment().size(), 1);
        QCOMPARE(
            workspace.moment().first().messageId,
            root.messageId);

        workspace.accept(root);
        QCOMPARE(workspace.moment().size(), 1);
    }

    void persistentRootUsesXdgStateHome()
    {
#ifdef Q_OS_UNIX
        QTemporaryDir dir;
        const QByteArray previous =
            qgetenv("XDG_STATE_HOME");

        qputenv(
            "XDG_STATE_HOME",
            dir.path().toUtf8());

        QCOMPARE(
            StatePaths::persistentRoot(),
            QDir(dir.path()).filePath(QStringLiteral("cybou")));

        if (previous.isNull()) {
            qunsetenv("XDG_STATE_HOME");
        } else {
            qputenv("XDG_STATE_HOME", previous);
        }
#endif
    }

    void runtimeRootUsesXdgRuntimeDir()
    {
#ifdef Q_OS_UNIX
        QTemporaryDir dir;
        const QByteArray previous =
            qgetenv("XDG_RUNTIME_DIR");

        qputenv(
            "XDG_RUNTIME_DIR",
            dir.path().toUtf8());

        QCOMPARE(
            StatePaths::runtimeRoot(),
            QDir(dir.path()).filePath(QStringLiteral("cybou")));

        if (previous.isNull()) {
            qunsetenv("XDG_RUNTIME_DIR");
        } else {
            qputenv("XDG_RUNTIME_DIR", previous);
        }
#endif
    }

    void legacyMigrationPreservesExistingUnrelatedTargetEntry()
    {
        QTemporaryDir dir;
        const QString legacy =
            dir.filePath(QStringLiteral("legacy"));
        const QString target =
            dir.filePath(QStringLiteral("state"));

        QVERIFY(writeFile(
            QDir(legacy).filePath(QStringLiteral("journal.db")),
            QByteArrayLiteral("journal")));
        QVERIFY(writeFile(
            QDir(target).filePath(
                QStringLiteral("desktop-layout-version")),
            QByteArrayLiteral("2\n")));

        QString error;
        QVERIFY2(
            StatePaths::migrateLegacy(
                legacy,
                target,
                &error),
            qPrintable(error));

        QCOMPARE(
            readFile(
                QDir(target).filePath(QStringLiteral("journal.db"))),
            QByteArrayLiteral("journal"));
        QCOMPARE(
            readFile(
                QDir(target).filePath(
                    QStringLiteral("desktop-layout-version"))),
            QByteArrayLiteral("2\n"));
    }

    void legacyMigrationFailsClosedOnCollision()
    {
        QTemporaryDir dir;
        const QString legacy =
            dir.filePath(QStringLiteral("legacy"));
        const QString target =
            dir.filePath(QStringLiteral("state"));

        QVERIFY(writeFile(
            QDir(legacy).filePath(QStringLiteral("journal.db")),
            QByteArrayLiteral("old")));
        QVERIFY(writeFile(
            QDir(target).filePath(QStringLiteral("journal.db")),
            QByteArrayLiteral("new")));

        QString error;
        QVERIFY(!StatePaths::migrateLegacy(
            legacy,
            target,
            &error));
        QVERIFY(error.contains(QStringLiteral("collision")));

        QCOMPARE(
            readFile(
                QDir(legacy).filePath(QStringLiteral("journal.db"))),
            QByteArrayLiteral("old"));
        QCOMPARE(
            readFile(
                QDir(target).filePath(QStringLiteral("journal.db"))),
            QByteArrayLiteral("new"));
    }
};

QTEST_MAIN(TestM1Runtime)
#include "tst_m1_runtime.moc"
