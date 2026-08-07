// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include <QObject>
#include <QString>
#include <QTimer>
#include <QtQml/qqmlregistration.h>

class DockAccess final : public QObject
{
    Q_OBJECT
    QML_NAMED_ELEMENT(DockAccess)

    Q_PROPERTY(bool pinned READ pinned NOTIFY pinnedChanged)
    Q_PROPERTY(QString lastError READ lastError NOTIFY lastErrorChanged)

public:
    explicit DockAccess(QObject *parent = nullptr);

    bool pinned() const { return m_pinned; }
    QString lastError() const { return m_lastError; }

    /// Briefly reveal the native auto-hide Mind panel. Used by hover/click discovery.
    Q_INVOKABLE void peek();

    /// Toggle a deliberate pinned-open state. Used by Meta+M and explicit click.
    Q_INVOKABLE void togglePinned();

    /// Return the panel to its normal auto-hide contract.
    Q_INVOKABLE void release();

Q_SIGNALS:
    void pinnedChanged();
    void lastErrorChanged();

private:
    void applyHiding(const QString &mode, const QString &context);
    void setPinned(bool pinned);
    void setLastError(const QString &error);

    QTimer m_restoreTimer;
    bool m_pinned{false};
    QString m_lastError;
};
