// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT
#pragma once

#include <QByteArray>
#include <QDateTime>
#include <QString>
#include <QStringList>
#include <QUuid>

namespace cybou {

inline constexpr quint16 kLifecycleSchemaVersion = 1;

enum class LifecycleMode : quint8 { Awake = 1, Idle, Consolidating, Maintenance, Recovering, Degraded, Suspended };
enum class LifecycleRunStatus : quint8 { Requested = 1, Active, Completed, Interrupted, Failed };

QString lifecycleModeToString(LifecycleMode mode);
QString lifecycleRunStatusToString(LifecycleRunStatus status);
bool canTransition(LifecycleMode from, LifecycleMode to) noexcept;

struct LifecycleRun {
    quint16 schemaVersion{kLifecycleSchemaVersion};
    QUuid runId;
    QString kind;
    QString policyId;
    QDateTime requestedAt;
    quint64 inputHighWaterMark{0};
    QStringList requiredCapabilities;
    QStringList optionalCapabilities;
    LifecycleRunStatus status{LifecycleRunStatus::Requested};
    QStringList completedWork;
    QStringList missingWork;
    QString terminalCause;

    bool isValid() const;
    bool isTerminal() const noexcept;
};

QByteArray encodeLifecycleRun(const LifecycleRun &run);
LifecycleRun decodeLifecycleRun(const QByteArray &encoded, QString *error = nullptr);

} // namespace cybou

