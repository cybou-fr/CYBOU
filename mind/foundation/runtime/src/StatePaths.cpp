// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/runtime/StatePaths.h"

#include <QDir>
#include <QFileInfo>
#include <QStandardPaths>
#include <QStringList>
#include <QtGlobal>

namespace cybou {

namespace {

QString cleanAbsolute(const QString &path)
{
    return QDir::cleanPath(QFileInfo(path).absoluteFilePath());
}

void setError(QString *error, const QString &message)
{
    if (error) {
        *error = message;
    }
}

} // namespace

QString StatePaths::persistentRoot()
{
#ifdef Q_OS_UNIX
    QString stateHome = qEnvironmentVariable("XDG_STATE_HOME");
    if (stateHome.isEmpty() || !QDir::isAbsolutePath(stateHome)) {
        const QString home =
            QStandardPaths::writableLocation(QStandardPaths::HomeLocation);
        stateHome = QDir(home).filePath(QStringLiteral(".local/state"));
    }
    return cleanAbsolute(QDir(stateHome).filePath(QStringLiteral("cybou")));
#else
    const QString appData =
        QStandardPaths::writableLocation(QStandardPaths::AppLocalDataLocation);
    return cleanAbsolute(QDir(appData).filePath(QStringLiteral("cybou")));
#endif
}

QString StatePaths::runtimeRoot()
{
#ifdef Q_OS_UNIX
    const QString runtime = qEnvironmentVariable("XDG_RUNTIME_DIR");
    if (runtime.isEmpty() || !QDir::isAbsolutePath(runtime)) {
        return {};
    }
    return cleanAbsolute(QDir(runtime).filePath(QStringLiteral("cybou")));
#else
    const QString runtime =
        QStandardPaths::writableLocation(QStandardPaths::RuntimeLocation);
    return runtime.isEmpty()
        ? QString()
        : cleanAbsolute(QDir(runtime).filePath(QStringLiteral("cybou")));
#endif
}

QString StatePaths::legacyPresenceRoot()
{
    const QString appData =
        QStandardPaths::writableLocation(QStandardPaths::AppDataLocation);
    return cleanAbsolute(QDir(appData).filePath(QStringLiteral("cybou")));
}

bool StatePaths::migrateLegacy(
    const QString &legacyRoot,
    const QString &persistentRoot,
    QString *error)
{
    if (error) {
        error->clear();
    }

    const QString source = cleanAbsolute(legacyRoot);
    const QString target = cleanAbsolute(persistentRoot);

    if (source == target) {
        return true;
    }

    QDir sourceDir(source);
    if (!sourceDir.exists()) {
        return true;
    }

    const QFileInfoList entries = sourceDir.entryInfoList(
        QDir::AllEntries | QDir::NoDotAndDotDot | QDir::Hidden | QDir::System,
        QDir::Name);

    if (entries.isEmpty()) {
        QDir().rmdir(source);
        return true;
    }

    if (!QDir().mkpath(target)) {
        setError(
            error,
            QStringLiteral("cannot create canonical state root %1").arg(target));
        return false;
    }

    for (const QFileInfo &entry : entries) {
        const QString destination =
            QDir(target).filePath(entry.fileName());
        if (QFileInfo::exists(destination)) {
            setError(
                error,
                QStringLiteral("state migration collision for %1")
                    .arg(entry.fileName()));
            return false;
        }
    }

    QStringList moved;
    for (const QFileInfo &entry : entries) {
        const QString sourcePath = entry.absoluteFilePath();
        const QString destination =
            QDir(target).filePath(entry.fileName());

        if (!QDir().rename(sourcePath, destination)) {
            bool rollbackOk = true;
            for (auto it = moved.crbegin(); it != moved.crend(); ++it) {
                const QString movedPath = QDir(target).filePath(*it);
                const QString originalPath = QDir(source).filePath(*it);
                if (!QDir().rename(movedPath, originalPath)) {
                    rollbackOk = false;
                }
            }

            setError(
                error,
                rollbackOk
                    ? QStringLiteral(
                          "cannot migrate state entry %1; migration rolled back")
                          .arg(entry.fileName())
                    : QStringLiteral(
                          "cannot migrate state entry %1 and rollback was incomplete")
                          .arg(entry.fileName()));
            return false;
        }

        moved.append(entry.fileName());
    }

    QDir().rmdir(source);
    return true;
}

bool StatePaths::migrateLegacy(QString *error)
{
    return migrateLegacy(
        legacyPresenceRoot(),
        persistentRoot(),
        error);
}

} // namespace cybou
