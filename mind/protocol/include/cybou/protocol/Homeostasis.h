// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#pragma once

#include <QByteArray>
#include <QDateTime>
#include <QList>
#include <QString>
#include <QStringList>
#include <QUuid>

namespace cybou {

inline constexpr quint16 kHomeostasisSchemaVersion = 2;

enum class MeasurementKind : quint8 {
    Gauge = 1,
    Counter,
    Duration,
    Bytes,
};

enum class MeasurementStatus : quint8 {
    Current = 1,
    Stale,
    Unknown,
    Unsupported,
};

QString measurementKindToString(MeasurementKind kind);
QString measurementStatusToString(MeasurementStatus status);

struct HomeostaticMeasurement {
    QString metricId;
    QString sourceId;
    MeasurementKind kind{MeasurementKind::Gauge};
    MeasurementStatus status{MeasurementStatus::Unknown};
    double value{0.0};
    bool hasValue{false};
    QString unit;
    QDateTime observedAt;
    QDateTime validUntil;
    QString reason;

    bool isValid() const;
};

struct HomeostasisSnapshot {
    quint16 schemaVersion{kHomeostasisSchemaVersion};
    QUuid snapshotId;
    QDateTime observedAt;
    QStringList authorizedPolicyIds;
    QList<HomeostaticMeasurement> measurements;

    bool isValid() const;
    bool authorizes(const QString &policyId) const;
};

QByteArray encodeHomeostasisSnapshot(const HomeostasisSnapshot &snapshot);
HomeostasisSnapshot decodeHomeostasisSnapshot(
    const QByteArray &encoded,
    QString *error = nullptr);

} // namespace cybou
