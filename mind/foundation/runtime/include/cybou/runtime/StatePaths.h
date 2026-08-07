// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include <QString>

namespace cybou {

/// Stable state locations shared by the current in-process runtime and future daemons.
///
/// The persistent root deliberately does not depend on QApplication/QCoreApplication identity:
/// moving Presence out of plasmashell must not move or fork the biography.
class StatePaths
{
public:
    static QString persistentRoot();

    /// The path used by Presence before M1. It depended on AppDataLocation and therefore on the
    /// hosting process identity (normally plasmashell).
    static QString legacyPresenceRoot();

    /// Migrate every entry from a legacy Cybou state directory into the canonical persistent
    /// root without overwriting anything already present there.
    ///
    /// The operation preflights all destination names first and rolls back entries already moved
    /// if a later rename fails. An existing unrelated entry such as desktop-layout-version in the
    /// target is preserved.
    static bool migrateLegacy(
        const QString &legacyRoot,
        const QString &persistentRoot,
        QString *error = nullptr);

    /// Convenience overload for the actual current/legacy paths.
    static bool migrateLegacy(QString *error = nullptr);
};

} // namespace cybou
