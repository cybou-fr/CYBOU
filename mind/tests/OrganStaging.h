// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include <QFile>
#include <QFileInfo>
#include <QString>
#include <QTemporaryDir>

namespace cybou::testing {

/// Copies Mind binaries into one directory, the way an installed package lays them out.
///
/// eventd grants an organ identity only to an executable sitting beside itself: the installed
/// package puts them all in one place, so a binary somewhere else is not the organ it is named
/// after. The build tree scatters them across per-target directories, so a process test that runs
/// them from there has its organs refused for exactly the same reason an impostor is - and would be
/// testing a layout production never has.
///
/// Copied rather than symlinked, because `/proc/<pid>/exe` resolves through symlinks and a link
/// would still report the original scattered path.
class StagedInstall
{
public:
    bool isValid() const { return m_root.isValid(); }

    /// Stage `path` and return its location in the staged directory, or an empty string on failure.
    QString stage(const QString &path)
    {
        if (path.isEmpty() || !m_root.isValid()) {
            return {};
        }

        const QString staged = m_root.filePath(QFileInfo(path).fileName());
        if (QFile::exists(staged)) {
            return staged;
        }
        if (!QFile::copy(path, staged)) {
            return {};
        }
        if (!QFile::setPermissions(
                staged,
                QFile::ReadOwner | QFile::WriteOwner | QFile::ExeOwner | QFile::ReadGroup
                    | QFile::ExeGroup)) {
            return {};
        }
        return staged;
    }

    /// Stage the binary named by an environment variable.
    QString stageFromEnvironment(const char *variable)
    {
        return stage(qEnvironmentVariable(variable));
    }

private:
    QTemporaryDir m_root;
};

} // namespace cybou::testing
