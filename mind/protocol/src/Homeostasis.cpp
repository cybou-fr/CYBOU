// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

#include "cybou/protocol/Homeostasis.h"

#include <QCborArray>
#include <QCborMap>
#include <QCborValue>
#include <QSet>

#include <cmath>

namespace cybou {
namespace {

void setError(QString *error, const QString &message)
{
    if (error) {
        *error = message;
    }
}

bool validKind(MeasurementKind kind)
{
    return kind >= MeasurementKind::Gauge && kind <= MeasurementKind::Bytes;
}

bool validStatus(MeasurementStatus status)
{
    return status >= MeasurementStatus::Current && status <= MeasurementStatus::Unsupported;
}

QString timestamp(const QDateTime &value)
{
    return value.isValid() ? value.toUTC().toString(Qt::ISODateWithMs) : QString();
}

QDateTime parseTimestamp(const QCborValue &value)
{
    return QDateTime::fromString(value.toString(), Qt::ISODateWithMs);
}

bool integerInRange(const QCborValue &value, qint64 minimum, qint64 maximum)
{
    return value.isInteger() && value.toInteger() >= minimum && value.toInteger() <= maximum;
}

} // namespace

QString measurementKindToString(MeasurementKind kind)
{
    switch (kind) {
    case MeasurementKind::Gauge: return QStringLiteral("gauge");
    case MeasurementKind::Counter: return QStringLiteral("counter");
    case MeasurementKind::Duration: return QStringLiteral("duration");
    case MeasurementKind::Bytes: return QStringLiteral("bytes");
    }
    return QStringLiteral("unknown");
}

QString measurementStatusToString(MeasurementStatus status)
{
    switch (status) {
    case MeasurementStatus::Current: return QStringLiteral("current");
    case MeasurementStatus::Stale: return QStringLiteral("stale");
    case MeasurementStatus::Unknown: return QStringLiteral("unknown");
    case MeasurementStatus::Unsupported: return QStringLiteral("unsupported");
    }
    return QStringLiteral("unknown");
}

bool HomeostaticMeasurement::isValid() const
{
    if (metricId.trimmed().isEmpty() || sourceId.trimmed().isEmpty()
        || !validKind(kind) || !validStatus(status) || !observedAt.isValid()) {
        return false;
    }
    if (hasValue && (!std::isfinite(value) || unit.trimmed().isEmpty())) {
        return false;
    }
    if (status == MeasurementStatus::Current) {
        return hasValue && validUntil.isValid() && validUntil >= observedAt
            && reason.trimmed().isEmpty();
    }
    if (status == MeasurementStatus::Stale) {
        return hasValue && validUntil.isValid() && validUntil >= observedAt;
    }
    return !hasValue && !validUntil.isValid() && !reason.trimmed().isEmpty();
}

bool HomeostasisSnapshot::isValid() const
{
    if (schemaVersion != kHomeostasisSchemaVersion || snapshotId.isNull()
        || !observedAt.isValid() || schedulingAuthorized) {
        return false;
    }
    QSet<QString> ids;
    for (const HomeostaticMeasurement &measurement : measurements) {
        const QString id = measurement.metricId.trimmed();
        if (!measurement.isValid() || measurement.observedAt > observedAt || ids.contains(id)) {
            return false;
        }
        if (measurement.status == MeasurementStatus::Current
            && measurement.validUntil < observedAt) {
            return false;
        }
        if (measurement.status == MeasurementStatus::Stale
            && measurement.validUntil >= observedAt) {
            return false;
        }
        ids.insert(id);
    }
    return !measurements.isEmpty();
}

QByteArray encodeHomeostasisSnapshot(const HomeostasisSnapshot &snapshot)
{
    QCborMap root;
    root.insert(QStringLiteral("schemaVersion"), snapshot.schemaVersion);
    root.insert(QStringLiteral("snapshotId"), snapshot.snapshotId.toString(QUuid::WithoutBraces));
    root.insert(QStringLiteral("observedAt"), timestamp(snapshot.observedAt));
    root.insert(QStringLiteral("schedulingAuthorized"), snapshot.schedulingAuthorized);
    QCborArray measurements;
    for (const HomeostaticMeasurement &measurement : snapshot.measurements) {
        QCborMap item;
        item.insert(QStringLiteral("metricId"), measurement.metricId);
        item.insert(QStringLiteral("sourceId"), measurement.sourceId);
        item.insert(QStringLiteral("kind"), static_cast<qint64>(measurement.kind));
        item.insert(QStringLiteral("status"), static_cast<qint64>(measurement.status));
        item.insert(QStringLiteral("value"), measurement.value);
        item.insert(QStringLiteral("hasValue"), measurement.hasValue);
        item.insert(QStringLiteral("unit"), measurement.unit);
        item.insert(QStringLiteral("observedAt"), timestamp(measurement.observedAt));
        item.insert(QStringLiteral("validUntil"), timestamp(measurement.validUntil));
        item.insert(QStringLiteral("reason"), measurement.reason);
        measurements.append(item);
    }
    root.insert(QStringLiteral("measurements"), measurements);
    return root.toCborValue().toCbor();
}

HomeostasisSnapshot decodeHomeostasisSnapshot(const QByteArray &encoded, QString *error)
{
    if (error) {
        error->clear();
    }
    const QCborValue value = QCborValue::fromCbor(encoded);
    if (!value.isMap()) {
        setError(error, QStringLiteral("homeostasis payload is not a CBOR map"));
        return {};
    }
    const QCborMap root = value.toMap();
    for (const QString &field : {QStringLiteral("schemaVersion"), QStringLiteral("snapshotId"),
                                 QStringLiteral("observedAt"), QStringLiteral("schedulingAuthorized"),
                                 QStringLiteral("measurements")}) {
        if (!root.contains(field)) {
            setError(error, QStringLiteral("homeostasis snapshot missing field: ") + field);
            return {};
        }
    }
    if (!integerInRange(root.value(QStringLiteral("schemaVersion")), 1, 1)
        || !root.value(QStringLiteral("snapshotId")).isString()
        || !root.value(QStringLiteral("observedAt")).isString()
        || !root.value(QStringLiteral("schedulingAuthorized")).isBool()
        || !root.value(QStringLiteral("measurements")).isArray()) {
        setError(error, QStringLiteral("unsupported homeostasis schema or measurements"));
        return {};
    }

    HomeostasisSnapshot snapshot;
    snapshot.snapshotId = QUuid(root.value(QStringLiteral("snapshotId")).toString());
    snapshot.observedAt = parseTimestamp(root.value(QStringLiteral("observedAt")));
    snapshot.schedulingAuthorized = root.value(QStringLiteral("schedulingAuthorized")).toBool();
    for (const QCborValue &value : root.value(QStringLiteral("measurements")).toArray()) {
        if (!value.isMap()) {
            setError(error, QStringLiteral("homeostatic measurement is not a map"));
            return {};
        }
        const QCborMap item = value.toMap();
        for (const QString &field : {QStringLiteral("metricId"), QStringLiteral("sourceId"),
                                     QStringLiteral("kind"), QStringLiteral("status"),
                                     QStringLiteral("value"), QStringLiteral("hasValue"),
                                     QStringLiteral("unit"), QStringLiteral("observedAt"),
                                     QStringLiteral("validUntil"), QStringLiteral("reason")}) {
            if (!item.contains(field)) {
                setError(error, QStringLiteral("homeostatic measurement missing field: ") + field);
                return {};
            }
        }
        if (!integerInRange(item.value(QStringLiteral("kind")), 1, 4)
            || !integerInRange(item.value(QStringLiteral("status")), 1, 4)
            || !item.value(QStringLiteral("metricId")).isString()
            || !item.value(QStringLiteral("sourceId")).isString()
            || !(item.value(QStringLiteral("value")).isDouble()
                 || item.value(QStringLiteral("value")).isInteger())
            || !item.value(QStringLiteral("hasValue")).isBool()
            || !item.value(QStringLiteral("unit")).isString()
            || !item.value(QStringLiteral("observedAt")).isString()
            || !item.value(QStringLiteral("validUntil")).isString()
            || !item.value(QStringLiteral("reason")).isString()) {
            setError(error, QStringLiteral("unknown homeostatic measurement enum"));
            return {};
        }
        HomeostaticMeasurement measurement;
        measurement.metricId = item.value(QStringLiteral("metricId")).toString();
        measurement.sourceId = item.value(QStringLiteral("sourceId")).toString();
        measurement.kind = static_cast<MeasurementKind>(item.value(QStringLiteral("kind")).toInteger());
        measurement.status = static_cast<MeasurementStatus>(item.value(QStringLiteral("status")).toInteger());
        measurement.value = item.value(QStringLiteral("value")).toDouble();
        measurement.hasValue = item.value(QStringLiteral("hasValue")).toBool();
        measurement.unit = item.value(QStringLiteral("unit")).toString();
        measurement.observedAt = parseTimestamp(item.value(QStringLiteral("observedAt")));
        measurement.validUntil = parseTimestamp(item.value(QStringLiteral("validUntil")));
        measurement.reason = item.value(QStringLiteral("reason")).toString();
        snapshot.measurements.append(measurement);
    }
    if (!snapshot.isValid()) {
        setError(error, QStringLiteral("invalid homeostasis snapshot"));
        return {};
    }
    return snapshot;
}

} // namespace cybou
