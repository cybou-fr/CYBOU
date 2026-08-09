// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include "cybou/protocol/Health.h"
#include "cybou/protocol/Homeostasis.h"

#include <QObject>

namespace cybou {

class HealthService : public QObject
{
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.cybou.Mind.Health1")

public:
    explicit HealthService(const QString &path, QObject *parent = nullptr);

    bool isReady() const { return m_ready; }
    QString startupError() const { return m_error; }

public Q_SLOTS:
    bool Ready() const { return m_ready; }
    QString Health() const;
    QString LastError() const { return m_error; }
    bool HasSnapshot() const { return m_hasSnapshot; }
    QByteArray Snapshot() const;
    bool HasMeasurements() const { return m_hasHomeostasis; }
    QByteArray Measurements() const;
    bool Refresh();

Q_SIGNALS:
    void Changed();

private:
    bool load();
    bool save(const CapabilitySnapshot &snapshot);

    QString m_path;
    CapabilitySnapshot m_snapshot;
    HomeostasisSnapshot m_homeostasis;
    bool m_hasSnapshot{false};
    bool m_hasHomeostasis{false};
    bool m_refreshing{false};
    bool m_ready{false};
    QString m_error;
};

} // namespace cybou
