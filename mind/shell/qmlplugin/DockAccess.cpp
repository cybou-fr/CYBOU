// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "DockAccess.h"

#include <QDBusConnection>
#include <QDBusMessage>
#include <QDBusPendingCallWatcher>

namespace {

QString panelScript(const QString &mode)
{
    return QStringLiteral(R"JS(
(function() {
    const ps = panels();
    for (let i = 0; i < ps.length; ++i) {
        const panel = ps[i];
        if (panel.widgets("org.cybou.presence").length > 0) {
            panel.hiding = "%1";
            print("cybou-minddock:%1");
            return;
        }
    }
    print("cybou-minddock:missing");
})();
)JS")
        .arg(mode);
}

} // namespace

DockAccess::DockAccess(QObject *parent)
    : QObject(parent)
{
    m_restoreTimer.setSingleShot(true);
    m_restoreTimer.setInterval(1800);

    connect(
        &m_restoreTimer,
        &QTimer::timeout,
        this,
        [this]() {
            if (!m_pinned) {
                applyHiding(
                    QStringLiteral("autohide"),
                    QStringLiteral("restore"));
            }
        });
}

void DockAccess::peek()
{
    if (m_pinned) {
        return;
    }

    m_restoreTimer.stop();
    applyHiding(
        QStringLiteral("none"),
        QStringLiteral("peek"));
    m_restoreTimer.start();
}

void DockAccess::togglePinned()
{
    m_restoreTimer.stop();

    if (m_pinned) {
        setPinned(false);
        applyHiding(
            QStringLiteral("autohide"),
            QStringLiteral("unpin"));
        return;
    }

    setPinned(true);
    applyHiding(
        QStringLiteral("none"),
        QStringLiteral("pin"));
}

void DockAccess::release()
{
    m_restoreTimer.stop();
    setPinned(false);
    applyHiding(
        QStringLiteral("autohide"),
        QStringLiteral("release"));
}

void DockAccess::applyHiding(
    const QString &mode,
    const QString &context)
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    if (!bus.isConnected()) {
        setLastError(
            QStringLiteral("Plasma session D-Bus is unavailable"));
        return;
    }

    QDBusMessage message = QDBusMessage::createMethodCall(
        QStringLiteral("org.kde.plasmashell"),
        QStringLiteral("/PlasmaShell"),
        QStringLiteral("org.kde.PlasmaShell"),
        QStringLiteral("evaluateScript"));
    message << panelScript(mode);

    auto *watcher = new QDBusPendingCallWatcher(
        bus.asyncCall(message),
        this);

    connect(
        watcher,
        &QDBusPendingCallWatcher::finished,
        this,
        [this, watcher, context]() {
            const QDBusMessage reply = watcher->reply();

            QString failure;
            if (reply.type() == QDBusMessage::ErrorMessage) {
                failure = QStringLiteral("%1: %2")
                              .arg(
                                  context,
                                  reply.errorMessage());
            } else if (
                !reply.arguments().isEmpty()
                && reply.arguments().first().toString().contains(
                    QStringLiteral("cybou-minddock:missing"))) {
                failure =
                    QStringLiteral("%1: Mind Dock panel was not found")
                        .arg(context);
            }

            if (!failure.isEmpty()) {
                if (context == QLatin1String("pin")) {
                    setPinned(false);
                } else if (
                    context == QLatin1String("unpin")
                    || context == QLatin1String("release")) {
                    setPinned(true);
                }
                setLastError(failure);
            } else {
                setLastError(QString());
            }

            watcher->deleteLater();
        });
}

void DockAccess::setPinned(bool pinned)
{
    if (m_pinned == pinned) {
        return;
    }

    m_pinned = pinned;
    Q_EMIT pinnedChanged();
}

void DockAccess::setLastError(const QString &error)
{
    if (m_lastError == error) {
        return;
    }

    m_lastError = error;
    Q_EMIT lastErrorChanged();
}
