// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include <QString>

namespace cybou {

class StatePaths
{
public:
    static QString persistentRoot();
    static QString runtimeRoot();
    static QString legacyPresenceRoot();

    static bool migrateLegacy(
        const QString &legacyRoot,
        const QString &persistentRoot,
        QString *error = nullptr);

    static bool migrateLegacy(QString *error = nullptr);
};

} // namespace cybou
