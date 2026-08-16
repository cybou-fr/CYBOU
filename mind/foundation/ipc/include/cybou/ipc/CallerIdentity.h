// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include <QDBusConnection>
#include <QString>

namespace cybou {

/// The Mind binary behind a D-Bus peer, or empty when it is not one.
///
/// A caller's claims about itself are not evidence. This resolves the peer to a process, the
/// process to its executable, and accepts the answer only when that executable sits in the same
/// directory as the asking service's own -- a directory a user cannot write to without already
/// being able to replace Mind outright. Matching on a name alone would let anyone build an ELF,
/// call it `cybou-contextd`, and inherit whatever that name is entitled to.
///
/// Returns the undecorated organ name (`predictord`, not `.cybou-predictord-wrapped`), so callers
/// can map it to whatever authority they grant. Empty means "unknown", which every caller must
/// treat as the least privilege it offers rather than as an error to work around.
///
/// This is the same binding Event1 uses to keep an organ from speaking as another. It lives here so
/// that the second question asked of a caller reuses the first answer's mechanism instead of
/// growing a slightly different one.
QString callerBinaryName(const QDBusConnection &connection, const QString &service);

/// The directory Mind's binaries were installed into, derived from the running executable.
///
/// Derived rather than configured on purpose: a trusted path read from the environment would be
/// settable by any process able to restart the service, which is the same user these checks exist
/// to constrain. Empty when it cannot be resolved, which makes every check fail closed.
QString trustedBinaryDirectory();

/// The undecorated Mind binary name for an executable path, or empty when it is not one.
///
/// Separated from the bus lookup so it can be tested without a bus, and so the rule about which
/// paths are trusted lives in exactly one place.
QString mindBinaryNameForExecutable(const QString &executablePath);

} // namespace cybou
