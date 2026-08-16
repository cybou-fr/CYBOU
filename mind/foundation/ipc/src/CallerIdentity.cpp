// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/ipc/CallerIdentity.h"

#include <QDBusConnectionInterface>
#include <QDBusReply>
#include <QFile>
#include <QFileInfo>

namespace cybou {

QString trustedBinaryDirectory()
{
    static const QString directory = [] {
        const QString self = QFile::symLinkTarget(QStringLiteral("/proc/self/exe"));
        return self.isEmpty() ? QString() : QFileInfo(self).absolutePath();
    }();
    return directory;
}

QString mindBinaryNameForExecutable(const QString &executablePath)
{
    if (executablePath.isEmpty()) {
        return {};
    }

    const QString trusted = trustedBinaryDirectory();
    if (trusted.isEmpty() || QFileInfo(executablePath).absolutePath() != trusted) {
        return {};
    }

    // The Nix build wraps Qt applications, so the running executable is `.cybou-contextd-wrapped`
    // rather than `cybou-contextd`. Undoing that decoration is what makes this work against the
    // installed package rather than only a development build.
    QString name = QFileInfo(executablePath).fileName();
    if (name.startsWith(QLatin1Char('.'))) {
        name.remove(0, 1);
    }
    if (name.endsWith(QLatin1String("-wrapped"))) {
        name.chop(QStringLiteral("-wrapped").size());
    }
    if (!name.startsWith(QLatin1String("cybou-"))) {
        return {};
    }
    name.remove(0, QStringLiteral("cybou-").size());
    return name;
}

QString callerBinaryName(const QDBusConnection &connection, const QString &service)
{
    if (service.isEmpty()) {
        return {};
    }

    QDBusConnectionInterface *bus = connection.interface();
    if (!bus) {
        return {};
    }

    const QDBusReply<uint> pid = bus->servicePid(service);
    if (!pid.isValid()) {
        return {};
    }

    return mindBinaryNameForExecutable(
        QFile::symLinkTarget(QStringLiteral("/proc/%1/exe").arg(pid.value())));
}

} // namespace cybou
